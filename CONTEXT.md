# CONTEXT.md — Rhythm 领域词汇与代码导航

本文是 agent 的工作文档：领域词汇的准确定义 + 代码地图 + 非显而易见的工作约定。
功能总览和架构介绍见 README.md；本文不重复，只记录 README 没有的、agent 需要知道的东西。

## 领域词汇

| 词 | 定义 |
|----|------|
| **Track** | 资料库中的一首曲目。`sourceType` 决定播放方式：`local`（filePath）、`youtube`/`bilibili`/`direct_url`（sourceUrl）。`id == -1` 表示未入库（如刚解析出的 URL 曲目）；`addTrack` 返回带真实 DB id 的副本 |
| **resolve** | 把粘贴的 URL（YouTube/Bilibili/直链）解析成可播放信息（ResolvedInfo）。可能触发 yt-dlp 首次下载（~40MB），有进度状态轮询 |
| **import** | 仅入库不播放：持久化到 SQLite + 刷新列表 + 显示导入提示（#71 拆分出的 `importResolved`）。本地文件导入和 URL 导入在此汇合，行为一致 |
| **play** | 完整播放流程：`player.stop()` 停旧曲目（#51）→ 按 sourceType 播 → `isPlaying = true` → 建队列。`playResolved` = import + play |
| **Queue** | 播放队列（RhythmQueue），与资料库列表（tracks）是两回事。`refreshLibrary()` 会同步队列（#69：`replace` + `jumpTo` 当前曲目） |
| **Playlist** | 用户自建歌单，独立于播放队列 |
| **PlayMode** | sequential / shuffle / singleLoop / listLoop，由 RhythmQueue 持有 |

## 代码地图

### 三层结构

```
rust-core/          Rust 共享核心（audio 引擎、library、queue、playlist、resolver、metadata）
  src/ffi/mod.rs    C-ABI 导出层：JSON 字符串进出，snake_case 键
macos/              SwiftUI (AppKit) 客户端，SPM 可执行目标
  Rhythm/Models/RhythmCore.swift   FFI 封装：RhythmLibrary / RhythmPlayer / RhythmQueue + JSON 编解码
  Rhythm/AppState.swift             全局状态中枢：所有业务逻辑在这一个类里
  Rhythm/Views/                     按区域分目录：Library / PlayerBar / Playlist / Sidebar / Tray
windows/            WinUI 3 C++ 客户端，镜像 macos 的 AppState（AppState.cpp）
scripts/            build-macos.sh / build-rust-macos.sh / build-windows.*
```

### 关键路径

- **URL 导入**：`PlayerBarView` 输入 → `AppState.resolveAndImport` → `resolveURL`（FFI）→ `importResolved`（仅入库，不打断播放）
- **播放**：双击曲目 → `playTrack` → `player.playFile/playURL` → `recordPlay`
- **进度/音量**：`PlayerBarView` 的 Slider → `player.seek/setVolume`（FFI 直通）
- **测试**：`macos/Tests/`（AppStateTests + RhythmThemeTests）。AppState 测试用真实临时数据库，不用 mock

### 新增引擎能力的三层套路（#70 的教训）

引擎已有能力 ≠ UI 可用。任何新能力都要打通三层，缺一层就是 bug：
1. `rust-core/src/ffi/mod.rs` 导出 `rhythm_player_*` 函数
2. `macos/Rhythm/Models/RhythmCore.swift` 加封装方法
3. 对应的 SwiftUI 视图接入

## 工作约定

- **Issue 引用**：修复的根因注释里写 `#NN`（如 `// #51: stop old playback`）。新修复沿用此习惯
- **TDD seams**：测试写在公开行为层（AppState 的方法），不测私有实现；每个 seam 一组测试
- **L10n**：全部文案走 `L10n` 枚举（`isChinese` 三元），不在视图里写死字符串
- **品牌色**：只用 `RhythmTheme` 模块的 token（如 `.rhythmAccent`），不硬编码色值
- **构建产物**：放 `build/` 目录（scripts/build-macos.sh 生成 Rhythm.app）

## 坑（非显而易见，踩过才写）

- **`swift test` 默认跑不起来**：本机 xcode-select 指向 CommandLineTools，缺少 XCTest。需 `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer swift test`
- **Rust 核心用 release 构建**：macos 可执行文件链接 `../target/release` 的 dylib，改 Rust 后需 `cargo build --release`，否则跑的是旧核心
- **URL 曲目存页面 URL 不存 CDN 链接**：CDN 链接带过期 deadline，播放时从缓存重新解析
- **刷新列表必须从 DB 重载**（`refreshLibrary`）：手动拼接 tracks 会导致 ForEach ID 碰撞（#66）和队列失同步（#69）
- **播放新曲目前必须 `player.stop()`**：旧播放线程不终止会抢占输出设备（#51）

## 文档地图

| 文档 | 受众 | 内容 |
|------|------|------|
| README.md / README.en.md | 人 | 功能总览、架构介绍、下载安装 |
| docs/deep-testing-plan.md | 人 | 主题色彩测试方案（L0-L4） |
| docs/testing/ | 人+agent | 测试基础设施：palette 同步、对比检查脚本、各层测试源码 |
| 本文 CONTEXT.md | agent | 领域词汇 + 导航 + 约定 |
