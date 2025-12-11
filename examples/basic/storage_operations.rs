//! Storage operations example
//!
//! Demonstrates how to use the key-value storage for:
//! - Storing individual items
//! - Batch operations for better performance
//! - Updating and retrieving data

use navia_core::ffi::interface::DidComInterface;
use navia_core::ffi::types::KeyValue;
use std::error::Error;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("storage.db");

    let interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:test-mediator".to_string(),
    );
    let seed = vec![42u8; 32];
    interface
        .open(db_path.to_string_lossy().to_string(), seed)
        .await?;

    // Single insert
    println!("Storing individual contacts...");
    interface
        .insert(
            "contacts".to_string(),
            "alice".to_string(),
            r#"{"name": "Alice", "did": "did:peer:alice123"}"#.to_string(),
        )
        .await?;

    // Batch insert for better performance
    println!("\nBatch inserting multiple contacts...");
    let contacts = vec![
        KeyValue {
            key: "bob".to_string(),
            value: r#"{"name": "Bob", "did": "did:peer:bob456"}"#.to_string(),
            metadata: Some("verified".to_string()),
        },
        KeyValue {
            key: "charlie".to_string(),
            value: r#"{"name": "Charlie", "did": "did:peer:charlie789"}"#.to_string(),
            metadata: None,
        },
    ];
    interface
        .insert_batch("contacts".to_string(), contacts)
        .await?;

    // Retrieve data
    println!("\nRetrieving Alice's contact:");
    let alice_data = interface
        .get("contacts".to_string(), "alice".to_string())
        .await?;
    if !alice_data.is_empty() {
        println!("  Value: {alice_data}");
    }

    // Batch retrieve
    println!("\nBatch retrieving contacts:");
    let keys = vec![
        "alice".to_string(),
        "bob".to_string(),
        "charlie".to_string(),
    ];
    let results = interface.get_batch("contacts".to_string(), keys).await?;

    for kv in results {
        println!("  Contact {}: {}", kv.key, kv.value);
        if let Some(meta) = kv.metadata {
            println!("    Metadata: {meta}");
        }
    }

    // Update existing data
    println!("\nUpdating Bob's contact...");
    interface
        .update(
            "contacts".to_string(),
            "bob".to_string(),
            r#"{"name": "Bob Smith", "did": "did:peer:bob456", "updated": true}"#.to_string(),
        )
        .await?;

    Ok(())
}
