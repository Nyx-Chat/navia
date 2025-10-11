//! Navia - Secure DIDComm v2 messaging library for mobile applications.
//!
//! Navia provides a high-performance, secure implementation of the DIDComm v2
//! messaging protocol, designed specifically for mobile applications through
//! Kotlin and Swift bindings via UniFFI.
//!
//! # Features
//!
//! - **DIDComm v2 Protocol**: Full implementation of encrypted, authenticated messaging
//! - **Secure Storage**: Encrypted database using Askar with hardware-backed keys
//! - **Peer DID Support**: Generate and manage did:peer identifiers
//! - **Cross-Platform**: Native bindings for Kotlin (Android) and Swift (iOS)
//! - **Async/Await**: Modern async API for all I/O operations
//! - **Batch Operations**: Efficient bulk storage operations
//!
//! # Architecture
//!
//! The library is organized in layers:
//!
//! ```text
//! ┌─────────────────────────────────┐
//! │   Mobile App (Kotlin/Swift)     │
//! ├─────────────────────────────────┤
//! │        UniFFI Bindings          │
//! ├─────────────────────────────────┤
//! │      FFI Layer (types.rs)       │
//! ├─────────────────────────────────┤
//! │    Core Logic (handler.rs)      │
//! ├─────────────────────────────────┤
//! │   Infrastructure (storage.rs)   │
//! └─────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Kotlin Example
//!
//! ```kotlin
//! // Initialize
//! val didcomm = DidComInterface("")
//! didcomm.open("/data/navia.db", secureKeyGen())
//!
//! // Generate DID
//! val myDid = didcomm.generateDid("https://example.com/didcomm", listOf())
//!
//! // Send message
//! val message = DidCommMessage(
//!     id = UUID.randomUUID().toString(),
//!     msgType = "https://didcomm.org/basicmessage/2.0/message",
//!     body = """{"content": "Hello!"}""",
//!     from = myDid,
//!     to = listOf(recipientDid)
//! )
//! val encrypted = didcomm.pack(message, myDid, recipientDid)
//! ```
//!
//! # Security Considerations
//!
//! - **Encryption at Rest**: All stored data is encrypted using Askar
//! - **Key Management**: Private keys never leave secure storage
//! - **Hardware Security**: Integrates with Android Keystore / iOS Keychain
//! - **Forward Secrecy**: Supports DIDComm forward secrecy extensions
//!
//! # Performance
//!
//! - **Async I/O**: Non-blocking operations using Tokio
//! - **Batch Operations**: Minimize database transactions
//! - **Native Performance**: Rust core with minimal FFI overhead
//! - **16KB Alignment**: Optimized for Android 15+ requirements

// Internal modules
pub mod error;
pub mod core;

// FFI layer (only public interface)
pub mod ffi;

// Re-export the FFI interface for UniFFI
pub use ffi::interface::DidComInterface;
pub use ffi::types::DidCommError;

// Set up UniFFI scaffolding
uniffi::setup_scaffolding!();
