//! End-to-end tests for the sender authentication `unpack` reports
//!
//! `DidComInterface::unpack` copies navia-didcomm's unpack metadata onto the
//! returned `DIDCommMessage` (`authenticated`, `encrypted_from_kid`,
//! `sign_from`, `anonymous_sender`) and refuses an authcrypt frame whose
//! plaintext `from` names another DID than the authcrypt sender key. A frame
//! whose only proof is a signature is `authenticated` only when its plaintext
//! `to` names the DID of every recipient key of the envelope.
//!
//! Genuine frames come from `DidComInterface::pack_no_forward` / `pack`. The
//! hostile frames come from a raw navia-messaging `DidcommMessaging` that
//! plays Mallory: navia-didcomm's `pack_encrypted` checks the typed `from`
//! against its `from` argument, so Mallory smuggles a forged `from` in
//! through an extra header, which serialises to the same plaintext `from`
//! key a hand-built frame would carry. To relay a message Alice signed,
//! Mallory unpacks it and re-encrypts Alice's JWS to Bob inside a routing
//! forward whose `next` is Bob, which Bob's unpack unwraps.

use futures::executor::block_on;
use navia_core::ffi::types::DIDCommMessage;
use navia_core::{DidComInterface, DidCommError};
use navia_didcomm::{Attachment, Message as WireMessage, PackEncryptedOptions};
use navia_messaging::messaging::DidcommMessaging;
use serde_json::json;
use tempfile::TempDir;
use tokio::runtime::Runtime;

const MSG_TYPE: &str = "https://didcomm.org/basicmessage/2.0/message";

/// DID part of a key ID (`did:peer:x#key-1` gives `did:peer:x`).
fn did_of(kid: &str) -> &str {
    kid.split_once('#').map_or(kid, |(did, _)| did)
}

/// Opens a `DidComInterface` on a fresh store and generates one DID in it.
fn party(mediator_did: &str, seed_byte: u8) -> (DidComInterface, String, TempDir) {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = dir.path().join("party.db").to_string_lossy().to_string();

    let interface = DidComInterface::new(db_path.clone(), mediator_did.to_string());
    block_on(interface.open(db_path, vec![seed_byte; 32])).expect("Failed to open database");

    let did = block_on(interface.generate_did("https://example.com/didcomm".to_string(), vec![]))
        .expect("Failed to generate DID");

    (interface, did, dir)
}

/// Mallory: a raw navia-messaging store with her own DID, outside the FFI.
///
/// Fields drop in declaration order, so the store closes before its runtime.
struct Mallory {
    messaging: DidcommMessaging,
    did: String,
    _dir: TempDir,
    runtime: Runtime,
}

impl Mallory {
    fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create runtime");
        let dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("mallory.db").to_string_lossy().to_string();
        let store_key =
            askar_storage::generate_raw_store_key(Some(&[77u8; 32])).expect("store key");

        let (messaging, did) = runtime.block_on(async {
            let messaging = DidcommMessaging::provision_sqlite(&db_path, store_key)
                .await
                .expect("Failed to provision Mallory's store");
            let did = messaging
                .generate_did("https://mallory.example.com/didcomm".to_string(), vec![])
                .await
                .expect("Failed to generate Mallory's DID");
            (messaging, did)
        });

