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
/// # Sender authentication fields (set by `unpack` only)
///
/// The last four fields describe how navia-didcomm authenticated the frame
/// that `unpack` decrypted. They carry meaning only on a message returned by
/// `unpack`. Every other path that creates a `DIDCommMessage` (a message the
/// app builds for `pack` / `pack_no_forward`, or any other conversion) sets
/// `authenticated = false`, both kids to `None` and `anonymous_sender = false`,
/// and `pack` / `pack_no_forward` ignore whatever the caller puts in them.
/// UniFFI gives these four fields default values, so Kotlin and Swift callers
/// that build a message for `pack` can leave them out.
///
/// * `authenticated` - `true` when the plaintext `from` is proven and the
///   frame is tied to this recipient: `from` is set, navia-didcomm
///   authenticated the frame (authcrypt with a resolved sender key, or a
///   verified signature), every sender key it authenticated with
///   (`encrypted_from_kid`, `sign_from`) belongs to the `from` DID, and
///   either the frame is authcrypt-encrypted or, when a signature is the only
///   proof, every recipient key of its encrypted envelope belongs to a DID
///   named in `to`. An unsigned anoncrypt frame, a plaintext frame, a frame
///   without `from`, a frame whose signer kid names another DID, and a signed
///   frame that was never encrypted or was relayed to a recipient its `to`
///   does not name all come back `false`. (`unpack` refuses an authcrypt
///   frame whose `from` names another DID than `encrypted_from_kid`
///   outright.) The key that proves `from` is `encrypted_from_kid` when it is
///   set and `sign_from` otherwise, so `encrypted_from_kid` can be `None` on
///   an authenticated message. A consumer that accepts only authcrypt frames checks
///   `authenticated && encrypted_from_kid.is_some()`. The flag does not make
///   a frame fresh: a genuine frame can be delivered again, so deduplicate by
///   `id`.
/// * `encrypted_from_kid` - Key ID (`did#fragment`) of the sender key that
///   authcrypt-encrypted the frame; `None` for anoncrypt, signed-only or
///   plaintext frames.
/// * `sign_from` - Key ID of the key whose signature navia-didcomm verified;
///   `None` when the frame was not signed. On a signed frame that is not
///   authcrypt-encrypted, this is the key that proves `from`.
/// * `anonymous_sender` - `true` when the frame arrived in an anoncrypt
///   envelope, which hides the sender from intermediaries (an anoncrypt frame
///   on its own, or anoncrypt wrapping authcrypt). It says nothing about
///   whether the sender was authenticated; read `authenticated` for that.
///
/// # Examples
///
/// ```
/// use navia_core::ffi::types::DIDCommMessage;
/// // Basic message
/// let message = DIDCommMessage {
///     id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
///     msg_type: "https://example.org/protocols/1.0/ping".to_string(),
///     body: "{}".to_string(),
///     from: Some("did:peer:sender123".to_string()),
///     to: vec!["did:peer:recipient456".to_string()],
///     // Outgoing message: the sender authentication fields stay unset
///     authenticated: false,
///     encrypted_from_kid: None,
///     sign_from: None,
///     anonymous_sender: false,
/// };
///
/// // Message with JSON body
/// let message_with_content = DIDCommMessage {
///     id: "msg-001".to_string(),
///     msg_type: "https://didcomm.org/basicmessage/2.0/message".to_string(),
///     body: r#"{"content": "Hello, World!", "sent_time": "2024-01-01T12:00:00Z"}"#.to_string(),
///     from: Some("did:peer:alice".to_string()),
///     to: vec!["did:peer:bob".to_string()],
///     authenticated: false,
///     encrypted_from_kid: None,
///     sign_from: None,
///     anonymous_sender: false,
/// };
///
/// assert_eq!(message.msg_type, "https://example.org/protocols/1.0/ping");
/// assert_eq!(message_with_content.to.len(), 1);
/// ```
#[derive(uniffi::Record, Clone, Debug)]
pub struct DIDCommMessage {
    /// Unique message identifier
    pub id: String,
    /// Protocol message type URI
    pub msg_type: String,
    /// Message body as JSON string
    pub body: String,
    /// Sender DID (optional for anonymous messages). On an unpacked message,
    /// trust it only when `authenticated` is `true`.
    pub from: Option<String>,
    /// List of recipient DIDs
    pub to: Vec<String>,
    /// Set by `unpack` only: `true` when the plaintext `from` is proven by the
    /// sender key(s) navia-didcomm authenticated and the frame is tied to this
    /// recipient (authcrypt, or a signature whose `to` names every recipient
    /// key's DID). `false` on every other path.
    #[uniffi(default = false)]
    pub authenticated: bool,
    /// Set by `unpack` only: key ID of the authcrypt sender key, `None` when
    /// the frame was not authcrypt-encrypted. `None` on every other path.
    #[uniffi(default = None)]
    pub encrypted_from_kid: Option<String>,
    /// Set by `unpack` only: key ID of the verified signature, `None` when the
    /// frame was not signed; it proves `from` when `encrypted_from_kid` is
    /// `None`. `None` on every other path.
    #[uniffi(default = None)]
    pub sign_from: Option<String>,
    /// Set by `unpack` only: `true` when the frame arrived in an anoncrypt
    /// envelope. `false` on every other path.
    #[uniffi(default = false)]
    pub anonymous_sender: bool,
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
/// ```
/// use navia_core::ffi::types::KeyValue;
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
///
/// assert_eq!(setting.key, "theme");
/// assert!(contact.metadata.is_some());
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
