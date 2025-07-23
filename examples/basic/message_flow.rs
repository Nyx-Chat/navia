//! Basic message flow example
//!
//! This example demonstrates the complete flow of:
//! 1. Creating a DIDComm interface
//! 2. Generating DIDs
//! 3. Packing (encrypting) a message
//! 4. Unpacking (decrypting) a message

use navia_core::ffi::interface::DidComInterface;
use navia_core::ffi::types::DIDCommMessage;
use std::error::Error;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a temporary directory for the database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");

    // Create interface
    let interface = DidComInterface::new(db_path.to_string_lossy().to_string());

    // Initialize with a secure seed (use proper randomness in production!)
    let seed = vec![42u8; 32];
    interface
        .open(db_path.to_string_lossy().to_string(), seed)
        .await?;

    // Generate DIDs for Alice and Bob
    let alice_did = interface
        .generate_did(
            "https://alice.example.com/didcomm".to_string(),
            vec![], // No routing keys
        )
        .await?;
    println!("Alice's DID: {alice_did}");

    let bob_did = interface
        .generate_did("https://bob.example.com/didcomm".to_string(), vec![])
        .await?;
    println!("Bob's DID: {bob_did}");

    // Create a message from Alice to Bob
    let message = DIDCommMessage {
        id: "msg-123".to_string(),
        msg_type: "https://didcomm.org/basicmessage/2.0/message".to_string(),
        body: r#"{"content": "Hello Bob! This is a secure message."}"#.to_string(),
        from: Some(alice_did.clone()),
        to: vec![bob_did.clone()],
    };

    // Pack (encrypt) the message
    println!("\nPacking message...");
    let packed = interface
        .pack(message.clone(), alice_did.clone(), bob_did.clone())
        .await?;
    println!("Packed message length: {} bytes", packed.len());

    // Unpack (decrypt) the message
    println!("\nUnpacking message...");
    let unpacked = interface.unpack(packed).await?;

    println!("Unpacked message:");
    println!("  ID: {}", unpacked.id);
    println!("  Type: {}", unpacked.msg_type);
    println!("  From: {:?}", unpacked.from);
    println!("  To: {:?}", unpacked.to);
    println!("  Body: {}", unpacked.body);

    Ok(())
}
