# Navia - DIDComm Messaging Library for Android

Navia is a high-performance DIDComm v2 messaging library for Android, built with Rust and exposed to Kotlin/Java through UniFFI bindings. It provides secure, encrypted peer-to-peer messaging capabilities using the DIDComm protocol.

## Features

- 🚀 **Pure UniFFI/JNA** - No JNI complexity, automatic Rust async → Kotlin suspend function conversion
- 🔐 **DIDComm v2 Protocol** - Full implementation with encryption and authentication
- 📦 **Native Kotlin Types** - Structured data types, no JSON string manipulation required
- ⚡ **Async/Await Support** - Non-blocking operations throughout
- 🗄️ **Aries Askar Storage** - Secure key management and storage
- 📱 **Android Optimized** - Built specifically for Android applications

## Installation

Navia is distributed as a private Android AAR package via GitHub Packages.

### Add to your project

In your app's `settings.gradle.kts`:

```kotlin
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/Nyx-Chat/navia")
            credentials {
                // Use local.properties or environment variables
                username = providers.gradleProperty("gpr.user").orNull 
                    ?: System.getenv("GITHUB_ACTOR")
                password = providers.gradleProperty("gpr.token").orNull 
                    ?: System.getenv("GITHUB_TOKEN")
            }
        }
    }
}
```

In your app's `build.gradle.kts`:

```kotlin
dependencies {
    implementation("com.nyx:navia:1.0.4")
    implementation("net.java.dev.jna:jna:5.13.0@aar")
}
```

### Authentication Setup

Create `local.properties` in your project root:
```properties
gpr.user=your-github-username
gpr.token=your-personal-access-token-with-read-packages
```

## Quick Start

### 1. Initialize Navia

```kotlin
import uniffi.navia_core.*

// Create interface instance
val didcomm = DidComInterface("/path/to/database")

// Initialize with seed for deterministic key generation
val seed = "your-secure-seed".toByteArray()
didcomm.open("/path/to/database", seed)
```

### 2. Generate a DID

```kotlin
// Generate a new DID with service endpoint
val did = didcomm.generateDid(
    uri = "https://example.com/didcomm",
    routingKeys = listOf() // Add routing keys if needed
)
println("Generated DID: $did")
```

### 3. Pack Messages (Encrypt)

```kotlin
// Create a structured message
val message = DidCommMessage(
    id = UUID.randomUUID().toString(),
    msgType = "https://example.org/protocols/chat/1.0/message",
    body = "Hello, secure world!",
    from = myDid,
    to = listOf(recipientDid)
)

// Pack (encrypt) the message
val packedMessage = didcomm.pack(
    msg = message,
    from = myDid,
    to = recipientDid
)
```

### 4. Unpack Messages (Decrypt)

```kotlin
// Unpack returns a structured message
val unpackedMessage: DidCommMessage = didcomm.unpack(packedMessage)

println("From: ${unpackedMessage.from}")
println("Type: ${unpackedMessage.msgType}")
println("Body: ${unpackedMessage.body}")
```

### 5. Database Operations

```kotlin
// Store key-value data
didcomm.insert("connections", "alice", "did:peer:alice...")

// Retrieve data
val aliceDid = didcomm.get("connections", "alice")

// Batch operations for better performance
val items = listOf(
    KeyValue(key = "alice", value = "did:peer:alice..."),
    KeyValue(key = "bob", value = "did:peer:bob...")
)
didcomm.insertBatch("connections", items)
```

## Architecture

Navia uses a layered architecture:

```
┌─────────────────────────────────┐
│     Kotlin/Android App          │
├─────────────────────────────────┤
│      UniFFI Bindings            │  ← Auto-generated Kotlin code
├─────────────────────────────────┤
│      Rust Library (JNA)         │  ← Core DIDComm implementation
├─────────────────────────────────┤
│      Aries Askar                │  ← Secure storage
└─────────────────────────────────┘
```

### Key Components

- **DidComInterface**: Main entry point for all operations
- **DidCommMessage**: Structured message type for type-safe operations
- **DidCommError**: Comprehensive error handling
- **Async Runtime**: Integrated Tokio runtime with proper context handling

## API Reference

### Types

#### DidCommMessage
```kotlin
data class DidCommMessage(
    val id: String,
    val msgType: String,
    val body: String,
    val from: String?,
    val to: List<String>
)
```

#### KeyValue
```kotlin
data class KeyValue(
    val key: String,
    val value: String,
    val metadata: String? = null
)
```

#### DidCommError
- `DatabaseError`: Storage operation failures
- `ParsingError`: Message parsing failures
- `DidGenerationError`: DID creation failures
- `PackingError`: Encryption failures
- `UnpackingError`: Decryption failures
- `GeneralError`: Other errors

### Core Methods

All methods are suspend functions (async):

- `open(path: String, seed: ByteArray)`: Initialize database
- `generateDid(uri: String, routingKeys: List<String>): String`: Create new DID
- `pack(msg: DidCommMessage, from: String, to: String): String`: Encrypt message
- `unpack(msg: String): DidCommMessage`: Decrypt message
- `insert/get/update/remove`: Database operations
- `insertBatch/getBatch`: Batch operations

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

# This generates:
# - Native libraries in android/src/main/jniLibs/
# - Kotlin bindings in android/src/main/java/
# - AAR package in android/build/outputs/
```

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development instructions including:
- Local development setup
- Testing procedures
- Publishing new versions
- CI/CD workflows
- Troubleshooting

## Performance Optimizations

Navia includes several performance optimizations:

- ✅ **Shared Tokio Runtime**: Single runtime instance, no per-call overhead
- ✅ **Zero JSON Serialization**: Native Kotlin types throughout
- ✅ **Batch Operations**: Efficient bulk database operations
- ✅ **Connection Pooling**: Built-in Aries Askar connection management
- ✅ **Async/Await**: Non-blocking operations for UI responsiveness

## Version History

- **1.0.4**: Fixed Tokio runtime context for FFI calls
- **1.0.3**: Published to GitHub Packages
- **1.0.0**: Initial release with pure UniFFI architecture

## License

Proprietary - Nyx Chat

## Support

For issues, questions, or contributions, please contact the Nyx development team.