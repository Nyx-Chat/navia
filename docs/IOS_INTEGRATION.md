# iOS Integration Guide

This guide covers how to integrate Navia into your iOS application.

## Installation

Navia reaches iOS as a Swift package: the root `Package.swift` of this
repository. Its library product `Navia` is made of two targets:

- `navia_coreFFI`, a binary target: `navia_coreFFI.xcframework`, the Rust
  library as a static library (`libnavia_core.a`) with an `arm64` device slice
  and an `arm64` simulator slice (Apple Silicon simulators only), each with the
  C header and module map UniFFI generates. It needs iOS 17.0 or later.
- `Navia`, the UniFFI Swift bindings (`ios/Navia/Generated/navia_core.swift`),
  compiled in Swift 5 language mode and linked against the system frameworks
  and libraries the static library needs (`Security`, `CoreFoundation`,
  `libiconv`).

The XCFramework is a build output and is not committed, so the package works
by local path only: a dependency on this repository by git URL finds no
XCFramework. Build it from source, or wrap the files of a CI run or a release
in a package of your own.

The Swift bindings and the XCFramework must come from the same build. At the
first call the bindings compare their UniFFI API checksums with the library's,
and stop the app with `UniFFI API checksum mismatch` when they differ.

### Build from source (local path)

```bash
scripts/build-xcframework.sh     # or: make build-ios
```

The machine that builds needs:

- Xcode with the iOS SDKs (the command-line tools alone do not have them)
- `rustup`; the script adds the `aarch64-apple-ios` and `aarch64-apple-ios-sim`
  targets to the active toolchain when they are missing
- Read access to the private `Nyx-Chat/navia-didcomm` and
  `Nyx-Chat/navia-messaging` repositories, with git credentials cargo can use
  (for example `net.git-fetch-with-cli = true` in `~/.cargo/config.toml`), since
  cargo fetches both from GitHub. With `CARGO_NET_OFFLINE=true` it builds from
  the dependencies cargo has already fetched.

It writes:

| Path | Contents |
|---|---|
| `ios/build/navia_coreFFI.xcframework` | The XCFramework `Package.swift` uses |
| `ios/build/navia_coreFFI.xcframework.zip` | The XCFramework, zipped |
| `ios/build/navia_coreFFI.xcframework.zip.checksum` | The zip's SwiftPM checksum (`swift package compute-checksum`) |
| `ios/build/navia_core.swift` | The Swift bindings of this build |

and regenerates `ios/Navia/Generated` (bindings, header and module map) from
the same build. Commit `ios/Navia/Generated` with every Rust change to the FFI:
CI fails when the committed copy differs from what the build generates.

Then add the checkout as a local package: in Xcode, **File > Add Package
Dependencies… > Add Local…**, or in a manifest:

```swift
dependencies: [
    .package(name: "Navia", path: "../navia"),
],
targets: [
    .target(name: "MyApp", dependencies: [.product(name: "Navia", package: "Navia")]),
]
```

`ios/SPMSmoke` is such a package. Its test opens a store, generates a DID,
packs a message and unpacks it on a simulator; run it after a build:

```bash
cd ios/SPMSmoke
xcodebuild test -scheme SPMSmoke -destination 'platform=iOS Simulator,name=iPhone 17'
```

(any iPhone simulator on iOS 17 or later; `xcrun simctl list devices available`
lists them).

That links only the simulator slice. To link the same test against the device
slice, without a device or signing:

```bash
xcodebuild build-for-testing -scheme SPMSmoke -destination 'generic/platform=iOS' CODE_SIGNING_ALLOWED=NO
```

CI runs both.

### From a CI build or a release

Every PR Validation run uploads the artifact `navia-ios-spm`, kept 7 days. It
holds the three files a consumer needs from one build:
`navia_coreFFI.xcframework.zip`, `navia_coreFFI.xcframework.zip.checksum` and
`navia_core.swift`.

