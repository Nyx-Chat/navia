//! Core DIDComm message handling logic
//!
//! This module re-exports DidcommMessaging from navia_messaging and provides
//! additional wrapper functions with validation, audit logging, and message conversion.

pub use navia_messaging::messaging::DidcommMessaging;

use crate::core::audit::{audit_log, SecurityEvent};
use crate::core::didcomm::message::{Message, MessageBody};
use crate::error::specific::StorageError;
use crate::error::{NaviaError, PackingError, UnpackingError};
use navia_didcomm::{Message as DIDCommMessage, UnpackMetadata};
use serde_json::json;
use std::time::SystemTime;

/// Converts internal Message representation to DIDComm format.
pub fn message_to_didcomm(msg: &Message) -> DIDCommMessage {
    let body = match &msg.body {
        MessageBody::String(s) => json!(s),
        MessageBody::Object(obj) => obj.clone(),
    };

    let mut builder = DIDCommMessage::build(msg.id.clone(), msg.msg_type.clone(), body);

    for recipient in &msg.to {
        builder = builder.to(recipient.clone());
    }

    if let Some(from) = &msg.from {
        builder = builder.from(from.clone());
    }

    builder.finalize()
}

/// Converts DIDComm format message to internal representation.
pub fn didcomm_to_message(msg: DIDCommMessage) -> Result<Message, NaviaError> {
    let body = if msg.body.is_string() {
        MessageBody::String(msg.body.as_str().unwrap_or("").to_string())
    } else {
        MessageBody::Object(msg.body)
    };

    Ok(Message {
        id: msg.id,
        msg_type: msg.type_,
        body,
        from: msg.from,
        to: msg.to.unwrap_or_default(),
        headers: Default::default(),
    })
}

/// Audit logs a message pack event
pub fn audit_pack(from: Option<&str>, to: &str, message_id: &str) {
    audit_log(
        SecurityEvent::MessagePacked {
            from: from.map(|s| s.to_string()),
            to: to.to_string(),
            message_id: message_id.to_string(),
            timestamp: SystemTime::now(),
        },
        None,
    );
}

/// Audit logs a message unpack event
pub fn audit_unpack(metadata: &UnpackMetadata, message_id: &str) {
    audit_log(
        SecurityEvent::MessageUnpacked {
            from: metadata.encrypted_from_kid.clone(),
            to: None,
            message_id: message_id.to_string(),
            timestamp: SystemTime::now(),
        },
        None,
    );
}

/// Audit logs a DID generation event
pub fn audit_did_generated(did: &str) {
    audit_log(
        SecurityEvent::DidGenerated {
            did: did.to_string(),
            timestamp: SystemTime::now(),
        },
        None,
    );
}

/// Maps navia_messaging errors to specific NaviaError variants for packing.
pub fn map_packing_error(
    err: navia_messaging::error::Error,
    to: &str,
    from: Option<&str>,
) -> NaviaError {
    let error_str = err.to_string();
    if error_str.contains("Sender key not found") {
        NaviaError::from(PackingError::SenderKeyNotFound {
            did: from.unwrap_or("unknown").to_string(),
        })
    } else if error_str.contains("Recipient keys not found") {
        NaviaError::from(PackingError::RecipientKeyNotFound {
            did: to.to_string(),
        })
    } else if error_str.contains("Invalid recipient DID") {
        NaviaError::from(PackingError::InvalidRecipientDid {
            did: to.to_string(),
        })
    } else {
        NaviaError::from(PackingError::EncryptionFailed { details: error_str })
    }
}

/// Why an inbound frame could not be unpacked, as far as a redelivery is concerned.
///
/// The first five come from the frame itself and fail the same way on every
/// redelivery, so a consumer acknowledges them to the mediator. `Transient` is a
/// store, I/O or DID-resolution failure a later attempt can get past, or a
/// failure that cannot be told apart from one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnpackFailure {
    /// Not a well-formed JWE, JWS or JWM (a wrong signature, truncated JSON and
    /// key material that cannot be decoded included, such as a multicodec prefix
    /// that cannot be read). A multibase key without the `z` prefix or with an
    /// unknown or wrong multicodec prefix is `IllegalArgument` instead.
    Malformed,
    /// Addressed to keys this store does not hold (or a key removed while it
    /// is unpacked).
    SecretNotFound,
    /// No algorithm this build supports can open it.
    NoCompatibleCrypto,
    /// Uses a crypto algorithm, method or sender or signer key type this build
    /// does not support.
    Unsupported,
    /// Carries an argument navia-didcomm rejects (multibase key material
    /// without the `z` prefix or with an unknown or wrong multicodec prefix,
    /// for one).
    IllegalArgument,
    /// A later attempt can get past it, or it cannot be told apart from such a
    /// failure (some faults of the frame itself included, see
    /// `classify_didcomm_kind`).
    Transient,
}

