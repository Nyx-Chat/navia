//! FFI type definitions for UniFFI
//! 
//! These types are exposed through the FFI boundary and are
//! designed to be simple DTOs that map well to Kotlin/Swift.

/// DIDComm message structure for UniFFI
#[derive(uniffi::Record, Clone, Debug)]
pub struct DIDCommMessage {
    pub id: String,
    pub msg_type: String,
    pub body: String,
    pub from: Option<String>,
    pub to: Vec<String>,
}

/// Simple key-value structure for batch operations
#[derive(uniffi::Record, Clone, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub metadata: Option<String>,  // For future extensibility
}

// Re-export error from error module
pub use crate::error::ffi::DidCommError;