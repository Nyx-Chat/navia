#!/bin/bash

# Build script for Android targets
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Navia for Android...${NC}"

# Set up paths
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/.."
RUST_DIR="$PROJECT_ROOT/rust"
ANDROID_DIR="$PROJECT_ROOT/android"
ANDROID_LIBS_DIR="$ANDROID_DIR/src/main/jniLibs"

# Android targets
TARGETS=(
    "aarch64-linux-android"    # arm64-v8a
    "armv7-linux-androideabi"  # armeabi-v7a
    "i686-linux-android"       # x86
    "x86_64-linux-android"     # x86_64
)

# Corresponding Android ABI names
ANDROID_ABIS=(
    "arm64-v8a"
    "armeabi-v7a"
    "x86"
    "x86_64"
)

# Ensure Android targets are installed
echo -e "${YELLOW}Ensuring Android targets are installed...${NC}"
for target in "${TARGETS[@]}"; do
    rustup target add "$target"
done

# Set up Android NDK
if [ -z "$ANDROID_NDK_HOME" ]; then
    # Try to find NDK in the default location
    DEFAULT_NDK_PATH="/Users/bogdanboksan/Library/Android/sdk/ndk"
    if [ -d "$DEFAULT_NDK_PATH" ]; then
        # Find the latest NDK version
        NDK_VERSION=$(ls -1 "$DEFAULT_NDK_PATH" | sort -V | tail -n 1)
        if [ -n "$NDK_VERSION" ]; then
            export ANDROID_NDK_HOME="$DEFAULT_NDK_PATH/$NDK_VERSION"
            echo -e "${YELLOW}ANDROID_NDK_HOME not set, using: $ANDROID_NDK_HOME${NC}"
        else
            echo -e "${RED}Error: No NDK version found in $DEFAULT_NDK_PATH${NC}"
            exit 1
        fi
    else
        echo -e "${RED}Error: ANDROID_NDK_HOME is not set and NDK not found at default location${NC}"
        echo "Please set ANDROID_NDK_HOME to your Android NDK path"
        exit 1
    fi
fi

# Create output directories
mkdir -p "$ANDROID_LIBS_DIR"
for abi in "${ANDROID_ABIS[@]}"; do
    mkdir -p "$ANDROID_LIBS_DIR/$abi"
done

# Set up cargo-ndk if available, otherwise set up CC/AR manually
if command -v cargo-ndk &> /dev/null; then
    echo -e "${GREEN}Using cargo-ndk for building...${NC}"
    # Build for each target using cargo-ndk
    cd "$RUST_DIR"
    for i in "${!TARGETS[@]}"; do
        target="${TARGETS[$i]}"
        abi="${ANDROID_ABIS[$i]}"
        
        echo -e "${YELLOW}Building for $target ($abi)...${NC}"
        
        # Build the Rust library with cargo-ndk
        cargo ndk --target "$target" --platform 21 -- build --release
        
        # Copy the library to the Android project
        cp "target/$target/release/libnavia_core.so" "$ANDROID_LIBS_DIR/$abi/"
        
        echo -e "${GREEN}✓ Built for $abi${NC}"
    done
else
    echo -e "${YELLOW}cargo-ndk not found, setting up manual build environment...${NC}"
    
    # Set up environment variables for Android build
    export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin:$PATH"
    
    # Build for each target
    cd "$RUST_DIR"
    for i in "${!TARGETS[@]}"; do
        target="${TARGETS[$i]}"
        abi="${ANDROID_ABIS[$i]}"
        
        echo -e "${YELLOW}Building for $target ($abi)...${NC}"
        
        # Set up target-specific environment variables
        case "$target" in
            "aarch64-linux-android")
                export CC="aarch64-linux-android21-clang"
                export AR="llvm-ar"
                ;;
            "armv7-linux-androideabi")
                export CC="armv7a-linux-androideabi21-clang"
                export AR="llvm-ar"
                ;;
            "i686-linux-android")
                export CC="i686-linux-android21-clang"
                export AR="llvm-ar"
                ;;
            "x86_64-linux-android")
                export CC="x86_64-linux-android21-clang"
                export AR="llvm-ar"
                ;;
        esac
        
        # Build the Rust library
        cargo build --target "$target" --release
        
        # Copy the library to the Android project
        cp "target/$target/release/libnavia_core.so" "$ANDROID_LIBS_DIR/$abi/"
        
        echo -e "${GREEN}✓ Built for $abi${NC}"
    done
fi

# Generate Kotlin bindings
echo -e "${YELLOW}Generating Kotlin bindings...${NC}"
cd "$RUST_DIR/navia-core"
# Use one of the Android target libraries for binding generation
TARGET_LIB="target/aarch64-linux-android/release/libnavia_core.so"
if [ -f "../$TARGET_LIB" ]; then
    cargo run --features cli --bin uniffi-bindgen generate --library "../$TARGET_LIB" --language kotlin --out-dir "$ANDROID_DIR/src/main/java"
else
    echo -e "${RED}Error: Target library not found at ../$TARGET_LIB${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Build complete!${NC}"
echo -e "${GREEN}Libraries are in: $ANDROID_LIBS_DIR${NC}"
echo -e "${GREEN}Kotlin bindings are in: $ANDROID_DIR/src/main/java${NC}"