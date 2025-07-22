//! Error scenario tests
//!
//! These tests verify that the library handles error conditions gracefully
//! and returns appropriate error messages.

use futures::executor::block_on;
use navia_core::ffi::{DIDCommMessage, DidComInterface, DidCommError};
use tempfile::TempDir;
use zeroize::Zeroize;

fn create_test_interface() -> (DidComInterface, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(db_path.to_string_lossy().to_string());
    let mut seed = vec![42u8; 32];
    block_on(interface.open(db_path.to_string_lossy().to_string(), seed.clone()))
        .expect("Failed to open database");

    // Verify seed is zeroized in our local copy
    seed.zeroize();

    (interface, temp_dir)
}

#[test]
fn test_unpack_invalid_message() {
    let (interface, _temp_dir) = create_test_interface();

    // Try to unpack various invalid messages
    let invalid_messages = vec![
        "not json at all",
        "{}", // Valid JSON but not a DIDComm message
        r#"{"invalid": "structure"}"#,
        r#"{"ciphertext": "invalid base64!@#$"}"#,
    ];

    for invalid_msg in invalid_messages {
        let result = block_on(interface.unpack(invalid_msg.to_string()));

        assert!(
            result.is_err(),
            "Expected error for invalid message: {}",
            invalid_msg
        );

        match result.unwrap_err() {
            DidCommError::UnpackingError { message } => {
                assert!(!message.is_empty(), "Error message should not be empty");
            }
            _ => panic!("Expected UnpackingError for invalid message"),
        }
    }
}

#[test]
fn test_pack_without_recipient_keys() {
    let (interface, _temp_dir) = create_test_interface();

    // Generate a DID for sender
    let sender_did =
        block_on(interface.generate_did("https://sender.example.com/didcomm".to_string(), vec![]))
            .expect("Failed to generate sender DID");

    // Use a non-existent recipient DID
    let fake_recipient = "did:peer:nonexistent123";

    let message = DIDCommMessage {
        id: "test-123".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: "Hello".to_string(),
        from: Some(sender_did.clone()),
        to: vec![fake_recipient.to_string()],
    };

    // This should fail because we can't resolve the recipient's keys
    let result = block_on(interface.pack(message, sender_did, fake_recipient.to_string()));

    assert!(result.is_err());
    match result.unwrap_err() {
        DidCommError::PackingError { message } => {
            assert!(message.contains("resolve") || message.contains("DID"));
        }
        _ => panic!("Expected PackingError for unresolvable recipient"),
    }
}

#[test]
fn test_unpack_message_for_wrong_recipient() {
    let (alice_interface, _alice_dir) = create_test_interface();
    let (bob_interface, _bob_dir) = create_test_interface();
    let (charlie_interface, _charlie_dir) = create_test_interface();

    // Generate DIDs
    let alice_did = block_on(
        alice_interface.generate_did("https://alice.example.com/didcomm".to_string(), vec![]),
    )
    .expect("Failed to generate Alice's DID");

    let bob_did =
        block_on(bob_interface.generate_did("https://bob.example.com/didcomm".to_string(), vec![]))
            .expect("Failed to generate Bob's DID");

    let _charlie_did = block_on(
        charlie_interface.generate_did("https://charlie.example.com/didcomm".to_string(), vec![]),
    )
    .expect("Failed to generate Charlie's DID");

    // Create a message from Alice to Bob
    let message = DIDCommMessage {
        id: "private-message".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: "Secret message for Bob only".to_string(),
        from: Some(alice_did.clone()),
        to: vec![bob_did.clone()],
    };

    // Alice packs the message for Bob
    let packed_message = block_on(alice_interface.pack(message, alice_did, bob_did))
        .expect("Failed to pack message");

    // Charlie tries to unpack the message (should fail)
    let result = block_on(charlie_interface.unpack(packed_message.clone()));

    assert!(result.is_err());
    match result.unwrap_err() {
        DidCommError::UnpackingError { message } => {
            // The error should indicate that decryption failed
            assert!(!message.is_empty());
        }
        _ => panic!("Expected UnpackingError when wrong recipient tries to decrypt"),
    }

    // Bob should be able to unpack it successfully
    let unpacked = block_on(bob_interface.unpack(packed_message))
        .expect("Bob should be able to unpack the message");

    assert_eq!(unpacked.body, "Secret message for Bob only");
}

