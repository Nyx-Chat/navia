//! Integration tests for the complete DIDComm message flow
//!
//! These tests verify the end-to-end functionality of the library,
//! ensuring all components work together correctly.

use futures::executor::block_on;
use navia_core::ffi::types::{DIDCommMessage, KeyValue};
use navia_core::DidComInterface;
use tempfile::TempDir;
use zeroize::Zeroize;

/// Creates a mediator interface and returns its DID
fn create_mediator() -> (DidComInterface, String, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("mediator.db");

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

/// Helper function to create a test database with a temporary directory and shared mediator
fn create_test_db_with_mediator(mediator_did: &str, seed_byte: u8) -> (DidComInterface, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        mediator_did.to_string(),
    );

    // Initialize with a test seed
    let mut seed = vec![seed_byte; 32];
    block_on(interface.open(db_path.to_string_lossy().to_string(), seed.clone()))
        .expect("Failed to open database");

    // Verify seed is zeroized in our local copy
    seed.zeroize();

    (interface, temp_dir)
}

/// Helper function to create a simple test database (for tests that don't use pack/unpack)
fn create_simple_test_db() -> (DidComInterface, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(
        db_path.to_string_lossy().to_string(),
        "did:peer:placeholder".to_string(),
    );

    let mut seed = vec![42u8; 32];
    block_on(interface.open(db_path.to_string_lossy().to_string(), seed.clone()))
        .expect("Failed to open database");

    seed.zeroize();

    (interface, temp_dir)
}

