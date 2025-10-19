//! Integration tests for the complete DIDComm message flow
//!
//! These tests verify the end-to-end functionality of the library,
//! ensuring all components work together correctly.

use futures::executor::block_on;
use navia_core::ffi::types::{DIDCommMessage, KeyValue};
use navia_core::DidComInterface;
use tempfile::TempDir;
use zeroize::Zeroize;

/// Helper function to create a test database with a temporary directory
fn create_test_db() -> (DidComInterface, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let interface = DidComInterface::new(db_path.to_string_lossy().to_string());

    // Initialize with a test seed
    let mut seed = vec![42u8; 32]; // Test seed - use secure random in production
    block_on(interface.open(db_path.to_string_lossy().to_string(), seed.clone()))
        .expect("Failed to open database");

    // Verify seed is zeroized in our local copy
    seed.zeroize();

    (interface, temp_dir)
}

#[test]
fn test_full_message_flow() {
    // Create two interfaces to simulate two parties
    let (alice_interface, _alice_dir) = create_test_db();
    let (bob_interface, _bob_dir) = create_test_db();

    // Generate DIDs for both parties
    let alice_did = block_on(
        alice_interface.generate_did("https://alice.example.com/didcomm".to_string(), vec![]),
    )
    .expect("Failed to generate Alice's DID");

    let bob_did =
        block_on(bob_interface.generate_did("https://bob.example.com/didcomm".to_string(), vec![]))
            .expect("Failed to generate Bob's DID");

    // Create a message from Alice to Bob
    let original_message = DIDCommMessage {
        id: "test-message-123".to_string(),
        msg_type: "https://didcomm.org/basicmessage/2.0/message".to_string(),
        body: r#"{"content": "Hello Bob!", "timestamp": "2024-01-01T12:00:00Z"}"#.to_string(),
        from: Some(alice_did.clone()),
        to: vec![bob_did.clone()],
    };

    // Alice packs (encrypts) the message
    let packed_message = block_on(alice_interface.pack(
        original_message.clone(),
        alice_did.clone(),
        bob_did.clone(),
    ))
    .expect("Failed to pack message");

    // Verify the packed message is encrypted (should be JSON with ciphertext)
    assert!(packed_message.contains("ciphertext"));
    assert!(packed_message.contains("protected"));

    // Bob unpacks (decrypts) the message
    let unpacked_message =
        block_on(bob_interface.unpack(packed_message)).expect("Failed to unpack message");

    // Verify the unpacked message matches the original
    assert_eq!(unpacked_message.id, original_message.id);
    assert_eq!(unpacked_message.msg_type, original_message.msg_type);
    // Parse both as JSON to compare content, not formatting
    let original_body: serde_json::Value = serde_json::from_str(&original_message.body).unwrap();
    let unpacked_body: serde_json::Value = serde_json::from_str(&unpacked_message.body).unwrap();
    assert_eq!(unpacked_body, original_body);
    assert_eq!(unpacked_message.from, Some(alice_did));
    assert_eq!(unpacked_message.to, vec![bob_did]);
}

#[test]
fn test_anonymous_message() {
    let (alice_interface, _alice_dir) = create_test_db();
    let (bob_interface, _bob_dir) = create_test_db();

    // Generate only Bob's DID (Alice will send anonymously)
    let bob_did =
        block_on(bob_interface.generate_did("https://bob.example.com/didcomm".to_string(), vec![]))
            .expect("Failed to generate Bob's DID");

    // Create an anonymous message (no 'from' field)
    let anonymous_message = DIDCommMessage {
        id: "anon-message-456".to_string(),
        msg_type: "https://example.org/protocols/1.0/ping".to_string(),
        body: "{}".to_string(),
        from: None,
        to: vec![bob_did.clone()],
    };

    // For anonymous messages, we still need a sender DID for encryption keys
    // but it won't be included in the message
    let alice_did = block_on(
        alice_interface.generate_did("https://alice.example.com/didcomm".to_string(), vec![]),
    )
    .expect("Failed to generate Alice's DID");

    // Pack without authentication (anonymous)
    let packed_message =
        block_on(alice_interface.pack(anonymous_message.clone(), alice_did, bob_did.clone()))
            .expect("Failed to pack anonymous message");

    // Bob unpacks the message
    let unpacked_message =
        block_on(bob_interface.unpack(packed_message)).expect("Failed to unpack anonymous message");

    // Verify the message is anonymous
    assert_eq!(unpacked_message.from, None);
    assert_eq!(unpacked_message.id, anonymous_message.id);
}

#[test]
fn test_storage_operations() {
    let (interface, _temp_dir) = create_test_db();

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
    let (interface, _temp_dir) = create_test_db();

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
    let (interface, _temp_dir) = create_test_db();

    // Generate a DID
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
    };

    // Pack and unpack the message
    let packed = block_on(interface.pack(message.clone(), did.clone(), did.clone()))
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
