//! FFI/UniFFI layer - Thin wrapper around core functionality
//! 
//! This module contains all UniFFI-specific code including:
//! - UniFFI object definitions
//! - FFI-specific types
//! - Conversions between FFI and core types

pub mod interface;
pub mod types;
pub mod conversions;

pub use interface::DidComInterface;
pub use types::{DIDCommMessage, KeyValue, DidCommError};