```bash
gh run download <run-id> -R Nyx-Chat/navia -n navia-ios-spm
```

The release workflow attaches the same files to the GitHub release it creates,
as `navia_coreFFI.xcframework-<version>.zip`,
`navia_coreFFI.xcframework-<version>.zip.checksum` and
`navia_core-<version>.swift`. Releases up to 1.5.0 do not have them; their
`Navia.xcframework-<version>.zip` is the retired framework described below.

Wrap the files in a package of your own, with the same settings as the root
`Package.swift`:

```swift
targets: [
    // The unzipped XCFramework, or the zip by URL with the checksum file's
    // contents: .binaryTarget(name: "navia_coreFFI", url: "…", checksum: "…")
    .binaryTarget(name: "navia_coreFFI", path: "navia_coreFFI.xcframework"),
    .target(
        name: "Navia",
        dependencies: ["navia_coreFFI"],
        path: "Sources/Navia", // navia_core.swift
        swiftSettings: [.swiftLanguageMode(.v5)],
        linkerSettings: [
            .linkedFramework("Security"),
            .linkedFramework("CoreFoundation"),
            .linkedLibrary("iconv"),
        ]
    ),
]
```

### Swift 6 apps

UniFFI 0.28's generated Swift is not concurrency-clean under the Swift 6
language mode, so the `Navia` target compiles it in Swift 5 mode. A Swift 6
module imports it with `@preconcurrency import Navia`. None of the generated
types are `Sendable`: keep each `DidComInterface` inside one isolation domain,
such as one actor. `ios/SPMSmoke` is a Swift 6 consumer.

### Retired: CocoaPods and `Navia.framework`

`Navia.podspec`, `ios/Navia.xcodeproj` and `scripts/build-for-ios.sh` are gone.
Neither path they served worked:

- The pod did not link. Inside Xcode its script built only the current SDK's
  target and never created `rust/target/universal/<mode>`, the only
  directories the podspec's `LIBRARY_SEARCH_PATHS` named; the crate built no
  `staticlib`; and `-lnavia_core` was set on the pod target only, not on the
  app.
- `Navia.framework`, which releases up to 1.5.0 attach as
  `Navia.xcframework-<version>.zip`, links `libnavia_core.dylib` by its absolute
  build path, and nothing embeds that dylib, so an app that embeds the
  framework stops at launch with dyld's `Library not loaded`. Do not use it.

## Basic Usage

### Import and Initialize

```swift
import Navia
import Foundation

class NaviaManager {
    private(set) var interface: DidComInterface?
    
    func initialize(mediatorDid: String) async throws {
        // Open database
        let documentsPath = NSSearchPathForDirectoriesInDomains(.documentDirectory, .userDomainMask, true)[0]
        let dbPath = "\(documentsPath)/navia.db"
        
        // Create interface; pack() wraps every message in a forward for this mediator
        let interface = DidComInterface(path: dbPath, mediatorDid: mediatorDid)
        
        // Generate or retrieve secure seed (32 bytes)
        let seed = try getSecureSeed()
        
        try await interface.open(path: dbPath, seed: seed)
        self.interface = interface
    }
    
    private func getSecureSeed() throws -> Data {
        // Use iOS Keychain to store/retrieve the seed
        // This is a simplified example - implement proper Keychain handling
        return Data(repeating: 42, count: 32)
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
func sendMessage(from senderDID: String, to recipientDID: String, content: String) async throws {
    guard let interface = interface else {
        throw NaviaError.notInitialized
    }
    
    let message = DidCommMessage(
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
        to: [recipientDID]
    )
    
    // Send encryptedMessage over your transport layer (HTTP, WebSocket, etc.)
    try await sendOverTransport(encryptedMessage, to: recipientDID)
}
```

### Receive Messages

