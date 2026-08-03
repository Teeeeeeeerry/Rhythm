# Rhythm

跨平台音乐播放器。本地音乐 + 在线链接，Win/Mac 双平台。

[English](README.en.md) | 中文

## 功能

- 本地音乐导入，按 Artist/Album 或首字母浏览
- 粘贴 YouTube/Bilibili 链接直接播音频流
- 本地与在线内容混合播放列表，链接失效自动跳过
- 全文搜索（标题/艺人/专辑/流派）
- 播放队列：顺序 / 随机 / 单曲循环 / 列表循环
- 专辑封面自动提取与显示
- 系统媒体键支持（播放/暂停/上一首/下一首）
- 全局快捷键（Space 播放暂停、Cmd+左右箭头切歌）
- 系统托盘模式（关闭窗口不退出）
- 中英文界面（跟随系统语言、支持手动切换）
- 支持格式：MP3、AAC、FLAC、WAV、OGG、ALAC、APE、WMA、AIFF、WavPack、MP4/M4A

## 架构

Rhythm 的架构分为两层。上层是平台原生 UI：macOS 端用 Swift 和 AppKit 编写，Windows 端用 C++ 和 WinUI 3 编写，两套 UI 各自遵循对应平台的设计语言，但在功能上保持对等。下层是一个用 Rust 编写的共享核心库，负责所有与平台无关的逻辑——音频解码与播放、元数据提取、资料库管理、播放列表、播放队列、URL 解析——然后通过 C-ABI 编译为 .dylib 和 .dll，供两端 UI 直接调用。这种双原生 UI + 单一 Rust 核心的策略，既能保证界面与系统深度融合、内存占用可控，又避免了在两端重复实现相同的底层逻辑。

## 开发状态

| Phase | 内容 | 状态 |
|-------|------|------|
| 1 | Rust 核心库 | 完成 |
| 2 | macOS UI | 完成 |
| 3 | Windows UI | 完成 |
| 4 | 打磨 | 完成 |

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

尚未确定。在选定许可证之前，本仓库保留所有权利。
