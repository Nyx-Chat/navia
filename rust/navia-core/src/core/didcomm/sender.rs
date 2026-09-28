//! Sender authentication for unpacked DIDComm frames
//!
//! navia-didcomm reports which sender keys authenticated a frame
//! (`UnpackMetadata::encrypted_from_kid` for authcrypt, `sign_from` for a
//! verified signature) but never compares them with the plaintext `from`, nor
//! the plaintext `to` with the keys the frame was encrypted to. A sender that
//! holds its own key can therefore authcrypt or sign a plaintext whose `from`
//! names somebody else, and anyone who obtains a signed message can
//! re-encrypt it to a recipient its signer never addressed. This module binds
//! the plaintext `from` to those keys:
//!
//! - an authcrypt frame whose `from` names another DID than
//!   `encrypted_from_kid` is refused with `UnpackingError::SenderMismatch`;
//! - `authenticated` is `true` only when `from` is set, navia-didcomm
//!   authenticated the frame, every sender key it used belongs to `from`, and
//!   the frame is tied to this recipient: by authcrypt, or, when a signature
//!   is the only proof, by an encrypted envelope whose every recipient key
//!   belongs to a DID the signed `to` names.
//!
//! Unsigned anoncrypt frames and plaintext frames carry no sender key, so
//! they pass through with `authenticated = false`. A frame signed by a key of
//! another DID and not authcrypt-encrypted, and a signed frame relayed to a
//! recipient its `to` does not name, also pass through with
//! `authenticated = false` rather than being refused: only the authcrypt
//! mismatch is a hard error.

use crate::error::{NaviaResult, UnpackingError};
use navia_didcomm::UnpackMetadata;

/// How the plaintext `from` of an unpacked frame relates to the sender keys
/// navia-didcomm authenticated it with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenderAuthentication {
    /// `true` when the plaintext `from` is proven by the sender key(s) and the
    /// frame is tied to this recipient
    pub authenticated: bool,
    /// Key ID of the authcrypt sender key, if the frame was authcrypt-encrypted
    pub encrypted_from_kid: Option<String>,
    /// Key ID of the verified signature, if the frame was signed
    pub sign_from: Option<String>,
    /// `true` when the frame arrived in an anoncrypt envelope
    pub anonymous_sender: bool,
}

/// Returns the DID part of a key ID (`did:peer:x#key-1` gives `did:peer:x`).
///
/// This is the split navia-didcomm applies when it resolves a sender kid, so
/// the result is the DID whose document vouched for the key. A value without
/// a fragment comes back unchanged.
pub fn did_of_kid(kid: &str) -> &str {
    kid.split_once('#').map_or(kid, |(did, _)| did)
}

/// `true` when the frame was encrypted and every recipient key of its
/// envelope belongs to a DID the plaintext `to` names.
///
/// This is what ties a frame whose only proof is a signature to this
/// recipient: the key this wallet decrypted with is one of
/// `encrypted_to_kids`, so it belongs to a DID the signer addressed.
/// navia-didcomm lists every recipient key of the envelope there, not only
/// the ones this wallet holds, so "at least one key" would let a relay add a
/// key of its own DID and pass; every key has to match.
fn encrypted_to_addressees(to: &[String], metadata: &UnpackMetadata) -> bool {
    metadata.encrypted
        && metadata.encrypted_to_kids.as_deref().is_some_and(|kids| {
            !kids.is_empty()
                && kids
                    .iter()
                    .all(|kid| to.iter().any(|did| did == did_of_kid(kid)))
        })
}

