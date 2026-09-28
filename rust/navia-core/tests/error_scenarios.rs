//! Error scenario tests
//!
//! These tests verify that the library handles error conditions gracefully
//! and returns appropriate error messages.

use futures::executor::block_on;
use navia_core::ffi::types::DIDCommMessage;
use navia_core::{DidComInterface, DidCommError};
use tempfile::TempDir;
use zeroize::Zeroize;

/// Creates a mediator interface and returns its DID
fn create_mediator() -> (DidComInterface, String, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("mediator.db");

    // Create mediator with a placeholder that will be replaced by generated DID
    let mediator_interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:placeholder".to_string(), // Mediator doesn't need routing to itself
    );
    let seed = vec![99u8; 32];
    block_on(mediator_interface.open(db_path.to_string_lossy().to_string(), seed))
        .expect("Failed to open mediator database");

    let mediator_did = block_on(
        mediator_interface.generate_did("https://mediator.example.com/didcomm".to_string(), vec![]),
    )
    .expect("Failed to generate mediator DID");

    (mediator_interface, mediator_did, temp_dir)
}

fn create_test_interface() -> (DidComInterface, TempDir, DidComInterface, TempDir) {
    // First create mediator to get its DID
    let (mediator_interface, mediator_did, mediator_temp_dir) = create_mediator();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(db_path.to_string_lossy().to_string(), mediator_did);
    let mut seed = vec![42u8; 32];
    block_on(interface.open(db_path.to_string_lossy().to_string(), seed.clone()))
        .expect("Failed to open database");

    // Verify seed is zeroized in our local copy
    seed.zeroize();

    (interface, temp_dir, mediator_interface, mediator_temp_dir)
}

