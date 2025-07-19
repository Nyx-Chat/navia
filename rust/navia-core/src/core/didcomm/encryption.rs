//! Encryption and decryption utilities
//! 
//! This module handles the cryptographic operations for DIDComm messages.

use crate::error::core::{CoreError, CoreResult};
use didcomm::{PackEncryptedOptions, UnpackOptions};

/// Default packing options for encrypted messages
pub fn default_pack_options() -> PackEncryptedOptions {
    PackEncryptedOptions::default()
}

/// Default unpacking options for encrypted messages
pub fn default_unpack_options() -> UnpackOptions {
    UnpackOptions::default()
}

/// Validate that the required fields are present for packing
pub fn validate_pack_params(to: &str, from: Option<&str>) -> CoreResult<()> {
    if to.is_empty() {
        return Err(CoreError::Validation("Recipient 'to' cannot be empty".to_string()));
    }
    
    if let Some(f) = from {
        if f.is_empty() {
            return Err(CoreError::Validation("Sender 'from' cannot be empty".to_string()));
        }
    }
    
    Ok(())
}

/// Validate message for unpacking
pub fn validate_unpack_message(msg: &str) -> CoreResult<()> {
    if msg.is_empty() {
        return Err(CoreError::Validation("Message cannot be empty".to_string()));
    }
    
    // Basic check for JWE structure
    if !msg.contains('.') {
        return Err(CoreError::Validation("Invalid message format".to_string()));
    }
    
    Ok(())
}