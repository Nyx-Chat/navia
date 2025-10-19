//! Unified error type that combines all specific errors
//!
//! Since we're building from scratch, we can use a clean error design
//! without legacy compatibility concerns.

use super::specific::{
    DidError, PackingError, ResolutionError, SerializationError, StorageError, UnpackingError,
    ValidationError,
};
use crate::core::metrics::METRICS;
use thiserror::Error;

/// Unified error type for all navia-core operations
#[derive(Debug, Clone, Error)]
pub enum NaviaError {
    /// Errors during message packing (encryption)
    #[error(transparent)]
    Packing(PackingError),

    /// Errors during message unpacking (decryption)
    #[error(transparent)]
    Unpacking(UnpackingError),

    /// Errors during DID operations
    #[error(transparent)]
    Did(DidError),

    /// Errors during storage operations
    #[error(transparent)]
    Storage(StorageError),

    /// Errors during validation
    #[error(transparent)]
    Validation(ValidationError),

    /// Errors during resolution operations
    #[error(transparent)]
    Resolution(#[from] ResolutionError),

    /// Errors during serialization/deserialization
    #[error(transparent)]
    Serialization(#[from] SerializationError),

    /// Errors from external libraries
    #[error("External error: {0}")]
    External(String),
}

/// Type alias for Results using NaviaError
pub type NaviaResult<T> = Result<T, NaviaError>;

impl From<PackingError> for NaviaError {
    fn from(err: PackingError) -> Self {
        let error = NaviaError::Packing(err);
        METRICS.log_error("packing", &error.to_string());
        error
    }
}

impl From<UnpackingError> for NaviaError {
    fn from(err: UnpackingError) -> Self {
        let error = NaviaError::Unpacking(err);
        METRICS.log_error("unpacking", &error.to_string());
        error
    }
}

impl From<StorageError> for NaviaError {
    fn from(err: StorageError) -> Self {
        let error = NaviaError::Storage(err);
        METRICS.log_error("storage", &error.to_string());
        error
    }
}

impl From<DidError> for NaviaError {
    fn from(err: DidError) -> Self {
        let error = NaviaError::Did(err);
        METRICS.log_error("did", &error.to_string());
        error
    }
}

impl From<ValidationError> for NaviaError {
    fn from(err: ValidationError) -> Self {
        let error = NaviaError::Validation(err);
        METRICS.log_error("validation", &error.to_string());
        error
    }
}

// Convenience conversions from common external errors
impl From<serde_json::Error> for NaviaError {
    fn from(err: serde_json::Error) -> Self {
        NaviaError::Serialization(SerializationError::JsonError {
            context: "json operation".to_string(),
            details: err.to_string(),
        })
    }
}

impl From<askar_storage::Error> for NaviaError {
    fn from(err: askar_storage::Error) -> Self {
        NaviaError::Storage(StorageError::OperationFailed {
            operation: "askar".to_string(),
            details: err.to_string(),
        })
    }
}

impl From<navia_messaging::error::Error> for NaviaError {
    fn from(err: navia_messaging::error::Error) -> Self {
        NaviaError::Storage(StorageError::OperationFailed {
            operation: "navia_messaging".to_string(),
            details: err.to_string(),
        })
    }
}
