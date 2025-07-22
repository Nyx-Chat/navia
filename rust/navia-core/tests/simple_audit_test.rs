//! Simple audit logging test
//!
//! Just verifies that audit logging is integrated and doesn't crash.

use futures::executor::block_on;
use navia_core::core::audit::{init_audit_logger, ConsoleAuditLogger};
use navia_core::ffi::DidComInterface;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_audit_logging_integration() {
    // Initialize console logger (output goes to stderr)
    let logger = Arc::new(ConsoleAuditLogger::new("[TEST]"));
    init_audit_logger(logger);

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(db_path.to_string_lossy().to_string());
    let seed = vec![42u8; 32];

    // This should trigger audit logging for database opening
    block_on(interface.open(db_path.to_string_lossy().to_string(), seed))
        .expect("Failed to open database");

    // This should trigger audit logging for DID generation
    let did = block_on(interface.generate_did("https://example.com/didcomm".to_string(), vec![]))
        .expect("Failed to generate DID");

    assert!(did.starts_with("did:peer:"));

    // This should trigger audit logging for storage operation
    block_on(interface.insert(
        "test_category".to_string(),
        "test_key".to_string(),
        "test_value".to_string(),
    ))
    .expect("Failed to insert");

    // Test validation failure logging
    let result = block_on(interface.generate_did("not-a-valid-uri".to_string(), vec![]));
    assert!(result.is_err());

    // If we got here without crashing, audit logging is working
    eprintln!("Audit logging integration test completed successfully");
}
