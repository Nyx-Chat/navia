//! Error scenario tests
//!
//! These tests verify that the library handles error conditions gracefully
//! and returns appropriate error messages.

use futures::executor::block_on;
use navia_core::ffi::types::DIDCommMessage;
use navia_core::{DidComInterface, DidCommError};
use navia_didcomm::{Attachment, Message as WireMessage, PackEncryptedOptions};
use navia_messaging::messaging::DidcommMessaging;
use serde_json::{json, Value};
use tempfile::TempDir;
use zeroize::Zeroize;

/// Creates a mediator interface and returns its DID
fn create_mediator() -> (DidComInterface, String, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("mediator.db");

    // Create mediator with a placeholder that will be replaced by generated DID
    let mediator_interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:placeholder".to_string(), // Mediator doesn't need routing to itself
    );
    let seed = vec![99u8; 32];
    block_on(mediator_interface.open(db_path.to_string_lossy().to_string(), seed))
        .expect("Failed to open mediator database");

    let mediator_did = block_on(
        mediator_interface.generate_did("https://mediator.example.com/didcomm".to_string(), vec![]),
    )
    .expect("Failed to generate mediator DID");

    (mediator_interface, mediator_did, temp_dir)
}

fn create_test_interface() -> (DidComInterface, TempDir, DidComInterface, TempDir) {
    // First create mediator to get its DID
    let (mediator_interface, mediator_did, mediator_temp_dir) = create_mediator();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(db_path.to_string_lossy().to_string(), mediator_did);
    let mut seed = vec![42u8; 32];
    block_on(interface.open(db_path.to_string_lossy().to_string(), seed.clone()))
        .expect("Failed to open database");

    // Verify seed is zeroized in our local copy
    seed.zeroize();

    (interface, temp_dir, mediator_interface, mediator_temp_dir)
}

#[test]
fn test_unpack_invalid_message() {
    let (interface, _temp_dir, _mediator, _mediator_dir) = create_test_interface();

    // Try to unpack various invalid messages
    let invalid_messages = vec![
        "not json at all",
        "{}", // Valid JSON but not a DIDComm message
        r#"{"invalid": "structure"}"#,
        r#"{"ciphertext": "invalid base64!@#$"}"#,
    ];

    for invalid_msg in invalid_messages {
        let result = block_on(interface.unpack(invalid_msg.to_string()));

        assert!(
            result.is_err(),
            "Expected error for invalid message: {invalid_msg}"
        );

        match result.unwrap_err() {
            DidCommError::UnpackingError { message } => {
                assert!(!message.is_empty(), "Error message should not be empty");
            }
            _ => panic!("Expected UnpackingError for invalid message"),
        }
    }
}

/// Opens a `DidComInterface` on a fresh store and generates one DID in it.
fn party(seed_byte: u8) -> (DidComInterface, String, TempDir) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = dir.path().join("party.db").to_string_lossy().to_string();

    let interface = DidComInterface::new(db_path.clone(), "did:peer:unused-mediator".to_string());
    block_on(interface.open(db_path, vec![seed_byte; 32])).expect("Failed to open database");

    let did = block_on(interface.generate_did("https://example.com/didcomm".to_string(), vec![]))
        .expect("Failed to generate DID");

    (interface, did, dir)
}

