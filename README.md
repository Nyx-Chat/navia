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

- 📱 [Android Integration Guide](ANDROID_INTEGRATION.md) - Complete Android setup and usage
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
    implementation("com.nyx:navia:1.5.0")
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
```

Navia is published to GitHub Packages, so your Gradle repositories need
`https://maven.pkg.github.com/Nyx-Chat/navia` with a GitHub user and a token
that has the `read:packages` scope (see
[Troubleshooting](docs/TROUBLESHOOTING.md#failed-to-resolve-comnyxnaviaxxx)).

#### iOS

Add to your `Podfile`:

```ruby
pod 'Navia', :git => 'https://github.com/Nyx-Chat/navia.git', :tag => 'v1.5.0'
```

The pod does not link today, and the release XCFramework may fail to load at
launch; see [Installation in the iOS Integration Guide](docs/IOS_INTEGRATION.md#installation)
before you use either.

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

// Receive: act on `from` only when unpack proved it
val received = didcomm.unpack(incomingFrame)
val sender = received.from
if (received.authenticated && sender != null) {
    // sender is the DID of the key that proves it: received.encryptedFromKid
    // for an authcrypt frame (what Navia's pack sends), received.signFrom
    // otherwise. encryptedFromKid is null on a signature-only frame; require
    // it too if you accept authcrypt proof only.
    handleMessageFrom(sender, received)
}
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

// Receive: act on `from` only when unpack proved it
let received = try await didcomm.unpack(msg: incomingFrame)
if received.authenticated, let sender = received.from {
    // sender is the DID of the key that proves it: received.encryptedFromKid
    // for an authcrypt frame (what Navia's pack sends), received.signFrom
    // otherwise. encryptedFromKid is nil on a signature-only frame; require
    // it too if you accept authcrypt proof only.
    handleMessage(from: sender, received)
}
```

`unpack` fills `authenticated`, `encryptedFromKid`, `signFrom` and
`anonymousSender` from the unpack metadata and refuses an authcrypt frame whose
plaintext `from` names another DID than its sender key (`UnpackingError`).
`authenticated` also requires the frame to be tied to you: authcrypt does that
on its own; a signature-only frame needs every recipient key of its envelope
to belong to a DID in its `to`, so a message signed for someone else and
relayed to you comes back `false`. A message you build for `pack` leaves
these four fields at their defaults; see
[Sender authentication fields](docs/API.md#sender-authentication-fields).

For detailed integration instructions, see the [Android Integration Guide](ANDROID_INTEGRATION.md) or [iOS Integration Guide](docs/IOS_INTEGRATION.md).

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
   - Triggers on a push to `main` that changes `rust/navia-core/Cargo.toml`
   - Tags `v<version>` itself, and skips the release when that tag already exists
   - Takes the binaries from the release PR's own PR Validation run
   - Publishes the AAR to GitHub Packages
   - Creates the GitHub release and attaches `Navia.xcframework-<version>.zip`

### Publishing New Version

Bump every version in one release commit:

1. `rust/navia-core/Cargo.toml` `version`, and the `navia-core` entry in `rust/Cargo.lock`
2. `android/gradle.properties` `VERSION_NAME`
3. `Navia.podspec` `spec.version`
4. `ios/Navia.xcodeproj/project.pbxproj` `MARKETING_VERSION = <version>`, in all
   four build configurations (Navia and NaviaTests, Debug and Release). Nothing
   derives it from `Cargo.toml`, so it stays behind unless it is bumped here.
5. The install snippets in `README.md`, `ANDROID_INTEGRATION.md`,
   `docs/IOS_INTEGRATION.md` and `docs/TROUBLESHOOTING.md`
6. `CHANGELOG.md`: the section for the new version

Then merge the commit to `main` through a pull request. The push that changes
`rust/navia-core/Cargo.toml` starts the release; do not tag by hand, since the
workflow skips a version whose tag already exists.

The release does not build the libraries itself. It publishes the Android
libraries and iOS frameworks that the release PR's own PR Validation run built
(the successful run for the PR's head commit), never those of another PR. So:

- Merge only after PR Validation passed on the PR's final commit, and within 7
  days of that run, when its artifacts expire.
- PR Validation builds the PR merged into `main` as `main` stood when it ran. If
  `main` has moved since, update the PR branch so validation runs again.
- When no such run holds the artifacts, the release fails before it tags or
  publishes anything. Re-run all jobs of the PR's PR Validation run from the
  Actions tab, then re-run the failed release workflow.

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