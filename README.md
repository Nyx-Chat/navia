# Navia - DIDComm Messaging Library for Android/Kotlin

Navia is a pure Kotlin/Android library that provides DIDComm messaging functionality using Rust and UniFFI bindings.

## Structure

```
navia/
├── rust/           # Rust core implementation
│   └── navia-core/ # DIDComm messaging implementation
├── android/        # Android library wrapper
└── scripts/        # Build scripts
```

## Building

### Prerequisites

1. Rust toolchain with Android targets:
```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

2. Android NDK
   - The build script will automatically detect NDK at: `/Users/bogdanboksan/Library/Android/sdk/ndk`
   - If not found, set `ANDROID_NDK_HOME` environment variable manually

3. Android SDK and Gradle

4. cargo-ndk (recommended for easier Android builds):
```bash
cargo install cargo-ndk
```

### Build Steps

#### Complete Build Process

1. **Build Rust library for host platform (required for binding generation):**
```bash
cd /Users/bogdanboksan/Work/Projects/Pigeon/navia/rust/navia-core
cargo build --release
```

2. **Build for Android and generate Kotlin bindings:**
```bash
cd /Users/bogdanboksan/Work/Projects/Pigeon/navia/scripts
./build-android.sh
```

This script will:
- Automatically detect and set up Android NDK
- Build the Rust library for all Android architectures
- Generate Kotlin bindings using UniFFI
- Place libraries in `android/src/main/jniLibs/`
- Place Kotlin bindings in `android/src/main/java/`

3. **Build the Android library:**
```bash
cd ../android
./gradlew build
```

### Rebuilding After Changes

When you make changes to the Rust code:

1. **For development/testing on host:**
```bash
cd /Users/bogdanboksan/Work/Projects/Pigeon/navia/rust/navia-core
cargo build --release
```

2. **For Android deployment:**
```bash
cd /Users/bogdanboksan/Work/Projects/Pigeon/navia/scripts
./build-android.sh
```

### Important Notes

- This project uses UniFFI's **macro approach**, not UDL files
- The binding generation requires a compiled library (`.dylib` on macOS)
- Always build for the host platform first before generating bindings
- The Android build script uses `cargo-ndk` for proper cross-compilation

### Troubleshooting

**NDK not found error:**
- The script automatically looks in `/Users/bogdanboksan/Library/Android/sdk/ndk`
- Ensure you have at least one NDK version installed via Android Studio

**Binding generation fails:**
- Make sure you've built the library for your host platform first
- The uniffi-bindgen needs the compiled `.dylib` file

**Missing Android targets:**
- Run: `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`

**Build fails with clang errors:**
- Install `cargo-ndk`: `cargo install cargo-ndk`
- The build script will use it automatically

## Usage

### Initialize the library

```kotlin
import com.navia.DidComInterface
import com.navia.DidCommError

// Create an instance
val didcomm = DidComInterface("/path/to/database")

// Open the database with a seed
val seed = "your-seed-string".toByteArray()
try {
    didcomm.open("/path/to/database", seed)
} catch (e: DidCommError) {
    // Handle error
}
```

### Generate a DID

```kotlin
try {
    val did = didcomm.generateDid(
        uri = "https://example.com/didcomm",
        routingKeys = listOf()
    )
    println("Generated DID: $did")
} catch (e: DidCommError) {
    // Handle error
}
```

### Pack a message

```kotlin
// Using the new structured message type (recommended)
val message = DidCommMessage(
    id = "1234567890",
    msgType = "https://example.org/protocols/lets_do_lunch/1.0/proposal",
    body = "Your message content here",
    from = "did:peer:1234...",
    to = listOf("did:peer:5678...")
)

try {
    val packedMessage = didcomm.pack(
        msg = message,
        from = "did:peer:1234...",
        to = "did:peer:5678..."
    )
    println("Packed message: $packedMessage")
} catch (e: DidCommError) {
    // Handle error
}

// Legacy JSON string method (still supported)
val messageJson = """
{
    "id": "1234567890",
    "type": "https://example.org/protocols/lets_do_lunch/1.0/proposal",
    "from": "did:peer:1234...",
    "to": ["did:peer:5678..."],
    "body": {
        "messagespecificattribute": "value"
    }
}
"""

try {
    val packedMessage = didcomm.packJson(
        msg = messageJson,
        from = "did:peer:1234...",
        to = "did:peer:5678..."
    )
    println("Packed message: $packedMessage")
} catch (e: DidCommError) {
    // Handle error
}
```

### Unpack a message

```kotlin
// Using the new structured return type (recommended)
try {
    val unpackedMessage: DidCommMessage = didcomm.unpack(packedMessage)
    println("Message ID: ${unpackedMessage.id}")
    println("Message Type: ${unpackedMessage.msgType}")
    println("Message Body: ${unpackedMessage.body}")
    println("From: ${unpackedMessage.from}")
    println("To: ${unpackedMessage.to}")
} catch (e: DidCommError) {
    // Handle error
}

// Legacy JSON string method (still supported)
try {
    val unpackedJson: String = didcomm.unpackJson(packedMessage)
    println("Unpacked message: $unpackedJson")
} catch (e: DidCommError) {
    // Handle error
}
```

### Database operations

```kotlin
// Insert
didcomm.insert("category", "key", "value")

// Get
val value = didcomm.get("category", "key")

// Update
didcomm.update("category", "key", "new_value")

// Remove
didcomm.remove("category", "key")

// Batch operations (more efficient for multiple items)
// Batch insert
val items = listOf(
    KeyValue(key = "key1", value = "value1", metadata = null),
    KeyValue(key = "key2", value = "value2", metadata = null),
    KeyValue(key = "key3", value = "value3", metadata = null)
)
didcomm.insertBatch("category", items)

// Batch get
val keys = listOf("key1", "key2", "key3")
val results: List<KeyValue> = didcomm.getBatch("category", keys)
results.forEach { kv ->
    println("${kv.key}: ${kv.value}")
}
```

## Error Handling

All methods throw `DidCommError` which can be one of:
- `DatabaseError`
- `ParsingError`
- `DidGenerationError`
- `PackingError`
- `UnpackingError`
- `GeneralError`

## Publishing

Navia is published as a private Android AAR package to GitHub Packages.

### Version Release

To publish a new version:

1. Create and push a version tag:
```bash
git tag v1.0.4
git push origin v1.0.4
```

2. The GitHub Actions workflow will automatically:
   - Build the library for all architectures
   - Generate Kotlin bindings
   - Publish to GitHub Packages

### Manual Publishing

For manual publishing during development:

```bash
# Build with package configuration
cd scripts
./build-for-android.sh --package

# Publish to GitHub Packages
cd ../android
./gradlew publish -PVERSION_NAME=1.0.4
```

### Consuming the Package

In your Android project's `settings.gradle.kts`:

```kotlin
dependencyResolutionManagement {
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/Nyx-Chat/navia")
            credentials {
                username = "your-github-username"
                password = "your-github-token-with-read-packages"
            }
        }
    }
}
```

Then in your `build.gradle.kts`:

```kotlin
dependencies {
    implementation("com.nyx:navia:1.0.4")
    implementation("net.java.dev.jna:jna:5.13.0@aar")
}
```

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development instructions.

## License

Proprietary - Nyx Chat