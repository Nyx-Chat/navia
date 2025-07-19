//! Navia - DIDComm messaging library
//! 
//! This library provides DIDComm v2 messaging capabilities with
//! a clean architecture separating FFI concerns from core logic.

// Internal modules - the old structure is still here for now
// to ensure we don't break anything during refactoring
mod askardb;
mod error;

// New architecture modules
pub mod core;
pub mod ffi;
pub mod infrastructure;

// Re-export the FFI interface for UniFFI
pub use ffi::{DidComInterface, DidCommError};

// Set up UniFFI scaffolding
uniffi::setup_scaffolding!();