use affinidi_did_resolver_cache_sdk::errors::DIDCacheError;
use serde::Serialize;
use serde_json::error::Category;
use ssi_dids_core::InvalidDID;
use std::fmt;

#[derive(thiserror::Error, Debug, Copy, Clone, Eq, PartialEq, Serialize)]
pub enum ErrorKind {
    #[error("DID not resolved")]
    DIDNotResolved,

    #[error("DID URL not found")]
    DIDUrlNotFound,

    #[error("Secret not found")]
    SecretNotFound,

    #[error("Malformed")]
    Malformed,

    #[error("IO error")]
    IoError,

    #[error("Invalid state")]
    InvalidState,

    #[error("No compatible crypto")]
    NoCompatibleCrypto,

    #[error("Unsupported crypto or method")]
    Unsupported,

    #[error("Illegal argument")]
    IllegalArgument,
}

#[derive(Debug, thiserror::Error)]
#[error("{kind}: {source:#}")]
pub struct Error {
    kind: ErrorKind,
    pub source: anyhow::Error,
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn new<E>(kind: ErrorKind, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind,
            source: anyhow::Error::new(source),
        }
    }

    pub fn msg<D>(kind: ErrorKind, msg: D) -> Self
    where
        D: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self {
            kind,
            source: anyhow::Error::msg(msg),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        match err.classify() {
            Category::Io | Category::Eof => Error::msg(ErrorKind::InvalidState, err.to_string()),
            _ => Error::msg(ErrorKind::Malformed, err.to_string()),
        }
    }
}

impl From<askar_storage::Error> for Error {
    fn from(err: askar_storage::Error) -> Self {
        Error::new(ErrorKind::InvalidState, err)
    }
}

impl From<DIDCacheError> for Error {
    fn from(err: DIDCacheError) -> Self {
        Error::new(ErrorKind::InvalidState, err)
    }
}

impl<T: Send + Sync + fmt::Debug + fmt::Display + 'static> From<InvalidDID<T>> for Error {
    fn from(err: InvalidDID<T>) -> Self {
        Error::new(ErrorKind::DIDNotResolved, err)
    }
}

pub fn err_msg<D>(kind: ErrorKind, msg: D) -> Error
where
    D: fmt::Display + fmt::Debug + Send + Sync + 'static,
{
    Error::msg(kind, msg)
}

pub type Result<T> = std::result::Result<T, Error>;