/// Packs a message from `from_did` to `to_did` without a forward: an authcrypt
/// frame whose protected header names the sender key in `skid` and `apu`.
fn authcrypt_frame(sender: &DidComInterface, from_did: &str, to_did: &str) -> String {
    let message = DIDCommMessage {
        id: "error-scenario".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: r#"{"content": "hello"}"#.to_string(),
        from: Some(from_did.to_string()),
        to: vec![to_did.to_string()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    block_on(sender.pack_no_forward(message, from_did.to_string(), vec![to_did.to_string()]))
        .expect("Failed to pack")
}

/// Unpacks `frame` and hands back the error, failing the test if the frame is
/// accepted.
///
/// Outside a Tokio runtime, `DidComInterface::unpack` spawns the unpack on the
/// interface's own runtime, and a panic in that task surfaces as
/// `DidCommError::GeneralError`.
fn unpack_error(interface: &DidComInterface, frame: &str) -> DidCommError {
    match block_on(interface.unpack(frame.to_string())) {
        Ok(message) => panic!("unpack accepted {frame:?}: {message:?}"),
        Err(err) => err,
    }
}

/// `unpack_error` from inside a Tokio runtime, where `DidComInterface::unpack`
/// awaits the unpack on the caller's runtime, so a panic in it unwinds into the
/// caller.
fn unpack_error_in_runtime(interface: &DidComInterface, frame: &str) -> DidCommError {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    match runtime.block_on(interface.unpack(frame.to_string())) {
        Ok(message) => panic!("unpack accepted {frame:?}: {message:?}"),
        Err(err) => err,
    }
}

/// What `unpack` is expected to throw: the FFI error and the start of its
/// message, which is the navia-core variant's own prefix followed by the
/// navia-messaging kind.
#[derive(Debug)]
enum Expected {
    Unpacking(&'static str),
    Database(&'static str),
}

/// `UnpackingError::MalformedMessage` over a navia-messaging `Malformed`.
const MALFORMED: Expected = Expected::Unpacking("Malformed message: Malformed: ");
/// `UnpackingError::DecryptionFailed` over a navia-messaging `Unsupported`.
const UNSUPPORTED: Expected =
    Expected::Unpacking("Decryption failed: Unsupported crypto or method: ");
/// `StorageError::OperationFailed` over a navia-messaging `DIDNotResolved`.
const DID_NOT_RESOLVED: Expected =
    Expected::Database("Storage operation failed: unpack - DID not resolved: ");
/// `StorageError::OperationFailed` over a navia-messaging `DIDUrlNotFound`.
const DID_URL_NOT_FOUND: Expected =
    Expected::Database("Storage operation failed: unpack - DID URL not found: ");

fn assert_unpack_error(err: &DidCommError, expected: &Expected, case: &str) {
    let (message, prefix) = match (err, expected) {
        (DidCommError::UnpackingError { message }, Expected::Unpacking(prefix))
        | (DidCommError::DatabaseError { message }, Expected::Database(prefix)) => {
            (message, prefix)
        }
        (other, expected) => panic!("{case}: expected {expected:?}, got {other:?}"),
    };
    assert!(message.starts_with(prefix), "{case}: {message}");
}

/// Asserts that unpacking `frame` fails as `expected` both outside and inside a
/// Tokio runtime, the two ways `DidComInterface::unpack` runs the unpack.
fn assert_unpack_error_on_both_paths(
    interface: &DidComInterface,
    frame: &str,
    expected: &Expected,
    case: &str,
) {
    assert_unpack_error(&unpack_error(interface, frame), expected, case);
    assert_unpack_error(
        &unpack_error_in_runtime(interface, frame),
        expected,
        &format!("{case} (in a Tokio runtime)"),
    );
}

const BASE64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Unpadded base64url, the encoding of the JWE fields.
fn base64url_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let bits = chunk.iter().enumerate().fold(0u32, |bits, (i, &byte)| {
            bits | (u32::from(byte) << (16 - 8 * i))
        });
        // n bytes take n + 1 characters.
        for i in 0..=chunk.len() {
            out.push(char::from(
                BASE64URL[((bits >> (18 - 6 * i)) & 63) as usize],
            ));
        }
    }
    out
}

/// The inverse of `base64url_encode`.
fn base64url_decode(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut bits, mut pending) = (0u32, 0u32);
    for c in text.bytes() {
        let value = BASE64URL
            .iter()
            .position(|&b| b == c)
            .expect("a base64url character");
        bits = (bits << 6) | value as u32;
        pending += 6;
        if pending >= 8 {
            pending -= 8;
            out.push((bits >> pending) as u8);
            bits &= (1 << pending) - 1;
        }
    }
    out
}

/// base58btc, the encoding behind the multibase `z` prefix of a did:key.
fn base58btc_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    // Base-58 digits, least significant first.
    let mut digits: Vec<u32> = Vec::new();
    for &byte in bytes {
        let mut carry = u32::from(byte);
        for digit in &mut digits {
            carry += *digit << 8;
            *digit = carry % 58;
            carry /= 58;
        }
        while carry > 0 {
            digits.push(carry % 58);
            carry /= 58;
        }
    }

    let mut out = "1".repeat(bytes.iter().take_while(|&&byte| byte == 0).count());
    out.extend(
        digits
            .iter()
            .rev()
            .map(|&digit| char::from(ALPHABET[digit as usize])),
    );
    out
}

