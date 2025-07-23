//! Core business logic - Pure Rust, no FFI dependencies
//!
//! This module contains the core DIDComm functionality without
//! any UniFFI or FFI-specific code. This allows the core logic
//! to be tested and used independently of the FFI layer.

pub mod audit;
pub mod constants;
pub mod didcomm;
pub mod health;
pub mod metrics;
pub mod storage;
pub mod validation;

// Re-export commonly used types
pub use didcomm::handler::DidcommMessaging;
pub use storage::traits::{MessageStorage, SecretStorage};