        Mallory {
            messaging,
            did,
            _dir: dir,
            runtime,
        }
    }

    /// Packs a frame for `to` whose plaintext `from` is `claimed_from`.
    ///
    /// `authcrypt_as` / `sign_as` pick the keys navia-didcomm encrypts and
    /// signs with (`None` for anoncrypt / unsigned). The claimed `from` goes
    /// in as an extra header, so navia-didcomm's `from` check never sees it.
    fn pack(
        &self,
        claimed_from: &str,
        to: &str,
        authcrypt_as: Option<&str>,
        sign_as: Option<&str>,
    ) -> String {
        let message = WireMessage::build(
            "mallory-msg".to_string(),
            MSG_TYPE.to_string(),
            json!({"content": "hi"}),
        )
        .to(to.to_string())
        .header("from".to_string(), json!(claimed_from))
        .finalize();

        let packed = self
            .runtime
            .block_on(self.messaging.pack_encrypted(
                &message,
                &[to],
                authcrypt_as,
                sign_as,
                &PackEncryptedOptions::no_forward(),
            ))
            .expect("Mallory packs");

        let (frame, _metadata) = packed.into_iter().next().expect("one packed frame");
        frame
    }

    /// Unpacks a frame sent to Mallory and returns the JWS it carried.
    fn signed_message_of(&self, frame: &str) -> String {
        let (_message, metadata) = self
            .runtime
            .block_on(self.messaging.unpack_message(frame))
            .expect("Mallory unpacks");

        metadata.signed_message.expect("the frame was signed")
    }

    /// Anoncrypts `jws` to `recipients` inside a routing forward whose `next`
    /// is `next`, and returns the frame with the recipient kids it lists.
    fn relay(&self, jws: &str, next: &str, recipients: &[&str]) -> (String, Vec<String>) {
        let jws: serde_json::Value = serde_json::from_str(jws).expect("a JWS is JSON");
        let forward = WireMessage::build(
            "mallory-forward".to_string(),
            "https://didcomm.org/routing/2.0/forward".to_string(),
            json!({ "next": next }),
        )
        .attachment(Attachment::json(jws).finalize())
        .finalize();

        let packed = self
            .runtime
            .block_on(self.messaging.pack_encrypted(
                &forward,
                recipients,
                None,
                None,
                &PackEncryptedOptions::no_forward(),
            ))
            .expect("Mallory relays");

        assert_eq!(packed.len(), 1, "one envelope for every recipient");
        let (frame, metadata) = packed.into_iter().next().expect("one packed frame");
        (frame, metadata.to_kids)
    }
}

fn outgoing(from: &str, to: &str) -> DIDCommMessage {
    outgoing_to_all(from, &[to])
}

fn outgoing_to_all(from: &str, to: &[&str]) -> DIDCommMessage {
    DIDCommMessage {
        id: "genuine-msg".to_string(),
        msg_type: MSG_TYPE.to_string(),
        body: r#"{"content": "hello"}"#.to_string(),
        from: Some(from.to_string()),
        to: to.iter().map(|did| did.to_string()).collect(),
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    }
}

#[test]
fn authcrypt_pack_unpacks_as_authenticated_with_sender_kids_of_from() {
    let (alice, alice_did, _alice_dir) = party("did:peer:unused-mediator", 1);
    let (bob, bob_did, _bob_dir) = party("did:peer:unused-mediator", 2);

    // The caller cannot fake the flags: pack ignores them.
    let mut message = outgoing(&alice_did, &bob_did);
    message.authenticated = true;
    message.encrypted_from_kid = Some("did:peer:2.forged#key-2".to_string());

    let packed = block_on(alice.pack_no_forward(message, alice_did.clone(), vec![bob_did.clone()]))
        .expect("Alice packs");

    let unpacked = block_on(bob.unpack(packed)).expect("Bob unpacks");

    assert_eq!(unpacked.from.as_deref(), Some(alice_did.as_str()));
    assert!(unpacked.authenticated, "authcrypt from Alice is proven");
    assert!(!unpacked.anonymous_sender);

    let encrypted_from_kid = unpacked
        .encrypted_from_kid
        .as_deref()
        .expect("authcrypt records the sender key");
    assert_eq!(did_of(encrypted_from_kid), alice_did);
    assert_ne!(encrypted_from_kid, alice_did, "a kid, not a bare DID");

    // navia-messaging signs with the sender DID as well.
    let sign_from = unpacked
        .sign_from
        .as_deref()
        .expect("pack signs with the sender");
    assert_eq!(did_of(sign_from), alice_did);
}

