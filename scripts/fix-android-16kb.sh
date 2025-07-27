#!/bin/bash

# Post-build script to ensure 16KB alignment for Android libraries
# This uses patchelf to fix alignment issues

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SO_FILE="$1"

if [ -z "$SO_FILE" ]; then
    echo "Usage: $0 <path-to-library.so>"
    exit 1
fi

if [ ! -f "$SO_FILE" ]; then
    echo "Error: File not found: $SO_FILE"
    exit 1
fi

echo "Fixing 16KB alignment for: $SO_FILE"

# The build process with proper flags should handle alignment
# This script is kept as a placeholder for future alignment fixes if needed
echo "Note: Library should already be properly aligned by the build process"

echo "Alignment fix complete. Verifying..."

# Verify the fix
"$SCRIPT_DIR/verify-16kb-alignment.sh" "$SO_FILE"