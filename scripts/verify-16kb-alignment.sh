#!/bin/bash

# Script to verify 16KB alignment of Android native libraries

if [ -z "$1" ]; then
    echo "Usage: $0 <path-to-library.so>"
    exit 1
fi

LIBRARY="$1"
NDK_PATH="${ANDROID_NDK_HOME:-$HOME/Library/Android/sdk/ndk/28.2.13676358}"
READELF="$NDK_PATH/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-readelf"

if [ ! -f "$READELF" ]; then
    echo "Error: llvm-readelf not found at $READELF"
    exit 1
fi

echo "Checking 16KB alignment for: $LIBRARY"
echo "========================================="

# Check LOAD segments
echo "LOAD segments:"
$READELF -l "$LIBRARY" | grep LOAD | while read line; do
    # Extract offset and virtual address
    offset=$(echo $line | awk '{print $2}')
    vaddr=$(echo $line | awk '{print $3}')
    
    # Convert hex to decimal and check alignment
    offset_dec=$((16#${offset#0x}))
    vaddr_dec=$((16#${vaddr#0x}))
    
    offset_aligned=$((offset_dec % 16384))
    vaddr_aligned=$((vaddr_dec % 16384))
    
    if [ $offset_aligned -eq 0 ] && [ $vaddr_aligned -eq 0 ]; then
        echo "✅ $line"
    else
        echo "❌ $line"
        echo "   Offset alignment: $offset_aligned (should be 0)"
        echo "   VirtAddr alignment: $vaddr_aligned (should be 0)"
    fi
done

echo ""
echo "Summary:"
if $READELF -l "$LIBRARY" | grep LOAD | grep -q "0x4000$"; then
    echo "✅ All LOAD segments have 16KB alignment flag (0x4000)"
else
    echo "❌ Some LOAD segments missing 16KB alignment flag"
fi