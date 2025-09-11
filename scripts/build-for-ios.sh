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

# Set up Cargo config for iOS cross-compilation
mkdir -p .cargo
cat > .cargo/config.toml << EOF
[target.aarch64-apple-ios]
linker = "$(xcrun --sdk iphoneos --find clang)"
ar = "$(xcrun --sdk iphoneos --find ar)"

[target.x86_64-apple-ios]  
linker = "$(xcrun --sdk iphonesimulator --find clang)"
ar = "$(xcrun --sdk iphonesimulator --find ar)"

[target.aarch64-apple-ios-sim]
linker = "$(xcrun --sdk iphonesimulator --find clang)" 
ar = "$(xcrun --sdk iphonesimulator --find ar)"

[env]
SDKROOT = "$(xcrun --sdk iphoneos --show-sdk-path)"
EOF

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
    
    # iOS cross-compilation is now handled by .cargo/config.toml
    
    # Build with cargo
    cargo build --target "$TARGET" $CARGO_FLAGS
    
    # For iOS, we always get dylib from cdylib crate type
    # Check what library file actually exists
    if [ -f "target/$TARGET/$BUILD_MODE/libnavia_core.a" ]; then
        LIB_EXT="a"
    elif [ -f "target/$TARGET/$BUILD_MODE/libnavia_core.dylib" ]; then
        LIB_EXT="dylib"
    else
        echo "ERROR: No libnavia_core library found in target/$TARGET/$BUILD_MODE/"
        ls -la "target/$TARGET/$BUILD_MODE/" | grep libnavia_core || echo "No libnavia_core files found"
        exit 1
    fi
    
    LIB_PATH="target/$TARGET/$BUILD_MODE/libnavia_core.$LIB_EXT"
    
    # Verify the library file exists
    if [ ! -f "$LIB_PATH" ]; then
        echo "ERROR: Expected library not found at $LIB_PATH"
        echo "Available files in target/$TARGET/$BUILD_MODE/:"
        ls -la "target/$TARGET/$BUILD_MODE/" | grep libnavia_core || echo "No libnavia_core files found"
        exit 1
    fi
    
    BUILT_LIBS+=("$LIB_PATH")
done

# For iOS, we'll use the first built library for UniFFI generation
# Multiple architectures will be handled at the XCFramework level
if [ ${#BUILT_LIBS[@]} -gt 1 ]; then
    # Check if we can create a universal binary by looking for compatible architectures
    SIMULATOR_LIBS=()
    DEVICE_LIBS=()
    
    for i in "${!TARGETS[@]}"; do
        TARGET="${TARGETS[$i]%%:*}"
        LIB="${BUILT_LIBS[$i]}"
        
        if [[ "$TARGET" == *"-sim" ]]; then
            SIMULATOR_LIBS+=("$LIB")
        else
            DEVICE_LIBS+=("$LIB")
        fi
    done
    
    # Try to create universal binaries for simulator and device separately
    mkdir -p "target/universal/$BUILD_MODE"
    
    # Use the same extension as the built libraries
    if [ "$BUILD_MODE" = "release" ]; then
        OUTPUT_EXT="a"
    else
        OUTPUT_EXT="$LIB_EXT"
    fi
    
    # Create simulator universal binary if we have multiple simulator targets
    if [ ${#SIMULATOR_LIBS[@]} -gt 1 ]; then
        echo -e "${YELLOW}Creating universal simulator binary...${NC}"
        lipo -create "${SIMULATOR_LIBS[@]}" -output "target/universal/$BUILD_MODE/libnavia_core_sim.$OUTPUT_EXT" 2>/dev/null || {
            echo -e "${YELLOW}Cannot combine simulator libraries, using first one...${NC}"
            cp "${SIMULATOR_LIBS[0]}" "target/universal/$BUILD_MODE/libnavia_core_sim.$OUTPUT_EXT"
        }
        # Store but don't use for UniFFI generation yet
    elif [ ${#SIMULATOR_LIBS[@]} -eq 1 ]; then
        cp "${SIMULATOR_LIBS[0]}" "target/universal/$BUILD_MODE/libnavia_core_sim.$OUTPUT_EXT"
        # Store but don't use for UniFFI generation yet
    fi
    
    # Create device universal binary if we have multiple device targets
    if [ ${#DEVICE_LIBS[@]} -gt 1 ]; then
        echo -e "${YELLOW}Creating universal device binary...${NC}"
        lipo -create "${DEVICE_LIBS[@]}" -output "target/universal/$BUILD_MODE/libnavia_core_device.$OUTPUT_EXT" 2>/dev/null || {
            echo -e "${YELLOW}Cannot combine device libraries, using first one...${NC}"
            cp "${DEVICE_LIBS[0]}" "target/universal/$BUILD_MODE/libnavia_core_device.$OUTPUT_EXT"
        }
        # Use original device library for UniFFI generation to avoid metadata issues
        FINAL_LIB="${DEVICE_LIBS[0]}"
    elif [ ${#DEVICE_LIBS[@]} -eq 1 ]; then
        cp "${DEVICE_LIBS[0]}" "target/universal/$BUILD_MODE/libnavia_core_device.$OUTPUT_EXT"
        FINAL_LIB="${DEVICE_LIBS[0]}"
    fi
    
    # Create the main library for Xcode to find (use simulator for local dev, device for CI)
    if [ -f "target/universal/$BUILD_MODE/libnavia_core_sim.$OUTPUT_EXT" ]; then
        cp "target/universal/$BUILD_MODE/libnavia_core_sim.$OUTPUT_EXT" "target/universal/$BUILD_MODE/libnavia_core.$OUTPUT_EXT"
        echo -e "${YELLOW}Created main library from simulator build${NC}"
    elif [ -f "target/universal/$BUILD_MODE/libnavia_core_device.$OUTPUT_EXT" ]; then
        cp "target/universal/$BUILD_MODE/libnavia_core_device.$OUTPUT_EXT" "target/universal/$BUILD_MODE/libnavia_core.$OUTPUT_EXT"
        echo -e "${YELLOW}Created main library from device build${NC}"
    fi
    
    # If we don't have a device library, use simulator for UniFFI generation
    if [ -z "$FINAL_LIB" ] && [ -n "${SIMULATOR_LIBS[0]}" ]; then
        FINAL_LIB="${SIMULATOR_LIBS[0]}"
    fi
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