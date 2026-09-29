//! Conversion of an unpacked frame into the FFI message
//!
//! `DidComInterface::unpack` hands navia-didcomm's plaintext message and its
//! `UnpackMetadata` to [`DIDCommMessage::from_unpacked`], which refuses a
//! forged authcrypt `from`, binds a signature-only frame to its plaintext
//! `to`, and copies the sender authentication onto the FFI record. Every
//! other path that builds a `DIDCommMessage` leaves those fields at
//! `authenticated = false`, `None` kids and `anonymous_sender = false`.

use crate::core::didcomm::sender::authenticate_sender;
use crate::error::NaviaResult;
use crate::ffi::types::DIDCommMessage;
use navia_didcomm::{Message as NaviaMessage, UnpackMetadata};

impl DIDCommMessage {
    /// Builds the FFI message for a frame `unpack` decrypted, with its sender
    /// authentication taken from `metadata`.
    ///
    /// The body, id, type and recipients convert exactly as
    /// `From<navia_didcomm::Message>` converts them.
    ///
    /// # Errors
    ///
    /// `UnpackingError::SenderMismatch` when the frame is authcrypt-encrypted
    /// and its plaintext `from` names another DID than the sender key (see
    /// [`authenticate_sender`]).
    pub(crate) fn from_unpacked(
        message: NaviaMessage,
        metadata: &UnpackMetadata,
    ) -> NaviaResult<Self> {
        let to = message.to.as_deref().unwrap_or_default();
        let sender = authenticate_sender(message.from.as_deref(), to, metadata)?;

        Ok(DIDCommMessage {
            authenticated: sender.authenticated,
            encrypted_from_kid: sender.encrypted_from_kid,
            sign_from: sender.sign_from,
            anonymous_sender: sender.anonymous_sender,
            ..DIDCommMessage::from(message)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::core::didcomm::message::{Message, MessageBody};
    use crate::ffi::types::{DIDCommMessage, DidCommError};
    use navia_didcomm::{Message as NaviaMessage, UnpackMetadata};
    use serde_json::json;

    const MSG_TYPE: &str = "https://didcomm.org/basicmessage/2.0/message";
    const ALICE: &str = "did:peer:2.alice";
    const ALICE_ENC_KID: &str = "did:peer:2.alice#key-2";
    const ALICE_SIGN_KID: &str = "did:peer:2.alice#key-1";
    const MALLORY: &str = "did:peer:2.mallory";
    const MALLORY_ENC_KID: &str = "did:peer:2.mallory#key-2";
    const BOB: &str = "did:peer:2.bob";
    const BOB_ENC_KID: &str = "did:peer:2.bob#key-2";

    fn wire_message(from: Option<&str>) -> NaviaMessage {
        wire_message_to(from, BOB)
    }

    fn wire_message_to(from: Option<&str>, to: &str) -> NaviaMessage {
        let mut builder =
            NaviaMessage::build("msg-1".to_string(), MSG_TYPE.to_string(), json!({"a": 1}))
                .to(to.to_string());
        if let Some(from) = from {
            builder = builder.from(from.to_string());
        }
        builder.finalize()
    }

    fn metadata(
        encrypted_from_kid: Option<&str>,
        sign_from: Option<&str>,
        anonymous_sender: bool,
    ) -> UnpackMetadata {
        UnpackMetadata {
            encrypted: encrypted_from_kid.is_some() || anonymous_sender,
            authenticated: encrypted_from_kid.is_some() || sign_from.is_some(),
            non_repudiation: sign_from.is_some(),
            anonymous_sender,
            re_wrapped_in_forward: false,
            encrypted_from_kid: encrypted_from_kid.map(str::to_string),
            encrypted_to_kids: Some(vec![BOB_ENC_KID.to_string()]),
            sign_from: sign_from.map(str::to_string),
            from_prior_issuer_kid: None,
            enc_alg_auth: None,
            enc_alg_anon: None,
            sign_alg: None,
            signed_message: None,
            from_prior: None,
            message_ids: Vec::new(),
        }
    }

    #[test]
    fn authcrypt_frame_maps_to_authenticated_message_with_kids() {
        let unpacked = DIDCommMessage::from_unpacked(
            wire_message(Some(ALICE)),
            &metadata(Some(ALICE_ENC_KID), Some(ALICE_SIGN_KID), false),
        )
        .expect("genuine sender");

        assert!(unpacked.authenticated);
        assert_eq!(unpacked.encrypted_from_kid.as_deref(), Some(ALICE_ENC_KID));
        assert_eq!(unpacked.sign_from.as_deref(), Some(ALICE_SIGN_KID));
        assert!(!unpacked.anonymous_sender);
        // The rest converts as `From<navia_didcomm::Message>` does.
        assert_eq!(unpacked.id, "msg-1");
        assert_eq!(unpacked.msg_type, MSG_TYPE);
        assert_eq!(unpacked.from.as_deref(), Some(ALICE));
        assert_eq!(unpacked.to, vec![BOB.to_string()]);
        assert_eq!(unpacked.body, r#"{"a":1}"#);
    }

    #[test]
    fn anoncrypt_frame_maps_to_unauthenticated_message() {
        let unpacked =
            DIDCommMessage::from_unpacked(wire_message(Some(ALICE)), &metadata(None, None, true))
                .expect("anoncrypt passes");

        assert!(!unpacked.authenticated);
        assert_eq!(unpacked.encrypted_from_kid, None);
        assert_eq!(unpacked.sign_from, None);
        assert!(unpacked.anonymous_sender);
        assert_eq!(unpacked.from.as_deref(), Some(ALICE));
    }

    #[test]
    fn signature_only_frame_is_authenticated_only_when_its_to_names_the_recipient() {
        // Anoncrypt to Bob's key, signed by Alice: the plaintext `to` decides.
        let signed_only = metadata(None, Some(ALICE_SIGN_KID), true);

        let addressed_here =
            DIDCommMessage::from_unpacked(wire_message_to(Some(ALICE), BOB), &signed_only)
                .expect("signed by from");
        let relayed =
            DIDCommMessage::from_unpacked(wire_message_to(Some(ALICE), MALLORY), &signed_only)
                .expect("relayed, not forged");

        assert!(addressed_here.authenticated);
        assert!(!relayed.authenticated);
        assert_eq!(relayed.to, vec![MALLORY.to_string()]);
        assert_eq!(relayed.sign_from.as_deref(), Some(ALICE_SIGN_KID));
        assert_eq!(relayed.encrypted_from_kid, None);
    }

    #[test]
    fn forged_authcrypt_from_surfaces_as_permanent_unpacking_error() {
        let err = DIDCommMessage::from_unpacked(
            wire_message(Some(ALICE)),
            &metadata(Some(MALLORY_ENC_KID), None, false),
        )
        .expect_err("forged from");

        // `UnpackingError` is the permanent class: Kotlin
        // `DidCommException.UnpackingException`, Swift `DidCommError.UnpackingError`.
        match DidCommError::from(err) {
            DidCommError::UnpackingError { message } => {
                assert!(message.contains("Sender mismatch"), "message: {message}");
                // The error names neither DID, so it is safe to log.
                assert!(!message.contains(ALICE), "message: {message}");
                assert!(!message.contains("mallory"), "message: {message}");
            }
            other => panic!("expected UnpackingError, got {other:?}"),
        }
    }

    #[test]
    fn authcrypt_frame_without_from_keeps_the_sender_kid() {
        let unpacked = DIDCommMessage::from_unpacked(
            wire_message(None),
            &metadata(Some(ALICE_ENC_KID), None, false),
        )
        .expect("no from to forge");

        assert!(!unpacked.authenticated);
        assert_eq!(unpacked.from, None);
        assert_eq!(unpacked.encrypted_from_kid.as_deref(), Some(ALICE_ENC_KID));
    }

    #[test]
    fn other_conversions_leave_sender_authentication_unset() {
        let from_wire = DIDCommMessage::from(wire_message(Some(ALICE)));
        let from_core = DIDCommMessage::from(Message {
            id: "msg-2".to_string(),
            msg_type: MSG_TYPE.to_string(),
            body: MessageBody::String("hi".to_string()),
            from: Some(ALICE.to_string()),
            to: vec![BOB.to_string()],
            headers: Default::default(),
        });

        for message in [from_wire, from_core] {
            assert!(!message.authenticated);
            assert_eq!(message.encrypted_from_kid, None);
            assert_eq!(message.sign_from, None);
            assert!(!message.anonymous_sender);
        }
    }
}
