#!/bin/bash

# Quick test to ensure 16KB alignment is working
# Exit with error if any library has misaligned virtual addresses

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
FAILED=0

echo "Testing 16KB alignment for all architectures..."

for arch in arm64-v8a x86_64; do
    lib="../nyx-android/navia/src/main/jniLibs/$arch/libnavia_core.so"
    if [ -f "$lib" ]; then
        echo "Checking $arch..."
        if ! "$SCRIPT_DIR/verify-16kb-alignment.sh" "$lib"; then
            echo "❌ $arch failed alignment check"
            FAILED=1
        fi
    fi
done

if [ $FAILED -eq 0 ]; then
    echo "✅ All libraries are properly aligned for Android 15+"
    exit 0
else
    echo "❌ Some libraries are not properly aligned"
    exit 1
fi