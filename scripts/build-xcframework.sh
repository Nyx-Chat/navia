#!/bin/bash

# Builds Navia for iOS as a static XCFramework, with the Swift bindings
# generated from the same build. It writes, under ios/build:
#
# - navia_coreFFI.xcframework: libnavia_core.a (the crate's staticlib) for iOS
#   devices (aarch64-apple-ios) and Apple Silicon simulators
#   (aarch64-apple-ios-sim), each slice with
#   Headers/navia_coreFFI/{navia_coreFFI.h,module.modulemap}
# - navia_coreFFI.xcframework.zip, the XCFramework zipped, and
#   navia_coreFFI.xcframework.zip.checksum, the zip's SwiftPM checksum
# - navia_core.swift, the Swift bindings
#
# The bindings, the header and the module map are also written to
# ios/Navia/Generated, which the root Package.swift compiles as the Navia
# target and CI compares with the committed copy. They come from the device
# library of this build, so their UniFFI API checksums match the binary;
# bindings from any other build fail at the first call with "UniFFI API
# checksum mismatch".
#
# Usage: scripts/build-xcframework.sh
#
# Needs Xcode with the iOS SDKs. Honours CARGO_TARGET_DIR; with
# CARGO_NET_OFFLINE=true cargo, including the `cargo metadata` that
# uniffi-bindgen runs, builds from the fetched dependencies only.

set -euo pipefail

# Colors (disabled in CI)
if [ -t 1 ] && [ -z "${CI:-}" ]; then
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m'
else
    GREEN=''
    YELLOW=''
    NC=''
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NAVIA_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_DIR="$NAVIA_ROOT/rust"
GENERATED_DIR="$NAVIA_ROOT/ios/Navia/Generated"
OUTPUT_DIR="$NAVIA_ROOT/ios/build"
STAGING_DIR="$OUTPUT_DIR/xcframework-staging"

# Keep in step with `platforms` in Package.swift.
export IPHONEOS_DEPLOYMENT_TARGET="17.0"
DEVICE_TARGET="aarch64-apple-ios"
SIMULATOR_TARGET="aarch64-apple-ios-sim"

FFI_MODULE="navia_coreFFI"
XCFRAMEWORK="$OUTPUT_DIR/$FFI_MODULE.xcframework"
ZIP="$XCFRAMEWORK.zip"
CHECKSUM_FILE="$ZIP.checksum"

for tool in cargo xcrun xcodebuild swift ditto; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "ERROR: $tool not found" >&2
        exit 1
    fi
done
for sdk in iphoneos iphonesimulator; do
    if ! xcrun --sdk "$sdk" --show-sdk-path >/dev/null 2>&1; then
        echo "ERROR: the active Xcode ($(xcode-select -p)) has no $sdk SDK" >&2
        exit 1
    fi
done

# Link with the active Xcode's clang; rust/.cargo/config.toml names fixed
# paths under /Applications/Xcode.app, and these variables take precedence.
CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$(xcrun --sdk iphoneos --find clang)"
CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER="$(xcrun --sdk iphonesimulator --find clang)"
export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER

