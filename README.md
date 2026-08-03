# Rhythm

跨平台音乐播放器。本地音乐 + 在线链接，Win/Mac 双平台。

## 功能

- 本地音乐导入，按 Artist/Album 或首字母浏览
- 粘贴 YouTube/Bilibili 链接直接播音频流
- 本地与在线内容混合播放列表，链接失效自动跳过
- 全文搜索（标题/艺人/专辑/流派）
- 系统托盘模式
- 支持格式：MP3、AAC、FLAC、WAV、OGG、ALAC、APE、WMA、AIFF、WavPack、MP4/M4A

## 架构

```
UI (macOS Swift · Windows WinUI3)
─────────────────────────────────
       Rust Core (C-ABI)
─────────────────────────────────
 Metadata · Audio · Library
 Playlist · Resolver · FFI
```

## 开发状态

| Phase | 内容 | 状态 |
|-------|------|------|
| 1 | Rust 核心库 | 完成 |
| 2 | macOS UI | 完成 |
| 3 | Windows UI | 待开发 |
| 4 | 打磨 | 待开发 |

## 构建

依赖：Rust 1.70+、yt-dlp

```bash
cd rust-core && cargo build --release
cargo test
```

## 技术选型

- 音频：symphonia（解码）+ cpal（输出）
- 元数据：lofty + symphonia
- 数据库：SQLite + FTS5
- URL 解析：yt-dlp

## 许可

MIT
