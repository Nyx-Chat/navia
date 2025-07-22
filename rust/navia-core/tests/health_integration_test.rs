//! Integration tests for health check functionality

use navia_core::ffi::interface::DidComInterface;
use tempfile::TempDir;

#[test]
fn test_health_check_integration() {
    use futures::executor::block_on;
    
    // Create a temporary directory for the database
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db").to_string_lossy().to_string();
    
    // Create interface
    let interface = DidComInterface::new(db_path.clone());
    
    // Before opening, health check should indicate not healthy
    assert!(!block_on(interface.is_healthy()));
    
    // Open the database
    let seed = vec![0u8; 32];
    block_on(interface.open(db_path.clone(), seed)).unwrap();
    
    // Now it should be healthy
    assert!(block_on(interface.is_healthy()));
    
    // Get detailed health check
    let health_json = block_on(interface.check_health()).unwrap();
    let health: serde_json::Value = serde_json::from_str(&health_json).unwrap();
    
    // Verify the structure
    assert_eq!(health["status"], "Healthy");
    assert!(health["components"].is_array());
    
    let components = health["components"].as_array().unwrap();
    assert!(components.len() >= 3); // Storage, Cryptography, RateLimiter
    
    // Check that all components are healthy
    for component in components {
        let name = component["name"].as_str().unwrap();
        let status = &component["status"];
        
        println!("Component {}: {:?}", name, status);
        
        // All should be healthy or degraded (rate limiter might be degraded)
        assert!(status == "Healthy" || status.as_object().unwrap().contains_key("Degraded"));
    }
    
    println!("Health check JSON: {}", serde_json::to_string_pretty(&health).unwrap());
}

#[test]
fn test_health_check_before_init() {
    use futures::executor::block_on;
    
    let interface = DidComInterface::new("unused_path".to_string());
    
    // Should not be healthy before initialization
    assert!(!block_on(interface.is_healthy()));
    
    // Detailed check should return an error
    let result = block_on(interface.check_health());
    assert!(result.is_err());
    
    if let Err(e) = result {
        println!("Expected error: {:?}", e);
    }
}