/// The key id of a did:key over `key`, behind the multicodec prefix `codec`.
fn did_key_kid(codec: &[u8], key: &[u8]) -> String {
    let multibase = format!("z{}", base58btc_encode(&[codec, key].concat()));
    format!("did:key:{multibase}#{multibase}")
}

/// Sets the top-level `field` of a JSON frame to `value`.
fn with_field(frame: &str, field: &str, value: Value) -> String {
    let mut frame: Value = serde_json::from_str(frame).expect("the frame is JSON");
    frame[field] = value;
    frame.to_string()
}

/// Decodes the protected header of a JSON frame, lets `edit` change it and
/// encodes it back. The frame is not re-encrypted.
fn with_protected_header(frame: &str, edit: impl FnOnce(&mut Value)) -> String {
    let parsed: Value = serde_json::from_str(frame).expect("the frame is JSON");
    let protected = parsed["protected"]
        .as_str()
        .expect("the frame has a protected header");
    let mut header: Value =
        serde_json::from_slice(&base64url_decode(protected)).expect("the protected header is JSON");
    edit(&mut header);
    with_field(
        frame,
        "protected",
        base64url_encode(header.to_string().as_bytes()).into(),
    )
}

/// Names `kid` as the sender key of an authcrypt frame. The frame is not
/// re-encrypted.
fn with_sender_kid(frame: &str, kid: &str) -> String {
    // skid and apu both name the sender key and must agree.
    with_protected_header(frame, |header| {
        header["skid"] = kid.into();
        header["apu"] = base64url_encode(kid.as_bytes()).into();
    })
}

