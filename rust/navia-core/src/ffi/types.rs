//! FFI type definitions for UniFFI
//! 
//! These types are exposed through the FFI boundary and are
//! designed to be simple DTOs that map well to Kotlin/Swift.

/// DIDComm message structure for cross-language FFI communication.
/// 
/// This type represents a DIDComm v2 message in a format that can be
/// easily passed between Rust and Kotlin/Swift through UniFFI.
/// 
/// # Fields
/// 
/// * `id` - Unique message identifier. Should be a UUID or similar unique string.
/// * `msg_type` - Message type URI indicating the protocol and message type
///   (e.g., "https://didcomm.org/basicmessage/2.0/message")
/// * `body` - Message body as a JSON string. The structure depends on the protocol.
/// * `from` - Optional sender DID. Required for authenticated messages.
/// * `to` - List of recipient DIDs. Must contain at least one recipient for packing.
/// 
/// # Examples
/// 
/// ```ignore
/// # use crate::ffi::DIDCommMessage;
/// // Basic message
/// let message = DIDCommMessage {
///     id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
///     msg_type: "https://example.org/protocols/1.0/ping".to_string(),
///     body: "{}".to_string(),
///     from: Some("did:peer:sender123".to_string()),
///     to: vec!["did:peer:recipient456".to_string()],
/// };
/// 
/// // Message with JSON body
/// let message_with_content = DIDCommMessage {
///     id: "msg-001".to_string(),
///     msg_type: "https://didcomm.org/basicmessage/2.0/message".to_string(),
///     body: r#"{"content": "Hello, World!", "sent_time": "2024-01-01T12:00:00Z"}"#.to_string(),
///     from: Some("did:peer:alice".to_string()),
///     to: vec!["did:peer:bob".to_string()],
/// };
/// ```
#[derive(uniffi::Record, Clone, Debug)]
pub struct DIDCommMessage {
    /// Unique message identifier
    pub id: String,
    /// Protocol message type URI
    pub msg_type: String,
    /// Message body as JSON string
    pub body: String,
    /// Sender DID (optional for anonymous messages)
    pub from: Option<String>,
    /// List of recipient DIDs
    pub to: Vec<String>,
}

/// Key-value pair for batch database operations.
/// 
/// Used for efficient batch insertions and retrievals to minimize
/// database transaction overhead.
/// 
/// # Fields
/// 
/// * `key` - Unique identifier within a category
/// * `value` - String value to store (can be JSON)
/// * `metadata` - Reserved for future use (e.g., timestamps, tags)
/// 
/// # Examples
/// 
/// ```ignore
/// # use crate::ffi::KeyValue;
/// // Simple key-value
/// let setting = KeyValue {
///     key: "theme".to_string(),
///     value: "dark".to_string(),
///     metadata: None,
/// };
/// 
/// // JSON value with metadata
/// let contact = KeyValue {
///     key: "alice".to_string(),
///     value: r#"{"did": "did:peer:alice", "name": "Alice Smith"}"#.to_string(),
///     metadata: Some("imported:2024-01-01".to_string()),
/// };
/// ```
#[derive(uniffi::Record, Clone, Debug)]
pub struct KeyValue {
    /// Unique key within the category
    pub key: String,
    /// Value to store (any string, including JSON)
    pub value: String,
    /// Optional metadata for future extensibility
    pub metadata: Option<String>,
}

// Re-export error from error module
pub use crate::error::ffi::DidCommError;