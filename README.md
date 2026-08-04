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

初步开发完成。当前版本 **v0.4.3 "Germ"**。

### 已知限制

- **URL 流式播放**：URL 解析器（yt-dlp 集成）已就绪，但 HTTP 流式下载 + 解码 + 播放管道尚未实现（[#11](https://github.com/Teeeeeeeerry/Rhythm/issues/11)）。目前仅本地文件播放可用。
- **URL 输入 UI**：界面上尚未提供粘贴 URL 的入口（[#12](https://github.com/Teeeeeeeerry/Rhythm/issues/12)）。

### 待实现

| 功能 | 状态 | Issue |
|------|------|-------|
| 本地音乐播放（MP3/FLAC/AAC/WAV/OGG/ALAC/APE/WMA/AIFF/WavPack/MP4） | ✅ 完成 | — |
| 资料库管理 + FTS5 全文搜索 | ✅ 完成 | — |
| 播放列表（混合本地/在线，M3U8 导入导出） | ✅ 完成 | — |
| 播放队列（顺序/随机/单曲循环/列表循环） | ✅ 完成 | — |
| 专辑封面自动提取 | ✅ 完成 | — |
| 系统媒体键 + 托盘模式 | ✅ 完成 | — |
| 中英文界面 | ✅ 完成 | — |
| URL 流式播放（YouTube/Bilibili/直链） | 🔧 待实现 | [#11](https://github.com/Teeeeeeeerry/Rhythm/issues/11) |
| URL 输入 UI | 🔧 待实现 | [#12](https://github.com/Teeeeeeeerry/Rhythm/issues/12) |

## 构建

### 前置依赖

- **Rust** 1.70+（[rustup.rs](https://rustup.rs)）
- **yt-dlp**（URL 解析，可选：仅本地播放时不需要）
- **macOS**：Xcode 15+ 或 Command Line Tools + Swift 5.9+
- **Windows**：Visual Studio 2022 + Windows App SDK + CMake 3.20+

### macOS

```bash
# 1. 构建 Rust 核心库
cargo build --release -p rhythm-core

# 2. 构建 Swift UI
cd macos && swift build -c release

# 3. 打包 .app（在 macos/build/ 目录）
mkdir -p build/Rhythm.app/Contents/{MacOS,Resources,Frameworks}
cp .build/release/Rhythm build/Rhythm.app/Contents/MacOS/
cp ../target/release/librhythm_core.dylib build/Rhythm.app/Contents/Frameworks/
cp Rhythm/Resources/Info.plist build/Rhythm.app/Contents/
sed -i '' 's/\$(EXECUTABLE_NAME)/Rhythm/' build/Rhythm.app/Contents/Info.plist

# 4. 签名并创建 DMG
codesign --force --deep --sign - build/Rhythm.app
hdiutil create -volname Rhythm -srcfolder build/Rhythm.app -ov -format UDZO build/Rhythm.dmg
```

或使用一键脚本：
```bash
bash scripts/build-macos.sh
```

### Windows

```bash
# 1. 构建 Rust 核心 DLL（在 Windows 上或交叉编译）
cargo build --release -p rhythm-core --target x86_64-pc-windows-msvc

# 2. 构建 WinUI 3 应用
cd windows && mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
```

或使用一键脚本（在 Windows 上）：
```bat
scripts\build-windows.bat
```

### 运行测试

```bash
cargo test -p rhythm-core
```

## 技术选型

- 音频：symphonia（解码）+ cpal（输出）
- 元数据：lofty + symphonia
- 数据库：SQLite + FTS5
- URL 解析：yt-dlp

## 许可

尚未确定。在选定许可证之前，本仓库保留所有权利。