/// Binds the plaintext `from` of an unpacked frame to the sender keys in its
/// unpack metadata, and the frame to this recipient.
///
/// `to` is the plaintext `to` of the frame (empty when it has none).
///
/// Authcrypt ties the frame to its recipients by construction: only the
/// holder of the sender's key-agreement secret can encrypt from that key to
/// this wallet's key. A signature does not: it stays valid after anyone
/// re-encrypts the signed message, so when a signature is the only proof
/// (`encrypted_from_kid` is `None`), `authenticated` also needs the
/// envelope's recipient keys to belong to DIDs in `to`. A signed frame
/// relayed to a recipient its `to` does not name, or one that was never
/// encrypted, comes back `authenticated = false`.
///
/// # Errors
///
/// `UnpackingError::SenderMismatch` when the frame was authcrypt-encrypted
/// (`encrypted_from_kid` is set), `from` is set, and `from` is not the DID
/// part of `encrypted_from_kid`: a forged `from`. A redelivery fails the same
/// way, so the error is permanent.
pub fn authenticate_sender(
    from: Option<&str>,
    to: &[String],
    metadata: &UnpackMetadata,
) -> NaviaResult<SenderAuthentication> {
    let encrypted_from_did = metadata.encrypted_from_kid.as_deref().map(did_of_kid);

    if let (Some(from), Some(sender_did)) = (from, encrypted_from_did) {
        if from != sender_did {
            return Err(UnpackingError::SenderMismatch.into());
        }
    }

    // Every sender key navia-didcomm authenticated with must belong to `from`,
    // or the frame is not proof of `from`. The authcrypt kid is compared again
    // here so `authenticated` never depends on the early return above.
    let authenticated = from.is_some_and(|from| {
        let encrypter_is_from = encrypted_from_did.is_none_or(|did| did == from);
        let signer_is_from = metadata
            .sign_from
            .as_deref()
            .is_none_or(|kid| did_of_kid(kid) == from);
        // Authcrypt binds sender and recipient keys; a signature alone binds
        // only the sender, so the envelope must be addressed to `to`.
        let bound_to_recipient = if metadata.encrypted_from_kid.is_some() {
            true
        } else if metadata.sign_from.is_some() {
            encrypted_to_addressees(to, metadata)
        } else {
            false
        };

        metadata.authenticated && bound_to_recipient && encrypter_is_from && signer_is_from
    });

    Ok(SenderAuthentication {
        authenticated,
        encrypted_from_kid: metadata.encrypted_from_kid.clone(),
        sign_from: metadata.sign_from.clone(),
        anonymous_sender: metadata.anonymous_sender,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::NaviaError;

    const ALICE: &str = "did:peer:2.alice";
    const ALICE_ENC_KID: &str = "did:peer:2.alice#key-2";
    const ALICE_SIGN_KID: &str = "did:peer:2.alice#key-1";
    const BOB: &str = "did:peer:2.bob";
    const BOB_ENC_KID: &str = "did:peer:2.bob#key-2";
    const MALLORY: &str = "did:peer:2.mallory";
    const MALLORY_ENC_KID: &str = "did:peer:2.mallory#key-2";
    const MALLORY_SIGN_KID: &str = "did:peer:2.mallory#key-1";

    /// The plaintext `to` of a frame addressed to Bob.
    fn to_bob() -> Vec<String> {
        vec![BOB.to_string()]
    }

    fn kids(kids: &[&str]) -> Option<Vec<String>> {
        Some(kids.iter().map(|kid| kid.to_string()).collect())
    }

    /// Metadata with every flag off, as navia-didcomm starts an unpack.
    fn plaintext_metadata() -> UnpackMetadata {
        UnpackMetadata {
            encrypted: false,
            authenticated: false,
            non_repudiation: false,
            anonymous_sender: false,
            re_wrapped_in_forward: false,
            encrypted_from_kid: None,
            encrypted_to_kids: None,
            sign_from: None,
            from_prior_issuer_kid: None,
            enc_alg_auth: None,
            enc_alg_anon: None,
            sign_alg: None,
            signed_message: None,
            from_prior: None,
            message_ids: Vec::new(),
        }
    }

    /// Metadata navia-didcomm reports for an authcrypt frame to Bob,
    /// optionally signed.
    fn authcrypt_metadata(from_kid: &str, sign_kid: Option<&str>) -> UnpackMetadata {
        UnpackMetadata {
            encrypted: true,
            authenticated: true,
            non_repudiation: sign_kid.is_some(),
            encrypted_from_kid: Some(from_kid.to_string()),
            encrypted_to_kids: kids(&[BOB_ENC_KID]),
            sign_from: sign_kid.map(str::to_string),
            ..plaintext_metadata()
        }
    }

    /// Metadata navia-didcomm reports for an anoncrypt frame whose envelope
    /// lists `to_kids`, optionally signed.
    fn anoncrypt_metadata(sign_kid: Option<&str>, to_kids: &[&str]) -> UnpackMetadata {
        UnpackMetadata {
            encrypted: true,
            authenticated: sign_kid.is_some(),
            non_repudiation: sign_kid.is_some(),
            anonymous_sender: true,
            encrypted_to_kids: kids(to_kids),
            sign_from: sign_kid.map(str::to_string),
            ..plaintext_metadata()
        }
    }

    #[test]
    fn did_of_kid_strips_the_fragment() {
        assert_eq!(did_of_kid(ALICE_ENC_KID), ALICE);
        assert_eq!(did_of_kid(ALICE), ALICE);
        assert_eq!(did_of_kid("did:peer:2.a#k#extra"), "did:peer:2.a");
    }

    #[test]
    fn authcrypt_signed_by_from_is_authenticated() {
        let metadata = authcrypt_metadata(ALICE_ENC_KID, Some(ALICE_SIGN_KID));

        let sender =
            authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect("genuine sender");

        assert_eq!(
            sender,
            SenderAuthentication {
                authenticated: true,
                encrypted_from_kid: Some(ALICE_ENC_KID.to_string()),
                sign_from: Some(ALICE_SIGN_KID.to_string()),
                anonymous_sender: false,
            }
        );
    }

    #[test]
    fn authcrypt_without_signature_is_authenticated() {
        let metadata = authcrypt_metadata(ALICE_ENC_KID, None);

        let sender =
            authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect("genuine sender");

        assert!(sender.authenticated);
        assert_eq!(sender.encrypted_from_kid.as_deref(), Some(ALICE_ENC_KID));
        assert_eq!(sender.sign_from, None);
    }

    #[test]
    fn authcrypt_is_authenticated_whatever_the_plaintext_to_says() {
        // Only Alice can authcrypt from her key to Bob's, so the envelope
        // alone ties the frame to Bob; `to` is not consulted.
        let metadata = authcrypt_metadata(ALICE_ENC_KID, Some(ALICE_SIGN_KID));

        let sender = authenticate_sender(Some(ALICE), &[MALLORY.to_string()], &metadata)
            .expect("genuine sender");

        assert!(sender.authenticated);
    }

    #[test]
    fn authcrypt_with_forged_from_is_refused() {
        let metadata = authcrypt_metadata(MALLORY_ENC_KID, Some(MALLORY_SIGN_KID));

        let err = authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect_err("forged from");

        assert!(matches!(
            err,
            NaviaError::Unpacking(UnpackingError::SenderMismatch)
        ));
    }

    #[test]
    fn authcrypt_with_forged_from_is_refused_even_when_signed_by_from() {
        // The signature names Alice, but the authcrypt key is Mallory's.
        let metadata = authcrypt_metadata(MALLORY_ENC_KID, Some(ALICE_SIGN_KID));

        let err = authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect_err("forged from");

        assert!(matches!(
            err,
            NaviaError::Unpacking(UnpackingError::SenderMismatch)
        ));
    }

    #[test]
    fn from_that_only_shares_a_prefix_with_the_sender_did_is_refused() {
        let metadata = authcrypt_metadata("did:peer:2.alice-evil#key-2", None);

        let err = authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect_err("forged from");

        assert!(matches!(
            err,
            NaviaError::Unpacking(UnpackingError::SenderMismatch)
        ));
    }

    #[test]
    fn authcrypt_without_from_passes_unauthenticated() {
        // navia-messaging's own forward wrapper is authcrypt without `from`.
        let metadata = authcrypt_metadata(ALICE_ENC_KID, None);

        let sender = authenticate_sender(None, &[], &metadata).expect("no from to forge");

        assert!(!sender.authenticated);
        assert_eq!(sender.encrypted_from_kid.as_deref(), Some(ALICE_ENC_KID));
    }

    #[test]
    fn authcrypt_signed_by_another_did_is_not_authenticated() {
        let metadata = authcrypt_metadata(ALICE_ENC_KID, Some(MALLORY_SIGN_KID));

        let sender =
            authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect("kid matches from");

        assert!(!sender.authenticated);
        assert_eq!(sender.sign_from.as_deref(), Some(MALLORY_SIGN_KID));
    }

    #[test]
    fn anoncrypt_frame_passes_unauthenticated() {
        let metadata = anoncrypt_metadata(None, &[BOB_ENC_KID]);

        let sender =
            authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect("anoncrypt passes");

        assert_eq!(
            sender,
            SenderAuthentication {
                authenticated: false,
                encrypted_from_kid: None,
                sign_from: None,
                anonymous_sender: true,
            }
        );
    }

    #[test]
    fn anoncrypt_frame_signed_by_from_and_addressed_here_is_authenticated() {
        let metadata = anoncrypt_metadata(Some(ALICE_SIGN_KID), &[BOB_ENC_KID]);

        let sender =
            authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect("signed by from");

        assert!(sender.authenticated);
        assert!(sender.anonymous_sender);
        assert_eq!(sender.encrypted_from_kid, None);
        assert_eq!(sender.sign_from.as_deref(), Some(ALICE_SIGN_KID));
    }

    #[test]
    fn signed_frame_relayed_to_a_recipient_its_to_does_not_name_is_not_authenticated() {
        // Mallory anoncrypts to Bob a message Alice signed for Mallory.
        let metadata = anoncrypt_metadata(Some(ALICE_SIGN_KID), &[BOB_ENC_KID]);

        let sender = authenticate_sender(Some(ALICE), &[MALLORY.to_string()], &metadata)
            .expect("no authcrypt kid");

        assert!(!sender.authenticated);
        assert_eq!(sender.sign_from.as_deref(), Some(ALICE_SIGN_KID));
    }

    #[test]
    fn relay_adding_its_own_recipient_key_does_not_make_the_frame_authenticated() {
        // The envelope lists Mallory's key next to Bob's, so one kid matches
        // `to = [Mallory]`; Bob's does not, and Bob decrypted with his.
        let metadata = anoncrypt_metadata(Some(ALICE_SIGN_KID), &[BOB_ENC_KID, MALLORY_ENC_KID]);

        let sender = authenticate_sender(Some(ALICE), &[MALLORY.to_string()], &metadata)
            .expect("no authcrypt kid");

        assert!(!sender.authenticated);
    }

    #[test]
    fn signed_frame_to_every_listed_recipient_is_authenticated() {
        let metadata = anoncrypt_metadata(Some(ALICE_SIGN_KID), &[BOB_ENC_KID, MALLORY_ENC_KID]);

        let sender = authenticate_sender(
            Some(ALICE),
            &[MALLORY.to_string(), BOB.to_string()],
            &metadata,
        )
        .expect("signed by from");

        assert!(sender.authenticated);
    }

    #[test]
    fn signed_frame_that_was_never_encrypted_is_not_authenticated() {
        // A bare JWS names no recipient key, so nothing ties it to Bob.
        let metadata = UnpackMetadata {
            authenticated: true,
            non_repudiation: true,
            sign_from: Some(ALICE_SIGN_KID.to_string()),
            ..plaintext_metadata()
        };

        let sender = authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect("signed");

        assert!(!sender.authenticated);
        assert_eq!(sender.sign_from.as_deref(), Some(ALICE_SIGN_KID));
    }

    #[test]
    fn anoncrypt_frame_signed_by_another_did_passes_unauthenticated() {
        let metadata = anoncrypt_metadata(Some(MALLORY_SIGN_KID), &[BOB_ENC_KID]);

        let sender =
            authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect("no authcrypt kid");

        assert!(!sender.authenticated);
        assert_eq!(sender.sign_from.as_deref(), Some(MALLORY_SIGN_KID));
    }

    #[test]
    fn plaintext_frame_passes_unauthenticated() {
        let sender = authenticate_sender(Some(ALICE), &to_bob(), &plaintext_metadata())
            .expect("plaintext passes");

        assert!(!sender.authenticated);
        assert!(!sender.anonymous_sender);
        assert_eq!(sender.encrypted_from_kid, None);
        assert_eq!(sender.sign_from, None);
    }

    #[test]
    fn authenticated_flag_without_sender_key_is_not_trusted() {
        // Defensive: navia-didcomm always records the kid it authenticated with.
        let metadata = UnpackMetadata {
            authenticated: true,
            ..anoncrypt_metadata(None, &[BOB_ENC_KID])
        };

        let sender =
            authenticate_sender(Some(ALICE), &to_bob(), &metadata).expect("no kid to compare");

        assert!(!sender.authenticated);
    }
}
