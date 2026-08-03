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

Initial development is complete.

## Build

Prerequisites: Rust 1.70+, yt-dlp

```bash
cd rust-core && cargo build --release
cargo test
```

## Tech Stack

- Audio: symphonia (decoding) + cpal (output)
- Metadata: lofty + symphonia
- Database: SQLite + FTS5
- URL resolution: yt-dlp

## License

To be determined. All rights reserved until a license is chosen.
