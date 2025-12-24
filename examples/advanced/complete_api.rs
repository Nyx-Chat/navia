//! Complete API demonstration
//!
//! This example demonstrates ALL operations available in the Navia API,
//! including those not covered in other examples.

use navia_core::ffi::interface::DidComInterface;
use navia_core::ffi::types::{DIDCommMessage, KeyValue};
use std::error::Error;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("complete.db");

    println!("=== Complete Navia API Demonstration ===\n");

    // 1. Constructor
    println!("1. Creating interface...");
    let interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:test-mediator".to_string(),
    );

    // 2. Open database
    println!("2. Opening database...");
    let seed = vec![42u8; 32];
    interface
        .open(db_path.to_string_lossy().to_string(), seed)
        .await?;

    // 3. Health check
    println!("3. Checking health...");
    let is_healthy = interface.is_healthy().await;
    println!("   System healthy: {is_healthy}");

    // 4. Generate DIDs
    println!("\n4. Generating DIDs...");
    let alice_did = interface
        .generate_did("https://alice.example.com/didcomm".to_string(), vec![])
        .await?;
    let bob_did = interface
        .generate_did("https://bob.example.com/didcomm".to_string(), vec![])
        .await?;
    println!("   Alice: {alice_did}");
    println!("   Bob: {bob_did}");

    // 5. Storage operations - Insert
    println!("\n5. Storage: Insert operations...");
    interface
        .insert(
            "settings".to_string(),
            "theme".to_string(),
            "dark".to_string(),
        )
        .await?;

    // 6. Storage - Get
    println!("6. Storage: Get operation...");
    let theme = interface
        .get("settings".to_string(), "theme".to_string())
        .await?;
    if !theme.is_empty() {
        println!("   Theme: {theme}");
    }

    // 7. Storage - Update
    println!("\n7. Storage: Update operation...");
    interface
        .update(
            "settings".to_string(),
            "theme".to_string(),
            "light".to_string(),
        )
        .await?;

    // 8. Storage - Batch insert
    println!("\n8. Storage: Batch insert...");
    let batch_items = vec![
        KeyValue {
            key: "language".to_string(),
            value: "en".to_string(),
            metadata: Some("default".to_string()),
        },
        KeyValue {
            key: "notifications".to_string(),
            value: "enabled".to_string(),
            metadata: None,
        },
        KeyValue {
            key: "sync".to_string(),
            value: "auto".to_string(),
            metadata: Some("network".to_string()),
        },
    ];
    interface
        .insert_batch("settings".to_string(), batch_items)
        .await?;

    // 9. Storage - Batch get
    println!("\n9. Storage: Batch get...");
    let keys = vec![
        "theme".to_string(),
        "language".to_string(),
        "notifications".to_string(),
        "sync".to_string(),
    ];
    let results = interface.get_batch("settings".to_string(), keys).await?;
    for kv in results {
        println!("   Key: {}, Value: {}", kv.key, kv.value);
        if let Some(meta) = kv.metadata {
            println!("   Metadata: {meta}");
        }
    }

    // 10. Message operations - Pack
    println!("\n10. Packing a message...");
    let message = DIDCommMessage {
        id: "msg-complete-demo".to_string(),
        msg_type: "https://example.org/protocols/demo/1.0/message".to_string(),
        body: r#"{"content": "Testing all APIs", "timestamp": "2024-01-01T12:00:00Z"}"#.to_string(),
        from: Some(alice_did.clone()),
        to: vec![bob_did.clone()],
    };
    let packed = interface
        .pack(message.clone(), alice_did.clone(), vec![bob_did.clone()])
        .await?;
    println!("   Packed length: {} bytes", packed.len());

    // 11. Message operations - Unpack
    println!("\n11. Unpacking the message...");
    let unpacked = interface.unpack(packed).await?;
    println!("   Message ID: {}", unpacked.id);
    println!("   Type: {}", unpacked.msg_type);

    // 12. Storage - Remove
    println!("\n12. Storage: Remove operation...");
    interface
        .remove("settings".to_string(), "sync".to_string())
        .await?;
    let removed = interface
        .get("settings".to_string(), "sync".to_string())
        .await?;
    println!("   'sync' key removed: {}", removed.is_empty());

    // 13. Error log export (if any errors occurred)
    println!("\n13. Exporting error logs...");
    let error_logs = interface.export_error_logs()?;
    println!("   Error logs: {error_logs}");

    // 14. Clear error logs
    println!("\n14. Clearing error logs...");
    interface.clear_error_logs()?;

    // 15. Final health check
    println!("\n15. Final health check...");
    let final_health = interface.is_healthy().await;
    println!("   System still healthy: {final_health}");

    println!("\n=== All API operations completed successfully! ===");

    Ok(())
}
