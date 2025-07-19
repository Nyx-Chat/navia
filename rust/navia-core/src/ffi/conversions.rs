//! Type conversions between FFI and core types
//! 
//! This module handles converting between the simple FFI types
//! exposed through UniFFI and the richer core domain types.

use crate::ffi::types::{DIDCommMessage, KeyValue};
use crate::core::didcomm::message::{Message, MessageBody};
use serde_json::json;

impl From<DIDCommMessage> for Message {
    fn from(ffi_msg: DIDCommMessage) -> Self {
        // Convert the simple string body to a JSON object with "content" field
        let body = MessageBody::Object(json!({
            "content": ffi_msg.body
        }));
        
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

impl From<Message> for DIDCommMessage {
    fn from(core_msg: Message) -> Self {
        // Extract the body content
        let body = match &core_msg.body {
            MessageBody::String(s) => s.clone(),
            MessageBody::Object(obj) => {
                // Try to extract "content" field, otherwise convert to string
                obj.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
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

/// Convert FFI KeyValue to tuple for core operations
impl From<KeyValue> for (String, String) {
    fn from(kv: KeyValue) -> Self {
        (kv.key, kv.value)
    }
}

/// Convert tuple to FFI KeyValue
impl From<(String, String)> for KeyValue {
    fn from(tuple: (String, String)) -> Self {
        KeyValue {
            key: tuple.0,
            value: tuple.1,
            metadata: None,
        }
    }
}

/// Helper function to convert optional value tuple to KeyValue if value exists
pub fn optional_tuple_to_keyvalue(key: String, value: Option<String>) -> Option<KeyValue> {
    value.map(|val| KeyValue {
        key,
        value: val,
        metadata: None,
    })
}