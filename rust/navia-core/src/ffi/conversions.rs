//! Type conversions between FFI and core types
//!
//! This module handles converting between the simple FFI types
//! exposed through UniFFI and the richer core domain types.
//!
//! # Body rule
//!
//! `DIDCommMessage::body` is text on the FFI side and a JSON value on the wire.
//! Both directions apply one rule, so plain text survives a pack/unpack round
//! trip byte for byte:
//!
//! | FFI body text                                            | Wire `body` (JSON value) |
//! |----------------------------------------------------------|--------------------------|
//! | first non-whitespace char is `{` or `[`, parses as JSON  | that object or array     |
//! | anything else (including malformed JSON)                 | a JSON string            |
//!
//! Unpack maps the wire value back to text:
//! - a JSON string becomes its contents verbatim, without JSON quoting
//! - an object or array becomes its compact JSON text (whitespace and key
//!   order are normalised, the value is JSON-equal)
//! - a number, bool or null becomes its JSON text (`42`, `true`, `null`).
//!   Pack emits these only as strings, so they arrive from other DIDComm
//!   senders; their JSON text is lossless and reads naturally.

use crate::core::didcomm::message::{Message, MessageBody};
use crate::ffi::types::{DIDCommMessage, KeyValue};
use navia_didcomm::Message as NaviaMessage;
use serde_json::Value;

/// Converts FFI DIDCommMessage to a core Message type.
///
/// This conversion handles the transformation of the message body
/// (the pack half of the module-level body rule):
/// - If the body looks like JSON (starts with '{' or '['), it attempts to parse it
/// - If parsing succeeds, stores it as a structured Object
/// - If parsing fails or it's not JSON, stores it as a plain String
///
/// This approach provides flexibility for different message formats while
/// maintaining type safety in the core domain.
impl From<DIDCommMessage> for Message {
    fn from(ffi_msg: DIDCommMessage) -> Self {
        // Intelligently parse the body based on its content
        let body = if ffi_msg.body.trim().starts_with('{') || ffi_msg.body.trim().starts_with('[') {
            // Try to parse as JSON
            match serde_json::from_str::<serde_json::Value>(&ffi_msg.body) {
                Ok(json_value) => MessageBody::Object(json_value),
                Err(_) => MessageBody::String(ffi_msg.body), // Graceful fallback for malformed JSON
            }
        } else {
            MessageBody::String(ffi_msg.body)
        };

        Message {
            id: ffi_msg.id,
            msg_type: ffi_msg.msg_type,
            body,
            from: ffi_msg.from,
            to: ffi_msg.to,
            headers: Default::default(),
        }
    }
}

/// Converts core Message to FFI DIDCommMessage type.
///
/// This conversion serializes the message body:
/// - String bodies are passed through as-is
/// - Object bodies are serialized to JSON strings
///
/// The serialization is guaranteed to produce valid JSON for Object variants,
/// falling back to "{}" in the unlikely event of serialization failure.
impl From<Message> for DIDCommMessage {
    fn from(core_msg: Message) -> Self {
        // Serialize body to string format required by FFI
        let body = match &core_msg.body {
            MessageBody::String(s) => s.clone(),
            MessageBody::Object(obj) => {
                // Convert JSON object to string representation
                serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string())
            }
        };

        DIDCommMessage {
            id: core_msg.id,
            msg_type: core_msg.msg_type,
            body,
            from: core_msg.from,
            to: core_msg.to,
        }
    }
}

/// Converts FFI KeyValue to a simple tuple for core operations.
///
/// This strips away the optional metadata field, providing just the
/// key-value pair needed by core storage operations.
///
/// # Note
///
/// The metadata field is intentionally discarded as it's reserved for
/// future use and not currently processed by the storage layer.
impl From<KeyValue> for (String, String) {
    fn from(kv: KeyValue) -> Self {
        (kv.key, kv.value)
    }
}

/// Converts a key-value tuple to FFI KeyValue type.
///
/// This is used when returning data from core storage operations,
/// adding the required metadata field (set to None).
impl From<(String, String)> for KeyValue {
    fn from(tuple: (String, String)) -> Self {
        KeyValue {
            key: tuple.0,
            value: tuple.1,
            metadata: None,
        }
    }
}

/// Renders a wire JSON body as FFI body text (the unpack half of the
/// module-level body rule).
///
/// A JSON string comes back verbatim; every other value comes back as its
/// compact JSON text.
fn body_to_ffi_text(body: Value) -> String {
    match body {
        Value::String(text) => text,
        other => serde_json::to_string(&other).unwrap_or_else(|_| "{}".to_string()),
    }
}

