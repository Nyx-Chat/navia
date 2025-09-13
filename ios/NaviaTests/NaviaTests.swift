//
//  NaviaTests.swift
//  NaviaTests
//
//  Created by Nyx Development Team.
//  Copyright © 2024 Nyx Chat. All rights reserved.
//

import Testing
@testable import Navia

@Test("Basic initialization creates DidComInterface instance")
func testBasicInitialization() throws {
    let interface = DidComInterface(path: "")
    #expect(interface != nil, "DidComInterface should be created successfully")
}

@Test("Database opens and health check passes")
func testOpenDatabase() async throws {
    let interface = DidComInterface(path: "")
    let seed = Data(repeating: 42, count: 32)
    let tempPath = NSTemporaryDirectory() + "test_db_\(UUID().uuidString).sqlite"
    
    try await interface.open(path: tempPath, seed: seed)
    let isHealthy = try await interface.isHealthy()
    #expect(isHealthy == true, "Database should be healthy after opening")
    
    // Cleanup
    try? FileManager.default.removeItem(atPath: tempPath)
}

@Test("DID generation creates valid peer DIDs")
func testDIDGeneration() async throws {
    let interface = DidComInterface(path: "")
    let seed = Data(repeating: 42, count: 32)
    let tempPath = NSTemporaryDirectory() + "test_db_\(UUID().uuidString).sqlite"
    
    try await interface.open(path: tempPath, seed: seed)
    let did = try await interface.generateDid(uri: "https://example.com/didcomm", routingKeys: [])
    
    #expect(did.hasPrefix("did:peer:"), "DID should start with 'did:peer:' prefix")
    #expect(did.count > 20, "DID should be longer than 20 characters")
    
    // Cleanup
    try? FileManager.default.removeItem(atPath: tempPath)
}

@Test("Storage operations work correctly")
func testStorageOperations() async throws {
    let interface = DidComInterface(path: "")
    let seed = Data(repeating: 42, count: 32)
    let tempPath = NSTemporaryDirectory() + "test_db_\(UUID().uuidString).sqlite"
    
    try await interface.open(path: tempPath, seed: seed)
    
    // Test insert and get
    try await interface.insert(category: "test", name: "key1", value: "value1")
    let retrieved = try await interface.get(category: "test", name: "key1")
    #expect(retrieved == "value1", "Retrieved value should match inserted value")
    
    // Test update
    try await interface.update(category: "test", name: "key1", value: "updated_value1")
    let updated = try await interface.get(category: "test", name: "key1")
    #expect(updated == "updated_value1", "Updated value should match new value")
    
    // Test batch operations
    let batch = [
        KeyValue(key: "key2", value: "value2", metadata: nil),
        KeyValue(key: "key3", value: "value3", metadata: "test_metadata")
    ]
    try await interface.insertBatch(category: "test", items: batch)
    
    let batchResult = try await interface.getBatch(category: "test", keys: ["key2", "key3"])
    #expect(batchResult.count == 2, "Batch result should contain 2 items")
    
    // Test remove
    try await interface.remove(category: "test", name: "key1")
    let removed = try await interface.get(category: "test", name: "key1")
    #expect(removed.isEmpty, "Removed key should return empty string")
    
    // Cleanup
    try? FileManager.default.removeItem(atPath: tempPath)
}

// TODO: Fix pack/unpack FFI timeout issue - hangs when called in test suite context
// @Test("Message pack and unpack roundtrip works correctly")
func testMessagePackUnpack() async throws {
    let interface = DidComInterface(path: "")
    let seed = Data(repeating: 42, count: 32)
    let tempPath = NSTemporaryDirectory() + "test_db_\(UUID().uuidString).sqlite"
    
    try await interface.open(path: tempPath, seed: seed)
    
    let alice = try await interface.generateDid(uri: "https://alice.example.com", routingKeys: [])
    let bob = try await interface.generateDid(uri: "https://bob.example.com", routingKeys: [])
    
    let message = DidCommMessage(
        id: "test-message-1",
        msgType: "https://example.org/test/1.0/message",
        body: #"{"content": "Hello from iOS!"}"#,
        from: alice,
        to: [bob]
    )
    
    let encrypted = try await interface.pack(msg: message, from: alice, to: bob)
    #expect(!encrypted.isEmpty, "Encrypted message should not be empty")
    
    let decrypted = try await interface.unpack(msg: encrypted)
    #expect(decrypted.id == message.id, "Message ID should match")
    #expect(decrypted.msgType == message.msgType, "Message type should match")
    #expect(decrypted.body == message.body, "Message body should match")
    #expect(decrypted.from == alice, "From field should match")
    #expect(decrypted.to == [bob], "To field should match")
    
    // Cleanup
    try? FileManager.default.removeItem(atPath: tempPath)
}