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
- 中英文界面（macOS 跟随系统语言并可手动切换；Windows 跟随系统语言，注册表 `HKCU\Software\Rhythm\AppLanguage` 可手动覆盖）
- 支持格式：MP3、AAC、FLAC、WAV、OGG、ALAC、APE、WMA、AIFF、WavPack、MP4/M4A

## 架构

Rhythm 的架构分为两层。上层是平台原生 UI：macOS 端用 Swift 和 AppKit 编写，Windows 端用 C++ 和 WinUI 3 编写，两套 UI 各自遵循对应平台的设计语言，但在功能上保持对等。下层是一个用 Rust 编写的共享核心库，负责所有与平台无关的逻辑——音频解码与播放、元数据提取、资料库管理、播放列表、播放队列、URL 解析——然后通过 C-ABI 编译为 .dylib 和 .dll，供两端 UI 直接调用。这种双原生 UI + 单一 Rust 核心的策略，既能保证界面与系统深度融合、内存占用可控，又避免了在两端重复实现相同的底层逻辑。

## 开发状态

初步开发完成。当前版本 **v0.5.106 "Motif"**（与 `Cargo.toml` 同步，版本提升随每次发布更新本行）。

### 实现状态

| 功能 | 状态 | Issue |
|------|------|-------|
| 本地音乐播放（MP3/FLAC/AAC/WAV/OGG/ALAC/APE/WMA/AIFF/WavPack/MP4） | 完成 | — |
| 资料库管理 + FTS5 全文搜索 | 完成 | — |
| 播放列表（混合本地/在线，M3U8 导入导出） | 完成 | — |
| 播放队列（顺序/随机/单曲循环/列表循环） | 完成 | — |
| 专辑封面自动提取 | 完成 | — |
| 系统媒体键 + 托盘模式 | 完成 | — |
| 中英文界面 | 完成（macOS 手动切换 + Windows 注册表覆盖） | [#141](https://github.com/Teeeeeeeerry/Rhythm/issues/141)、[#145](https://github.com/Teeeeeeeerry/Rhythm/issues/145) |
| URL 流式播放（YouTube/Bilibili/直链） | 完成 | [#11](https://github.com/Teeeeeeeerry/Rhythm/issues/11) |
| URL 输入 UI | 完成 | [#12](https://github.com/Teeeeeeeerry/Rhythm/issues/12) |
| URL 解析错误上报 + yt-dlp 自动安装 | 完成 | [#21](https://github.com/Teeeeeeeerry/Rhythm/issues/21) |
| 托盘菜单可用性修复 | 完成 | [#24](https://github.com/Teeeeeeeerry/Rhythm/issues/24) |
| 在线播放链路修复（音频输出丢采样、缓冲状态、seek） | 完成 | [#23](https://github.com/Teeeeeeeerry/Rhythm/issues/23) |

## 播放在线链接

**开箱即用，无需手动安装任何东西。** 粘贴 YouTube / Bilibili 链接就能播——首次播放时 Rhythm 会自动下载解析所需的 yt-dlp（约 36 MB），下载进度显示在链接输入框旁边，之后不再重复下载。

具体行为：

- 组件存放在 `~/Library/Application Support/Rhythm/bin/`（Windows：`%LOCALAPPDATA%\Rhythm\bin\`）
- 只从 yt-dlp 官方 GitHub Release 下载，并按官方发布的 `SHA2-256SUMS` 校验，校验不通过即丢弃
- 每 7 天在后台检查一次更新；如果某个站点因版本过旧解析失败，会自动升级后重试一次
- 系统里已经装了 yt-dlp（Homebrew、MacPorts、pip、scoop、winget 等）则直接复用，不会重复下载
- 优先选择 AAC/M4A 音轨（内置解码器不支持 Opus），并沿用 yt-dlp 报告的请求头——B 站 CDN 缺少 Referer 会返回 403

如果需要自己管理 yt-dlp：

```bash
export RHYTHM_NO_AUTO_INSTALL=1              # 关闭自动下载
export RHYTHM_YTDLP_PATH=/your/path/to/yt-dlp # 指定自己的二进制
```

解析失败时，应用会直接说明原因（网络错误 / 超时 / 视频不可用 / 需要升级 yt-dlp），详细记录写入日志：

- macOS：`~/Library/Logs/Rhythm/resolver.log`
- Windows：`%LOCALAPPDATA%\Rhythm\logs\resolver.log`

## 构建

### 前置依赖

- **Rust** 1.70+（[rustup.rs](https://rustup.rs)）
- yt-dlp 无需预先安装：首次播放在线链接时由应用自动获取
- **macOS**：Xcode 15+ 或 Command Line Tools + Swift 5.9+
- **Windows**：Visual Studio 2022 + Windows App SDK + CMake 3.20+

### macOS

```bash
# 1. 构建 Rust 核心库
cargo build --release -p rhythm-core

# 2. 构建 Swift UI
cd macos && swift build -c release

# 3. 打包 .app（在项目根目录 build/ 下）
mkdir -p "$PROJECT_ROOT/build/Rhythm.app/Contents/"{MacOS,Resources,Frameworks}
cp .build/release/Rhythm "$PROJECT_ROOT/build/Rhythm.app/Contents/MacOS/"
cp ../target/release/librhythm_core.dylib "$PROJECT_ROOT/build/Rhythm.app/Contents/Frameworks/"
cp Rhythm/Resources/Info.plist "$PROJECT_ROOT/build/Rhythm.app/Contents/"
sed -i '' 's/\$(EXECUTABLE_NAME)/Rhythm/' "$PROJECT_ROOT/build/Rhythm.app/Contents/Info.plist"

# 4. 签名
codesign --force --deep --sign - "$PROJECT_ROOT/build/Rhythm.app"
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