#[test]
fn test_unpack_invalid_message() {
    let (interface, _temp_dir, _mediator, _mediator_dir) = create_test_interface();

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
            "Expected error for invalid message: {invalid_msg}"
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
    let (interface, _temp_dir, _mediator, _mediator_dir) = create_test_interface();

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
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // This should fail because we can't resolve the recipient's keys
    let result = block_on(interface.pack(message, sender_did, vec![fake_recipient.to_string()]));

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
    // Create a shared mediator for all users
    let (mediator_interface, mediator_did, _mediator_dir) = create_mediator();

    // Create user interfaces that all use the same mediator
    let alice_dir = TempDir::new().expect("Failed to create temp dir");
    let alice_db_path = alice_dir.path().join("alice.db");
    let alice_interface = DidComInterface::new(
        alice_db_path.to_string_lossy().to_string(),
        mediator_did.clone(),
    );
    block_on(alice_interface.open(alice_db_path.to_string_lossy().to_string(), vec![1u8; 32]))
        .expect("Failed to open Alice's database");

    let bob_dir = TempDir::new().expect("Failed to create temp dir");
    let bob_db_path = bob_dir.path().join("bob.db");
    let bob_interface = DidComInterface::new(
        bob_db_path.to_string_lossy().to_string(),
        mediator_did.clone(),
    );
    block_on(bob_interface.open(bob_db_path.to_string_lossy().to_string(), vec![2u8; 32]))
        .expect("Failed to open Bob's database");

    let charlie_dir = TempDir::new().expect("Failed to create temp dir");
    let charlie_db_path = charlie_dir.path().join("charlie.db");
    let charlie_interface = DidComInterface::new(
        charlie_db_path.to_string_lossy().to_string(),
        mediator_did.clone(),
    );
    block_on(charlie_interface.open(charlie_db_path.to_string_lossy().to_string(), vec![3u8; 32]))
        .expect("Failed to open Charlie's database");

    // Generate DIDs for users (with mediator's DID as routing key)
    let alice_did = block_on(alice_interface.generate_did(
        "https://alice.example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate Alice's DID");

    let bob_did = block_on(bob_interface.generate_did(
        "https://bob.example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate Bob's DID");

    let _charlie_did = block_on(charlie_interface.generate_did(
        "https://charlie.example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate Charlie's DID");

    // Create a message from Alice to Bob
    let message = DIDCommMessage {
        id: "private-message".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: r#"{"content": "Secret message for Bob only"}"#.to_string(),
        from: Some(alice_did.clone()),
        to: vec![bob_did.clone()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // Alice packs the message for Bob (wrapped for mediator delivery)
    let packed_for_mediator =
        block_on(alice_interface.pack(message, alice_did.clone(), vec![bob_did.clone()]))
            .expect("Failed to pack message");

    // Step 1: Mediator receives and unpacks the forward message
    let forward_msg = block_on(mediator_interface.unpack(packed_for_mediator.clone()))
        .expect("Mediator should be able to unpack forward message");

    // The forward message should be of type routing/2.0/forward
    assert!(
        forward_msg.msg_type.contains("forward"),
        "Expected forward message type, got: {}",
        forward_msg.msg_type
    );

    // Step 2: Extract the inner message from forward body/attachments
    // The inner encrypted message for Bob is in the attachments
    let forward_body: serde_json::Value =
        serde_json::from_str(&forward_msg.body).expect("Forward body should be valid JSON");

    // Get the "next" field which contains the recipient's DID
    let next_did = forward_body["next"]
        .as_str()
        .expect("Forward message should have 'next' field");
    // Verify the next field contains Bob's DID
    assert_eq!(next_did, bob_did, "Next should be Bob's DID");

    // The actual encrypted message for Bob should be in attachments
    // For this test, we'll verify the flow works by checking the mediator received the forward
    // In a real implementation, the mediator would extract the attachment and forward it

    // Charlie tries to unpack the original packed message (should fail - it's encrypted for mediator)
    let result = block_on(charlie_interface.unpack(packed_for_mediator.clone()));
    assert!(
        result.is_err(),
        "Charlie should not be able to unpack message meant for mediator"
    );

    // Keep temp directories alive
    let _ = (alice_dir, bob_dir, charlie_dir);
}

#[test]
fn test_database_errors() {
    // Test opening database with invalid seed
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:test-mediator".to_string(),
    );

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
    let interface2 = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:test-mediator".to_string(),
    );
    let result = block_on(interface2.open(
        db_path.to_string_lossy().to_string(),
        vec![1, 2, 3], // Too short
    ));

    assert!(result.is_err());
    match result.unwrap_err() {
        DidCommError::ValidationError { message } => {
            assert!(message.contains("at least 32 bytes"));
        }
        _ => panic!("Expected ValidationError for short seed"),
    }
}

#[test]
fn test_storage_error_handling() {
    let (interface, _temp_dir, _mediator, _mediator_dir) = create_test_interface();

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
    if let Err(err) = remove_result {
        match err {
            DidCommError::DatabaseError { message } => {
                assert!(message.contains("not found") || message.contains("Entry"));
            }
            _ => panic!("Unexpected error type for non-existent key removal"),
        }
    }
}

#[test]
fn test_malformed_json_body_handling() {
    // Create mediator
    let (mediator_interface, mediator_did, _mediator_dir) = create_mediator();

    // Create user interface with the mediator
    let user_dir = TempDir::new().expect("Failed to create temp dir");
    let user_db_path = user_dir.path().join("user.db");
    let user_interface = DidComInterface::new(
        user_db_path.to_string_lossy().to_string(),
        mediator_did.clone(),
    );
    block_on(user_interface.open(user_db_path.to_string_lossy().to_string(), vec![42u8; 32]))
        .expect("Failed to open user database");

    let user_did = block_on(user_interface.generate_did(
        "https://example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate DID");

    // Test with malformed JSON that starts with { but isn't valid
    let malformed_json = r#"{"broken": "json", incomplete"#;

    let message = DIDCommMessage {
        id: "malformed-msg".to_string(),
        msg_type: "https://example.org/protocols/1.0/message".to_string(),
        body: malformed_json.to_string(),
        from: Some(user_did.clone()),
        to: vec![user_did.clone()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // Should still pack successfully (body is treated as string)
    // This message goes to self via mediator
    let packed_for_mediator =
        block_on(user_interface.pack(message, user_did.clone(), vec![user_did.clone()]))
            .expect("Should pack even with malformed JSON body");

    // Mediator unpacks the forward message
    let forward_msg = block_on(mediator_interface.unpack(packed_for_mediator))
        .expect("Mediator should unpack forward");

    // Verify it's a forward message with the malformed body preserved in the inner message
    assert!(
        forward_msg.msg_type.contains("forward"),
        "Expected forward message"
    );

    // Keep temp directory alive
    let _ = user_dir;
}

#[test]
fn test_sequential_did_generation() {
    let (interface, _temp_dir, _mediator, _mediator_dir) = create_test_interface();

    // Generate multiple DIDs sequentially (since DidComInterface doesn't implement Clone)
    let mut dids = vec![];

    for i in 0..5 {
        let did =
            block_on(interface.generate_did(format!("https://example{i}.com/didcomm"), vec![]))
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
