# Navia Package Setup Guide

This guide explains how to set up Navia as a private Android package using GitHub Packages.

## Overview

Instead of copying files or using submodules, Navia is published as a proper Android library (AAR) that nyx-android can depend on.

## Structure

```
navia/                          # Rust library repository
├── rust/                       # Rust source code
├── android/                    # Android library module
│   ├── build.gradle.kts       # Android library config
│   └── src/main/              # Generated artifacts go here
├── scripts/
│   ├── build-for-android.sh          # Local development
│   └── build-for-android-package.sh  # Package build
└── .github/workflows/
    └── publish.yml            # Publishes to GitHub Packages

nyx-android/                   # Android app repository
├── app/                       # Your app
└── build.gradle.kts          # Configure to use package
```

## Publishing Navia (One-time setup)

### 1. Create a GitHub Personal Access Token

1. Go to GitHub Settings → Developer settings → Personal access tokens
2. Create a token with `write:packages` and `read:packages` permissions
3. Save this token securely

### 2. Test local build

```bash
cd navia/scripts
./build-for-android-package.sh
```

This creates the Android library structure with all architectures.

### 3. Publish to GitHub Packages

```bash
# Tag a version
cd navia
git tag v1.0.0
git push origin v1.0.0

# This triggers the GitHub Action to publish
```

Or manually:
```bash
cd navia/android
./gradlew publish -PVERSION_NAME=1.0.0 \
  -Pgpr.user=YOUR_GITHUB_USERNAME \
  -Pgpr.key=YOUR_GITHUB_TOKEN
```

## Using in nyx-android

### 1. Configure authentication

Create `~/.gradle/gradle.properties`:
```properties
gpr.user=YOUR_GITHUB_USERNAME
gpr.key=YOUR_GITHUB_TOKEN
```

### 2. Update nyx-android settings

In `nyx-android/settings.gradle.kts`, remove the navia module reference:
```kotlin
// Remove these lines:
// include(":navia")
// project(":navia").projectDir = File(rootDir.parentFile, "navia/android")
```

### 3. Add repository and dependency

In `nyx-android/build.gradle.kts` (or app/build.gradle.kts):

```kotlin
repositories {
    google()
    mavenCentral()
    maven {
        name = "GitHubPackages"
        url = uri("https://maven.pkg.github.com/Nyx-Chat/navia")
        credentials {
            username = project.findProperty("gpr.user") as String? ?: System.getenv("USERNAME")
            password = project.findProperty("gpr.key") as String? ?: System.getenv("TOKEN")
        }
    }
}

dependencies {
    // Remove local project dependency:
    // implementation(project(":navia"))
    
    // Add package dependency:
    implementation("com.nyx:navia:1.0.0")
}
```

### 4. For CI/CD

In GitHub Actions for nyx-android:
```yaml
- name: Configure Gradle
  run: |
    echo "gpr.user=${{ github.actor }}" >> ~/.gradle/gradle.properties
    echo "gpr.key=${{ secrets.GITHUB_TOKEN }}" >> ~/.gradle/gradle.properties
```

## Version Management

### Publishing new versions

1. Make changes to Rust code
2. Update version in `navia/android/gradle.properties`
3. Create and push tag:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```
4. GitHub Action publishes automatically

### Using new versions

In nyx-android, update the dependency:
```kotlin
implementation("com.nyx:navia:1.0.1")
```

## Benefits

1. **Version Control**: Use specific versions, can rollback
2. **Caching**: Gradle caches packages locally
3. **Professional**: Standard Android development practice
4. **CI/CD Friendly**: No submodules or file copying
5. **Private**: Only accessible with authentication

## Troubleshooting

### "Could not find com.nyx:navia"
- Check authentication is configured
- Verify package was published successfully
- Check repository URL is correct

### "Received status code 401"
- GitHub token may be expired
- Token needs `read:packages` permission

### Local Development

For rapid iteration during development:
1. Use `build-for-android.sh` to build directly into nyx-android
2. Temporarily use local project dependency
3. Switch back to package dependency for commits