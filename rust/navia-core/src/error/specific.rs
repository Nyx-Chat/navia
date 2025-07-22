//! Specific error types for detailed error handling
//! 
//! These provide more actionable error information than generic string errors.

use thiserror::Error;

/// Errors that can occur during message packing (encryption)
#[derive(Debug, Clone, Error)]
pub enum PackingError {
    /// Sender's private key not found in storage
    #[error("Sender key not found for DID: {did}")]
    SenderKeyNotFound { did: String },
    
    /// Recipient's public key could not be resolved
    #[error("Recipient unreachable - DID: {did}, reason: {reason}")]
    RecipientUnreachable { did: String, reason: String },
    
    /// Encryption operation failed
    #[error("Encryption failed: {details}")]
    EncryptionFailed { details: String },
    
    /// Message size exceeds maximum allowed
    #[error("Message too large: {size} bytes exceeds maximum of {max_size} bytes")]
    MessageTooLarge { size: usize, max_size: usize },
    
    /// Recipient key not found for DID
    #[error("Recipient key not found for DID: {did}")]
    RecipientKeyNotFound { did: String },
    
    /// Invalid recipient DID
    #[error("Invalid recipient DID: {did}")]
    InvalidRecipientDid { did: String },
    
    /// Encryption algorithm error
    #[error("Encryption failed with algorithm {algorithm}: {details}")]
    AlgorithmEncryptionFailed { algorithm: String, details: String },
}

/// Errors that can occur during message unpacking (decryption)
#[derive(Debug, Clone, Error)]
pub enum UnpackingError {
    /// No matching recipient key found for decryption
    #[error("No matching recipient key found: {details}")]
    RecipientKeyNotFound { details: String },
    
    /// Message signature verification failed
    #[error("Signature verification failed for sender: {sender}")]
    SignatureVerificationFailed { sender: String },
    
    /// Message format is invalid or corrupted
    #[error("Invalid message format: {details}")]
    InvalidMessageFormat { details: String },
    
    /// Decryption operation failed
    #[error("Decryption failed: {details}")]
    DecryptionFailed { details: String },
    
    /// Invalid signature
    #[error("Invalid signature from signer: {signer}")]
    InvalidSignature { signer: String },
    
    /// Malformed message
    #[error("Malformed message: {details}")]
    MalformedMessage { details: String },
    
    /// Decryption algorithm error
    #[error("Decryption failed with algorithm {algorithm}: {details}")]
    AlgorithmDecryptionFailed { algorithm: String, details: String },
}

/// Errors that can occur during DID operations
#[derive(Debug, Clone, Error)]
pub enum DidError {
    /// Failed to generate cryptographic keys
    #[error("Key generation failed: {details}")]
    KeyGenerationFailed { details: String },
    
    /// DID resolution failed
    #[error("Failed to resolve DID {did}: {reason}")]
    ResolutionFailed { did: String, reason: String },
    
    /// Invalid DID format
    #[error("Invalid DID format: {did}")]
    InvalidFormat { did: String },
    
    /// Rate limit exceeded for DID generation
    #[error("Rate limit exceeded: {details}")]
    RateLimitExceeded { details: String },
    
    /// DID generation failed
    #[error("DID generation failed: {details}")]
    GenerationFailed { details: String },
}

/// Errors that can occur during storage operations
#[derive(Debug, Clone, Error)]
pub enum StorageError {
    /// Database connection failed
    #[error("Database connection failed: {details}")]
    ConnectionFailed { details: String },
    
    /// Entry not found in storage
    #[error("Entry not found - category: {category}, key: {key}")]
    EntryNotFound { category: String, key: String },
    
    /// Storage operation failed
    #[error("Storage operation failed: {operation} - {details}")]
    OperationFailed { operation: String, details: String },
    
    /// Database is locked
    #[error("Database locked: {details}")]
    DatabaseLocked { details: String },
}

/// Errors that can occur during validation
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    /// Invalid DID format
    #[error("Invalid DID: {value} - {reason}")]
    InvalidDid { value: String, reason: String },
    
    /// Invalid URI format
    #[error("Invalid URI: {value} - {reason}")]
    InvalidUri { value: String, reason: String },
    
    /// Invalid storage key
    #[error("Invalid storage key: {value} - {reason}")]
    InvalidStorageKey { value: String, reason: String },
    
    /// Invalid seed
    #[error("Invalid seed: {reason}")]
    InvalidSeed { reason: String },
    
    /// Value exceeds size limit
    #[error("Value too large: {name} exceeds {max_size} bytes")]
    SizeExceeded { name: String, max_size: usize },
    
    /// Invalid message format
    #[error("Invalid message format: {details}")]
    InvalidMessageFormat { details: String },
}

// CoreError conversions removed - using NaviaError directly

/// Errors that can occur during resolution operations
#[derive(Debug, Clone, Error)]
pub enum ResolutionError {
    /// Resolver error
    #[error("Resolver error: {details}")]
    ResolverError { details: String },
    
    /// DID not found
    #[error("DID not found: {did}")]
    DidNotFound { did: String },
    
    /// Service endpoint not found
    #[error("Service endpoint not found for DID: {did}")]
    ServiceNotFound { did: String },
}

/// Errors that can occur during serialization operations
#[derive(Debug, Clone, Error)]
pub enum SerializationError {
    /// JSON serialization/deserialization error
    #[error("JSON error in {context}: {details}")]
    JsonError { context: String, details: String },
    
    /// Invalid data format
    #[error("Invalid data format: {details}")]
    InvalidFormat { details: String },
}