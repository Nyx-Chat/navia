//
//  NaviaTests.swift
//  NaviaTests
//
//  Created by Nyx Development Team.
//  Copyright © 2024 Nyx Chat. All rights reserved.
//

import XCTest
@testable import Navia

final class NaviaTests: XCTestCase {

    override func setUpWithError() throws {
        // Put setup code here. This method is called before the invocation of each test method in the class.
    }

    override func tearDownWithError() throws {
        // Put teardown code here. This method is called after the invocation of each test method in the class.
    }

    func testBasicInitialization() throws {
        // Test that we can create a DidComInterface instance
        let interface = DidComInterface(path: "")
        XCTAssertNotNil(interface)
    }

    func testOpenDatabase() async throws {
        // Test database opening with in-memory database
        let interface = DidComInterface(path: "")
        let seed = Array(repeating: UInt8(42), count: 32)
        
        do {
            try await interface.open(path: ":memory:", seed: seed)
            let isHealthy = try await interface.isHealthy()
            XCTAssertTrue(isHealthy)
        } catch {
            XCTFail("Failed to open database: \(error)")
        }
    }

    func testDIDGeneration() async throws {
        // Test DID generation
        let interface = DidComInterface(path: "")
        let seed = Array(repeating: UInt8(42), count: 32)
        
        do {
            try await interface.open(path: ":memory:", seed: seed)
            let did = try await interface.generateDid(uri: "https://example.com/didcomm", routingKeys: [])
            
            XCTAssertTrue(did.hasPrefix("did:peer:"))
            XCTAssertGreaterThan(did.count, 20)
        } catch {
            XCTFail("Failed to generate DID: \(error)")
        }
    }

    func testMessagePackUnpack() async throws {
        // Test message pack/unpack roundtrip
        let interface = DidComInterface(path: "")
        let seed = Array(repeating: UInt8(42), count: 32)
        
        do {
            try await interface.open(path: ":memory:", seed: seed)
            
            let alice = try await interface.generateDid(uri: "https://alice.example.com", routingKeys: [])
            let bob = try await interface.generateDid(uri: "https://bob.example.com", routingKeys: [])
            
            let message = DIDCommMessage(
                id: "test-message-1",
                msgType: "https://example.org/test/1.0/message",
                body: #"{"content": "Hello from iOS!"}"#,
                from: alice,
                to: [bob]
            )
            
            let encrypted = try await interface.pack(msg: message, from: alice, to: bob)
            XCTAssertFalse(encrypted.isEmpty)
            
            let decrypted = try await interface.unpack(msg: encrypted)
            XCTAssertEqual(decrypted.id, message.id)
            XCTAssertEqual(decrypted.msgType, message.msgType)
            XCTAssertEqual(decrypted.body, message.body)
            XCTAssertEqual(decrypted.from, message.from)
            XCTAssertEqual(decrypted.to, message.to)
        } catch {
            XCTFail("Failed message pack/unpack test: \(error)")
        }
    }

    func testStorageOperations() async throws {
        // Test storage operations
        let interface = DidComInterface(path: "")
        let seed = Array(repeating: UInt8(42), count: 32)
        
        do {
            try await interface.open(path: ":memory:", seed: seed)
            
            // Test insert and get
            try await interface.insert(category: "test", name: "key1", value: "value1")
            let retrieved = try await interface.get(category: "test", name: "key1")
            XCTAssertEqual(retrieved, "value1")
            
            // Test update
            try await interface.update(category: "test", name: "key1", value: "updated_value1")
            let updated = try await interface.get(category: "test", name: "key1")
            XCTAssertEqual(updated, "updated_value1")
            
            // Test batch operations
            let batch = [
                KeyValue(key: "key2", value: "value2", metadata: nil),
                KeyValue(key: "key3", value: "value3", metadata: "test_metadata")
            ]
            try await interface.insertBatch(category: "test", items: batch)
            
            let batchResult = try await interface.getBatch(category: "test", keys: ["key2", "key3"])
            XCTAssertEqual(batchResult.count, 2)
            
            // Test remove
            try await interface.remove(category: "test", name: "key1")
            let removed = try await interface.get(category: "test", name: "key1")
            XCTAssertTrue(removed.isEmpty)
        } catch {
            XCTFail("Failed storage operations test: \(error)")
        }
    }

    func testPerformanceExample() throws {
        // This is an example of a performance test case.
        self.measure {
            // Put the code you want to measure the time of here.
        }
    }
}