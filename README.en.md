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
- Chinese / English interface (macOS follows system language with manual override; Windows follows system language, with manual override via registry `HKCU\Software\Rhythm\AppLanguage`)
- Supported formats: MP3, AAC, FLAC, WAV, OGG, ALAC, APE, WMA, AIFF, WavPack, MP4/M4A

## Architecture

Rhythm has a two-layer architecture. The upper layer is the platform-native UI: the macOS client is written in Swift with AppKit, and the Windows client is written in C++ with WinUI 3. Each UI follows its platform's design language while keeping feature parity. The lower layer is a shared Rust core library that handles all platform-independent logic — audio decoding and playback, metadata extraction, library management, playlists, play queue, URL resolution — compiled as `.dylib` and `.dll` via the C-ABI for direct consumption by both UIs. This dual-native UI + single Rust core strategy ensures deep OS integration and controlled memory usage, while avoiding duplicated logic across platforms.

## Development Status

Initial development is complete. Current version: **v0.5.203 "Motif"** (the single source is `[workspace.package] version` in `Cargo.toml`; this line is synced by `python3 scripts/tasks.py bump-version` - do not edit it by hand).

### Implementation Status

| Feature | Status | Issue |
|---------|--------|-------|
| Local audio playback (MP3/FLAC/AAC/WAV/OGG/ALAC/APE/WMA/AIFF/WavPack/MP4) | Done | — |
| Library management + FTS5 full-text search | Done | — |
| Local import (folder / single file / batch, with per-count partial success) | Done (parity on both platforms) | [#218](https://github.com/Teeeeeeeerry/Rhythm/issues/218) |
| Playlists (mixed local/online, M3U8 import/export; import policy has a single source in the core) | Done | [#217](https://github.com/Teeeeeeeerry/Rhythm/issues/217) |
| Play queue (sequential/shuffle/single-loop/list-loop) | Done | — |
| Album cover extraction | Done | — |
| System media keys + tray mode | Done | — |
| Chinese/English localization | Done (macOS manual override + Windows registry override) | [#141](https://github.com/Teeeeeeeerry/Rhythm/issues/141), [#145](https://github.com/Teeeeeeeerry/Rhythm/issues/145) |
| Failure and status copy (the core picks which copy; both UIs only fill templates) | Done (identical on both platforms, in both languages) | [#216](https://github.com/Teeeeeeeerry/Rhythm/issues/216) |
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
- **Windows**: Visual Studio 2022 (or Build Tools, with the MSVC and Windows 10/11 SDK components) + CMake 3.21+ + Python 3.
  The Windows App SDK, C++/WinRT and nlohmann/json need **no manual install**: on the first configure
  `windows/cmake/RhythmWindowsDeps.cmake` downloads them at pinned versions from NuGet / GitHub, verifies
  SHA-256, generates the projection headers and caches everything in `build/windows-deps/` (the first
  configure needs network access, #386)

Building and testing both go through one cross-platform task entry; the task
names are the same on both platforms (#221):

```bash
python3 scripts/tasks.py                      # list every task
python3 scripts/tasks.py build                # build the app for this platform
python3 scripts/tasks.py test                 # full test suite for this platform
python3 scripts/tasks.py bump-version         # bump the version (last digit +1, or pass e.g. 0.6.0)
python3 scripts/tasks.py check-no-emoji       # zero-emoji check
python3 scripts/tasks.py compare-screenshots  # L2 screenshot pixel diff against goldens
```

Exit codes: `0` all green / `1` a step failed (a filter that leaves zero steps also counts, #343) / `2` usage error.

### macOS

```bash
python3 scripts/tasks.py build
```

Builds the Rust core and the Swift executable, then assembles `build/Rhythm.app`:
rewrites the dynamic library reference to the bundled copy, ad-hoc signs it, and
asserts the bundle no longer references the build tree (otherwise it only runs on
this machine).

### Windows

```bat
python3 scripts\tasks.py build
```

Builds the Rust core, the CMake behaviour library and the WinUI 3 app (MSBuild, #428), producing `build\windows\Release\Rhythm.exe` (self-contained: no separate Windows App Runtime install).

### Run Tests

```bash
python3 scripts/tasks.py test     # both platforms first run the same static analysis: nine L0 checks + zero-emoji + two self-test suites
                                  # macOS then runs L1 unit tests (swift test + ASan)
                                  # Windows then runs L1 unit (--smoke adds L3; L2 screenshot diff not implemented yet, #387)
cargo test -p rhythm-core         # Rust core behaviour tests
```

The CI workflows (`testing/ci/ci.yml`, `visual.yml`) are **undeployed templates**: the repository has no
`.github/workflows/`, so the checks above only run locally; once deployed, CI calls the same commands (#346).

Use `--l0-only` for static analysis alone. Expected failures must be waived
explicitly (`--allow-expected-failures`, or `ALLOW_EXPECTED_FAILURES=1`); the
default strict mode exits non-zero as soon as any step goes red.

## Single-Source Conventions

Data shared across platforms is declared in exactly one place; everything else is generated or derived, and L0 checks catch drift:

| What | Single source | How it is derived | Issue |
|------|---------------|-------------------|-------|
| Version | `[workspace.package] version` in `Cargo.toml` | Doc copies synced by `bump-version`; the macOS bundle and Windows build config are written at build time | [#220](https://github.com/Teeeeeeeerry/Rhythm/issues/220) |
| Colour palette | `testing/palette.json` | `scripts/gen-palette.py` generates both platforms' theme code | [#219](https://github.com/Teeeeeeeerry/Rhythm/issues/219) |
| L10n keys | `contracts/l10n-keys.json` | `scripts/gen-l10n.py` generates both key tables and the Windows named accessors | [#167](https://github.com/Teeeeeeeerry/Rhythm/issues/167), [#371](https://github.com/Teeeeeeeerry/Rhythm/issues/371) |
| FFI contract | `contracts/ffi-contract.json` | `scripts/gen-ffi-bindings.py` generates both codecs; output must pass the text diff and compile on its own | [#180](https://github.com/Teeeeeeeerry/Rhythm/issues/180), [#323](https://github.com/Teeeeeeeerry/Rhythm/issues/323), [#369](https://github.com/Teeeeeeeerry/Rhythm/issues/369) |
| Build and test orchestration | `scripts/tasks.py` | Same task names on both platforms; the CI templates call the same commands (not yet deployed) | [#221](https://github.com/Teeeeeeeerry/Rhythm/issues/221), [#346](https://github.com/Teeeeeeeerry/Rhythm/issues/346) |

Do not hand-edit code inside generator markers: change the source and regenerate. On Windows, what to
render is decided by the view state in `windows/Rhythm/ViewState.h`; the XAML shell only pours the
result into controls ([#317](https://github.com/Teeeeeeeerry/Rhythm/issues/317)).

## Tech Stack

- Audio: symphonia (decoding) + cpal (output)
- Metadata: lofty + symphonia
- Database: SQLite + FTS5
- URL resolution: yt-dlp

## License

[MIT](LICENSE).