# cargo resolves a relative CARGO_TARGET_DIR against its working directory,
# and this script changes directory, so pin it to the caller's.
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    case "$CARGO_TARGET_DIR" in
        /*) ;;
        *) CARGO_TARGET_DIR="$PWD/$CARGO_TARGET_DIR" ;;
    esac
    export CARGO_TARGET_DIR
    TARGET_DIR="$CARGO_TARGET_DIR"
else
    TARGET_DIR="$RUST_DIR/target"
fi

if command -v rustup >/dev/null 2>&1; then
    INSTALLED_TARGETS="$(rustup target list --installed)"
    for target in "$DEVICE_TARGET" "$SIMULATOR_TARGET"; do
        if ! grep -qx "$target" <<< "$INSTALLED_TARGETS"; then
            echo -e "${YELLOW}Installing Rust target $target...${NC}"
            rustup target add "$target"
        fi
    done
fi

echo -e "${GREEN}Building navia_coreFFI.xcframework (iOS $IPHONEOS_DEPLOYMENT_TARGET+)...${NC}"

cd "$RUST_DIR"
for target in "$DEVICE_TARGET" "$SIMULATOR_TARGET"; do
    echo -e "${YELLOW}Building libnavia_core.a for $target...${NC}"
    cargo build --locked --release -p navia-core --lib --target "$target"
    if [ ! -f "$TARGET_DIR/$target/release/libnavia_core.a" ]; then
        echo "ERROR: $TARGET_DIR/$target/release/libnavia_core.a was not built" >&2
        exit 1
    fi
done
DEVICE_LIB="$TARGET_DIR/$DEVICE_TARGET/release/libnavia_core.a"
SIMULATOR_LIB="$TARGET_DIR/$SIMULATOR_TARGET/release/libnavia_core.a"

rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR/bindings" "$STAGING_DIR/headers/$FFI_MODULE"

echo -e "${YELLOW}Generating Swift bindings from $DEVICE_LIB...${NC}"
cd "$RUST_DIR/navia-core"
# --no-format: uniffi-bindgen otherwise runs swiftformat when the machine has it,
# so the output would depend on the machine. The committed ios/Navia/Generated is
# unformatted, and CI fails when the build's output differs from it.
cargo run --locked --features cli --bin uniffi-bindgen -- generate \
    --library "$DEVICE_LIB" \
    --language swift \
    --no-format \
    --out-dir "$STAGING_DIR/bindings"

for file in navia_core.swift "$FFI_MODULE.h"; do
    if [ ! -f "$STAGING_DIR/bindings/$file" ]; then
        echo "ERROR: uniffi-bindgen did not generate $file" >&2
        exit 1
    fi
done

# The headers sit in a navia_coreFFI subdirectory of each slice's Headers, so
# the module map does not collide with another XCFramework's module.modulemap
# in an app's build; clang finds it there by the module's name.
cp "$STAGING_DIR/bindings/$FFI_MODULE.h" "$STAGING_DIR/headers/$FFI_MODULE/"
cat > "$STAGING_DIR/headers/$FFI_MODULE/module.modulemap" << EOF
module $FFI_MODULE {
    header "$FFI_MODULE.h"
    export *
}
EOF

mkdir -p "$GENERATED_DIR"
cp "$STAGING_DIR/bindings/navia_core.swift" "$GENERATED_DIR/"
cp "$STAGING_DIR/headers/$FFI_MODULE/$FFI_MODULE.h" "$GENERATED_DIR/"
cp "$STAGING_DIR/headers/$FFI_MODULE/module.modulemap" "$GENERATED_DIR/"

echo -e "${YELLOW}Creating $XCFRAMEWORK...${NC}"
rm -rf "$XCFRAMEWORK" "$ZIP" "$CHECKSUM_FILE" "$OUTPUT_DIR/navia_core.swift"
xcodebuild -create-xcframework \
    -library "$DEVICE_LIB" -headers "$STAGING_DIR/headers" \
    -library "$SIMULATOR_LIB" -headers "$STAGING_DIR/headers" \
    -output "$XCFRAMEWORK"

cp "$GENERATED_DIR/navia_core.swift" "$OUTPUT_DIR/"

cd "$OUTPUT_DIR"
ditto -c -k --sequesterRsrc --keepParent "$FFI_MODULE.xcframework" "$ZIP"
swift package compute-checksum "$ZIP" > "$CHECKSUM_FILE"

rm -rf "$STAGING_DIR"

echo -e "${GREEN}✅ navia_coreFFI.xcframework built${NC}"
echo "XCFramework: $XCFRAMEWORK"
echo "Zip:         $ZIP"
echo "Checksum:    $CHECKSUM_FILE ($(cat "$CHECKSUM_FILE"))"
echo "Swift:       $OUTPUT_DIR/navia_core.swift"
