# Rhythm

A cross-platform music player — local music + online links, for both Windows and macOS.

[中文](README.md) | English

## Features

- Import local music, browse by Artist/Album or by first letter
- Paste YouTube/Bilibili links to play audio streams directly
- Mixed playlists combining local and online tracks, broken links auto-skipped
- Full-text search (title / artist / album / genre)
- Play queue: sequential / shuffle / single-loop / list-loop
- Automatic album cover extraction and display
- System media key support (play / pause / previous / next)
- Global keyboard shortcuts (Space to toggle playback, Cmd+arrows to skip)
- System tray mode (closing the window does not quit the app)
- Chinese / English interface (follows system language, with manual override)
- Supported formats: MP3, AAC, FLAC, WAV, OGG, ALAC, APE, WMA, AIFF, WavPack, MP4/M4A

## Architecture

Rhythm has a two-layer architecture. The upper layer is the platform-native UI: the macOS client is written in Swift with AppKit, and the Windows client is written in C++ with WinUI 3. Each UI follows its platform's design language while keeping feature parity. The lower layer is a shared Rust core library that handles all platform-independent logic — audio decoding and playback, metadata extraction, library management, playlists, play queue, URL resolution — compiled as `.dylib` and `.dll` via the C-ABI for direct consumption by both UIs. This dual-native UI + single Rust core strategy ensures deep OS integration and controlled memory usage, while avoiding duplicated logic across platforms.

## Development Status

Initial development is complete. Current version: **v0.5.0 "Germ"**.

### Known Limitations

- **URL Streaming**: The URL resolver (yt-dlp integration) is ready, but the HTTP streaming + decoding + playback pipeline is not yet implemented ([#11](https://github.com/Teeeeeeeerry/Rhythm/issues/11)). Only local file playback works at this time.
- **URL Input UI**: There is no UI for pasting URLs yet ([#12](https://github.com/Teeeeeeeerry/Rhythm/issues/12)).

### Implementation Status

| Feature | Status | Issue |
|---------|--------|-------|
| Local audio playback (MP3/FLAC/AAC/WAV/OGG/ALAC/APE/WMA/AIFF/WavPack/MP4) | ✅ Done | — |
| Library management + FTS5 full-text search | ✅ Done | — |
| Playlists (mixed local/online, M3U8 import/export) | ✅ Done | — |
| Play queue (sequential/shuffle/single-loop/list-loop) | ✅ Done | — |
| Album cover extraction | ✅ Done | — |
| System media keys + tray mode | ✅ Done | — |
| Chinese/English localization | ✅ Done | — |
| URL streaming (YouTube/Bilibili/direct links) | 🔧 Pending | [#11](https://github.com/Teeeeeeeerry/Rhythm/issues/11) |
| URL input UI | 🔧 Pending | [#12](https://github.com/Teeeeeeeerry/Rhythm/issues/12) |

## Build

### Prerequisites

- **Rust** 1.70+ ([rustup.rs](https://rustup.rs))
- **yt-dlp** (for URL resolution; optional if only playing local files)
- **macOS**: Xcode 15+ or Command Line Tools + Swift 5.9+
- **Windows**: Visual Studio 2022 + Windows App SDK + CMake 3.20+

### macOS

```bash
# 1. Build Rust core library
cargo build --release -p rhythm-core

# 2. Build Swift UI
cd macos && swift build -c release

# 3. Create .app bundle (output: macos/build/)
mkdir -p build/Rhythm.app/Contents/{MacOS,Resources,Frameworks}
cp .build/release/Rhythm build/Rhythm.app/Contents/MacOS/
cp ../target/release/librhythm_core.dylib build/Rhythm.app/Contents/Frameworks/
cp Rhythm/Resources/Info.plist build/Rhythm.app/Contents/
sed -i '' 's/\$(EXECUTABLE_NAME)/Rhythm/' build/Rhythm.app/Contents/Info.plist

# 4. Sign and create DMG
codesign --force --deep --sign - build/Rhythm.app
hdiutil create -volname Rhythm -srcfolder build/Rhythm.app -ov -format UDZO build/Rhythm.dmg
```

Or use the one-shot script:
```bash
bash scripts/build-macos.sh
```

### Windows

```bash
# 1. Build Rust core DLL (on Windows, or cross-compile)
cargo build --release -p rhythm-core --target x86_64-pc-windows-msvc

# 2. Build WinUI 3 app
cd windows && mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
```

Or use the one-shot script (on Windows):
```bat
scripts\build-windows.bat
```

### Run Tests

```bash
cargo test -p rhythm-core
```

## Tech Stack

- Audio: symphonia (decoding) + cpal (output)
- Metadata: lofty + symphonia
- Database: SQLite + FTS5
- URL resolution: yt-dlp

## License

To be determined. All rights reserved until a license is chosen.
