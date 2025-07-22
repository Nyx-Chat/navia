//! Integration tests for configuration management

use navia_core::ffi::interface::DidComInterface;

#[test]
fn test_development_configuration() {
    let interface = DidComInterface::new("unused".to_string());
    
    // Configure for development
    interface.configure_development().unwrap();
    
    // Can't test much without exposing config, but at least it doesn't error
}

#[test] 
fn test_configuration_already_initialized() {
    // Skip if running with other tests that might have initialized config
    if std::env::var("CARGO_TEST_THREADS").unwrap_or_default() != "1" {
        return;
    }
    
    let interface = DidComInterface::new("unused".to_string());
    
    // First configuration should succeed
    interface.configure_development().unwrap();
    
    // Second configuration should fail
    let result = interface.configure_development();
    assert!(result.is_err());
}