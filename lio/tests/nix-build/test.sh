#!/usr/bin/env bash
set -e

# Determine project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TARGET_DIR="$PROJECT_ROOT/target/release"

echo $TARGET_DIR

echo "Building the Nix package..."
nix build

echo ""
echo "Setting PKG_CONFIG_PATH..."
export PKG_CONFIG_PATH="$PROJECT_ROOT/result/lib/pkgconfig:$PKG_CONFIG_PATH"

echo ""
echo "Testing pkg-config queries..."
echo "1. Check if lio.pc is found:"
pkg-config --exists lio && echo "   ✓ lio.pc found" || echo "   ✗ lio.pc not found"

echo ""
echo "2. Package version:"
pkg-config --modversion lio

echo ""
echo "3. Compiler flags:"
pkg-config --cflags lio

echo ""
echo "4. Linker flags:"
pkg-config --libs lio

echo ""
echo "5. Static linker flags:"
pkg-config --libs --static lio

echo ""
echo "6. All variables:"
pkg-config --print-variables lio

echo ""
echo "Compiling test program with pkg-config..."
gcc $SCRIPT_DIR/test_pkgconfig.c $(pkg-config --cflags --libs lio) -o $TARGET_DIR/test_pkgconfig

echo ""
echo "Running test program..."

$TARGET_DIR/test_pkgconfig

echo ""
echo "✓ All tests passed!"
