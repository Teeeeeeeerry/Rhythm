#!/bin/bash
# Build the Rust core library for macOS
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RUST_DIR="$PROJECT_DIR/rust-core"

echo "==> Building rhythm-core for macOS..."

# Target macOS (should be native)
TARGET="${1:-aarch64-apple-darwin}"

cd "$RUST_DIR"
cargo build --release --target "$TARGET"

# Copy outputs
OUT_DIR="$PROJECT_DIR/build/macos"
mkdir -p "$OUT_DIR"

if [[ "$TARGET" == *"apple"* ]]; then
    cp "target/$TARGET/release/librhythm_core.dylib" "$OUT_DIR/"
    cp "include/rhythm_core.h" "$OUT_DIR/"
    echo "==> Output: $OUT_DIR/"
    echo "    librhythm_core.dylib"
    echo "    rhythm_core.h"
else
    echo "Error: Expected Apple target, got $TARGET"
    exit 1
fi

echo "==> Done!"
