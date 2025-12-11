//! Error handling patterns for Navia
//!
//! This example demonstrates how to properly handle different types of errors
//! that can occur when using the Navia library.

use navia_core::ffi::interface::DidComInterface;
use navia_core::ffi::types::DIDCommMessage;
use navia_core::DidCommError;
use std::error::Error;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("errors.db");

    let interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:test-mediator".to_string(),
    );

    // Example 1: Database initialization errors
    println!("=== Database Initialization Errors ===");

    // Try to open with invalid seed (too short)
    let short_seed = vec![1u8; 10]; // Too short!
    match interface
        .open(db_path.to_string_lossy().to_string(), short_seed)
        .await
    {
        Ok(_) => println!("Unexpected success"),
        Err(DidCommError::ValidationError { message }) => {
            println!("✓ Caught validation error: {message}");
        }
        Err(e) => println!("Unexpected error type: {e:?}"),
    }

    // Open with valid seed
    let valid_seed = vec![42u8; 32];
    interface
        .open(db_path.to_string_lossy().to_string(), valid_seed)
        .await?;

    // Example 2: DID generation errors
    println!("\n=== DID Generation Errors ===");

    // Invalid URI
    match interface
        .generate_did("not-a-valid-uri".to_string(), vec![])
        .await
    {
        Ok(_) => println!("Unexpected success"),
        Err(DidCommError::ValidationError { message }) => {
            println!("✓ Caught validation error: {message}");
        }
        Err(e) => println!("Unexpected error type: {e:?}"),
    }

    // Valid DID generation
    let alice_did = interface
        .generate_did("https://alice.example.com/didcomm".to_string(), vec![])
        .await?;

    // Example 3: Message packing errors
    println!("\n=== Message Packing Errors ===");

    // Empty recipient
    let bad_message = DIDCommMessage {
        id: "msg-1".to_string(),
        msg_type: "test".to_string(),
        body: "{}".to_string(),
        from: Some(alice_did.clone()),
        to: vec![], // Empty!
    };

    match interface
        .pack(
            bad_message,
            alice_did.clone(),
            vec![], // Empty recipient list!
        )
        .await
    {
        Ok(_) => println!("Unexpected success"),
        Err(DidCommError::ValidationError { message }) => {
            println!("✓ Caught validation error: {message}");
        }
        Err(e) => println!("Unexpected error type: {e:?}"),
    }

    // Example 4: Unpacking errors
    println!("\n=== Message Unpacking Errors ===");

    // Try to unpack invalid message
    match interface.unpack("not-a-jwe-message".to_string()).await {
        Ok(_) => println!("Unexpected success"),
        Err(DidCommError::UnpackingError { message }) => {
            println!("✓ Caught unpacking error: {message}");
        }
        Err(e) => println!("Unexpected error type: {e:?}"),
    }

    // Example 5: Storage errors
    println!("\n=== Storage Errors ===");

    // Try to get non-existent key
    let result = interface
        .get("contacts".to_string(), "unknown".to_string())
        .await?;
    if result.is_empty() {
        println!("✓ Correctly returned empty string for non-existent key");
    } else {
        println!("Unexpected value found: {result}");
    }

    println!("\n=== Error Handling Best Practices ===");
    println!("1. Always match on specific error types");
    println!("2. Log errors with context for debugging");
    println!("3. Provide user-friendly error messages");
    println!("4. Implement retry logic for transient errors");
    println!("5. Clean up resources in error paths");

    Ok(())
}
