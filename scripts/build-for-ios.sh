#!/bin/bash

# THE ONE BUILD SCRIPT FOR NAVIA iOS
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

echo -e "${GREEN}Building Navia for iOS...${NC}"

# Script location
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
NAVIA_ROOT="$SCRIPT_DIR/.."
RUST_DIR="$NAVIA_ROOT/rust"

# Default output locations
LOCAL_OUTPUT="$NAVIA_ROOT/../nyx-ios/Navia/Generated"
PACKAGE_OUTPUT="$NAVIA_ROOT/ios/Navia/Generated"

# Parse arguments
BUILD_MODE="debug"
BUILD_ALL_ARCHS="false"
OUTPUT_DIR="$LOCAL_OUTPUT"  # Default to local development
CONFIGURATION="Debug"       # Xcode configuration

for arg in "$@"; do
    case $arg in
        --release)
            BUILD_MODE="release"
            ;;
        --configuration)
            shift
            CONFIGURATION="$1"
            if [ "$CONFIGURATION" = "Release" ]; then
                BUILD_MODE="release"
            fi
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
            echo "  --configuration CONFIG Xcode configuration (Debug/Release)"
            echo "  --all-architectures    Build for all iOS architectures"
            echo "  --package              Build for packaging (outputs to ios/)"
            echo "  --help                 Show this help"
            echo ""
            echo "Examples:"
            echo "  $0                             # Local dev (debug, current arch)"
            echo "  $0 --release                   # Local dev (release, current arch)"
            echo "  $0 --configuration Release     # Xcode Release build"
            echo "  $0 --package                   # Package build (all archs, ios/)"
            echo "  $0 --all                       # Local dev (all archs)"
            exit 0
            ;;
    esac
    shift || true
done

# In CI or when --all-architectures is passed, build everything
if [ "$CI" = "true" ] || [ "$BUILD_ALL_ARCHS" = "true" ]; then
    echo -e "${YELLOW}Building for ALL iOS architectures (CI/Release mode)${NC}"
    TARGETS=(
        "aarch64-apple-ios:arm64"           # iOS device
        "x86_64-apple-ios:x86_64"           # iOS simulator (Intel)
        "aarch64-apple-ios-sim:arm64"       # iOS simulator (Apple Silicon)
    )
else
    # Local development - build for current architecture
    echo -e "${YELLOW}Building for local iOS development${NC}"
    # Detect if we're on Apple Silicon or Intel Mac for simulator
    if [[ $(uname -m) == "arm64" ]]; then
        TARGETS=("aarch64-apple-ios-sim:arm64")  # Apple Silicon simulator
    else
        TARGETS=("x86_64-apple-ios:x86_64")      # Intel simulator
    fi
fi

# Install Rust targets
echo -e "${YELLOW}Installing Rust targets...${NC}"
for target_pair in "${TARGETS[@]}"; do
    TARGET="${target_pair%%:*}"
    rustup target add "$TARGET"
done

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
mkdir -p "$OUTPUT_DIR"

# Build for each target
BUILT_LIBS=()
for target_pair in "${TARGETS[@]}"; do
    TARGET="${target_pair%%:*}"
    ARCH="${target_pair##*:}"
    
    echo -e "${YELLOW}Building for $TARGET ($ARCH)...${NC}"
    
    # Build with cargo
    cargo build --target "$TARGET" $CARGO_FLAGS
    
    # Store library path for lipo
    LIB_PATH="target/$TARGET/$BUILD_MODE/libnavia_core.a"
    BUILT_LIBS+=("$LIB_PATH")
done

# Create universal binary if we built multiple architectures
if [ ${#BUILT_LIBS[@]} -gt 1 ]; then
    echo -e "${YELLOW}Creating universal binary...${NC}"
    mkdir -p "target/universal/$BUILD_MODE"
    lipo -create "${BUILT_LIBS[@]}" -output "target/universal/$BUILD_MODE/libnavia_core.a"
    FINAL_LIB="target/universal/$BUILD_MODE/libnavia_core.a"
else
    FINAL_LIB="${BUILT_LIBS[0]}"
fi

# Generate Swift bindings
echo -e "${YELLOW}Generating Swift bindings...${NC}"
cd "$RUST_DIR/navia-core"

cargo run --features cli --bin uniffi-bindgen generate \
    --library "../$FINAL_LIB" \
    --language swift \
    --out-dir "$OUTPUT_DIR"

# Move the generated files to the expected locations
if [ -f "$OUTPUT_DIR/navia_core.swift" ]; then
    # The generated Swift file is already in the right place
    echo -e "${YELLOW}Swift bindings generated successfully${NC}"
else
    echo "ERROR: Swift bindings generation failed!"
    exit 1
fi

if [ -f "$OUTPUT_DIR/navia_coreFFI.h" ]; then
    # The generated header file is already in the right place
    echo -e "${YELLOW}FFI header generated successfully${NC}"
else
    echo "ERROR: FFI header generation failed!"
    exit 1
fi

echo -e "${GREEN}✅ iOS build complete!${NC}"
echo -e "${GREEN}Output: $OUTPUT_DIR${NC}"
echo -e "${GREEN}Library: $FINAL_LIB${NC}"

# Show file sizes for verification
if command -v ls >/dev/null 2>&1; then
    echo -e "${YELLOW}Generated files:${NC}"
    ls -la "$OUTPUT_DIR"
fi