/// Converts navia_didcomm Message to FFI DIDCommMessage type.
///
/// This conversion is used when unpacking messages received from
/// the navia-messaging library. The body follows the module-level body
/// rule: a JSON-string body reaches Kotlin/Swift as its plain text, and an
/// object, array, number, bool or null body reaches them as JSON text.
impl From<NaviaMessage> for DIDCommMessage {
    fn from(msg: NaviaMessage) -> Self {
        let body = body_to_ffi_text(msg.body);

        DIDCommMessage {
            id: msg.id,
            msg_type: msg.type_,
            body,
            from: msg.from,
            to: msg.to.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::didcomm::handler::message_to_didcomm;
    use serde_json::json;

    const MSG_TYPE: &str = "https://didcomm.org/basicmessage/2.0/message";
    const ALICE: &str = "did:peer:alice";
    const BOB: &str = "did:peer:bob";

    fn wire_message(body: Value) -> NaviaMessage {
        NaviaMessage::build("msg-1".to_string(), MSG_TYPE.to_string(), body)
            .from(ALICE.to_string())
            .to(BOB.to_string())
            .finalize()
    }

    fn unpacked_body(body: Value) -> String {
        DIDCommMessage::from(wire_message(body)).body
    }

    /// Runs FFI body text through the pack conversions, the plaintext wire
    /// JSON and the unpack conversion (the pack/unpack path minus encryption).
    fn round_trip(text: &str) -> String {
        let outgoing = DIDCommMessage {
            id: "msg-rt".to_string(),
            msg_type: MSG_TYPE.to_string(),
            body: text.to_string(),
            from: Some(ALICE.to_string()),
            to: vec![BOB.to_string()],
        };
        let core: Message = outgoing.into();
        let wire_json =
            serde_json::to_string(&message_to_didcomm(&core)).expect("wire message serialises");
        let received: NaviaMessage =
            serde_json::from_str(&wire_json).expect("wire message deserialises");
        DIDCommMessage::from(received).body
    }

    #[test]
    fn string_body_is_returned_as_plain_text() {
        assert_eq!(unpacked_body(json!("hello")), "hello");
    }

    #[test]
    fn string_body_with_quotes_newlines_and_unicode_is_returned_verbatim() {
        let text = "say \"hi\"\n\u{2713} \\ done";
        assert_eq!(unpacked_body(json!(text)), text);
    }

    #[test]
    fn string_body_holding_json_text_is_returned_verbatim() {
        let text = r#"{"a":1}"#;
        assert_eq!(unpacked_body(Value::String(text.to_string())), text);
    }

    #[test]
    fn empty_string_body_stays_empty() {
        assert_eq!(unpacked_body(json!("")), "");
    }

    #[test]
    fn object_body_becomes_json_text() {
        assert_eq!(unpacked_body(json!({"a": 1})), r#"{"a":1}"#);

        let nested = json!({"content": "hi", "meta": {"tags": ["x", "y"]}, "n": 2});
        let text = unpacked_body(nested.clone());
        let reparsed: Value = serde_json::from_str(&text).expect("object body is JSON text");
        assert_eq!(reparsed, nested);
    }

    #[test]
    fn array_body_becomes_json_text() {
        assert_eq!(unpacked_body(json!([1, 2])), "[1,2]");
        assert_eq!(unpacked_body(json!([])), "[]");
    }

    #[test]
    fn scalar_bodies_become_json_text() {
        assert_eq!(unpacked_body(json!(42)), "42");
        assert_eq!(unpacked_body(json!(-1.5)), "-1.5");
        assert_eq!(unpacked_body(json!(true)), "true");
        assert_eq!(unpacked_body(json!(false)), "false");
        assert_eq!(unpacked_body(Value::Null), "null");
    }

    #[test]
    fn envelope_fields_pass_through() {
        let msg = DIDCommMessage::from(wire_message(json!("hello")));
        assert_eq!(msg.id, "msg-1");
        assert_eq!(msg.msg_type, MSG_TYPE);
        assert_eq!(msg.from.as_deref(), Some(ALICE));
        assert_eq!(msg.to, vec![BOB.to_string()]);
    }

    #[test]
    fn missing_recipients_become_an_empty_list() {
        let anonymous =
            NaviaMessage::build("msg-2".to_string(), MSG_TYPE.to_string(), json!("hi")).finalize();
        let msg = DIDCommMessage::from(anonymous);
        assert!(msg.to.is_empty());
        assert!(msg.from.is_none());
        assert_eq!(msg.body, "hi");
    }

    #[test]
    fn plain_text_survives_pack_unpack_round_trip_verbatim() {
        for text in [
            "hello",
            "",
            "42",
            "true",
            "null",
            "\"already quoted\"",
            "say \"hi\"\n\u{2713}",
            "{broken",
            "[1, 2",
            "  padded  ",
        ] {
            assert_eq!(round_trip(text), text, "body {text:?}");
        }
    }

    #[test]
    fn json_object_and_array_text_survive_round_trip_as_equal_json() {
        for text in [
            r#"{ "content": "hi", "n": [1, 2] }"#,
            "[1, 2]",
            "  {\"a\": 1}",
        ] {
            let original: Value = serde_json::from_str(text).expect("fixture is JSON");
            let returned: Value =
                serde_json::from_str(&round_trip(text)).expect("returned body is JSON text");
            assert_eq!(returned, original, "body {text:?}");
        }
    }
}
