# iOS Integration Guide

This guide covers how to integrate Navia into your iOS application.

## Installation

### CocoaPods (Recommended)

Add to your `Podfile`:

```ruby
pod 'Navia', :git => 'https://github.com/Nyx-Chat/navia.git', :tag => 'v1.1.28'
```

Then run:
```bash
pod install
```

### Swift Package Manager

Add to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/Nyx-Chat/navia.git", .exact("1.1.28"))
]
```

### Manual Installation

1. Download the `Navia.xcframework` from the latest [GitHub release](https://github.com/Nyx-Chat/navia/releases)
2. Drag and drop it into your Xcode project
3. Ensure it's added to "Frameworks, Libraries, and Embedded Content"
4. Set "Embed" to "Embed & Sign"

## Basic Usage

### Import and Initialize

```swift
import Navia
import Foundation

class NaviaManager {
    private var interface: DidComInterface?
    
    func initialize() async throws {
        // Create interface
        interface = DidComInterface(path: "")
        
        // Generate or retrieve secure seed (32 bytes)
        let seed = try getSecureSeed()
        
        // Open database
        let documentsPath = NSSearchPathForDirectoriesInDomains(.documentDirectory, .userDomainMask, true)[0]
        let dbPath = "\(documentsPath)/navia.db"
        
        try await interface?.open(path: dbPath, seed: seed)
    }
    
    private func getSecureSeed() throws -> [UInt8] {
        // Use iOS Keychain to store/retrieve the seed
        // This is a simplified example - implement proper Keychain handling
        let seedData = Data(repeating: 42, count: 32)
        return Array(seedData)
    }
}
```

### Generate DIDs

```swift
func createDID() async throws -> String {
    guard let interface = interface else {
        throw NaviaError.notInitialized
    }
    
    let did = try await interface.generateDid(
        uri: "https://your-didcomm-endpoint.com", 
        routingKeys: []
    )
    
    print("Generated DID: \(did)")
    return did
}
```

### Send Messages

```swift
func sendMessage(to recipientDID: String, content: String) async throws {
    guard let interface = interface else {
        throw NaviaError.notInitialized
    }
    
    let message = DIDCommMessage(
        id: UUID().uuidString,
        msgType: "https://didcomm.org/basicmessage/2.0/message",
        body: """
        {
            "content": "\(content)",
            "sent_time": "\(ISO8601DateFormatter().string(from: Date()))"
        }
        """,
        from: senderDID,
        to: [recipientDID]
    )
    
    let encryptedMessage = try await interface.pack(
        msg: message,
        from: senderDID,
        to: recipientDID
    )
    
    // Send encryptedMessage over your transport layer (HTTP, WebSocket, etc.)
    try await sendOverTransport(encryptedMessage, to: recipientDID)
}
```

### Receive Messages

```swift
func receiveMessage(_ encryptedMessage: String) async throws -> DIDCommMessage {
    guard let interface = interface else {
        throw NaviaError.notInitialized
    }
    
    let message = try await interface.unpack(msg: encryptedMessage)
    
    print("Received message: \(message.body)")
    print("From: \(message.from ?? "anonymous")")
    
    return message
}
```

### Store Data

```swift
func storeContact(_ contact: Contact, withDID did: String) async throws {
    guard let interface = interface else {
        throw NaviaError.notInitialized
    }
    
    let contactData = try JSONEncoder().encode(contact)
    let contactJSON = String(data: contactData, encoding: .utf8)!
    
    try await interface.insert(
        category: "contacts",
        name: did,
        value: contactJSON
    )
}

func loadContact(withDID did: String) async throws -> Contact? {
    guard let interface = interface else {
        throw NaviaError.notInitialized
    }
    
    let contactJSON = try await interface.get(category: "contacts", name: did)
    
    guard !contactJSON.isEmpty else {
        return nil
    }
    
    let contactData = contactJSON.data(using: .utf8)!
    return try JSONDecoder().decode(Contact.self, from: contactData)
}
```

## iOS-Specific Considerations

### Keychain Integration

Use iOS Keychain Services for secure seed storage:

```swift
import Security

class KeychainHelper {
    static func storeSeed(_ seed: [UInt8]) throws {
        let seedData = Data(seed)
        
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "com.yourapp.navia",
            kSecAttrAccount as String: "navia-seed",
            kSecValueData as String: seedData,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]
        
        let status = SecItemAdd(query as CFDictionary, nil)
        
        if status == errSecDuplicateItem {
            // Update existing item
            let updateQuery: [String: Any] = [
                kSecClass as String: kSecClassGenericPassword,
                kSecAttrService as String: "com.yourapp.navia",
                kSecAttrAccount as String: "navia-seed"
            ]
            
            let updateAttributes: [String: Any] = [
                kSecValueData as String: seedData
            ]
            
            let updateStatus = SecItemUpdate(updateQuery as CFDictionary, updateAttributes as CFDictionary)
            guard updateStatus == errSecSuccess else {
                throw KeychainError.updateFailed(updateStatus)
            }
        } else if status != errSecSuccess {
            throw KeychainError.storeFailed(status)
        }
    }
    
    static func retrieveSeed() throws -> [UInt8] {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "com.yourapp.navia",
            kSecAttrAccount as String: "navia-seed",
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        guard status == errSecSuccess,
              let seedData = result as? Data else {
            throw KeychainError.retrieveFailed(status)
        }
        
        return Array(seedData)
    }
}

