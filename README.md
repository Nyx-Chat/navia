# Navia - DIDComm Messaging Library for Mobile

Navia is a high-performance DIDComm v2 messaging library for Android and iOS, built with Rust and exposed to Kotlin/Swift through UniFFI bindings. It provides secure, encrypted peer-to-peer messaging capabilities using the DIDComm protocol.

## Features

- 🚀 **Pure UniFFI** - No JNI/FFI complexity, automatic Rust async → Kotlin/Swift async conversion
- 🔐 **DIDComm v2 Protocol** - Full implementation with encryption and authentication
- 📦 **Native Types** - Structured data types for Kotlin/Swift, no JSON string manipulation required
- ⚡ **Async/Await Support** - Non-blocking operations throughout
- 🗄️ **Aries Askar Storage** - Secure key management and storage
- 📱 **Mobile Ready** - Android 15+ (16KB pages) and iOS 13+ support

## Documentation

- 📱 [Android Integration Guide](docs/ANDROID_INTEGRATION.md) - Complete Android setup and usage
- 🍎 [iOS Integration Guide](docs/IOS_INTEGRATION.md) - Complete iOS setup and usage
- 📚 [API Reference](docs/API.md) - Comprehensive API documentation and patterns
- 🔧 [Troubleshooting](docs/TROUBLESHOOTING.md) - Common issues and solutions
- ⚠️ [Error Handling](docs/ERROR_HANDLING.md) - Error handling patterns for mobile apps
- 💡 [Examples](examples/) - Runnable code examples

## Quick Start

### Installation

#### Android

Add to your app's `build.gradle.kts`:

```kotlin
dependencies {
    implementation("com.nyx:navia:1.1.28")
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
```

#### iOS

Add to your `Podfile`:

```ruby
pod 'Navia', :git => 'https://github.com/Nyx-Chat/navia.git', :tag => 'v1.1.28'
```

### Basic Usage

#### Android (Kotlin)

```kotlin
import uniffi.navia_core.*

// Initialize
val didcomm = DidComInterface("")
val seed = generateSecure32ByteSeed()
didcomm.open("/path/to/database", seed)

// Generate DID
val myDid = didcomm.generateDid(
    "https://example.com/didcomm",
    emptyList()
)

// Send encrypted message
val message = DidCommMessage(
    id = UUID.randomUUID().toString(),
    msgType = "https://didcomm.org/basicmessage/2.0/message",
    body = """{"content": "Hello, secure world!"}""",
    from = myDid,
    to = listOf(recipientDid)
)

val encrypted = didcomm.pack(message, myDid, recipientDid)
```

#### iOS (Swift)

```swift
import Navia

// Initialize
let didcomm = DidComInterface(path: "")
let seed = generateSecure32ByteSeed()
try await didcomm.open(path: "/path/to/database", seed: seed)

// Generate DID
let myDid = try await didcomm.generateDid(
    uri: "https://example.com/didcomm",
    routingKeys: []
)

// Send encrypted message
let message = DIDCommMessage(
    id: UUID().uuidString,
    msgType: "https://didcomm.org/basicmessage/2.0/message",
    body: #"{"content": "Hello, secure world!"}"#,
    from: myDid,
    to: [recipientDid]
)

let encrypted = try await didcomm.pack(msg: message, from: myDid, to: recipientDid)
```

For detailed integration instructions, see the [Android Integration Guide](docs/ANDROID_INTEGRATION.md) or [iOS Integration Guide](docs/IOS_INTEGRATION.md).

## Building from Source

### Prerequisites

1. **Rust toolchain** with mobile targets:
```bash
# Android targets
rustup target add aarch64-linux-android x86_64-linux-android

# iOS targets (macOS only)
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
```

2. **Platform-specific tools**:
   - **Android**: NDK r27c or r28 (required for 16KB page size support)
     - Set `ANDROID_NDK_ROOT` or install in standard location
     - ⚠️ **Important**: NDK r27+ is required for Android 15+ compatibility
   - **iOS**: Xcode 15+ with command line tools (macOS only)

### Build Process

```bash
# Clone the repository
git clone https://github.com/Nyx-Chat/navia.git
cd navia

# Build for Android (all architectures)
cd scripts
./build-for-android.sh --package

# Build for iOS (macOS only, all architectures)
./build-for-ios.sh --package

# Or use Make
cd ..
make build-android  # Builds Android package
make build-ios      # Builds iOS package
```

Build modes:
- **Local** (default): Builds only for current architecture (connected device/simulator)
- **All** (`--all`): Builds for all architectures locally
- **Package** (`--package`): Full build for distribution

See [Troubleshooting](docs/TROUBLESHOOTING.md) for build issues.

### Android 15+ Compatibility (16KB Page Size)

Starting with Android 15, devices require native libraries to have 16KB-aligned LOAD segments. Google Play enforces this requirement starting November 1, 2025.

**Navia is fully compatible with Android 15+ devices:**

- ✅ All 64-bit native libraries are automatically aligned to 16KB boundaries
- ✅ Build process includes post-build realignment using `realign-android-16kb.py`
- ✅ Every build is verified with `verify-16kb-alignment.sh`
- ✅ CI/CD validates alignment on every release

**How it works:**
1. Libraries are built with NDK r28 and alignment flags
2. Python script realigns virtual addresses to 16KB boundaries
3. Verification confirms all LOAD segments are properly aligned

**For developers:** No additional configuration needed - the build script handles everything automatically.

## Development

### Quick Start

```bash
git clone https://github.com/Nyx-Chat/navia.git
cd navia
make setup  # Installs git hooks and checks environment
```

That's it! The pre-commit hook will automatically format your code.

### Testing

```bash
# Run all tests
cd rust/navia-core
cargo test

# Run examples
cargo run --example message_flow
```

### CI/CD

The project uses GitHub Actions with three main workflows:

1. **PR Validation** (`pr-validation.yml`): Runs on pull requests
   - Rust tests and clippy
   - Android library build verification
   - Security audit

2. **PR Checks** (`pr-checks.yml`): Additional quality checks
   - Code formatting verification
   - Documentation tests
   - Coverage reporting

3. **Release and Publish** (`release-and-publish.yml`): Automated releases
   - Triggers on version tags (e.g., `v1.2.3`)
   - Builds for all architectures
   - Publishes to GitHub Packages
   - Creates GitHub releases with artifacts

### Publishing New Version

1. Update version in `rust/navia-core/Cargo.toml`
2. Commit and tag:
   ```bash
   git commit -am "Release v1.x.x"
   git tag v1.x.x
   git push origin main --tags
   ```
3. GitHub Actions automatically handles the rest

## Architecture

```
┌─────────────────────────────────┐
│     Kotlin/Android App          │
├─────────────────────────────────┤
│      UniFFI Bindings            │  ← Auto-generated
├─────────────────────────────────┤
│      Rust Library (JNA)         │  ← Core implementation
├─────────────────────────────────┤
│      Aries Askar                │  ← Secure storage
└─────────────────────────────────┘
```

## License

Proprietary - Nyx Chat

## Support

For issues, questions, or contributions, please contact the Nyx development team.