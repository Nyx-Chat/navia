//! Core message types
//!
//! These are the internal representations of DIDComm messages,
//! independent of any FFI concerns.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core representation of a DIDComm message.
///
/// This is the internal representation used throughout the core library,
/// providing a clean abstraction over the DIDComm protocol details.
///
/// # Fields
///
/// * `id` - Unique message identifier (typically a UUID)
/// * `msg_type` - Message type URI indicating the protocol and message purpose
/// * `body` - The message content (can be a string or JSON object)
/// * `from` - Optional sender DID for authenticated messages
/// * `to` - List of recipient DIDs
/// * `headers` - Additional message headers for protocol extensions
///
/// # Example
///
/// ```ignore
/// let message = Message::new(
///     "unique-id-123".to_string(),
///     "https://didcomm.org/basicmessage/2.0/message".to_string(),
///     MessageBody::String("Hello, World!".to_string())
/// )
/// .from("did:peer:sender".to_string())
/// .to("did:peer:recipient".to_string());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier
    pub id: String,
    /// Message type URI (protocol and message type)
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Message content
    pub body: MessageBody,
    /// Sender DID (optional for anonymous messages)
    pub from: Option<String>,
    /// List of recipient DIDs
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    /// Additional headers for protocol extensions
    pub headers: HashMap<String, String>,
}

/// Represents the content of a DIDComm message.
///
/// The body can be either a simple string or a complex JSON object,
/// depending on the protocol requirements.
///
/// # Variants
///
/// * `String` - Simple text content
/// * `Object` - Structured JSON data
///
/// # Serialization
///
/// The `#[serde(untagged)]` attribute means this enum is serialized
/// directly as its content without a wrapper, making it transparent
/// in the JSON representation.
///
/// # Examples
///
/// ```ignore
/// // Simple string body
/// let body = MessageBody::String("Hello!".to_string());
///
/// // Complex JSON body
/// let body = MessageBody::Object(json!({
///     "content": "Hello!",
///     "timestamp": "2024-01-01T12:00:00Z"
/// }));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageBody {
    /// Simple string content
    String(String),
    /// Structured JSON object
    Object(serde_json::Value),
}

impl Message {
    /// Creates a new message with the required fields.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique message identifier
    /// * `msg_type` - Message type URI
    /// * `body` - Message content
    ///
    /// # Returns
    ///
    /// A new `Message` instance with empty recipients and no sender
    pub fn new(id: String, msg_type: String, body: MessageBody) -> Self {
        Self {
            id,
            msg_type,
            body,
            from: None,
            to: Vec::new(),
            headers: HashMap::new(),
        }
    }

    /// Sets the sender DID use the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `from` - The sender's DID
    ///
    /// # Returns
    ///
    /// Self-for method chaining
    pub fn from(mut self, from: String) -> Self {
        self.from = Some(from);
        self
    }

    /// Adds a single recipient using the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `to` - A recipient's DID
    ///
    /// # Returns
    ///
    /// Self-for method chaining
    pub fn to(mut self, to: String) -> Self {
        self.to.push(to);
        self
    }

    /// Sets all recipients at once using the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `recipients` - Vector of recipient DIDs
    ///
    /// # Returns
    ///
    /// Self-for method chaining
    ///
    /// # Note
    ///
    /// This replaces any previously added recipients
    pub fn to_all(mut self, recipients: Vec<String>) -> Self {
        self.to = recipients;
        self
    }
}

/// Represents an encrypted DIDComm message.
///
/// This structure holds the encrypted payload and metadata needed
/// for recipients to decrypt the message.
///
/// # Fields
///
/// * `ciphertext` - The encrypted message content
/// * `protected` - Protected headers (base64url encoded)
/// * `recipients` - Per-recipient encryption information
///
/// # Note
///
/// This type is primarily used internally during the packing/unpacking
/// process and is not typically exposed through the public API.
#[derive(Debug, Clone)]
pub struct EncryptedMessage {
    /// The encrypted message payload
    pub ciphertext: String,
    /// Base64url-encoded protected headers
    pub protected: Option<String>,
    /// Per-recipient encryption metadata
    pub recipients: Vec<Recipient>,
}

/// Encryption metadata for a specific recipient.
///
/// Contains the encrypted content encryption key (CEK) and
/// associated headers needed for decryption.
#[derive(Debug, Clone)]
pub struct Recipient {
    /// Encrypted content encryption key for this recipient
    pub encrypted_key: String,
    /// Additional headers for key agreement
    pub header: RecipientHeader,
}

/// Headers associated with a recipient's encrypted key.
///
/// These headers contain the information needed to decrypt
/// the content encryption key using key agreement.
#[derive(Debug, Clone)]
pub struct RecipientHeader {
    /// Key identifier for the recipient's public key
    pub kid: String,
    /// Ephemeral public key used in key agreement
    pub epk: Option<serde_json::Value>,
    /// Initialization vector for key wrapping (if used)
    pub iv: Option<String>,
}
