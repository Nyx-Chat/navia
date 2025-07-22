//! Type conversions between FFI and core types
//! 
//! This module handles converting between the simple FFI types
//! exposed through UniFFI and the richer core domain types.

use crate::ffi::types::{DIDCommMessage, KeyValue};
use crate::core::didcomm::message::{Message, MessageBody};

/// Converts FFI DIDCommMessage to core Message type.
/// 
/// This conversion handles the transformation of the message body:
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

/// Helper function to convert optional storage results to KeyValue.
/// 
/// This is useful for batch operations where some keys might not exist.
/// 
/// # Arguments
/// 
/// * `key` - The key that was queried
/// * `value` - Optional value from storage
/// 
/// # Returns
/// 
/// * `Some(KeyValue)` if a value was found
/// * `None` if the key didn't exist in storage
/// 
/// # Example
/// 
/// ```ignore
/// let result = storage.get("contacts", "alice").await?;
/// let kv = optional_tuple_to_keyvalue("alice".to_string(), result);
/// ```
pub fn optional_tuple_to_keyvalue(key: String, value: Option<String>) -> Option<KeyValue> {
    value.map(|val| KeyValue {
        key,
        value: val,
        metadata: None,
    })
}