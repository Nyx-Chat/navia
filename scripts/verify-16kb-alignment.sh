#!/bin/bash

# Script to verify 16KB alignment of Android native libraries

if [ -z "$1" ]; then
    echo "Usage: $0 <path-to-library.so>"
    exit 1
fi

LIBRARY="$1"

# Use ANDROID_NDK_HOME if set, otherwise try to find NDK
if [ -z "$ANDROID_NDK_HOME" ]; then
    echo "Error: ANDROID_NDK_HOME not set"
    exit 1
fi
NDK_PATH="$ANDROID_NDK_HOME"

# Detect host OS for NDK path
if [[ "$OSTYPE" == "darwin"* ]]; then
    NDK_HOST="darwin-x86_64"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    NDK_HOST="linux-x86_64"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "win32" ]]; then
    NDK_HOST="windows-x86_64"
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

READELF="$NDK_PATH/toolchains/llvm/prebuilt/$NDK_HOST/bin/llvm-readelf"

if [ ! -f "$READELF" ]; then
    echo "Error: llvm-readelf not found at $READELF"
    exit 1
fi

echo "Checking 16KB alignment for: $LIBRARY"
echo "========================================="

# Check LOAD segments
echo "LOAD segments:"
vaddr_misaligned=false
offset_misaligned=false
has_align_flag=true

$READELF -l "$LIBRARY" | grep LOAD | while read line; do
    # Extract offset and virtual address
    offset=$(echo $line | awk '{print $2}')
    vaddr=$(echo $line | awk '{print $3}')
    align=$(echo $line | awk '{print $NF}')
    
    # Convert hex to decimal and check alignment
    offset_dec=$((16#${offset#0x}))
    vaddr_dec=$((16#${vaddr#0x}))
    
    offset_aligned=$((offset_dec % 16384))
    vaddr_aligned=$((vaddr_dec % 16384))
    
    # Check virtual address alignment (CRITICAL for Android 15+)
    if [ $vaddr_aligned -eq 0 ]; then
        vaddr_status="✅"
    else
        vaddr_status="❌"
        echo "vaddr_misaligned=true" >> /tmp/verify_status_$$
    fi
    
    # Check file offset alignment (less critical)
    if [ $offset_aligned -ne 0 ]; then
        echo "offset_misaligned=true" >> /tmp/verify_status_$$
    fi
    
    # Check alignment flag
    if [ "$align" != "0x4000" ]; then
        echo "has_align_flag=false" >> /tmp/verify_status_$$
    fi
    
    echo "$vaddr_status $line"
    if [ $vaddr_aligned -ne 0 ] || [ $offset_aligned -ne 0 ]; then
        [ $offset_aligned -ne 0 ] && echo "   File offset alignment: $offset_aligned (non-zero is okay if VirtAddr is aligned)"
        [ $vaddr_aligned -ne 0 ] && echo "   ❌ VirtAddr alignment: $vaddr_aligned (MUST be 0 for Android 15+)"
    fi
done

# Read status from temp file
if [ -f /tmp/verify_status_$$ ]; then
    source /tmp/verify_status_$$
    rm -f /tmp/verify_status_$$
fi

echo ""
echo "Summary:"
echo "========================================="

if [ "$has_align_flag" = true ]; then
    echo "✅ Alignment flag: All LOAD segments have 0x4000 flag"
else
    echo "❌ Alignment flag: Missing 16KB alignment flag"
fi

if [ "$vaddr_misaligned" = false ]; then
    echo "✅ Virtual addresses: All LOAD segments are 16KB aligned"
    echo "   This library should work on Android 15+ devices"
else
    echo "❌ Virtual addresses: Some segments are NOT 16KB aligned"
    echo "   This library WILL CRASH on Android 15+ devices"
fi

if [ "$offset_misaligned" = true ]; then
    echo "⚠️  File offsets: Not 16KB aligned (this is usually okay)"
fi

# Exit with error if virtual addresses are misaligned
if [ "$vaddr_misaligned" = true ]; then
    exit 1
fi