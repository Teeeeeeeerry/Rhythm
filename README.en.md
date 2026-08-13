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

Initial development is complete. Current version: **v0.5.26 "Motif"**.

### Implementation Status

| Feature | Status | Issue |
|---------|--------|-------|
| Local audio playback (MP3/FLAC/AAC/WAV/OGG/ALAC/APE/WMA/AIFF/WavPack/MP4) | Done | — |
| Library management + FTS5 full-text search | Done | — |
| Playlists (mixed local/online, M3U8 import/export) | Done | — |
| Play queue (sequential/shuffle/single-loop/list-loop) | Done | — |
| Album cover extraction | Done | — |
| System media keys + tray mode | Done | — |
| Chinese/English localization | Done | — |
| URL streaming (YouTube/Bilibili/direct links) | Done | [#11](https://github.com/Teeeeeeeerry/Rhythm/issues/11) |
| URL input UI | Done | [#12](https://github.com/Teeeeeeeerry/Rhythm/issues/12) |
| Resolver error reporting + yt-dlp auto-install | Done | [#21](https://github.com/Teeeeeeeerry/Rhythm/issues/21) |
| Tray menu usability fix | Done | [#24](https://github.com/Teeeeeeeerry/Rhythm/issues/24) |
| Streaming playback fixes (dropped samples, buffering state, seek) | Done | [#23](https://github.com/Teeeeeeeerry/Rhythm/issues/23) |

## Playing online links

**Nothing to install.** Paste a YouTube / Bilibili link and it plays — on the first link, Rhythm downloads the yt-dlp build it needs (~36 MB), showing progress next to the URL field. It is downloaded once.

What it does:

- Keeps its copy in `~/Library/Application Support/Rhythm/bin/` (Windows: `%LOCALAPPDATA%\Rhythm\bin\`)
- Downloads only from yt-dlp's official GitHub release, verified against the published `SHA2-256SUMS`; a mismatched download is discarded
- Checks for updates weekly in the background, and if a site rejects the current version, updates and retries once
- Reuses an existing yt-dlp (Homebrew, MacPorts, pip, scoop, winget, …) instead of downloading its own
- Prefers AAC/M4A audio (the bundled decoder has no Opus support) and reuses the headers yt-dlp reports — Bilibili's CDN answers 403 without a Referer

To manage yt-dlp yourself:

```bash
export RHYTHM_NO_AUTO_INSTALL=1               # turn off auto-download
export RHYTHM_YTDLP_PATH=/your/path/to/yt-dlp # use your own binary
```

When resolution fails, the app says why — network error, timeout, video unavailable, yt-dlp too old — and writes the details to a log:

- macOS: `~/Library/Logs/Rhythm/resolver.log`
- Windows: `%LOCALAPPDATA%\Rhythm\logs\resolver.log`

## Build

### Prerequisites

- **Rust** 1.70+ ([rustup.rs](https://rustup.rs))
- yt-dlp needs no prior install: the app fetches it on the first online link
- **macOS**: Xcode 15+ or Command Line Tools + Swift 5.9+
- **Windows**: Visual Studio 2022 + Windows App SDK + CMake 3.20+

### macOS

```bash
# 1. Build Rust core library
cargo build --release -p rhythm-core

# 2. Build Swift UI
cd macos && swift build -c release

# 3. Create .app bundle (output: $PROJECT_ROOT/build/)
mkdir -p "$PROJECT_ROOT/build/Rhythm.app/Contents/"{MacOS,Resources,Frameworks}
cp .build/release/Rhythm "$PROJECT_ROOT/build/Rhythm.app/Contents/MacOS/"
cp ../target/release/librhythm_core.dylib "$PROJECT_ROOT/build/Rhythm.app/Contents/Frameworks/"
cp Rhythm/Resources/Info.plist "$PROJECT_ROOT/build/Rhythm.app/Contents/"
sed -i '' 's/\$(EXECUTABLE_NAME)/Rhythm/' "$PROJECT_ROOT/build/Rhythm.app/Contents/Info.plist"

# 4. Sign
codesign --force --deep --sign - "$PROJECT_ROOT/build/Rhythm.app"
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
