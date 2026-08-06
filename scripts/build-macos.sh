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

# The Swift executable links the dylib by its absolute path in the build tree,
# so the bundled copy was never actually used and the app broke as soon as
# target/ was cleaned. Point both at the copy inside the bundle.
CORE_REF="$(otool -L "$BUNDLE_DIR/Contents/MacOS/Rhythm" \
    | awk '/librhythm_core\.dylib/ { print $1; exit }')"
if [ -n "$CORE_REF" ]; then
    install_name_tool -change "$CORE_REF" \
        "@executable_path/../Frameworks/librhythm_core.dylib" \
        "$BUNDLE_DIR/Contents/MacOS/Rhythm"
fi
install_name_tool -id "@executable_path/../Frameworks/librhythm_core.dylib" \
    "$BUNDLE_DIR/Contents/Frameworks/librhythm_core.dylib"

# Ad-hoc sign so the bundle launches on any Mac (must follow install_name_tool,
# which invalidates any existing signature)
codesign --force --deep --sign - "$BUNDLE_DIR" 2>/dev/null || true

# Fail loudly rather than shipping a bundle that only runs on this machine
if otool -L "$BUNDLE_DIR/Contents/MacOS/Rhythm" | grep -q "$PROJECT_DIR/target"; then
    echo "Error: bundle still references the build tree" >&2
    exit 1
fi

echo "==> App bundle: $BUNDLE_DIR"
echo "    Run: open $BUNDLE_DIR"
