//! Error handling module
//! 
//! This module defines error types for both the core domain
//! and FFI layer, along with conversions between them.

pub mod core;
pub mod ffi;

pub use self::core::{CoreError, CoreResult};
pub use self::ffi::DidCommError;

// Compatibility types for old code
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("DID not resolved: {0}")]
    DIDNotResolved(String),
    #[error("DID URL not found: {0}")]
    DIDUrlNotFound(String),
    #[error("Malformed: {0}")]
    Malformed(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    DIDNotResolved,
    DIDUrlNotFound,
    Malformed,
    InvalidState,
    IoError,
}

impl Error {
    pub fn new(kind: ErrorKind, err: impl std::fmt::Display) -> Self {
        match kind {
            ErrorKind::DIDNotResolved => Error::DIDNotResolved(err.to_string()),
            ErrorKind::DIDUrlNotFound => Error::DIDUrlNotFound(err.to_string()),
            ErrorKind::Malformed => Error::Malformed(err.to_string()),
            ErrorKind::InvalidState => Error::InvalidState(err.to_string()),
            ErrorKind::IoError => Error::IoError(err.to_string()),
        }
    }
    
    pub fn kind(&self) -> ErrorKind {
        match self {
            Error::DIDNotResolved(_) => ErrorKind::DIDNotResolved,
            Error::DIDUrlNotFound(_) => ErrorKind::DIDUrlNotFound,
            Error::Malformed(_) => ErrorKind::Malformed,
            Error::InvalidState(_) => ErrorKind::InvalidState,
            Error::IoError(_) => ErrorKind::IoError,
            Error::Other(_) => ErrorKind::InvalidState,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// Conversions
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Malformed(err.to_string())
    }
}

impl From<askar_storage::Error> for Error {
    fn from(err: askar_storage::Error) -> Self {
        Error::InvalidState(err.to_string())
    }
}