/// Classifies a failed `DidcommMessaging::unpack_message`.
///
/// The outer navia-messaging kind is read first. On the pinned v1.1.2 it is
/// navia-didcomm's kind, passed through 1:1 for the nine kinds both crates name.
/// A failed key store is `IoError`: navia-messaging's secrets resolver reports
/// it that way and navia-didcomm never raises `IoError` itself, so it stays
/// transient. `InvalidState` is navia-didcomm's own `InvalidState` or one of the
/// navia-didcomm kinds navia-messaging has no name for.
///
/// v1.1.1 wrapped every unpack failure in `InvalidState` (a failed key store
/// included, labelled `InvalidState` inside too), so the `InvalidState` arm still
/// reads the navia-didcomm kind underneath. navia-core depends on the same
/// navia-didcomm as navia-messaging (one entry in `Cargo.lock`), so that error is
/// read by type through `anyhow::Error::downcast_ref` rather than from its
/// `Debug` rendering, which nyx-org-gateway has to parse because it has no direct
/// navia-didcomm dependency. An `InvalidState` around anything else is
/// transient, so nothing is acknowledged by accident.
///
/// Both unpack branches of `DidComInterface::unpack` go through this one
/// function (via `map_unpacking_error`).
#[must_use]
pub fn classify_unpack_failure(err: &navia_messaging::error::Error) -> UnpackFailure {
    use navia_messaging::error::ErrorKind;

    // Exhaustive on purpose: a kind navia-messaging adds has to be placed here.
    match err.kind() {
        ErrorKind::Malformed => UnpackFailure::Malformed,
        ErrorKind::SecretNotFound => UnpackFailure::SecretNotFound,
        ErrorKind::NoCompatibleCrypto => UnpackFailure::NoCompatibleCrypto,
        ErrorKind::Unsupported => UnpackFailure::Unsupported,
        ErrorKind::IllegalArgument => UnpackFailure::IllegalArgument,
        ErrorKind::InvalidState => err
            .source
            .downcast_ref::<navia_didcomm::error::Error>()
            .map_or(UnpackFailure::Transient, |inner| {
                classify_didcomm_kind(inner.kind())
            }),
        ErrorKind::IoError | ErrorKind::DIDNotResolved | ErrorKind::DIDUrlNotFound => {
            UnpackFailure::Transient
        }
    }
}

