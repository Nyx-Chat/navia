# Navia - DIDComm Messaging Library for Android

Navia is a high-performance DIDComm v2 messaging library for Android, built with Rust and exposed to Kotlin/Java through UniFFI bindings. It provides secure, encrypted peer-to-peer messaging capabilities using the DIDComm protocol.

## Features

- 🚀 **Pure UniFFI/JNA** - No JNI complexity, automatic Rust async → Kotlin suspend function conversion
- 🔐 **DIDComm v2 Protocol** - Full implementation with encryption and authentication
- 📦 **Native Kotlin Types** - Structured data types, no JSON string manipulation required
- ⚡ **Async/Await Support** - Non-blocking operations throughout
- 🗄️ **Aries Askar Storage** - Secure key management and storage

## Documentation

- 📱 [Android Integration Guide](ANDROID_INTEGRATION.md) - Complete Android setup and usage
- 📚 [API Reference](docs/API.md) - Comprehensive API documentation and patterns
- 🔧 [Troubleshooting](docs/TROUBLESHOOTING.md) - Common issues and solutions
- ⚠️ Error Handling - Error handling patterns for mobile apps
- 💡 [Examples](examples/) - Runnable code examples

## Quick Start

### Installation

Add to your app's `build.gradle.kts`:

```kotlin
dependencies {
    implementation("com.nyx:navia:1.1.2")
    implementation("net.java.dev.jna:jna:5.13.0@aar")
}
```

### Basic Usage

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

For detailed integration instructions, see the [Android Integration Guide](ANDROID_INTEGRATION.md).

## Building from Source

### Prerequisites

1. **Rust toolchain** with Android targets:
```bash
rustup target add aarch64-linux-android armv7-linux-androideabi \
    i686-linux-android x86_64-linux-android
```

2. **Android NDK r25b** (auto-detected or set `ANDROID_NDK_ROOT`)

3. **cargo-ndk**:
```bash
cargo install cargo-ndk
```

### Build Process

```bash
# Clone the repository
git clone https://github.com/Nyx-Chat/navia.git
cd navia

# Build for all Android architectures
cd scripts
./build-for-android.sh --package
```

Build modes:
- **Local** (default): Builds only for connected device/emulator
- **All** (`--all`): Builds for all architectures locally
- **Package** (`--package`): Full build for distribution

See [Troubleshooting](docs/TROUBLESHOOTING.md) for build issues.

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