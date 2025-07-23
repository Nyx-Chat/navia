//! Storage API demonstration
//!
//! This example shows how to use the storage operations provided by
//! DidComInterface. For internal trait implementation details, see
//! the library source code.

use navia_core::ffi::interface::DidComInterface;
use navia_core::ffi::types::KeyValue;
use std::error::Error;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Storage API Example ===\n");

    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("storage.db");

    let interface = DidComInterface::new(db_path.to_string_lossy().to_string());
    let seed = vec![42u8; 32];
    interface
        .open(db_path.to_string_lossy().to_string(), seed)
        .await?;

    // Message storage operations
    println!("Storing individual items...");
    interface
        .insert(
            "contacts".to_string(),
            "alice".to_string(),
            r#"{"name": "Alice"}"#.to_string(),
        )
        .await?;
    interface
        .insert(
            "contacts".to_string(),
            "bob".to_string(),
            r#"{"name": "Bob"}"#.to_string(),
        )
        .await?;

    // Batch operations
    println!("\nBatch inserting...");
    let batch = vec![
        KeyValue {
            key: "charlie".to_string(),
            value: r#"{"name": "Charlie"}"#.to_string(),
            metadata: None,
        },
        KeyValue {
            key: "david".to_string(),
            value: r#"{"name": "David"}"#.to_string(),
            metadata: Some("colleague".to_string()),
        },
    ];
    interface
        .insert_batch("contacts".to_string(), batch)
        .await?;

    // Retrieve data
    println!("\nRetrieving data...");
    let alice = interface
        .get("contacts".to_string(), "alice".to_string())
        .await?;
    if !alice.is_empty() {
        println!("Alice: {alice}");
    }

    // Batch retrieve
    println!("\nBatch retrieving...");
    let keys = vec![
        "alice".to_string(),
        "bob".to_string(),
        "charlie".to_string(),
        "david".to_string(),
    ];
    let results = interface.get_batch("contacts".to_string(), keys).await?;

    for kv in results {
        println!("  {}: {}", kv.key, kv.value);
        if let Some(meta) = kv.metadata {
            println!("    Metadata: {meta}");
        }
    }

    // Update
    println!("\nUpdating Bob's data...");
    interface
        .update(
            "contacts".to_string(),
            "bob".to_string(),
            r#"{"name": "Bob Smith", "updated": true}"#.to_string(),
        )
        .await?;

    // Remove
    println!("\nRemoving Charlie...");
    interface
        .remove("contacts".to_string(), "charlie".to_string())
        .await?;

    // Verify removal
    let charlie = interface
        .get("contacts".to_string(), "charlie".to_string())
        .await?;
    println!("Charlie removed: {}", charlie.is_empty());

    println!("\nStorage operations completed!");

    Ok(())
}
