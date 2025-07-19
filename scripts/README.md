# Navia Build Script

## ONE Script: `build-for-android.sh`

This is the ONLY build script. It does everything.

### Usage

```bash
# Local development (builds for your device, debug mode)
./build-for-android.sh

# Local development (optimized build)
./build-for-android.sh --release

# Local development (all architectures)
./build-for-android.sh --all

# Package build (for publishing to GitHub Packages)
./build-for-android.sh --package
```

### What happens:

**Local mode** (default):
- Builds only for your connected device/emulator
- Outputs to `nyx-android/navia/src/main/`
- Fast iteration for development

**Package mode** (`--package`):
- Builds for ALL Android architectures
- Outputs to `navia/android/src/main/`
- Used by GitHub Actions to publish

**All architectures mode** (`--all`):
- Builds for ALL Android architectures
- Outputs to `nyx-android/navia/src/main/`
- Useful for testing multi-architecture support locally

That's it. One script, multiple modes.