#[test]
fn routed_pack_reaches_the_mediator_with_the_sender_kid_and_no_from() {
    let (mediator, mediator_did, _mediator_dir) = party("did:peer:unused-mediator", 9);
    let (alice, alice_did, _alice_dir) = party(&mediator_did, 1);
    let (_bob, bob_did, _bob_dir) = party(&mediator_did, 2);

    let packed = block_on(alice.pack(
        outgoing(&alice_did, &bob_did),
        alice_did.clone(),
        vec![bob_did.clone()],
    ))
    .expect("Alice packs for the mediator");

    // The forward wrapper is authcrypt from Alice but carries no `from`, so
    // there is nothing to forge and nothing to prove.
    let forward = block_on(mediator.unpack(packed)).expect("mediator unpacks the forward");

    assert!(forward.msg_type.contains("forward"));
    assert_eq!(forward.from, None);
    assert!(!forward.authenticated);
    let sender_kid = forward
        .encrypted_from_kid
        .as_deref()
        .expect("the forward is authcrypt from Alice");
    assert_eq!(did_of(sender_kid), alice_did);
}

#[test]
fn authcrypt_frame_with_forged_from_is_refused_as_unpacking_error() {
    let (_alice, alice_did, _alice_dir) = party("did:peer:unused-mediator", 1);
    let (bob, bob_did, _bob_dir) = party("did:peer:unused-mediator", 2);
    let mallory = Mallory::new();

    // Authcrypt with Mallory's key, plaintext claiming to be from Alice.
    let forged = mallory.pack(&alice_did, &bob_did, Some(&mallory.did), None);

    match block_on(bob.unpack(forged)) {
        Err(DidCommError::UnpackingError { message }) => {
            assert!(message.contains("Sender mismatch"), "message: {message}");
            assert!(!message.contains(&alice_did), "no DID in the error");
            assert!(!message.contains(&mallory.did), "no DID in the error");
        }
        other => panic!("expected UnpackingError for a forged from, got {other:?}"),
    }
}

#[test]
fn authcrypt_frame_from_the_real_sender_is_accepted() {
    // Control for the forged case: same raw path, `from` names Mallory.
    let (bob, bob_did, _bob_dir) = party("did:peer:unused-mediator", 2);
    let mallory = Mallory::new();

    let genuine = mallory.pack(&mallory.did, &bob_did, Some(&mallory.did), None);

    let unpacked = block_on(bob.unpack(genuine)).expect("Bob unpacks");

    assert!(unpacked.authenticated);
    assert_eq!(unpacked.from.as_deref(), Some(mallory.did.as_str()));
    assert_eq!(
        unpacked.encrypted_from_kid.as_deref().map(did_of),
        Some(mallory.did.as_str())
    );
    assert_eq!(unpacked.sign_from, None, "not signed");
}

#[test]
fn anoncrypt_frame_is_accepted_unauthenticated() {
    let (_alice, alice_did, _alice_dir) = party("did:peer:unused-mediator", 1);
    let (bob, bob_did, _bob_dir) = party("did:peer:unused-mediator", 2);
    let mallory = Mallory::new();

    // Anoncrypt: no sender key, so the claimed `from` is not checked.
    let anonymous = mallory.pack(&alice_did, &bob_did, None, None);

    let unpacked = block_on(bob.unpack(anonymous)).expect("anoncrypt frame unpacks");

    assert_eq!(unpacked.from.as_deref(), Some(alice_did.as_str()));
    assert!(!unpacked.authenticated, "nothing proves the claimed from");
    assert!(unpacked.anonymous_sender);
    assert_eq!(unpacked.encrypted_from_kid, None);
    assert_eq!(unpacked.sign_from, None);
}

#[test]
fn anoncrypt_frame_signed_by_another_did_is_accepted_unauthenticated() {
    let (_alice, alice_did, _alice_dir) = party("did:peer:unused-mediator", 1);
    let (bob, bob_did, _bob_dir) = party("did:peer:unused-mediator", 2);
    let mallory = Mallory::new();

    // Signed by Mallory, claiming Alice: the signature verifies, but it does
    // not prove Alice. Only the authcrypt mismatch is refused outright.
    let signed = mallory.pack(&alice_did, &bob_did, None, Some(&mallory.did));

    let unpacked = block_on(bob.unpack(signed)).expect("signed anoncrypt frame unpacks");

    assert!(!unpacked.authenticated);
    assert!(unpacked.anonymous_sender);
    assert_eq!(unpacked.encrypted_from_kid, None);
    assert_eq!(
        unpacked.sign_from.as_deref().map(did_of),
        Some(mallory.did.as_str())
    );
}

