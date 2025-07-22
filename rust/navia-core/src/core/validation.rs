//! Input validation for security-sensitive operations
//! 
//! This module provides validation functions to ensure input data
//! meets security requirements before processing.

use crate::core::audit::{audit_log, SecurityEvent};
use crate::error::{NaviaResult, ValidationError};
use std::time::SystemTime;

/// Maximum allowed length for a DID string
pub const MAX_DID_LENGTH: usize = 2048;

/// Maximum allowed length for a message body
pub const MAX_MESSAGE_BODY_LENGTH: usize = 1_048_576; // 1MB

/// Maximum allowed length for a URI
pub const MAX_URI_LENGTH: usize = 2048;

/// Maximum number of recipients allowed in a single message
pub const MAX_RECIPIENTS: usize = 100;

/// Maximum allowed seed length (in bytes)
pub const MAX_SEED_LENGTH: usize = 1024;

/// Validates a DID string
/// 
/// Ensures the DID:
/// - Is not empty
/// - Does not exceed maximum length
/// - Starts with "did:"
/// - Has a method name after "did:"
/// - Contains only valid characters
pub fn validate_did(did: &str) -> NaviaResult<()> {
    if did.is_empty() {
        let err = ValidationError::InvalidDid {
            value: did.to_string(),
            reason: "DID cannot be empty".to_string(),
        };
        audit_log(
            SecurityEvent::ValidationFailed {
                input_type: "DID".to_string(),
                reason: err.to_string(),
                timestamp: SystemTime::now(),
            },
            None,
        );
        return Err(err.into());
    }
    
    if did.len() > MAX_DID_LENGTH {
        return Err(ValidationError::InvalidDid {
            value: did.to_string(),
            reason: format!("exceeds maximum length of {} characters", MAX_DID_LENGTH),
        }.into());
    }
    
    if !did.starts_with("did:") {
        let err = ValidationError::InvalidDid {
            value: did.to_string(),
            reason: "must start with 'did:'".to_string(),
        };
        audit_log(
            SecurityEvent::ValidationFailed {
                input_type: "DID".to_string(),
                reason: err.to_string(),
                timestamp: SystemTime::now(),
            },
            None,
        );
        return Err(err.into());
    }
    
    // Check that there's a method name after "did:"
    if did.len() <= 4 || !did[4..].contains(':') {
        return Err(ValidationError::InvalidDid {
            value: did.to_string(),
            reason: "must have a method name (e.g., did:peer:...)".to_string(),
        }.into());
    }
    
    // Check for valid characters (alphanumeric, :, -, _, ., #, ?, =, &, /, %)
    if !did.chars().all(|c| {
        c.is_alphanumeric() || 
        matches!(c, ':' | '-' | '_' | '.' | '#' | '?' | '=' | '&' | '/' | '%')
    }) {
        return Err(ValidationError::InvalidDid {
            value: did.to_string(),
            reason: "contains invalid characters".to_string(),
        }.into());
    }
    
    Ok(())
}

/// Validates a URI string
/// 
/// Ensures the URI:
/// - Is not empty
/// - Does not exceed maximum length
/// - Starts with http:// or https://
pub fn validate_uri(uri: &str) -> NaviaResult<()> {
    if uri.is_empty() {
        return Err(ValidationError::InvalidUri {
            value: uri.to_string(),
            reason: "URI cannot be empty".to_string(),
        }.into());
    }
    
    if uri.len() > MAX_URI_LENGTH {
        return Err(ValidationError::InvalidUri {
            value: uri.to_string(),
            reason: format!("exceeds maximum length of {} characters", MAX_URI_LENGTH),
        }.into());
    }
    
    if !uri.starts_with("http://") && !uri.starts_with("https://") {
        return Err(ValidationError::InvalidUri {
            value: uri.to_string(),
            reason: "must start with http:// or https://".to_string(),
        }.into());
    }
    
    Ok(())
}

/// Validates a message body
/// 
/// Ensures the message body:
/// - Does not exceed maximum length (configurable)
pub fn validate_message_body(body: &str) -> NaviaResult<()> {
    let config = crate::core::config::get_config().read();
    let max_size = config.max_message_size;
    drop(config); // Release the lock
    
    if body.len() > max_size {
        return Err(ValidationError::SizeExceeded {
            name: "message body".to_string(),
            max_size,
        }.into());
    }
    
    Ok(())
}