/// Classifies the navia-didcomm kind inside a navia-messaging `InvalidState`.
///
/// On v1.1.2 the kinds that reach this function are navia-didcomm's
/// `InvalidState` and the kinds navia-messaging folds into it, all transient; the
/// permanent arms serve the v1.1.1 shape, which wrapped every kind.
///
/// The same split as nyx-org-gateway's `PERMANENT_UNPACK_KINDS`. `IoError`,
/// `InvalidState`, the DID resolution kinds, and the kinds the pinned
/// navia-didcomm declares but its unpack path never returns all stay transient.
///
/// Under v1.1.1 an inner `InvalidState` is also a store failure behind the
/// secrets resolver or a failed DID resolution. v1.1.2 reports those as
/// `IoError` and `DIDNotResolved`, which `classify_unpack_failure` keeps
/// transient before this function is reached. The pinned navia-didcomm 1.3.1
/// labels the faults of the frame itself by cause, where 1.3.0 gave most of them
/// `InvalidState`: truncated JSON (an empty frame included), an anoncrypt
/// envelope that carries `apu` or is addressed to other keys than the authcrypt
/// inside it, an authcrypt tag longer than 124 bytes, a signature `alg` that
/// does not match the signer's key type and key material that cannot be decoded
/// (a multicodec prefix that cannot be read included) are `Malformed`; a
/// recipient key removed during the unpack is `SecretNotFound`; a sender or
/// signer key of a type it does not support is `Unsupported`. Multibase key
/// material without the `z` prefix or with an unknown or wrong multicodec prefix
/// stays `IllegalArgument`, as in 1.3.0. A failed sender DID resolution keeps
/// the DID resolver's kind: `DIDNotResolved`, or `Malformed` / `Unsupported`
/// for a DID document navia-messaging's resolver cannot map. Two faults of the
/// frame still carry a kind a transient failure has too: a sender DID that does
/// not resolve is `DIDNotResolved`, and a sender kid missing from its DID
/// document is `DIDUrlNotFound`. The kind alone cannot tell those, or a genuine
/// navia-didcomm `InvalidState`, apart from a transient failure, so they stay
/// transient and fail the same way on every redelivery; a consumer has to cap
/// redeliveries per frame (the stored payload with the mediator's `delivery_id`
/// left out, since the mediator mints a new `delivery_id` for every delivery).
fn classify_didcomm_kind(kind: navia_didcomm::error::ErrorKind) -> UnpackFailure {
    use navia_didcomm::error::ErrorKind;

    match kind {
        ErrorKind::Malformed => UnpackFailure::Malformed,
        ErrorKind::SecretNotFound => UnpackFailure::SecretNotFound,
        ErrorKind::NoCompatibleCrypto => UnpackFailure::NoCompatibleCrypto,
        ErrorKind::Unsupported => UnpackFailure::Unsupported,
        ErrorKind::IllegalArgument => UnpackFailure::IllegalArgument,
        _ => UnpackFailure::Transient,
    }
}

