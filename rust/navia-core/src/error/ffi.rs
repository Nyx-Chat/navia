//! FFI-specific error types
//!
//! Error types that are exposed through UniFFI to Kotlin/Swift.

use crate::error::NaviaError;

/// Error types for DIDComm operations exposed through FFI.
///
/// These errors provide meaningful information about failures while
/// being simple enough to map cleanly to Kotlin/Swift exceptions.
/// All errors contain a human-readable message describing the failure.
///
/// # Error Categories
///
/// * `DatabaseError` - Storage-related failures (encryption, I/O, permissions)
/// * `ParsingError` - JSON parsing or data format errors
/// * `DidGenerationError` - DID creation or resolution failures
/// * `PackingError` - Message encryption failures (missing keys, invalid recipient)
/// * `UnpackingError` - Message decryption failures (corruption, wrong recipient)
/// * `GeneralError` - Other errors (initialization, invalid state)
///
/// # FFI Mapping
///
/// These errors are automatically converted to native exceptions:
/// - Kotlin: `DidCommException` with specific subclasses
/// - Swift: `DidCommError` enum with associated values
///
/// # Examples
///
/// ```ignore
/// # use crate::error::ffi::DidCommError;
/// # fn example() -> Result<(), DidCommError> {
/// // Database errors
/// return Err(DidCommError::DatabaseError {
///     message: "Failed to open database: Permission denied".to_string()
/// });
///
/// // Parsing errors
/// return Err(DidCommError::ParsingError {
///     message: "Invalid JSON in message body".to_string()
/// });
///
/// // Packing errors
/// return Err(DidCommError::PackingError {
///     message: "Sender keys not found in storage".to_string()
/// });
/// # Ok(())
/// # }
/// ```
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum DidCommError {
    /// Database or storage operation failed
    #[error("Database error: {message}")]
    DatabaseError { message: String },

    /// Data parsing or serialization failed
    #[error("Parsing error: {message}")]
    ParsingError { message: String },

    /// DID generation or resolution failed
    #[error("DID generation error: {message}")]
    DidGenerationError { message: String },

    /// Message encryption/packing failed
    #[error("Packing error: {message}")]
    PackingError { message: String },

    /// Message decryption/unpacking failed
    #[error("Unpacking error: {message}")]
    UnpackingError { message: String },

    /// Input validation failed
    #[error("Validation error: {message}")]
    ValidationError { message: String },

    /// General errors not covered by specific categories
    #[error("General error: {message}")]
    GeneralError { message: String },
}

impl From<NaviaError> for DidCommError {
    fn from(err: NaviaError) -> Self {
        match err {
            NaviaError::Storage(_) => DidCommError::DatabaseError {
                message: err.to_string(),
            },
            NaviaError::Serialization(_) => DidCommError::ParsingError {
                message: err.to_string(),
            },
            NaviaError::Did(_) | NaviaError::Resolution(_) => DidCommError::DidGenerationError {
                message: err.to_string(),
            },
            NaviaError::Packing(_) => DidCommError::PackingError {
                message: err.to_string(),
            },
            NaviaError::Unpacking(_) => DidCommError::UnpackingError {
                message: err.to_string(),
            },
            NaviaError::Validation(_) => DidCommError::ValidationError {
                message: err.to_string(),
            },
            NaviaError::External(_) => DidCommError::GeneralError {
                message: err.to_string(),
            },
        }
    }
}

impl From<serde_json::Error> for DidCommError {
    fn from(err: serde_json::Error) -> Self {
        DidCommError::ParsingError {
            message: err.to_string(),
        }
    }
}
