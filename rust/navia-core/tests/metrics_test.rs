//! Integration tests for error logging
//! 
//! Since we removed metrics collection and only log errors now,
//! these tests verify error logging functionality.

use navia_core::ffi::interface::DidComInterface;
use serde_json::Value;

#[test]
fn test_error_logging_initialization() {
    use tempfile::NamedTempFile;
    
    let interface = DidComInterface::new("unused".to_string());
    
    // Create temp file for error logs
    let temp_file = NamedTempFile::new().unwrap();
    interface.init_error_logging(temp_file.path().to_str().unwrap().to_string()).unwrap();
    
    // Enable error logging (should be on by default)
    interface.set_error_logging_enabled(true).unwrap();
    
    // Export should return valid JSON array (empty initially)
    let logs_json = interface.export_error_logs().unwrap();
    let logs: Vec<Value> = serde_json::from_str(&logs_json).unwrap();
    
    // Should be empty initially
    assert_eq!(logs.len(), 0);
}

#[test]
fn test_error_logging_can_be_disabled() {
    use tempfile::NamedTempFile;
    
    let interface = DidComInterface::new("unused".to_string());
    
    // Create temp file for error logs
    let temp_file = NamedTempFile::new().unwrap();
    interface.init_error_logging(temp_file.path().to_str().unwrap().to_string()).unwrap();
    
    // Disable error logging
    interface.set_error_logging_enabled(false).unwrap();
    
    // Clear any existing logs
    interface.clear_error_logs().unwrap();
    
    // Export should still work even when disabled
    let logs_json = interface.export_error_logs().unwrap();
    let logs: Vec<Value> = serde_json::from_str(&logs_json).unwrap();
    
    // Should be empty
    assert_eq!(logs.len(), 0);
}

#[test]
fn test_error_logs_export() {
    use tempfile::NamedTempFile;
    
    let interface = DidComInterface::new("unused".to_string());
    
    // Create temp file for error logs
    let temp_file = NamedTempFile::new().unwrap();
    interface.init_error_logging(temp_file.path().to_str().unwrap().to_string()).unwrap();
    
    // Export should return valid JSON
    let export = interface.export_error_logs().unwrap();
    let _data: Vec<Value> = serde_json::from_str(&export).unwrap();
}

#[test]
fn test_error_logs_clear() {
    use tempfile::NamedTempFile;
    
    let interface = DidComInterface::new("unused".to_string());
    
    // Create temp file for error logs
    let temp_file = NamedTempFile::new().unwrap();
    interface.init_error_logging(temp_file.path().to_str().unwrap().to_string()).unwrap();
    
    // Clear logs should not error
    interface.clear_error_logs().unwrap();
    
    // After clearing, export should show empty array
    let logs_json = interface.export_error_logs().unwrap();
    let logs: Vec<Value> = serde_json::from_str(&logs_json).unwrap();
    
    assert_eq!(logs.len(), 0);
}