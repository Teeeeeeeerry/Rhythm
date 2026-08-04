#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "==> Building Rust core..."
cd "$PROJECT_DIR"
cargo build --release -p rhythm-core

echo "==> Building macOS app..."
cd "$PROJECT_DIR/macos"
swift build -c release

echo "==> Creating app bundle..."
BUNDLE_DIR="$PROJECT_DIR/build/Rhythm.app"
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"
mkdir -p "$BUNDLE_DIR/Contents/Frameworks"

cp "$PROJECT_DIR/macos/.build/release/Rhythm" "$BUNDLE_DIR/Contents/MacOS/"
cp "$PROJECT_DIR/macos/Rhythm/Resources/Info.plist" "$BUNDLE_DIR/Contents/"
# Replace the Xcode build variable placeholder with the real executable name
sed -i '' 's/\$(EXECUTABLE_NAME)/Rhythm/' "$BUNDLE_DIR/Contents/Info.plist"
cp "$PROJECT_DIR/target/release/librhythm_core.dylib" "$BUNDLE_DIR/Contents/Frameworks/"

# Ad-hoc sign so the bundle launches on any Mac
codesign --force --deep --sign - "$BUNDLE_DIR" 2>/dev/null || true

echo "==> App bundle: $BUNDLE_DIR"
echo "    Run: open $BUNDLE_DIR"