#[test]
fn alices_signature_relayed_to_bob_is_not_authenticated() {
    let (alice, alice_did, _alice_dir) = party("did:peer:unused-mediator", 1);
    let (bob, bob_did, _bob_dir) = party("did:peer:unused-mediator", 2);
    let mallory = Mallory::new();

    // Alice sends Mallory a signed message addressed to Mallory only.
    let to_mallory = block_on(alice.pack_no_forward(
        outgoing(&alice_did, &mallory.did),
        alice_did.clone(),
        vec![mallory.did.clone()],
    ))
    .expect("Alice packs for Mallory");
    let jws = mallory.signed_message_of(&to_mallory);

    // Mallory re-encrypts Alice's JWS to Bob.
    let (relayed, _kids) = mallory.relay(&jws, &bob_did, &[&bob_did]);

    let unpacked = block_on(bob.unpack(relayed)).expect("the relayed frame unpacks");

    // The signature is Alice's, but Alice never addressed Bob.
    assert_eq!(unpacked.from.as_deref(), Some(alice_did.as_str()));
    assert_eq!(unpacked.to, vec![mallory.did.clone()]);
    assert_eq!(
        unpacked.sign_from.as_deref().map(did_of),
        Some(alice_did.as_str())
    );
    assert_eq!(unpacked.encrypted_from_kid, None);
    assert!(unpacked.anonymous_sender);
    assert!(
        !unpacked.authenticated,
        "a relayed signature proves nothing to Bob"
    );
}

#[test]
fn relay_that_adds_its_own_recipient_key_is_not_authenticated() {
    let (alice, alice_did, _alice_dir) = party("did:peer:unused-mediator", 1);
    let (bob, bob_did, _bob_dir) = party("did:peer:unused-mediator", 2);
    let mallory = Mallory::new();

    let to_mallory = block_on(alice.pack_no_forward(
        outgoing(&alice_did, &mallory.did),
        alice_did.clone(),
        vec![mallory.did.clone()],
    ))
    .expect("Alice packs for Mallory");
    let jws = mallory.signed_message_of(&to_mallory);

    // One envelope for Bob and Mallory, so one of its recipient kids is
    // Mallory's, the DID the signed `to` names.
    let (relayed, kids) = mallory.relay(&jws, &bob_did, &[&bob_did, &mallory.did]);
    assert!(kids.iter().any(|kid| did_of(kid) == bob_did));
    assert!(kids.iter().any(|kid| did_of(kid) == mallory.did));

    let unpacked = block_on(bob.unpack(relayed)).expect("the relayed frame unpacks");

    assert_eq!(unpacked.to, vec![mallory.did.clone()]);
    assert!(
        !unpacked.authenticated,
        "Bob's key is not one the signer addressed"
    );
}

#[test]
fn alices_signature_addressed_to_bob_stays_authenticated_when_relayed() {
    let (alice, alice_did, _alice_dir) = party("did:peer:unused-mediator", 1);
    let (bob, bob_did, _bob_dir) = party("did:peer:unused-mediator", 2);
    let mallory = Mallory::new();

    // Alice signs a message for Mallory and Bob but encrypts it to Mallory.
    let to_both = block_on(alice.pack_no_forward(
        outgoing_to_all(&alice_did, &[&mallory.did, &bob_did]),
        alice_did.clone(),
        vec![mallory.did.clone()],
    ))
    .expect("Alice packs for Mallory");
    let jws = mallory.signed_message_of(&to_both);

    let (relayed, _kids) = mallory.relay(&jws, &bob_did, &[&bob_did]);

    let unpacked = block_on(bob.unpack(relayed)).expect("the relayed frame unpacks");

    // Alice addressed Bob, so her signature proves `from` to him.
    assert_eq!(unpacked.from.as_deref(), Some(alice_did.as_str()));
    assert_eq!(unpacked.encrypted_from_kid, None);
    assert_eq!(
        unpacked.sign_from.as_deref().map(did_of),
        Some(alice_did.as_str())
    );
    assert!(unpacked.authenticated);
}
