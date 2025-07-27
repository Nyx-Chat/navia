#!/bin/bash

# THE ONE BUILD SCRIPT FOR NAVIA
# Handles all scenarios: local dev, packaging, CI/CD

set -e

# Colors (disabled in CI)
if [ -t 1 ] && [ -z "$CI" ]; then
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m'
else
    GREEN=''
    YELLOW=''
    NC=''
fi

echo -e "${GREEN}Building Navia for Android...${NC}"

# Script location
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
NAVIA_ROOT="$SCRIPT_DIR/.."
RUST_DIR="$NAVIA_ROOT/rust"

# Default output locations
LOCAL_OUTPUT="$NAVIA_ROOT/../nyx-android/navia/src/main"
PACKAGE_OUTPUT="$NAVIA_ROOT/android/src/main"

# Parse arguments
BUILD_MODE="debug"
BUILD_ALL_ARCHS="false"
OUTPUT_DIR="$LOCAL_OUTPUT"  # Default to local development

for arg in "$@"; do
    case $arg in
        --release)
            BUILD_MODE="release"
            ;;
        --all-architectures|--all)
            BUILD_ALL_ARCHS="true"
            ;;
        --package)
            OUTPUT_DIR="$PACKAGE_OUTPUT"
            BUILD_ALL_ARCHS="true"  # Always build all archs for package
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --release              Build in release mode (optimized)"
            echo "  --all-architectures    Build for all Android architectures"
            echo "  --package              Build for packaging (outputs to android/)"
            echo "  --help                 Show this help"
            echo ""
            echo "Examples:"
            echo "  $0                     # Local dev (debug, current arch)"
            echo "  $0 --release           # Local dev (release, current arch)"
            echo "  $0 --package           # Package build (all archs, android/)"
            echo "  $0 --all               # Local dev (all archs)"
            exit 0
            ;;
    esac
done

# In CI or when --all-architectures is passed, build everything
if [ "$CI" = "true" ] || [ "$BUILD_ALL_ARCHS" = "true" ]; then
    echo -e "${YELLOW}Building for ALL architectures (CI/Release mode)${NC}"
    TARGETS=(
        "aarch64-linux-android:arm64-v8a"
        "armv7-linux-androideabi:armeabi-v7a"
        "i686-linux-android:x86"
        "x86_64-linux-android:x86_64"
    )
else
    # Local development - build only for connected device or arm64
    echo -e "${YELLOW}Building for local development${NC}"
    if command -v adb &> /dev/null && adb devices | grep -q "device$"; then
        ABI=$(adb shell getprop ro.product.cpu.abi 2>/dev/null || echo "arm64-v8a")
    else
        ABI="arm64-v8a"
    fi
    
    case "$ABI" in
        "arm64-v8a") TARGET="aarch64-linux-android" ;;
        "armeabi-v7a") TARGET="armv7-linux-androideabi" ;;
        "x86") TARGET="i686-linux-android" ;;
        "x86_64") TARGET="x86_64-linux-android" ;;
        *) TARGET="aarch64-linux-android"; ABI="arm64-v8a" ;;
    esac
    
    TARGETS=("$TARGET:$ABI")
fi

# Install Rust targets
echo -e "${YELLOW}Installing Rust targets...${NC}"
for target_pair in "${TARGETS[@]}"; do
    TARGET="${target_pair%%:*}"
    rustup target add "$TARGET"
done

# Setup Android NDK
if [ -z "$ANDROID_NDK_HOME" ]; then
    # Prioritize NDK r28 for automatic 16KB support
    for NDK_PATH in \
        "$HOME/Library/Android/sdk/ndk/28."* \
        "$HOME/Android/Sdk/ndk/28."* \
        "$HOME/Library/Android/sdk/ndk/"* \
        "$HOME/Android/Sdk/ndk/"* \
        "/usr/local/android-sdk/ndk/"*
    do
        if [ -d "$NDK_PATH" ]; then
            export ANDROID_NDK_HOME="$NDK_PATH"
            echo -e "${YELLOW}Using NDK: $NDK_PATH${NC}"
            break
        fi
    done
fi

