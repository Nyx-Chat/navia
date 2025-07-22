//! Rate limiting tests
//! 
//! Tests that verify rate limiting is properly enforced.

use navia_core::ffi::{DidComInterface, DidCommError};
use navia_core::core::rate_limit::DID_GENERATION_LIMITER;
use tempfile::TempDir;
use futures::executor::block_on;

fn create_test_interface() -> (DidComInterface, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    
    let interface = DidComInterface::new(db_path.to_string_lossy().to_string());
    let seed = vec![42u8; 32];
    block_on(interface.open(db_path.to_string_lossy().to_string(), seed))
        .expect("Failed to open database");
    
    (interface, temp_dir)
}

#[test]
fn test_did_generation_rate_limit() {
    // This test verifies that rate limiting is enforced
    // The global rate limiter allows 10 DIDs per minute, but may have been used by other tests
    // So we just verify that rate limiting eventually kicks in
    
    let (interface, _temp_dir) = create_test_interface();
    
    // Try to generate many DIDs
    let mut generated_count = 0;
    let mut rate_limited = false;
    
    for i in 0..20 {
        let result = block_on(interface.generate_did(
            format!("https://example{}.com/didcomm", i),
            vec![]
        ));
        
        if result.is_ok() {
            generated_count += 1;
        } else {
            let err = result.unwrap_err();
            match err {
                DidCommError::DidGenerationError { message } if message.contains("Rate limit") => {
                    rate_limited = true;
                    break;
                }
                _ => panic!("Unexpected error: {:?}", err),
            }
        }
    }
    
    assert!(rate_limited, "Rate limiting should have kicked in");
    assert!(generated_count > 0, "Should generate at least one DID before rate limit");
    assert!(generated_count <= 10, "Should not exceed global rate limit of 10 per minute");
}

#[test]
fn test_rate_limit_resets() {
    let (interface, _temp_dir) = create_test_interface();
    
    // Clear any previous rate limit data
    DID_GENERATION_LIMITER.clear();
    
    // Generate one DID
    let did1 = block_on(interface.generate_did(
        "https://example.com/didcomm".to_string(),
        vec![]
    )).expect("First DID should succeed");
    
    assert!(did1.starts_with("did:peer:"));
    
    // The rate limiter allows 10 per minute, so we should be able to generate more
    let did2 = block_on(interface.generate_did(
        "https://example2.com/didcomm".to_string(),
        vec![]
    )).expect("Second DID should succeed");
    
    assert!(did2.starts_with("did:peer:"));
    assert_ne!(did1, did2);
}