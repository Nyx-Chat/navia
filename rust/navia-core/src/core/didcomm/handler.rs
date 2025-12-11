//! Core DIDComm message handling logic
//!
//! This module re-exports DidcommMessaging from navia_messaging and provides
//! additional wrapper functions with validation, audit logging, and message conversion.

pub use navia_messaging::messaging::DidcommMessaging;

use crate::core::audit::{audit_log, SecurityEvent};
use crate::core::didcomm::message::{Message, MessageBody};
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

/// Maps navia_messaging errors to specific NaviaError variants for unpacking.
pub fn map_unpacking_error(err: navia_messaging::error::Error) -> NaviaError {
    let error_str = err.to_string();
    if error_str.contains("Wrong recipient") || error_str.contains("Recipient key not found") {
        NaviaError::from(UnpackingError::RecipientKeyNotFound { details: error_str })
    } else if error_str.contains("Invalid signature") {
        NaviaError::from(UnpackingError::InvalidSignature {
            signer: "unknown".to_string(),
        })
    } else if error_str.contains("Malformed") || error_str.contains("Invalid format") {
        NaviaError::from(UnpackingError::MalformedMessage { details: error_str })
    } else {
        NaviaError::from(UnpackingError::DecryptionFailed { details: error_str })
    }
}
