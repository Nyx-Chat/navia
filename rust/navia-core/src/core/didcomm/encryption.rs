//! Encryption and decryption utilities for DIDComm messaging
//!
//! This module provides cryptographic operations and validation functions for
//! DIDComm message encryption. It handles the configuration of encryption
//! algorithms and ensures messages meet the requirements for secure transmission.
//!
//! # Encryption Standards
//!
//! DIDComm v2 uses JSON Web Encryption (JWE) for message confidentiality:
//! - Authenticated encryption by default (sender is verified)
//! - Anonymous encryption supported (no sender authentication)
//! - Perfect forward secrecy through ephemeral keys
//!
//! # Default Algorithms
//!
//! The default encryption configuration uses:
//! - Key Agreement: ECDH-ES (Elliptic Curve Diffie-Hellman Ephemeral Static)
//! - Content Encryption: A256GCM (AES-256 in GCM mode)
//!
//! # Example
//!
//! ```
//! use navia_core::core::didcomm::encryption::{
//!     default_pack_options,
//!     validate_pack_params
//! };
//!
//! // Validate parameters before packing
//! assert!(validate_pack_params("did:peer:bob", Some("did:peer:alice")).is_ok());
//!
//! // Get default encryption options
//! let pack_opts = default_pack_options();
//! ```

use crate::error::{NaviaError, NaviaResult, ValidationError};
use didcomm::{PackEncryptedOptions, UnpackOptions};

/// Returns the default options for packing (encrypting) DIDComm messages.
///
/// The default configuration provides:
/// - Authenticated encryption (sender's identity is cryptographically proven)
/// - Forward secrecy through ephemeral key generation
/// - Standard DIDComm v2 encryption algorithms
///
/// # Default Settings
///
/// - **Algorithm**: ECDH-ES+A256KW (key wrapping with AES-256)
/// - **Encryption**: A256GCM (AES-256 in Galois/Counter Mode)
/// - **Forward Secrecy**: Enabled (ephemeral keys for each message)
/// - **Protect Sender**: Enabled (sender's ID is encrypted)
///
/// # When to Use
///
/// Use these defaults for standard DIDComm messaging where:
/// - You want the recipient to know who sent the message
/// - You need forward secrecy
/// - You're following DIDComm v2 best practices
///
/// # Example
///
/// ```
/// use navia_core::core::didcomm::encryption::default_pack_options;
///
/// let pack_options = default_pack_options();
/// // Options are ready to use for packing
/// ```
pub fn default_pack_options() -> PackEncryptedOptions {
    PackEncryptedOptions::default()
}

/// Returns the default options for unpacking (decrypting) DIDComm messages.
///
/// The default configuration:
/// - Expects authenticated encryption (verifies sender when present)
/// - Validates message integrity
/// - Resolves sender DIDs automatically
///
/// # Default Settings
///
/// - **Expect Decrypt By All Keys**: false (use specific recipient key)
/// - **Unwrap Re-Wrapped In Forward**: true (support message forwarding)
///
/// # When to Use
///
/// Use these defaults for standard DIDComm message reception where:
/// - You want to verify the sender's identity when provided
/// - You support message forwarding through mediators
/// - You're following DIDComm v2 best practices
///
/// # Example
///
/// ```
/// use navia_core::core::didcomm::encryption::default_unpack_options;
///
/// let unpack_options = default_unpack_options();
/// // Options are ready to use for unpacking
/// ```
pub fn default_unpack_options() -> UnpackOptions {
    UnpackOptions::default()
}

/// Validates parameters before packing a DIDComm message.
///
/// Ensures that the required fields are present and valid before attempting
/// to encrypt a message. This prevents cryptographic operations from failing
/// due to invalid inputs.
///
/// # Arguments
///
/// * `to` - The recipient's DID (must not be empty)
/// * `from` - Optional sender's DID (if provided, must not be empty)
///
/// # Returns
///
/// * `Ok(())` if all parameters are valid
/// * `Err(NaviaError::Validation)` if validation fails
///
/// # Validation Rules
///
/// 1. Recipient (`to`) must not be empty
/// 2. Sender (`from`), if provided, must not be empty
/// 3. DIDs are not validated for format here (done elsewhere)
///
/// # Errors
///
/// * `ValidationError::InvalidDid` - If `to` or `from` is empty
///
/// # Example
///
/// ```
/// use navia_core::core::didcomm::encryption::validate_pack_params;
///
/// // Valid authenticated encryption
/// assert!(validate_pack_params("did:peer:bob", Some("did:peer:alice")).is_ok());
///
/// // Valid anonymous encryption
/// assert!(validate_pack_params("did:peer:bob", None).is_ok());
///
/// // Invalid - empty recipient
/// assert!(validate_pack_params("", Some("did:peer:alice")).is_err());
/// ```
pub fn validate_pack_params(to: &str, from: Option<&str>) -> NaviaResult<()> {
    if to.is_empty() {
        return Err(NaviaError::Validation(ValidationError::InvalidDid {
            value: to.to_string(),
            reason: "Recipient 'to' cannot be empty".to_string(),
        }));
    }

    if let Some(f) = from {
        if f.is_empty() {
            return Err(NaviaError::Validation(ValidationError::InvalidDid {
                value: f.to_string(),
                reason: "Sender 'from' cannot be empty".to_string(),
            }));
        }
    }

    Ok(())
}

/// Validates an encrypted message before unpacking.
///
/// Performs basic structural validation to ensure the message is a valid
/// JWE (JSON Web Encryption) format before attempting decryption. This
/// prevents unnecessary cryptographic operations on malformed data.
///
/// # Arguments
///
/// * `msg` - The encrypted message string to validate
///
/// # Returns
///
/// * `Ok(())` if the message appears to be valid JWE
/// * `Err(NaviaError::Validation)` if validation fails
///
/// # Validation Rules
///
/// 1. Message must not be empty
/// 2. Message must contain dots (JWE compact format has 5 parts separated by dots)
/// 3. Basic structure check only - full JWE validation happens during unpacking
///
/// # Errors
///
/// * `ValidationError::InvalidMessageFormat` - If the message is empty or
///   doesn't have the expected JWE structure
///
/// # Note
///
/// This is a quick sanity check. Full cryptographic validation happens
/// during the actual unpacking process.
///
/// # Example
///
/// ```
/// use navia_core::core::didcomm::encryption::validate_unpack_message;
///
/// // Valid JWE compact format
/// let jwe = "eyJhbGc.eyJlbmM.SomeIV.EncryptedContent.AuthTag";
/// assert!(validate_unpack_message(jwe).is_ok());
///
/// // Invalid - empty message
/// assert!(validate_unpack_message("").is_err());
///
/// // Invalid - not JWE format
/// assert!(validate_unpack_message("just plain text").is_err());
/// ```
pub fn validate_unpack_message(msg: &str) -> NaviaResult<()> {
    if msg.is_empty() {
        return Err(NaviaError::Validation(
            ValidationError::InvalidMessageFormat {
                details: "Message cannot be empty".to_string(),
            },
        ));
    }

    // Basic check for JWE structure (should have dots separating parts)
    if !msg.contains('.') {
        return Err(NaviaError::Validation(
            ValidationError::InvalidMessageFormat {
                details: "Invalid message format: expected JWE with dot separators".to_string(),
            },
        ));
    }

    Ok(())
}