/// An empty or truncated frame fails the same way on every redelivery, so it
/// surfaces as `UnpackingError` (`UnpackingError::MalformedMessage`), which a
/// consumer acknowledges. navia-didcomm 1.3.0 mapped serde_json's `Eof` to
/// `InvalidState`, which surfaced as `DatabaseError`; 1.3.1 reports `Malformed`.
#[test]
fn test_unpack_truncated_frame_is_malformed() {
    let (alice, alice_did, _alice_dir) = party(1);
    let (bob, bob_did, _bob_dir) = party(2);
    let frame = authcrypt_frame(&alice, &alice_did, &bob_did);
    assert!(frame.is_ascii());

    let cases = [
        ("an empty frame", String::new()),
        (
            "an unterminated object",
            r#"{"ciphertext": "abc""#.to_string(),
        ),
        ("a key without a value", r#"{"protected":"#.to_string()),
        ("half of a frame", frame[..frame.len() / 2].to_string()),
        (
            "a cut protected header",
            with_field(
                &frame,
                "protected",
                base64url_encode(br#"{"alg":"ECDH-1PU+A256KW","#).into(),
            ),
        ),
    ];

    for (case, frame) in cases {
        assert_unpack_error(&unpack_error(&bob, &frame), &MALFORMED, case);
    }
}

/// Faults of the envelope a sender controls surface as `UnpackingError`
/// (`UnpackingError::MalformedMessage`). navia-didcomm 1.3.0 gave them
/// `InvalidState`, which surfaced as `DatabaseError`.
#[test]
fn test_unpack_tampered_envelope_is_malformed() {
    let (alice, alice_did, _alice_dir) = party(1);
    let (bob, bob_did, _bob_dir) = party(2);
    let frame = authcrypt_frame(&alice, &alice_did, &bob_did);

    let cases = [
        (
            // An anoncrypt (ECDH-ES) envelope has no sender key to name in apu.
            "apu in an anoncrypt envelope",
            with_protected_header(&frame, |header| {
                assert_eq!(header["alg"], "ECDH-1PU+A256KW");
                header["alg"] = "ECDH-ES+A256KW".into();
            }),
        ),
        (
            // ECDH-1PU derives the key over the tag, which fits only up to 124 bytes.
            "an authcrypt tag of 125 bytes",
            with_field(&frame, "tag", base64url_encode(&[0u8; 125]).into()),
        ),
    ];

    for (case, frame) in cases {
        assert_unpack_error(&unpack_error(&bob, &frame), &MALFORMED, case);
    }
}

/// The authcrypt sender key is resolved before anything is decrypted.
/// navia-didcomm 1.3.1 keeps the DID resolver's kind when that resolution
/// fails, where 1.3.0 reported `InvalidState` (so `DatabaseError`).
/// navia-messaging's resolver resolves only did:peer and did:key and refuses
/// any other method as `Unsupported` without looking the DID up, so such a
/// sender surfaces as `UnpackingError::DecryptionFailed`. A sender DID document
/// the resolver cannot map surfaces as `UnpackingError` too: an unsupported key
/// type as `UnpackingError::DecryptionFailed`, a key that does not decode as
/// `UnpackingError::MalformedMessage`. A did:peer or did:key sender DID that
/// does not resolve surfaces as `DatabaseError` over `DIDNotResolved`, a
/// malformed one included unless it panics its method's resolver, and so does a
/// sender key its DID document does not list (`DIDUrlNotFound`); a consumer
/// redelivers those with a cap per frame.
#[test]
fn test_unpack_failed_sender_resolution_keeps_its_kind() {
    let (alice, alice_did, _alice_dir) = party(1);
    let (bob, bob_did, _bob_dir) = party(2);
    let frame = authcrypt_frame(&alice, &alice_did, &bob_did);

    // did-peer implements only numalgo 0 and 2, so a numalgo 4 DID does not
    // resolve. A numalgo 2 DID with an unknown purpose code (X) or with key
    // material that is not base58, and a did:key that is not base58, do not
    // resolve either: their resolvers return an error for them without
    // panicking. did:key checks only the key length, so both did:key documents
    // built here resolve, and the resolver then refuses them: a secp256k1 key
    // has no type here, and the P-256 bytes are not a valid key.
    let cases = [
        ("did:unknown:nobody#key-1".to_string(), UNSUPPORTED),
        ("did:peer:4nobody#key-1".to_string(), DID_NOT_RESOLVED),
        ("did:peer:2.Xz6Mk#key-1".to_string(), DID_NOT_RESOLVED),
        ("did:peer:2.Vz0OIl#key-1".to_string(), DID_NOT_RESOLVED),
        ("did:key:z0OIl#key-1".to_string(), DID_NOT_RESOLVED),
        (format!("{alice_did}#key-9"), DID_URL_NOT_FOUND),
        (did_key_kid(&[0xe7, 0x01], &[0x02; 33]), UNSUPPORTED),
        (did_key_kid(&[0x80, 0x24], &[0xff; 33]), MALFORMED),
    ];

    for (kid, expected) in cases {
        let tampered = with_sender_kid(&frame, &kid);
        assert_unpack_error(&unpack_error(&bob, &tampered), &expected, &kid);
    }
}

/// The sender DID comes from the unauthenticated `skid` and `apu`, so whoever
/// gets a frame to the recipient picks the DID the resolver is handed. A
/// did:web DID the DID cache client cannot parse is refused by method before
/// the cache client sees it: `UnpackingError::DecryptionFailed` over
/// `Unsupported`, which a consumer acknowledges, and never `GeneralError`, which
/// is what a panic inside the unpack would surface as.
#[test]
fn test_unpack_unparsable_did_web_sender_is_unsupported() {
    let (alice, alice_did, _alice_dir) = party(1);
    let (bob, bob_did, _bob_dir) = party(2);
    let frame = authcrypt_frame(&alice, &alice_did, &bob_did);

    let kid = "did:web:a!b#key-1";
    assert_unpack_error_on_both_paths(&bob, &with_sender_kid(&frame, kid), &UNSUPPORTED, kid);
}

/// A well-formed did:web sender DID is refused the same way, before anything
/// looks it up, so no DID document is fetched from the host the frame names.
#[test]
fn test_unpack_did_web_sender_is_refused_without_a_lookup() {
    let (alice, alice_did, _alice_dir) = party(1);
    let (bob, bob_did, _bob_dir) = party(2);
    let frame = authcrypt_frame(&alice, &alice_did, &bob_did);

    let kid = "did:web:example.com#key-1";
    assert_unpack_error_on_both_paths(&bob, &with_sender_kid(&frame, kid), &UNSUPPORTED, kid);
}

/// A did:peer sender DID cut off after its numalgo makes the did-peer resolver
/// slice past its end. navia-messaging's resolver contains that panic and
/// reports `Malformed`, so it surfaces as `UnpackingError::MalformedMessage`,
/// never as `GeneralError`.
#[test]
fn test_unpack_cut_off_did_peer_sender_is_malformed() {
    let (alice, alice_did, _alice_dir) = party(1);
    let (bob, bob_did, _bob_dir) = party(2);
    let frame = authcrypt_frame(&alice, &alice_did, &bob_did);

    let kid = "did:peer:2#key-1";
    assert_unpack_error_on_both_paths(&bob, &with_sender_kid(&frame, kid), &MALFORMED, kid);
}

/// A JSON JWS with one signature that names `kid` as the signer key. Nothing is
/// signed: the signature is a placeholder.
fn jws_signed_by(kid: &str) -> String {
    json!({
        "payload": base64url_encode(b"{}"),
        "signatures": [{
            "protected": base64url_encode(
                br#"{"typ":"application/didcomm-signed+json","alg":"EdDSA"}"#,
            ),
            "signature": "AA",
            "header": { "kid": kid },
        }],
    })
    .to_string()
}

/// The signer kid of a JWS sits in its unprotected header, and the signer DID
/// is resolved before the signature is verified, so whoever gets a JWS to the
/// recipient picks the DID the resolver is handed, as with an authcrypt sender.
/// navia-didcomm keeps the resolver's kind when the signer DID does not
/// resolve, so a did:web signer DID is refused by method without a lookup:
/// `UnpackingError::DecryptionFailed` over `Unsupported`, never `GeneralError`.
#[test]
fn test_unpack_did_web_signer_is_unsupported() {
    let (bob, _bob_did, _bob_dir) = party(2);

    for kid in ["did:web:a!b#key-1", "did:web:example.com#key-1"] {
        assert_unpack_error_on_both_paths(&bob, &jws_signed_by(kid), &UNSUPPORTED, kid);
    }
}

/// Anoncrypts a routing forward whose `next` is `next` to `to`. The FFI's
/// `DIDCommMessage` carries no attachments and a forward needs one, so a raw
/// navia-messaging store outside the FFI packs it.
fn anoncrypt_forward(next: &str, to: &str) -> String {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = dir.path().join("relay.db").to_string_lossy().to_string();
    let store_key = askar_storage::generate_raw_store_key(Some(&[77u8; 32])).expect("store key");

    let forward = WireMessage::build(
        "error-scenario-forward".to_string(),
        "https://didcomm.org/routing/2.0/forward".to_string(),
        json!({ "next": next }),
    )
    .attachment(Attachment::json(json!({})).finalize())
    .finalize();

    runtime.block_on(async {
        let messaging = DidcommMessaging::provision_sqlite(&db_path, store_key)
            .await
            .expect("Failed to provision the relay's store");
        let packed = messaging
            .pack_encrypted(
                &forward,
                &[to],
                None,
                None,
                &PackEncryptedOptions::no_forward(),
            )
            .await
            .expect("Failed to pack the forward");
        let (frame, _metadata) = packed.into_iter().next().expect("one packed frame");
        frame
    })
}

/// Unpack unwraps a routing forward inside an anoncrypt layer when the
/// recipient holds a key-agreement secret for its `next`, and a bare-DID `next`
/// is resolved for that, before anything is authenticated: an anoncrypt layer
/// has no sender. navia-didcomm keeps the resolver's kind there too, so a
/// did:web `next` is refused by method without a lookup:
/// `UnpackingError::DecryptionFailed` over `Unsupported`, never `GeneralError`.
#[test]
fn test_unpack_forward_to_did_web_next_is_unsupported() {
    let (bob, bob_did, _bob_dir) = party(2);

    for next in ["did:web:a!b", "did:web:example.com"] {
        let frame = anoncrypt_forward(next, &bob_did);
        assert_unpack_error_on_both_paths(&bob, &frame, &UNSUPPORTED, next);
    }
}

/// A bare plaintext frame whose `from_prior` JWT names `kid` as the issuer key.
/// Nothing is signed: the JWT signature is a placeholder. Packing verifies
/// `from_prior`, so the message is serialized rather than packed.
fn plaintext_with_from_prior(kid: &str) -> String {
    let header = json!({ "typ": "JWT", "alg": "EdDSA", "kid": kid }).to_string();
    let jwt = format!(
        "{}.{}.AA",
        base64url_encode(header.as_bytes()),
        base64url_encode(b"{}")
    );

    let message = WireMessage::build(
        "error-scenario-from-prior".to_string(),
        "https://example.org/protocols/1.0/message".to_string(),
        json!({}),
    )
    .from_prior(jwt)
    .finalize();
    serde_json::to_string(&message).expect("the message serializes")
}

/// The issuer kid of a plaintext's `from_prior` JWT sits in the JWT header, and
/// the issuer DID is resolved before the JWT signature is verified, so in a
/// bare or anoncrypted plaintext it is resolved before anything is
/// authenticated, and whoever gets the frame to the recipient picks the DID the
/// resolver is handed. navia-didcomm keeps the resolver's kind there too, so a
/// did:web issuer DID is refused by method without a lookup:
/// `UnpackingError::DecryptionFailed` over `Unsupported`, never `GeneralError`.
#[test]
fn test_unpack_did_web_from_prior_issuer_is_unsupported() {
    let (bob, _bob_did, _bob_dir) = party(2);

    for kid in ["did:web:a!b#key-1", "did:web:example.com#key-1"] {
        assert_unpack_error_on_both_paths(&bob, &plaintext_with_from_prior(kid), &UNSUPPORTED, kid);
    }
}

/// A JWS signer, forward `next` or `from_prior` issuer DID goes through the
/// same resolver as an authcrypt sender DID and fails with the same kind. A
/// did:peer DID cut off after its numalgo panics the did-peer resolver, which
/// navia-messaging's resolver reports as `Malformed`
/// (`UnpackingError::MalformedMessage`, never `GeneralError`). A did:peer DID
/// of a numalgo did-peer does not implement does not resolve, which surfaces
/// as `DatabaseError` over `DIDNotResolved` and fails the same way on every
/// redelivery.
#[test]
fn test_unpack_failed_did_peer_signer_next_and_issuer_keep_their_kind() {
    let (bob, bob_did, _bob_dir) = party(2);

    let cases = [
        ("did:peer:2", MALFORMED),
        ("did:peer:4nobody", DID_NOT_RESOLVED),
    ];

    for (did, expected) in cases {
        let kid = format!("{did}#key-1");
        let frames = [
            ("signer", jws_signed_by(&kid)),
            ("forward next", anoncrypt_forward(did, &bob_did)),
            ("from_prior issuer", plaintext_with_from_prior(&kid)),
        ];
        for (role, frame) in frames {
            assert_unpack_error_on_both_paths(&bob, &frame, &expected, &format!("{role} {did}"));
        }
    }
}

#[test]
fn test_pack_without_recipient_keys() {
    let (interface, _temp_dir, _mediator, _mediator_dir) = create_test_interface();

    // Generate a DID for sender
    let sender_did =
        block_on(interface.generate_did("https://sender.example.com/didcomm".to_string(), vec![]))
            .expect("Failed to generate sender DID");

    // Use a non-existent recipient DID
    let fake_recipient = "did:peer:nonexistent123";

    let message = DIDCommMessage {
        id: "test-123".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: "Hello".to_string(),
        from: Some(sender_did.clone()),
        to: vec![fake_recipient.to_string()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // This should fail because we can't resolve the recipient's keys
    let result = block_on(interface.pack(message, sender_did, vec![fake_recipient.to_string()]));

    assert!(result.is_err());
    match result.unwrap_err() {
        DidCommError::PackingError { message } => {
            assert!(message.contains("resolve") || message.contains("DID"));
        }
        _ => panic!("Expected PackingError for unresolvable recipient"),
    }
}

#[test]
fn test_unpack_message_for_wrong_recipient() {
    // Create a shared mediator for all users
    let (mediator_interface, mediator_did, _mediator_dir) = create_mediator();

    // Create user interfaces that all use the same mediator
    let alice_dir = TempDir::new().expect("Failed to create temp dir");
    let alice_db_path = alice_dir.path().join("alice.db");
    let alice_interface = DidComInterface::new(
        alice_db_path.to_string_lossy().to_string(),
        mediator_did.clone(),
    );
    block_on(alice_interface.open(alice_db_path.to_string_lossy().to_string(), vec![1u8; 32]))
        .expect("Failed to open Alice's database");

    let bob_dir = TempDir::new().expect("Failed to create temp dir");
    let bob_db_path = bob_dir.path().join("bob.db");
    let bob_interface = DidComInterface::new(
        bob_db_path.to_string_lossy().to_string(),
        mediator_did.clone(),
    );
    block_on(bob_interface.open(bob_db_path.to_string_lossy().to_string(), vec![2u8; 32]))
        .expect("Failed to open Bob's database");

    let charlie_dir = TempDir::new().expect("Failed to create temp dir");
    let charlie_db_path = charlie_dir.path().join("charlie.db");
    let charlie_interface = DidComInterface::new(
        charlie_db_path.to_string_lossy().to_string(),
        mediator_did.clone(),
    );
    block_on(charlie_interface.open(charlie_db_path.to_string_lossy().to_string(), vec![3u8; 32]))
        .expect("Failed to open Charlie's database");

    // Generate DIDs for users (with mediator's DID as routing key)
    let alice_did = block_on(alice_interface.generate_did(
        "https://alice.example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate Alice's DID");

    let bob_did = block_on(bob_interface.generate_did(
        "https://bob.example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate Bob's DID");

    let _charlie_did = block_on(charlie_interface.generate_did(
        "https://charlie.example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate Charlie's DID");

    // Create a message from Alice to Bob
    let message = DIDCommMessage {
        id: "private-message".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: r#"{"content": "Secret message for Bob only"}"#.to_string(),
        from: Some(alice_did.clone()),
        to: vec![bob_did.clone()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // Alice packs the message for Bob (wrapped for mediator delivery)
    let packed_for_mediator =
        block_on(alice_interface.pack(message, alice_did.clone(), vec![bob_did.clone()]))
            .expect("Failed to pack message");

    // Step 1: Mediator receives and unpacks the forward message
    let forward_msg = block_on(mediator_interface.unpack(packed_for_mediator.clone()))
        .expect("Mediator should be able to unpack forward message");

    // The forward message should be of type routing/2.0/forward
    assert!(
        forward_msg.msg_type.contains("forward"),
        "Expected forward message type, got: {}",
        forward_msg.msg_type
    );

    // Step 2: Extract the inner message from forward body/attachments
    // The inner encrypted message for Bob is in the attachments
    let forward_body: serde_json::Value =
        serde_json::from_str(&forward_msg.body).expect("Forward body should be valid JSON");

    // Get the "next" field which contains the recipient's DID
    let next_did = forward_body["next"]
        .as_str()
        .expect("Forward message should have 'next' field");
    // Verify the next field contains Bob's DID
    assert_eq!(next_did, bob_did, "Next should be Bob's DID");

    // The actual encrypted message for Bob should be in attachments
    // For this test, we'll verify the flow works by checking the mediator received the forward
    // In a real implementation, the mediator would extract the attachment and forward it

    // Charlie tries to unpack the original packed message (should fail - it's encrypted for mediator)
    let result = block_on(charlie_interface.unpack(packed_for_mediator.clone()));
    assert!(
        matches!(result, Err(DidCommError::UnpackingError { .. })),
        "A frame for someone else's keys can never be unpacked, so it surfaces as \
         UnpackingError, got {result:?}"
    );

    // Keep temp directories alive
    let _ = (alice_dir, bob_dir, charlie_dir);
}

#[test]
fn test_database_errors() {
    // Test opening database with invalid seed
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:test-mediator".to_string(),
    );

    // Try with empty seed - should fail with validation error
    let result = block_on(interface.open(db_path.to_string_lossy().to_string(), vec![]));

    assert!(result.is_err());
    match result.unwrap_err() {
        DidCommError::ValidationError { message } => {
            assert!(message.contains("empty"));
        }
        _ => panic!("Expected ValidationError for empty seed"),
    }

    // Test with wrong seed length - should fail with validation error
    let interface2 = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:test-mediator".to_string(),
    );
    let result = block_on(interface2.open(
        db_path.to_string_lossy().to_string(),
        vec![1, 2, 3], // Too short
    ));

    assert!(result.is_err());
    match result.unwrap_err() {
        DidCommError::ValidationError { message } => {
            assert!(message.contains("at least 32 bytes"));
        }
        _ => panic!("Expected ValidationError for short seed"),
    }
}

#[test]
fn test_storage_error_handling() {
    let (interface, _temp_dir, _mediator, _mediator_dir) = create_test_interface();

    // Test getting non-existent value (should return empty string, not error)
    let result = block_on(interface.get(
        "non_existent_category".to_string(),
        "non_existent_key".to_string(),
    ))
    .expect("Get should not fail for non-existent keys");

    assert_eq!(result, "");

    // Test removing non-existent value
    // Note: Some storage implementations may return an error for non-existent keys
    let remove_result = block_on(interface.remove(
        "non_existent_category".to_string(),
        "non_existent_key".to_string(),
    ));

    // It's acceptable for remove to either succeed or fail with a specific error
    if let Err(err) = remove_result {
        match err {
            DidCommError::DatabaseError { message } => {
                assert!(message.contains("not found") || message.contains("Entry"));
            }
            _ => panic!("Unexpected error type for non-existent key removal"),
        }
    }
}

#[test]
fn test_malformed_json_body_handling() {
    // Create mediator
    let (mediator_interface, mediator_did, _mediator_dir) = create_mediator();

    // Create user interface with the mediator
    let user_dir = TempDir::new().expect("Failed to create temp dir");
    let user_db_path = user_dir.path().join("user.db");
    let user_interface = DidComInterface::new(
        user_db_path.to_string_lossy().to_string(),
        mediator_did.clone(),
    );
    block_on(user_interface.open(user_db_path.to_string_lossy().to_string(), vec![42u8; 32]))
        .expect("Failed to open user database");

    let user_did = block_on(user_interface.generate_did(
        "https://example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate DID");

    // Test with malformed JSON that starts with { but isn't valid
    let malformed_json = r#"{"broken": "json", incomplete"#;

    let message = DIDCommMessage {
        id: "malformed-msg".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: malformed_json.to_string(),
        from: Some(user_did.clone()),
        to: vec![user_did.clone()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // Should still pack successfully (body is treated as string)
    // This message goes to self via mediator
    let packed_for_mediator =
        block_on(user_interface.pack(message, user_did.clone(), vec![user_did.clone()]))
            .expect("Should pack even with malformed JSON body");

    // Mediator unpacks the forward message
    let forward_msg = block_on(mediator_interface.unpack(packed_for_mediator))
        .expect("Mediator should unpack forward");

    // Verify it's a forward message with the malformed body preserved in the inner message
    assert!(
        forward_msg.msg_type.contains("forward"),
        "Expected forward message"
    );

    // Keep temp directory alive
    let _ = user_dir;
}

#[test]
fn test_sequential_did_generation() {
    let (interface, _temp_dir, _mediator, _mediator_dir) = create_test_interface();

    // Generate multiple DIDs sequentially (since DidComInterface doesn't implement Clone)
    let mut dids = vec![];

    for i in 0..5 {
        let did =
            block_on(interface.generate_did(format!("https://example{i}.com/didcomm"), vec![]))
                .expect("Failed to generate DID");
        dids.push(did);
    }

    // Verify all DIDs are unique
    let unique_dids: std::collections::HashSet<_> = dids.iter().collect();
    assert_eq!(
        unique_dids.len(),
        dids.len(),
        "Generated DIDs should be unique"
    );

    // Verify all DIDs start with the correct prefix
    for did in &dids {
        assert!(
            did.starts_with("did:peer:"),
            "DID should start with did:peer:"
        );
    }
}