#[test]
fn test_full_message_flow() {
    // Create shared mediator
    let (mediator_interface, mediator_did, _mediator_dir) = create_mediator();

    // Create two interfaces to simulate two parties, both using the same mediator
    let (alice_interface, _alice_dir) = create_test_db_with_mediator(&mediator_did, 1);
    let (bob_interface, _bob_dir) = create_test_db_with_mediator(&mediator_did, 2);

    // Generate DIDs for both parties with mediator routing
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

    // Create a message from Alice to Bob
    let original_message = DIDCommMessage {
        id: "test-message-123".to_string(),
        msg_type: "https://didcomm.org/basicmessage/2.0/message".to_string(),
        body: r#"{"content": "Hello Bob!", "timestamp": "2024-01-01T12:00:00Z"}"#.to_string(),
        from: Some(alice_did.clone()),
        to: vec![bob_did.clone()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // Alice packs (encrypts) the message for Bob via mediator
    let packed_for_mediator = block_on(alice_interface.pack(
        original_message.clone(),
        alice_did.clone(),
        vec![bob_did.clone()],
    ))
    .expect("Failed to pack message");

    // Verify the packed message is encrypted (should be JSON with ciphertext)
    assert!(packed_for_mediator.contains("ciphertext"));
    assert!(packed_for_mediator.contains("protected"));

    // Mediator receives and unpacks the forward message
    let forward_msg = block_on(mediator_interface.unpack(packed_for_mediator))
        .expect("Mediator should unpack forward message");

    // Verify it's a forward message
    assert!(
        forward_msg.msg_type.contains("forward"),
        "Expected forward message, got: {}",
        forward_msg.msg_type
    );

    // Parse forward body to verify structure
    let forward_body: serde_json::Value =
        serde_json::from_str(&forward_msg.body).expect("Forward body should be valid JSON");
    assert_eq!(forward_body["next"].as_str().unwrap(), bob_did);
}

#[test]
fn test_anonymous_message() {
    // Create shared mediator
    let (mediator_interface, mediator_did, _mediator_dir) = create_mediator();

    let (alice_interface, _alice_dir) = create_test_db_with_mediator(&mediator_did, 1);
    let (bob_interface, _bob_dir) = create_test_db_with_mediator(&mediator_did, 2);

    // Generate Bob's DID with mediator routing
    let bob_did = block_on(bob_interface.generate_did(
        "https://bob.example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate Bob's DID");

    // Create an anonymous message (no 'from' field)
    let anonymous_message = DIDCommMessage {
        id: "anon-message-456".to_string(),
        msg_type: "https://example.org/protocols/1.0/ping".to_string(),
        body: "{}".to_string(),
        from: None,
        to: vec![bob_did.clone()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // For anonymous messages, we still need a sender DID for encryption keys
    // but it won't be included in the message
    let alice_did = block_on(alice_interface.generate_did(
        "https://alice.example.com/didcomm".to_string(),
        vec![mediator_did.clone()],
    ))
    .expect("Failed to generate Alice's DID");

    // Pack without authentication (anonymous) - goes via mediator
    let packed_for_mediator =
        block_on(alice_interface.pack(anonymous_message.clone(), alice_did, vec![bob_did.clone()]))
            .expect("Failed to pack anonymous message");

    // Mediator unpacks and forwards
    let forward_msg = block_on(mediator_interface.unpack(packed_for_mediator))
        .expect("Mediator should unpack forward message");

    // Verify it's a forward message to Bob
    assert!(forward_msg.msg_type.contains("forward"));
    let forward_body: serde_json::Value = serde_json::from_str(&forward_msg.body).unwrap();
    assert_eq!(forward_body["next"].as_str().unwrap(), bob_did);
}

#[test]
fn test_storage_operations() {
    let (interface, _temp_dir) = create_simple_test_db();

    // Test basic storage operations
    block_on(interface.insert(
        "test_category".to_string(),
        "test_key".to_string(),
        "test_value".to_string(),
    ))
    .expect("Failed to insert");

    // Retrieve the value
    let retrieved = block_on(interface.get("test_category".to_string(), "test_key".to_string()))
        .expect("Failed to get");

    assert_eq!(retrieved, "test_value");

    // Update the value
    block_on(interface.update(
        "test_category".to_string(),
        "test_key".to_string(),
        "updated_value".to_string(),
    ))
    .expect("Failed to update");

    // Verify update
    let updated = block_on(interface.get("test_category".to_string(), "test_key".to_string()))
        .expect("Failed to get updated value");

    assert_eq!(updated, "updated_value");

    // Remove the value
    block_on(interface.remove("test_category".to_string(), "test_key".to_string()))
        .expect("Failed to remove");

    // Verify removal
    let removed = block_on(interface.get("test_category".to_string(), "test_key".to_string()))
        .expect("Failed to get removed value");

    assert_eq!(removed, "");
}

#[test]
fn test_batch_operations() {
    let (interface, _temp_dir) = create_simple_test_db();

    // Test batch insert
    let items = vec![
        KeyValue {
            key: "key1".to_string(),
            value: "value1".to_string(),
            metadata: None,
        },
        KeyValue {
            key: "key2".to_string(),
            value: "value2".to_string(),
            metadata: Some("test-metadata".to_string()),
        },
        KeyValue {
            key: "key3".to_string(),
            value: "value3".to_string(),
            metadata: None,
        },
    ];

    block_on(interface.insert_batch("batch_test".to_string(), items))
        .expect("Failed to insert batch");

    // Test batch get
    let keys = vec![
        "key1".to_string(),
        "key2".to_string(),
        "key3".to_string(),
        "key4".to_string(),
    ];
    let results =
        block_on(interface.get_batch("batch_test".to_string(), keys)).expect("Failed to get batch");

    // Verify results
    assert_eq!(results.len(), 3); // key4 doesn't exist
    assert!(results
        .iter()
        .any(|kv| kv.key == "key1" && kv.value == "value1"));
    assert!(results
        .iter()
        .any(|kv| kv.key == "key2" && kv.value == "value2"));
    assert!(results
        .iter()
        .any(|kv| kv.key == "key3" && kv.value == "value3"));
}

#[test]
fn test_message_with_complex_body() {
    // For self-messaging without mediator, use pack_no_forward
    let (interface, _temp_dir) = create_simple_test_db();

    // Generate a DID (no routing keys since it's for self-messaging)
    let did = block_on(interface.generate_did("https://example.com/didcomm".to_string(), vec![]))
        .expect("Failed to generate DID");

    // Create a message with complex JSON body
    let complex_body = r#"{
        "content": "Test message",
        "attachments": [
            {
                "id": "attachment-1",
                "mime_type": "application/json",
                "data": {
                    "base64": "eyJ0ZXN0IjogImRhdGEifQ=="
                }
            }
        ],
        "metadata": {
            "priority": "high",
            "tags": ["important", "urgent"]
        }
    }"#;

    let message = DIDCommMessage {
        id: "complex-message-789".to_string(),
        msg_type: "https://example.org/protocols/1.0/data-transfer".to_string(),
        body: complex_body.to_string(),
        from: Some(did.clone()),
        to: vec![did.clone()],
        authenticated: false,
        encrypted_from_kid: None,
        sign_from: None,
        anonymous_sender: false,
    };

    // Pack and unpack the message using pack_no_forward (no mediator routing)
    let packed =
        block_on(interface.pack_no_forward(message.clone(), did.clone(), vec![did.clone()]))
            .expect("Failed to pack complex message");

    let unpacked = block_on(interface.unpack(packed)).expect("Failed to unpack complex message");

    // Verify the complex body is preserved (compare as JSON to ignore formatting)
    let original_json: serde_json::Value = serde_json::from_str(complex_body).unwrap();
    let unpacked_json: serde_json::Value = serde_json::from_str(&unpacked.body).unwrap();
    assert_eq!(unpacked_json, original_json);

    // Parse and verify the JSON structure
    let parsed: serde_json::Value =
        serde_json::from_str(&unpacked.body).expect("Failed to parse unpacked body as JSON");

    assert_eq!(parsed["content"], "Test message");
    assert_eq!(parsed["attachments"][0]["id"], "attachment-1");
    assert_eq!(parsed["metadata"]["priority"], "high");
}

#[test]
fn test_plain_string_body_roundtrip() {
    // Plain-text bodies travel as JSON strings and come back from unpack
    // verbatim, without JSON quoting (regression guard for 1.3.2).
    let (interface, _temp_dir) = create_simple_test_db();

    let did = block_on(interface.generate_did("https://example.com/didcomm".to_string(), vec![]))
        .expect("Failed to generate DID");

    let bodies = [
        "hello",
        "",
        "42",
        "true",
        "null",
        "say \"hi\"\n\u{2713}",
        "{broken",
    ];

    for (index, body) in bodies.iter().enumerate() {
        let message = DIDCommMessage {
            id: format!("plain-body-{index}"),
            msg_type: "https://didcomm.org/basicmessage/2.0/message".to_string(),
            body: body.to_string(),
            from: Some(did.clone()),
            to: vec![did.clone()],
        };

        let packed = block_on(interface.pack_no_forward(message, did.clone(), vec![did.clone()]))
            .expect("Failed to pack plain-text message");
        let unpacked = block_on(interface.unpack(packed)).expect("Failed to unpack message");

        assert_eq!(
            unpacked.body, *body,
            "body {body:?} changed in the round trip"
        );
        assert_eq!(unpacked.id, format!("plain-body-{index}"));
    }
}
