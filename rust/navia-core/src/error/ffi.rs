//! FFI-specific error types
//! 
//! Error types that are exposed through UniFFI to Kotlin/Swift.

use crate::error::core::CoreError;

/// Simplified error type for UniFFI
#[derive(Debug, thiserror::Error)]
#[derive(uniffi::Error)]
#[uniffi(flat_error)]
pub enum DidCommError {
    #[error("Database error: {message}")]
    DatabaseError { message: String },
    
    #[error("Parsing error: {message}")]
    ParsingError { message: String },
    
    #[error("DID generation error: {message}")]
    DidGenerationError { message: String },
    
    #[error("Packing error: {message}")]
    PackingError { message: String },
    
    #[error("Unpacking error: {message}")]
    UnpackingError { message: String },
    
    #[error("General error: {message}")]
    GeneralError { message: String },
}

impl From<CoreError> for DidCommError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::Database(msg) | CoreError::Storage(msg) => DidCommError::DatabaseError { message: msg },
            CoreError::Parsing(msg) | CoreError::Serialization(msg) => DidCommError::ParsingError { message: msg },
            CoreError::DidGeneration(msg) | CoreError::Resolution(msg) => DidCommError::DidGenerationError { message: msg },
            CoreError::Packing(msg) => DidCommError::PackingError { message: msg },
            CoreError::Unpacking(msg) => DidCommError::UnpackingError { message: msg },
            CoreError::General(msg) | CoreError::Validation(msg) => DidCommError::GeneralError { message: msg },
        }
    }
}

impl From<serde_json::Error> for DidCommError {
    fn from(err: serde_json::Error) -> Self {
        DidCommError::ParsingError { message: err.to_string() }
    }
}

impl From<crate::error::Error> for DidCommError {
    fn from(err: crate::error::Error) -> Self {
        match err.kind() {
            crate::error::ErrorKind::DIDNotResolved | crate::error::ErrorKind::DIDUrlNotFound => {
                DidCommError::DidGenerationError { message: err.to_string() }
            }
            crate::error::ErrorKind::Malformed => {
                DidCommError::ParsingError { message: err.to_string() }
            }
            crate::error::ErrorKind::InvalidState | crate::error::ErrorKind::IoError => {
                DidCommError::DatabaseError { message: err.to_string() }
            }
        }
    }
}