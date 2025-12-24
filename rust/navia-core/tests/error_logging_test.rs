//! Test that errors are properly logged to metrics file

use navia_core::ffi::interface::DidComInterface;
use std::fs;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_errors_are_logged_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(
        db_path.to_str().unwrap().to_string(),
        "did:peer:test-mediator".to_string(),
    );

    // Create temp file for metrics
    let temp_file = NamedTempFile::new().unwrap();
    let metrics_path = temp_file.path().to_str().unwrap().to_string();
    interface.init_error_logging(metrics_path.clone()).unwrap();

    // Initialize the interface
    let seed = vec![0u8; 32];
    futures::executor::block_on(async {
        interface
            .open(db_path.to_str().unwrap().to_string(), seed)
            .await
            .unwrap();
    });

    // Try to unpack an invalid message - this should error and log
    let result = futures::executor::block_on(async {
        interface
            .unpack("invalid json not a didcomm message".to_string())
            .await
    });

    // Should have failed
    assert!(result.is_err());
    println!("Error: {:?}", result.err());

    // Read the metrics file
    let contents = fs::read_to_string(&metrics_path).unwrap();

    // Debug: print what's in the file
    println!("Metrics file contents:\n{contents}");

    // Should contain an error entry
    assert!(contents.contains("unpacking"));

    // The error message should be logged
    assert!(contents.contains("Error") || contents.contains("error"));
}