/// Maps a failed `DidcommMessaging::unpack_message` to the `NaviaError` its
/// classification (`classify_unpack_failure`) calls for.
///
/// A permanent failure becomes an `UnpackingError`, which reaches the FFI as
/// `DidCommError::UnpackingError` (Kotlin `DidCommException.UnpackingException`,
/// Swift `DidCommError.UnpackingError`): the frame can never be unpacked, so a
/// consumer acknowledges it to the mediator. A transient failure becomes
/// `StorageError::OperationFailed`, which reaches the FFI as
/// `DidCommError::DatabaseError` (Kotlin `DidCommException.DatabaseException`),
/// the variant navia-core already uses for navia-messaging failures; a consumer
/// leaves that frame for redelivery, with a cap per frame, because some
/// faults of the frame itself land here too (see `classify_didcomm_kind`). Both
/// conversions go through the `From` impls, so the failure is still counted in
/// the metrics.
pub fn map_unpacking_error(err: navia_messaging::error::Error) -> NaviaError {
    let details = err.to_string();
    match classify_unpack_failure(&err) {
        UnpackFailure::Malformed => UnpackingError::MalformedMessage { details }.into(),
        UnpackFailure::SecretNotFound => UnpackingError::RecipientKeyNotFound { details }.into(),
        UnpackFailure::NoCompatibleCrypto | UnpackFailure::Unsupported => {
            UnpackingError::DecryptionFailed { details }.into()
        }
        UnpackFailure::IllegalArgument => UnpackingError::InvalidMessageFormat { details }.into(),
        UnpackFailure::Transient => StorageError::OperationFailed {
            operation: "unpack".to_string(),
            details,
        }
        .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_unpack_failure, map_unpacking_error, UnpackFailure};
    use crate::error::ffi::DidCommError;
    use crate::error::specific::{StorageError, UnpackingError};
    use crate::error::NaviaError;
    use navia_didcomm::error::{Error as DidcommError, ErrorKind as DidcommKind};
    use navia_messaging::error::{Error as MessagingError, ErrorKind as MessagingKind};

    /// The shape navia-messaging v1.1.1's `DidcommMessaging::unpack` returned:
    /// navia-didcomm's error wrapped in navia-messaging's `InvalidState`.
    fn wrapped(kind: DidcommKind) -> MessagingError {
        MessagingError::new(MessagingKind::InvalidState, DidcommError::msg(kind, "test"))
    }

    /// The shape the pinned v1.1.2's `DidcommMessaging::unpack` returns:
    /// navia-didcomm's error converted with `From`, which carries its kind over
    /// (1:1, or `InvalidState` for a kind navia-messaging has no name for).
    fn passed_through(kind: DidcommKind) -> MessagingError {
        MessagingError::from(DidcommError::msg(kind, "test"))
    }

    /// What v1.1.2 returns when the key store fails during an unpack:
    /// `AskarSecretsResolver` wraps the store's error in a navia-didcomm
    /// `IoError`, navia-didcomm passes it up unchanged, and
    /// `DidcommMessaging::unpack` converts it with `From`.
    fn failed_key_store() -> MessagingError {
        let store = MessagingError::msg(MessagingKind::InvalidState, "pool closed");
        MessagingError::from(DidcommError::new(DidcommKind::IoError, store))
    }

    #[test]
    fn a_failed_key_store_is_io_error_and_surfaces_as_database_error() {
        assert_eq!(failed_key_store().kind(), MessagingKind::IoError);
        assert_eq!(
            classify_unpack_failure(&failed_key_store()),
            UnpackFailure::Transient
        );
        assert!(
            is_database(failed_key_store()),
            "a key store failure must not surface as UnpackingError"
        );
    }

    #[test]
    fn every_navia_didcomm_kind_as_v1_1_2_reports_it() {
        let permanent = [
            DidcommKind::Malformed,
            DidcommKind::SecretNotFound,
            DidcommKind::NoCompatibleCrypto,
            DidcommKind::Unsupported,
            DidcommKind::IllegalArgument,
        ];
        for kind in permanent {
            assert!(
                is_unpacking(passed_through(kind)),
                "{kind:?} should surface as UnpackingError"
            );
        }

        // Every other navia-didcomm kind. The first four keep their name in
        // navia-messaging; the rest fold into its `InvalidState`.
        let transient = [
            DidcommKind::IoError,
            DidcommKind::InvalidState,
            DidcommKind::DIDNotResolved,
            DidcommKind::DIDUrlNotFound,
            DidcommKind::DIDDocumentInvalid,
            DidcommKind::InvalidKeyMaterial,
            DidcommKind::KeyDerivationFailed,
            DidcommKind::EncryptionFailed,
            DidcommKind::DecryptionFailed,
            DidcommKind::SignatureVerificationFailed,
            DidcommKind::SignatureCreationFailed,
            DidcommKind::ProtocolViolation,
            DidcommKind::UnsupportedFormat,
            DidcommKind::MissingRequiredField,
            DidcommKind::CryptoOperationFailed,
            DidcommKind::Timeout,
            DidcommKind::ResourceExhausted,
        ];
        for kind in transient {
            assert_eq!(
                classify_unpack_failure(&passed_through(kind)),
                UnpackFailure::Transient,
                "{kind:?}"
            );
            assert!(
                is_database(passed_through(kind)),
                "{kind:?} should surface as DatabaseError"
            );
        }
    }

    fn is_unpacking(err: MessagingError) -> bool {
        matches!(
            DidCommError::from(map_unpacking_error(err)),
            DidCommError::UnpackingError { .. }
        )
    }

    fn is_database(err: MessagingError) -> bool {
        matches!(
            DidCommError::from(map_unpacking_error(err)),
            DidCommError::DatabaseError { .. }
        )
    }

    #[test]
    fn the_didcomm_kind_inside_invalid_state_decides() {
        let permanent = [
            (DidcommKind::Malformed, UnpackFailure::Malformed),
            (DidcommKind::SecretNotFound, UnpackFailure::SecretNotFound),
            (
                DidcommKind::NoCompatibleCrypto,
                UnpackFailure::NoCompatibleCrypto,
            ),
            (DidcommKind::Unsupported, UnpackFailure::Unsupported),
            (DidcommKind::IllegalArgument, UnpackFailure::IllegalArgument),
        ];
        for (kind, expected) in permanent {
            assert_eq!(
                classify_unpack_failure(&wrapped(kind)),
                expected,
                "{kind:?}"
            );
            assert!(
                is_unpacking(wrapped(kind)),
                "{kind:?} should surface as UnpackingError"
            );
        }

        let transient = [
            DidcommKind::InvalidState,
            DidcommKind::IoError,
            DidcommKind::DIDNotResolved,
            DidcommKind::DIDUrlNotFound,
            DidcommKind::Timeout,
            DidcommKind::ResourceExhausted,
        ];
        for kind in transient {
            assert_eq!(
                classify_unpack_failure(&wrapped(kind)),
                UnpackFailure::Transient,
                "{kind:?}"
            );
            assert!(
                is_database(wrapped(kind)),
                "{kind:?} should surface as DatabaseError"
            );
        }
    }

    #[test]
    fn navia_messagings_own_kind_decides_outside_invalid_state() {
        let permanent = [
            (MessagingKind::Malformed, UnpackFailure::Malformed),
            (MessagingKind::SecretNotFound, UnpackFailure::SecretNotFound),
            (
                MessagingKind::NoCompatibleCrypto,
                UnpackFailure::NoCompatibleCrypto,
            ),
            (MessagingKind::Unsupported, UnpackFailure::Unsupported),
            (
                MessagingKind::IllegalArgument,
                UnpackFailure::IllegalArgument,
            ),
        ];
        for (kind, expected) in permanent {
            let err = || MessagingError::msg(kind, "test");
            assert_eq!(classify_unpack_failure(&err()), expected, "{kind:?}");
            assert!(
                is_unpacking(err()),
                "{kind:?} should surface as UnpackingError"
            );
        }

        let transient = [
            MessagingKind::IoError,
            MessagingKind::DIDNotResolved,
            MessagingKind::DIDUrlNotFound,
            MessagingKind::InvalidState,
        ];
        for kind in transient {
            let err = || MessagingError::msg(kind, "test");
            assert_eq!(
                classify_unpack_failure(&err()),
                UnpackFailure::Transient,
                "{kind:?}"
            );
            assert!(
                is_database(err()),
                "{kind:?} should surface as DatabaseError"
            );
        }
    }

    #[test]
    fn an_unrecognised_wrapped_error_is_left_for_redelivery() {
        let io = || {
            MessagingError::new(
                MessagingKind::InvalidState,
                std::io::Error::other("Malformed"),
            )
        };
        // Reads like a navia-didcomm Debug rendering, but is only text: no string sniffing.
        let rendered =
            || MessagingError::msg(MessagingKind::InvalidState, "Error { kind: Malformed");

        assert_eq!(classify_unpack_failure(&io()), UnpackFailure::Transient);
        assert_eq!(
            classify_unpack_failure(&rendered()),
            UnpackFailure::Transient
        );
        assert!(is_database(io()));
        assert!(is_database(rendered()));
    }

    #[test]
    fn each_permanent_kind_keeps_the_closest_unpacking_variant() {
        assert!(matches!(
            map_unpacking_error(wrapped(DidcommKind::Malformed)),
            NaviaError::Unpacking(UnpackingError::MalformedMessage { .. })
        ));
        assert!(matches!(
            map_unpacking_error(wrapped(DidcommKind::SecretNotFound)),
            NaviaError::Unpacking(UnpackingError::RecipientKeyNotFound { .. })
        ));
        assert!(matches!(
            map_unpacking_error(wrapped(DidcommKind::NoCompatibleCrypto)),
            NaviaError::Unpacking(UnpackingError::DecryptionFailed { .. })
        ));
        assert!(matches!(
            map_unpacking_error(wrapped(DidcommKind::Unsupported)),
            NaviaError::Unpacking(UnpackingError::DecryptionFailed { .. })
        ));
        assert!(matches!(
            map_unpacking_error(wrapped(DidcommKind::IllegalArgument)),
            NaviaError::Unpacking(UnpackingError::InvalidMessageFormat { .. })
        ));
        assert!(matches!(
            map_unpacking_error(wrapped(DidcommKind::IoError)),
            NaviaError::Storage(StorageError::OperationFailed { operation, .. }) if operation == "unpack"
        ));

        match DidCommError::from(map_unpacking_error(wrapped(DidcommKind::SecretNotFound))) {
            DidCommError::UnpackingError { message } => {
                assert!(
                    message.contains("test"),
                    "the underlying detail is kept: {message}"
                );
            }
            other => panic!("expected UnpackingError, got {other:?}"),
        }
    }
}
