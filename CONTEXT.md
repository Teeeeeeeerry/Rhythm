# CONTEXT.md — Rhythm 领域词汇与代码导航

本文是 agent 的工作文档：领域词汇的准确定义 + 代码地图 + 非显而易见的工作约定。
功能总览和架构介绍见 README.md；本文不重复，只记录 README 没有的、agent 需要知道的东西。

## 领域词汇

| 词 | 定义 |
|----|------|
| **Track** | 资料库中的一首曲目。`sourceType` 决定播放方式：`local`（filePath）、`youtube`/`bilibili`/`direct_url`（sourceUrl）。`id == -1` 表示未入库（如刚解析出的 URL 曲目）；`addTrack` 返回带真实 DB id 的副本 |
| **resolve** | 把粘贴的 URL（YouTube/Bilibili/直链）解析成可播放信息（ResolvedInfo）。可能触发 yt-dlp 首次下载（~36MB，与 README 同步），有进度状态轮询 |
| **import** | 仅入库不播放：持久化到 SQLite + 刷新列表 + 显示导入提示（#71 拆分出的 `importResolved`）。本地文件导入和 URL 导入在此汇合，行为一致 |
| **play** | 完整播放流程：`player.stop()` 停旧曲目（#51）→ 按 sourceType 播 → `isPlaying = true` → 建队列。`playResolved` = import + play |
| **Queue** | 播放队列（RhythmQueue），与资料库列表（tracks）是两回事。`refreshLibrary()` 会同步队列（#69：`replace` + `jumpTo` 当前曲目） |
| **Playlist** | 用户自建歌单，独立于播放队列 |
| **PlayMode** | sequential / shuffle / singleLoop / listLoop，由 RhythmQueue 持有 |
| **行为清单** | 按模块维护的行为规格（`docs/testing/behavior/<模块>.md`），每条分主路径/边界/错误路径并标 P0/P1/P2；清单上每项都必须有自动化测试，行覆盖率只作参考。教义见 `docs/adr/0001-行为清单制测试教义.md` |
| **红测禁用** | 行为清单中因产品已知缺陷而无法通过的测试：照写但用 `XCTSkip`/`#[ignore]`/`Catch2 SKIP()` 禁用，禁用原因挂 issue 链接，修复时解禁。各清单末尾"红测登记"段留痕 |
| **最小接缝** | 为解锁测试引入的 trait/protocol 注入点（如 `RhythmPlayerProtocol` + SpyPlayer），与测试同批提交，不改变产品行为 |

## 代码地图

### 三层结构

```
rust-core/          Rust 共享核心（audio 引擎、library、queue、playlist、resolver、metadata、coordinator）
  src/coordinator/  播放协调器（#165 组）：起播/传输/自动切歌/队列同步/可用性的唯一出处，双端 UI 只是薄 adapter
  src/resolver/     解析（#168 组起按职责拆 cache/classify/stderr/install；播放期解析入口 resolve_for_playback 一次完成缓存命中/淘汰/重试/分类）
  src/ffi/mod.rs    C-ABI 导出层：结构化结果（成功载荷+分类错误一次返回）、事件回调、snake_case JSON；导出一律 unsafe extern "C"（#143）；契约单一声明见 contracts/ffi-contract.json（#180）
  tests/            行为测试（audio_engine / coordinator / library / metadata / ffi / playlist_m3u8 / resolver…）
macos/              SwiftUI (AppKit) 客户端，SPM 可执行目标
  Rhythm/Models/RhythmCore.swift   FFI 封装：RhythmLibrary / RhythmCoordinator（+ 事件订阅）/ resolver + JSON 编解码
  Rhythm/Models/L10n.swift          全部用户可见文案（中英）
  Rhythm/AppState.swift             全局状态中枢：渲染协调器状态 + UI 流程（导入/搜索/删除确认/URL 解析导入）
  Rhythm/Views/                     按区域分目录：Library / PlayerBar / Playlist / Sidebar / Tray
  RhythmTheme/Theme.swift           品牌色 token 独立 library target（测试可 import，见 Package.swift）
  Tests/                            AppStateTests + RhythmThemeTests 两个 testTarget
windows/            WinUI 3 C++ 客户端，镜像 macos 的 AppState（AppState.cpp）
  Rhythm/Bridge/RhythmCore.h        C++ 侧 FFI 封装 + IPlayer/ICoordinator 接缝 + Track/Playlist 模型（含来源徽标色表映射）
  Rhythm/L10n.h                     Windows 文案层（跟随系统语言 + 注册表覆盖，#141）
  tests/                            Catch2 行为测试（AppState / Bridge / L10n）
testing/            主题色彩测试基础设施：palette.json 单一事实来源 + L0-L4 + run-all.sh
docs/               adr/（决策记录）、testing/behavior/（各模块行为清单）、issues/（已调查的 bug 报告）
scripts/            build-macos.sh / build-rust-macos.sh / build-windows.* / check-no-emoji.py
```

### 关键路径

- **URL 导入**：`PlayerBarView` 输入 → `AppState.resolveAndImport` → `resolveURL`（FFI）→ `importResolved`（仅入库，不打断播放）
- **播放**：双击曲目 → `playTrack` → 协调器 `start`（先停后播 → 按来源分派 → recordPlay → 队列定位）→ 引擎播放；进度/状态/播完/失败经协调器事件回流（#172）
- **进度/音量**：`PlayerBarView` 的 Slider → `player.seek/setVolume`（FFI 直通）
- **测试**：`macos/Tests/`（AppStateTests + RhythmThemeTests）。AppState 测试用真实临时数据库 + SpyCoordinator（编排规则在 rust-core `coordinator_behavior.rs`，无音频设备依赖）；
  Rust 侧 `cargo test -p rhythm-core`，Windows 侧 `windows/tests/`（Catch2，SpyCoordinator 同 macOS），主题色彩链路 `bash testing/run-all.sh`