enum KeychainError: Error {
    case storeFailed(OSStatus)
    case updateFailed(OSStatus)
    case retrieveFailed(OSStatus)
}
```

### Background Processing

Handle app lifecycle events:

```swift
import UIKit

class AppDelegate: UIResponder, UIApplicationDelegate {
    var naviaManager = NaviaManager()
    
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        
        Task {
            do {
                try await naviaManager.initialize()
            } catch {
                print("Failed to initialize Navia: \(error)")
            }
        }
        
        return true
    }
    
    func applicationDidEnterBackground(_ application: UIApplication) {
        // Navia automatically handles database connections
        // No special cleanup needed
    }
    
    func applicationWillEnterForeground(_ application: UIApplication) {
        // Verify health status when returning to foreground
        Task {
            do {
                let isHealthy = try await naviaManager.interface?.isHealthy() ?? false
                if !isHealthy {
                    print("Navia health check failed - consider reinitializing")
                }
            } catch {
                print("Health check error: \(error)")
            }
        }
    }
}
```

### Memory Management

```swift
class NaviaManager {
    private var interface: DidComInterface?
    
    deinit {
        // UniFFI handles cleanup automatically
        // No manual cleanup required
    }
    
    func cleanup() {
        // Optional: explicitly nil the interface if needed
        interface = nil
    }
}
```

## Error Handling

```swift
enum NaviaError: Error {
    case notInitialized
    case invalidSeed
    case databaseError(String)
    case packingError(String)
    case unpackingError(String)
    case validationError(String)
}

extension DidCommError {
    func toNaviaError() -> NaviaError {
        switch self {
        case .DatabaseError(let message):
            return .databaseError(message)
        case .PackingError(let message):
            return .packingError(message)
        case .UnpackingError(let message):
            return .unpackingError(message)
        case .ValidationError(let message):
            return .validationError(message)
        case .GeneralError(let message):
            return .databaseError(message) // Generic fallback
        }
    }
}
```

## Testing

```swift
import XCTest
@testable import YourApp

class NaviaIntegrationTests: XCTestCase {
    var naviaManager: NaviaManager!
    
    override func setUp() async throws {
        naviaManager = NaviaManager()
        
        // Use in-memory database for tests
        let interface = DidComInterface(path: "")
        let testSeed = Array(repeating: UInt8(42), count: 32)
        try await interface.open(path: ":memory:", seed: testSeed)
        naviaManager.interface = interface
    }
    
    override func tearDown() {
        naviaManager = nil
    }
    
    func testDIDGeneration() async throws {
        let did = try await naviaManager.createDID()
        XCTAssertTrue(did.hasPrefix("did:peer:"))
    }
    
    func testMessageRoundTrip() async throws {
        let alice = try await naviaManager.createDID()
        let bob = try await naviaManager.createDID()
        
        let originalMessage = DIDCommMessage(
            id: "test-1",
            msgType: "test/message",
            body: #"{"test": true}"#,
            from: alice,
            to: [bob]
        )
        
        let encrypted = try await naviaManager.interface!.pack(
            msg: originalMessage,
            from: alice,
            to: bob
        )
        
        let decrypted = try await naviaManager.interface!.unpack(msg: encrypted)
        
        XCTAssertEqual(originalMessage.id, decrypted.id)
        XCTAssertEqual(originalMessage.body, decrypted.body)
    }
}
```

## Performance Tips

1. **Reuse DidComInterface**: Create once and reuse throughout your app's lifecycle
2. **Batch Operations**: Use `insertBatch` and `getBatch` for multiple storage operations
3. **Background Threads**: Navia is async-safe and can be called from background threads
4. **Memory Usage**: Monitor memory usage when handling many large messages

## Security Best Practices

1. **Seed Storage**: Always use iOS Keychain for seed storage
2. **Transport Security**: Use TLS/HTTPS for message transport
3. **Validation**: Validate all incoming messages and DIDs
4. **Logging**: Never log seeds, private keys, or message contents
5. **Background Protection**: Consider using `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` for Keychain items

## Troubleshooting

### Common Issues

**"Library not found" error:**
- Ensure the framework is properly embedded in your app target
- Check that the minimum deployment target is iOS 13.0

**Keychain access denied:**
- Check your app's entitlements and Keychain access groups
- Ensure you're using appropriate accessibility levels

**Build failures:**
- Clean build folder: `cmd+shift+k`
- Delete derived data
- Verify Xcode version compatibility

**Runtime crashes:**
- Check that all async calls are properly awaited
- Ensure database is initialized before other operations
- Verify seed is exactly 32 bytes

For more help, see the [troubleshooting guide](TROUBLESHOOTING.md) or open an issue on GitHub.