#[test]
fn test_database_errors() {
    // Test opening database with invalid seed
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(db_path.to_string_lossy().to_string());

    // Try with empty seed - should fail with validation error
    let result = block_on(interface.open(db_path.to_string_lossy().to_string(), vec![]));

    assert!(result.is_err());
    match result.unwrap_err() {
        DidCommError::ValidationError { message } => {
            assert!(message.contains("empty"));
        }
        _ => panic!("Expected ValidationError for empty seed"),
    }

    // Test with wrong seed length - should fail with validation error
    let interface2 = DidComInterface::new(db_path.to_string_lossy().to_string());
    let result = block_on(interface2.open(
        db_path.to_string_lossy().to_string(),
        vec![1, 2, 3], // Too short
    ));

    assert!(result.is_err());
    match result.unwrap_err() {
        DidCommError::ValidationError { message } => {
            assert!(message.contains("at least 16 bytes"));
        }
        _ => panic!("Expected ValidationError for short seed"),
    }
}

#[test]
fn test_storage_error_handling() {
    let (interface, _temp_dir) = create_test_interface();

    // Test getting non-existent value (should return empty string, not error)
    let result = block_on(interface.get(
        "non_existent_category".to_string(),
        "non_existent_key".to_string(),
    ))
    .expect("Get should not fail for non-existent keys");

    assert_eq!(result, "");

    // Test removing non-existent value
    // Note: Some storage implementations may return an error for non-existent keys
    let remove_result = block_on(interface.remove(
        "non_existent_category".to_string(),
        "non_existent_key".to_string(),
    ));

    // It's acceptable for remove to either succeed or fail with a specific error
    if remove_result.is_err() {
        match remove_result.unwrap_err() {
            DidCommError::DatabaseError { message } => {
                assert!(message.contains("not found") || message.contains("Entry"));
            }
            _ => panic!("Unexpected error type for non-existent key removal"),
        }
    }
}

#[test]
fn test_malformed_json_body_handling() {
    let (interface, _temp_dir) = create_test_interface();

    let did = block_on(interface.generate_did("https://example.com/didcomm".to_string(), vec![]))
        .expect("Failed to generate DID");

    // Test with malformed JSON that starts with { but isn't valid
    let malformed_json = r#"{"broken": "json", incomplete"#;

    let message = DIDCommMessage {
        id: "malformed-msg".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: malformed_json.to_string(),
        from: Some(did.clone()),
        to: vec![did.clone()],
    };

    // Should still pack successfully (body is treated as string)
    let packed = block_on(interface.pack(message, did.clone(), did.clone()))
        .expect("Should pack even with malformed JSON body");

    let unpacked = block_on(interface.unpack(packed)).expect("Should unpack successfully");

    // The malformed JSON should be preserved as a string
    assert_eq!(unpacked.body, malformed_json);
}

#[test]
fn test_sequential_did_generation() {
    let (interface, _temp_dir) = create_test_interface();

    // Generate multiple DIDs sequentially (since DidComInterface doesn't implement Clone)
    let mut dids = vec![];

    for i in 0..5 {
        let did =
            block_on(interface.generate_did(format!("https://example{}.com/didcomm", i), vec![]))
                .expect("Failed to generate DID");
        dids.push(did);
    }

    // Verify all DIDs are unique
    let unique_dids: std::collections::HashSet<_> = dids.iter().collect();
    assert_eq!(
        unique_dids.len(),
        dids.len(),
        "Generated DIDs should be unique"
    );

    // Verify all DIDs start with the correct prefix
    for did in &dids {
        assert!(
            did.starts_with("did:peer:"),
            "DID should start with did:peer:"
        );
    }
}