```swift
func receiveMessage(_ encryptedMessage: String) async throws -> DidCommMessage {
    guard let interface = interface else {
        throw NaviaError.notInitialized
    }
    
    let message = try await interface.unpack(msg: encryptedMessage)
    
    // Trust `from` only when unpack proved it
    if message.authenticated, let sender = message.from {
        print("From: \(sender)")
    } else {
        print("From: unauthenticated")
    }
    
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
                try await naviaManager.initialize(mediatorDid: mediatorDid)
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
            let isHealthy = await naviaManager.interface?.isHealthy() ?? false
            if !isHealthy {
                print("Navia health check failed - consider reinitializing")
            }
        }
    }
}
```

### Memory Management

```swift
class NaviaManager {
    private(set) var interface: DidComInterface?
    
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
    case parsingError(String)
    case didGenerationError(String)
    case packingError(String)
    case unpackingError(String)
    case validationError(String)
    case generalError(String)
}

extension DidCommError {
    func toNaviaError() -> NaviaError {
        switch self {
        case .DatabaseError(let message):
            return .databaseError(message)
        case .ParsingError(let message):
            return .parsingError(message)
        case .DidGenerationError(let message):
            return .didGenerationError(message)
        case .PackingError(let message):
            return .packingError(message)
        case .UnpackingError(let message):
            return .unpackingError(message)
        case .ValidationError(let message):
            return .validationError(message)
        case .GeneralError(let message):
            return .generalError(message)
        }
    }
}
```

## Testing

`ios/SPMSmoke` (see [Build from source](#build-from-source-local-path)) is a
working test against the package. An XCTest version of the same round trip,
with a throwaway store per test:

```swift
import Foundation
import XCTest
@preconcurrency import Navia

final class NaviaIntegrationTests: XCTestCase {
    private var directory: URL!
    private var interface: DidComInterface!
    
    override func setUp() async throws {
        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let dbPath = directory.appendingPathComponent("navia.db").path
        
        interface = DidComInterface(path: dbPath, mediatorDid: "did:peer:placeholder")
        try await interface.open(path: dbPath, seed: Data(repeating: 42, count: 32))
    }
    
    override func tearDown() async throws {
        interface = nil
        try? FileManager.default.removeItem(at: directory)
    }
    
    func testMessageRoundTrip() async throws {
        let did = try await interface.generateDid(uri: "https://example.com/didcomm", routingKeys: [])
        XCTAssertTrue(did.hasPrefix("did:peer:"))
        
        let originalMessage = DidCommMessage(
            id: "test-1",
            msgType: "https://didcomm.org/basicmessage/2.0/message",
            body: #"{"test":true}"#,
            from: did,
            to: [did]
        )
        
        // packNoForward: pack() would wrap the frame in a forward for the mediator
        let encrypted = try await interface.packNoForward(msg: originalMessage, from: did, to: [did])
        let decrypted = try await interface.unpack(msg: encrypted)
        
        XCTAssertEqual(originalMessage.id, decrypted.id)
        XCTAssertEqual(originalMessage.body, decrypted.body)
        XCTAssertTrue(decrypted.authenticated)
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

**"UniFFI API checksum mismatch" at the first Navia call:**
- The Swift bindings and the XCFramework come from different builds. Take both
  from one run of `scripts/build-xcframework.sh`, one `navia-ios-spm` artifact
  or one release

**"…/ios/build/navia_coreFFI.xcframework does not contain binary artifact" when the package resolves:**
- `ios/build/navia_coreFFI.xcframework` has not been built in this checkout;
  run `scripts/build-xcframework.sh`

**Undefined symbols such as `_SecTrustEvaluateWithError` or `_CFRelease` when the app links:**
- A package that wraps the XCFramework itself is missing the linker settings
  of the `Navia` target: `Security`, `CoreFoundation` and `iconv`

**A minimum platform version error for iOS:**
- The package and the XCFramework need iOS 17.0; raise the app's deployment
  target to iOS 17.0 or later

**No library for the simulator is found when building:**
- The simulator slice is `arm64` only: build on an Apple Silicon Mac; Intel
  simulators (`x86_64`) are not supported

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