//! Core domain error types
//! 
//! Error types used within the core business logic,
//! independent of any FFI concerns.

use std::fmt;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Clone)]
pub enum CoreError {
    Database(String),
    Parsing(String),
    DidGeneration(String),
    Packing(String),
    Unpacking(String),
    General(String),
    Storage(String),
    Resolution(String),
    Serialization(String),
    Validation(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Database(msg) => write!(f, "Database error: {}", msg),
            CoreError::Parsing(msg) => write!(f, "Parsing error: {}", msg),
            CoreError::DidGeneration(msg) => write!(f, "DID generation error: {}", msg),
            CoreError::Packing(msg) => write!(f, "Packing error: {}", msg),
            CoreError::Unpacking(msg) => write!(f, "Unpacking error: {}", msg),
            CoreError::General(msg) => write!(f, "General error: {}", msg),
            CoreError::Storage(msg) => write!(f, "Storage error: {}", msg),
            CoreError::Resolution(msg) => write!(f, "Resolution error: {}", msg),
            CoreError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            CoreError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for CoreError {}

// Conversions from external error types
impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        CoreError::Parsing(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for CoreError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        CoreError::General(err.to_string())
    }
}