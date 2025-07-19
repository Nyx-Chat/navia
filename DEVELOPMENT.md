# Navia Development Guide

This guide covers the development workflow for the Navia library, including local development, building, testing, and publishing.

## Prerequisites

- Rust (latest stable)
- Android NDK r25b
- Android SDK
- Java 17
- `cargo-ndk` tool

## Project Structure

```
navia/
├── rust/                    # Rust source code
│   ├── navia-core/         # Core library with UniFFI bindings
│   ├── navia-did/          # DID functionality
│   └── Cargo.toml          # Workspace configuration
├── android/                # Android library wrapper
│   ├── src/               # Generated Kotlin bindings
│   └── build.gradle.kts   # Android build configuration
├── scripts/               # Build scripts
│   └── build-for-android.sh
└── .github/workflows/     # CI/CD workflows
```

## Local Development

### 1. Building the Library

The project includes a unified build script that handles all build scenarios:

```bash
# Default: Build for local development
cd scripts
./build-for-android.sh

# Build in release mode
./build-for-android.sh --release

# Build for package publishing
./build-for-android.sh --package
```

The build script will:
1. Build Rust libraries for all Android architectures
2. Generate Kotlin bindings using UniFFI
3. Copy artifacts to the Android project
4. Build the Android AAR

### 2. Testing

Run Rust tests:
```bash
cd rust
cargo test
```

### 3. Dependency Management

The project uses specific dependency versions to ensure compatibility:

- `askar-storage`: Using git version from main branch for sqlx 0.8 compatibility
- `sqlx`: Version 0.8.x
- Other dependencies: Check `Cargo.toml` for versions

**Important**: Do not delete `Cargo.lock` as it maintains compatible dependency versions.

## Publishing

### Version Tagging

The library uses semantic versioning. To publish a new version:

```bash
# Create and push a version tag
git tag v1.0.4
git push origin v1.0.4
```

This will trigger the GitHub Actions workflow to build and publish the package.

### Manual Publishing

For local publishing, you need a Personal Access Token:

1. Create a PAT with `write:packages` scope on GitHub
2. Save it in `~/.gradle/gradle.properties`:
   ```properties
   gpr.user=your-github-username
   gpr.token=your-personal-access-token
   ```

3. Build and publish:
   ```bash
   cd scripts
   ./build-for-android.sh --package
   cd ../android
   ./gradlew publish -PVERSION_NAME=1.0.4
   ```

You can also trigger the publish workflow manually from GitHub:
1. Go to Actions tab on GitHub
2. Select "Publish Navia Package" workflow
3. Click "Run workflow"

### GitHub Packages Configuration

The library is published to GitHub Packages as a private package. The `android/build.gradle.kts` configures:

```kotlin
publishing {
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/Nyx-Chat/navia")
            credentials {
                // GitHub Actions uses these environment variables automatically
                // For local publishing, it falls back to gradle.properties
                username = System.getenv("GITHUB_ACTOR") ?: project.findProperty("gpr.user") as String?
                password = System.getenv("GITHUB_TOKEN") ?: project.findProperty("gpr.token") as String?
            }
        }
    }
}
```

This configuration allows:
- **GitHub Actions**: Automatic authentication using GITHUB_TOKEN
- **Local publishing**: Uses your PAT from `~/.gradle/gradle.properties`

## CI/CD

### Workflows

1. **CI Workflow** (`ci.yml`): Runs on every push and PR
   - Builds Rust code
   - Runs tests
   - Checks formatting

2. **Publish Workflow** (`publish.yml`): Runs on version tags
   - Builds release artifacts
   - Publishes to GitHub Packages

### Build Matrix

The CI builds and tests across:
- Ubuntu (latest)
- Rust stable toolchain
- Android NDK r25b

## Troubleshooting

### Dependency Conflicts

If you encounter sqlx-related build errors:
1. Ensure you're using the git version of `askar-storage`
2. Check that `Cargo.lock` exists and is committed
3. Run `cargo update -p package_name` for specific updates only

### Build Issues

1. **Missing NDK**: Ensure `ANDROID_NDK_ROOT` is set
2. **UniFFI errors**: Check that `uniffi-bindgen` is installed
3. **Gradle errors**: Ensure Java 17 is installed and active

### Publishing Issues

1. **Authentication**: Ensure `GITHUB_TOKEN` has `packages:write` permission
2. **Version conflicts**: Increment version in tag before publishing
3. **Build failures**: Check that local build works before tagging

## Development Workflow

1. Create feature branch from `main`
2. Make changes and test locally
3. Ensure `./build-for-android.sh` runs successfully
4. Create PR and wait for CI to pass
5. Merge to main
6. Tag release when ready to publish

## Security Considerations

- Never commit credentials or tokens
- Keep dependencies updated for security patches
- Review dependency changes carefully
- Use `cargo audit` to check for vulnerabilities