### 新增引擎能力的三层套路（#70 的教训）

引擎已有能力 ≠ UI 可用。任何新能力都要打通三层，缺一层就是 bug：
1. `rust-core/src/ffi/mod.rs` 导出 `rhythm_player_*` 函数
2. `macos/Rhythm/Models/RhythmCore.swift` 加封装方法
3. 对应的 SwiftUI 视图接入

## 工作约定

- **Issue 引用**：修复的根因注释里写 `#NN`（如 `// #51: stop old playback`）。新修复沿用此习惯
- **TDD seams**：测试写在公开行为层（AppState 的方法），不测私有实现；每个 seam 一组测试
- **L10n**：全部文案走 `L10n`（macOS `Models/L10n.swift` + `L10nKeys`、Windows `L10n.h` + `L10nKeys.h`，
  均由 `contracts/l10n-keys.json` 键表生成，#167 组）；视图里既不写死字符串也不就地写 inline 三元。
  新增文案只改键表再跑 `scripts/gen-l10n.py`；键表漂移由 L0 校验拦截（#185）
- **零 emoji（硬性）**：任何文本不得出现 emoji——代码注释、文档、测试、commit/PR 文案、与用户的对话输出一律禁止（ASCII 与普通符号如 `->` 除外）。提交前跑 `python3 scripts/check-no-emoji.py` 校验；发现即修，不得绕过
- **品牌色**：只用 `RhythmTheme` 模块的 token（如 `.rhythmAccent`），不硬编码色值
- **版本号单一出处**：版本号只改 `Cargo.toml` 的 `[workspace.package] version`；其余六处（依赖锁文件、两份 README 版本行、macOS `Info.plist`、
  `windows/CMakeLists.txt`、`testing/README.md` 状态表）是副本，随之同步。漂移由 `python3 testing/l0/check-version-drift.py` 拦截，
  已挂进 `testing/run-all.sh` 的 L0 段与 CI 静态分析作业（#251/#252/#253）
- **构建产物**：放 `build/` 目录（scripts/build-macos.sh 生成 Rhythm.app）

## 坑（非显而易见，踩过才写）

- **`swift test` 默认跑不起来**：本机 xcode-select 指向 CommandLineTools，缺少 XCTest。需 `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer swift test`
- **Rust 核心用 release 构建**：macos 可执行文件链接 `../target/release` 的 dylib，改 Rust 后需 `cargo build --release`，否则跑的是旧核心
- **URL 曲目存页面 URL 不存 CDN 链接**：CDN 链接带过期 deadline，播放时从缓存重新解析
- **刷新列表必须从 DB 重载**（`refreshLibrary`）：手动拼接 tracks 会导致 ForEach ID 碰撞（#66）和队列失同步（#69）
- **播放新曲目前必须 `player.stop()`**：旧播放线程不终止会抢占输出设备（#51）
- **YouTube 403 ≠ 链接过期**：googlevideo URL 有效期 ~6h（`expire`-`mt`），播放 403 时先解码 `expire` 判断；未过期却 403 是网络侧拒绝（常见于 ISP 托管的 Google Global Cache 节点 `cache.google.com` 故障、或出口 IP 被 YouTube 拉黑），换网络/VPN 才是出路（见 docs/issues/2026-08-18-youtube-403-misreported-as-expired.md）。#120 已修复：core 用 `RhythmError::Http` 分类（expired/cdn_rejected/other），UI 按分类给文案——仅真过期才建议重贴
- **解析缓存不淘汰失败条目**：`RESOLVED_CACHE`（1h TTL）命中即返回，播放 403 不会清条目——重贴同一链接在 TTL 内必然拿到同一个坏 CDN URL，用户建议"重新粘贴"结构性无效。#120 已修复：播放 403/过期时引擎淘汰条目并 `resolve_url_fresh` 绕过缓存重解析一次，仍败才报错
- **Windows 来源徽标色双主题**：`Track::SourceColor(sourceType, isDarkTheme)` 对齐 macOS `Theme.swift` rhythmSource* 的 dark/light 双端值；
  #147 起前景色与胶囊底共用单一表映射 `Track::SourceColorRGB`（`SourceColor()` 与 `SourceBackgroundBrush()` 都从它派生，改色只改一处）；未知来源回退 teal 文字色（dark `#ABC8D4` / light `#0D464D`），绝不返回系统 Gray（F4）。theme 由 `IsDarkTheme()` 解析——应用从不 pin `Application.RequestedTheme`，UI 跟随系统，故用 `UISettings` 前景色判断。Windows 侧校验：`python3 testing/l0/check-color-parity.py` + `testing/l1/windows`（#122 解除桩）。#121 已修复

## 文档地图

| 文档 | 受众 | 内容 |
|------|------|------|
| README.md / README.en.md | 人 | 功能总览、架构介绍、构建方式；版本行与 `Cargo.toml` 同步，每次发布更新 |
| docs/adr/ | 人+agent | 架构与流程决策记录（现有 0001 行为清单制测试教义） |
| docs/testing/behavior/ | 人+agent | 按模块的行为清单 + 红测登记（测试完整性的交付物） |
| docs/issues/ | 人+agent | 已调查并辑录的 bug 报告（issue 草稿，可直接贴 GitHub） |
| testing/deep-testing-plan.md | 人 | 主题色彩测试方案（L0-L4）+ F1-F8 修复状态 |
| testing/README.md | 人+agent | 测试基础设施：palette 同步、L0 脚本与自测、各层测试源码、当前状态表 |
| 本文 CONTEXT.md | agent | 领域词汇 + 导航 + 约定 |