if [ -z "$ANDROID_NDK_HOME" ]; then
    echo "ERROR: Android NDK not found!"
    echo "Please install Android NDK and set ANDROID_NDK_HOME"
    exit 1
fi

# Build Rust library
cd "$RUST_DIR"

# Ensure we use the committed Cargo.lock
if [ ! -f "Cargo.lock" ]; then
    echo "ERROR: Cargo.lock not found!"
    exit 1
fi

CARGO_FLAGS=""
if [ "$BUILD_MODE" = "release" ]; then
    CARGO_FLAGS="--release"
fi

# Create output directories
mkdir -p "$OUTPUT_DIR/java"
mkdir -p "$OUTPUT_DIR/jniLibs"

# Build for each target
for target_pair in "${TARGETS[@]}"; do
    TARGET="${target_pair%%:*}"
    ABI="${target_pair##*:}"
    
    echo -e "${YELLOW}Building for $TARGET ($ABI)...${NC}"
    
    # Detect host OS for NDK path
    CLANG_SUFFIX=""
    if [[ "$OSTYPE" == "darwin"* ]]; then
        NDK_HOST="darwin-x86_64"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        NDK_HOST="linux-x86_64"
    elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
        NDK_HOST="windows-x86_64"
        CLANG_SUFFIX=".cmd"
    else
        echo "Unsupported OS: $OSTYPE"
        exit 1
    fi
    
    # For NDK r28, we need to ensure proper 16KB alignment
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_HOST/bin/aarch64-linux-android21-clang${CLANG_SUFFIX}"
    export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_HOST/bin/armv7a-linux-androideabi21-clang${CLANG_SUFFIX}"
    export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_HOST/bin/i686-linux-android21-clang${CLANG_SUFFIX}"
    export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_HOST/bin/x86_64-linux-android21-clang${CLANG_SUFFIX}"
    
    # Add NDK to PATH for build tools
    export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_HOST/bin:$PATH"
    
    # Set CC for the target
    case "$TARGET" in
        "aarch64-linux-android") export CC="aarch64-linux-android21-clang${CLANG_SUFFIX}" ;;
        "armv7-linux-androideabi") export CC="armv7a-linux-androideabi21-clang${CLANG_SUFFIX}" ;;
        "i686-linux-android") export CC="i686-linux-android21-clang${CLANG_SUFFIX}" ;;
        "x86_64-linux-android") export CC="x86_64-linux-android21-clang${CLANG_SUFFIX}" ;;
    esac
    
    # Build with proper alignment
    # For NDK r27+, we need to set the minSdkVersion to ensure 16KB support
    if [[ "$ANDROID_NDK_HOME" == *"/27."* ]] || [[ "$ANDROID_NDK_HOME" == *"/28."* ]]; then
        export ANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON
    fi
    
    cargo build --target "$TARGET" $CARGO_FLAGS
    
    # Copy .so file
    mkdir -p "$OUTPUT_DIR/jniLibs/$ABI"
    SO_FILE="target/$TARGET/$BUILD_MODE/libnavia_core.so"
    DEST_FILE="$OUTPUT_DIR/jniLibs/$ABI/libnavia_core.so"
    
    cp "$SO_FILE" "$DEST_FILE"
    
    # Apply 16KB alignment fix if needed
    if [ -x "$SCRIPT_DIR/fix-android-16kb.sh" ]; then
        echo "Applying 16KB alignment fix..."
        "$SCRIPT_DIR/fix-android-16kb.sh" "$DEST_FILE" || true
    fi
done

# Generate Kotlin bindings (use first built library)
echo -e "${YELLOW}Generating Kotlin bindings...${NC}"
cd "$RUST_DIR/navia-core"

FIRST_TARGET="${TARGETS[0]%%:*}"
LIB_PATH="../target/$FIRST_TARGET/$BUILD_MODE/libnavia_core.so"

cargo run --features cli --bin uniffi-bindgen generate \
    --library "$LIB_PATH" \
    --language kotlin \
    --out-dir "$OUTPUT_DIR/java"

echo -e "${GREEN}✅ Build complete!${NC}"
echo -e "${GREEN}Output: $OUTPUT_DIR${NC}"

# If in CI and package mode, we're done
# (Android library build happens separately in the workflow)