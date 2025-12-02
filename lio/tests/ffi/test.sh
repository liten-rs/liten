#!/usr/bin/env bash
set -e

# Determine project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TARGET_DIR="$PROJECT_ROOT/target/release"

# Build library
make lio-cbuild

echo "lio C compilation successful"

# Compile C test
gcc  "$SCRIPT_DIR/c-src.c" \
    -L"$TARGET_DIR" \
    -o "$TARGET_DIR/test_ffi_c" \
    -llio \
    -I"$PROJECT_ROOT/lio/include"

echo "C compilation successful"

# # Link C executable
# gcc "$TARGET_DIR/test_ffi_c.o" \
#     -L"$TARGET_DIR" \
#     -llio \
#     -o "$TARGET_DIR/test_ffi_c"

# Run C test
$TARGET_DIR/test_ffi_c

echo "C test passed"

# Compile C++ test
g++ "$SCRIPT_DIR/cpp-src.cpp" \
    -L"$TARGET_DIR" \
    -o "$TARGET_DIR/test_ffi_cpp" \
    -llio \
    -I"$PROJECT_ROOT/lio/include"

echo "C++ compilation successful"

# # Run C++ test
$TARGET_DIR/test_ffi_cpp

echo "C++ test passed"
