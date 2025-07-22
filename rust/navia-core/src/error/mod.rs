//! Error handling module
//! 
//! This module defines error types for both the core domain
//! and FFI layer, along with conversions between them.

pub mod ffi;
pub mod specific;
pub mod unified;

// Re-export the unified error system
pub use self::unified::{NaviaError, NaviaResult};
pub use self::specific::{
    PackingError, UnpackingError, DidError, StorageError, ValidationError, SerializationError
};