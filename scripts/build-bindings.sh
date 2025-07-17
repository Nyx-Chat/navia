#!/bin/bash

# Build script for generating Kotlin bindings
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Navia library and generating Kotlin bindings...${NC}"

# Set up paths
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/.."
RUST_DIR="$PROJECT_ROOT/rust"
ANDROID_DIR="$PROJECT_ROOT/android"

# Build the library
echo -e "${YELLOW}Building Rust library...${NC}"
cd "$RUST_DIR"
cargo build --release --lib

# Build uniffi-bindgen
echo -e "${YELLOW}Building uniffi-bindgen...${NC}"
cargo build --release --bin uniffi-bindgen --features cli

# Generate Kotlin bindings
echo -e "${YELLOW}Generating Kotlin bindings...${NC}"
cd "$RUST_DIR/navia-core"
../../target/release/uniffi-bindgen generate --library ../../target/release/libnavia_core.dylib --language kotlin --out-dir "$ANDROID_DIR/src/main/java"

echo -e "${GREEN}✅ Build complete!${NC}"
echo -e "${GREEN}Kotlin bindings are in: $ANDROID_DIR/src/main/java${NC}"
echo -e "${YELLOW}Note: To build for Android targets, you'll need to set ANDROID_NDK_HOME and run build-android.sh${NC}"