/// Validates a seed for database encryption
/// 
/// Ensures the seed:
/// - Is not empty
/// - Has a reasonable length (between 16 and MAX_SEED_LENGTH bytes)
pub fn validate_seed(seed: &[u8]) -> NaviaResult<()> {
    if seed.is_empty() {
        return Err(ValidationError::InvalidSeed {
            reason: "Seed cannot be empty".to_string(),
        }.into());
    }
    
    let config = crate::core::config::get_config().read();
    let min_length = config.min_seed_length;
    drop(config); // Release the lock
    
    if seed.len() < min_length {
        return Err(ValidationError::InvalidSeed {
            reason: format!("Seed must be at least {} bytes", min_length),
        }.into());
    }
    
    if seed.len() > MAX_SEED_LENGTH {
        return Err(ValidationError::InvalidSeed {
            reason: format!("Seed exceeds maximum length of {} bytes", MAX_SEED_LENGTH),
        }.into());
    }
    
    Ok(())
}

/// Validates a list of recipients
/// 
/// Ensures:
/// - The list is not empty
/// - Does not exceed maximum number of recipients
/// - Each recipient DID is valid
pub fn validate_recipients(recipients: &[String]) -> NaviaResult<()> {
    if recipients.is_empty() {
        return Err(ValidationError::InvalidDid {
            value: "recipients list".to_string(),
            reason: "At least one recipient is required".to_string(),
        }.into());
    }
    
    if recipients.len() > MAX_RECIPIENTS {
        return Err(ValidationError::SizeExceeded {
            name: "recipients list".to_string(),
            max_size: MAX_RECIPIENTS,
        }.into());
    }
    
    for recipient in recipients {
        validate_did(recipient)?;
    }
    
    Ok(())
}

/// Validates a storage key
/// 
/// Ensures the key:
/// - Is not empty
/// - Contains only valid characters (alphanumeric, -, _)
/// - Does not exceed a reasonable length
pub fn validate_storage_key(key: &str) -> NaviaResult<()> {
    if key.is_empty() {
        return Err(ValidationError::InvalidStorageKey {
            value: key.to_string(),
            reason: "Storage key cannot be empty".to_string(),
        }.into());
    }
    
    if key.len() > 255 {
        return Err(ValidationError::InvalidStorageKey {
            value: key.to_string(),
            reason: "exceeds maximum length of 255 characters".to_string(),
        }.into());
    }
    
    if !key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(ValidationError::InvalidStorageKey {
            value: key.to_string(),
            reason: "contains invalid characters (only alphanumeric, -, _ allowed)".to_string(),
        }.into());
    }
    
    Ok(())
}

/// Validates a storage category
/// 
/// Ensures the category:
/// - Is not empty
/// - Contains only valid characters (alphanumeric, -, _)
/// - Does not exceed a reasonable length
pub fn validate_storage_category(category: &str) -> NaviaResult<()> {
    if category.is_empty() {
        return Err(ValidationError::InvalidStorageKey {
            value: category.to_string(),
            reason: "Storage category cannot be empty".to_string(),
        }.into());
    }
    
    if category.len() > 64 {
        return Err(ValidationError::InvalidStorageKey {
            value: category.to_string(),
            reason: "exceeds maximum length of 64 characters".to_string(),
        }.into());
    }
    
    if !category.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(ValidationError::InvalidStorageKey {
            value: category.to_string(),
            reason: "contains invalid characters (only alphanumeric, -, _ allowed)".to_string(),
        }.into());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_did() {
        assert!(validate_did("did:peer:123abc").is_ok());
        assert!(validate_did("did:example:123#key-1").is_ok());
        assert!(validate_did("did:web:example.com").is_ok());
    }
    
    #[test]
    fn test_invalid_did() {
        assert!(validate_did("").is_err());
        assert!(validate_did("not-a-did").is_err());
        assert!(validate_did("did:").is_err());
        assert!(validate_did("did:test:123<script>").is_err());
        
        let long_did = format!("did:peer:{}", "a".repeat(MAX_DID_LENGTH));
        assert!(validate_did(&long_did).is_err());
    }
    
    #[test]
    fn test_valid_uri() {
        assert!(validate_uri("https://example.com").is_ok());
        assert!(validate_uri("http://localhost:8080/path").is_ok());
    }
    
    #[test]
    fn test_invalid_uri() {
        assert!(validate_uri("").is_err());
        assert!(validate_uri("not-a-uri").is_err());
        assert!(validate_uri("ftp://example.com").is_err());
    }
    
    #[test]
    fn test_valid_seed() {
        assert!(validate_seed(&vec![0u8; 32]).is_ok());
        assert!(validate_seed(&vec![0u8; 16]).is_ok());
        assert!(validate_seed(&vec![0u8; 64]).is_ok());
    }
    
    #[test]
    fn test_invalid_seed() {
        assert!(validate_seed(&vec![]).is_err());
        assert!(validate_seed(&vec![0u8; 8]).is_err());
        assert!(validate_seed(&vec![0u8; MAX_SEED_LENGTH + 1]).is_err());
    }
}