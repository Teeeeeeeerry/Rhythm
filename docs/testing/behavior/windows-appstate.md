# Windows AppState 行为清单

- 模块：`windows/Rhythm/AppState.h` + `AppState.cpp`（WinUI 前端状态编排）
- 历史回归：`#21`（解析失败原因上报）、`#39`（URL 曲目持久化）
- 编排归属（#173 起）：起播（先停后播 #51、按来源分发、recordPlay、队列建立定位）、toggle/next/previous（有界跳过 #78）、队列同步（#69）、Finished 自动切歌全部在 rust-core 协调器（见 `coordinator.md` CO-xx）；AppState 只渲染状态，事件（progress/state/finished/error/track_changed）替代 500ms 定时器轮询（#172/#173）
- 测试设施（Wave 4a 已落地）：
  - Catch2 v3.5.4 header-only（`windows/tests/vendor/` amalgamated，BSL-1.0）+ CMake 测试 target `RhythmTests` 挂 ctest（`enable_testing()`）
  - 测试 main（`windows/tests/TestMain.cpp`）调一次 `winrt::init_apartment(apartment_type::single_threaded)` 以构造 `AppState`
  - 链接 `rhythm_core.dll.lib` + 临时 DB 路径注入（`OpenDatabase` 接受路径）
  - `nlohmann/json` 在测试 target 显式声明（`find_package(nlohmann_json CONFIG REQUIRED)`；主构建的隐式依赖已在此登记）
  - `ResolveAndPlay` 的 dispatcher 注入经 `SetDispatcherQueue`；完整链路测试用 `DispatcherQueueController::CreateOnDedicatedThread()`（专属线程自带消息循环），降级路径（无 dispatcher）直接测
  - 接缝（#173）：AppState 的编排经 `ICoordinator` seam，测试注入 `SpyCoordinator`（`windows/tests/TestHelpers.h`，顺序队列模型镜像协调器契约）；原「无音频设备 SKIP」用例全部转确定性断言（真规则在 rust-core）
  - 测试文件：`windows/tests/AppStateBehaviorTests.cpp`（WA）与 `BridgeBehaviorTests.cpp`（WB）

## 主路径（P0 — 合并门槛）

> 播放编排条目（WA-05~09、WA-18~22、WA-24/25 及 WA-15）自 #175 起并入
> `coordinator.md`（CO-01~28），双端不再按平台重复维护；本清单只保留
> AppState 的 UI 状态与流程（打开库、刷新、导入、搜索、URL 解析导入）。

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WA-01 | `OpenDatabase` | `Library` 创建 + `Tracks`/`Playlists` 填充 | 新测（待 Windows 验证） |
| WA-02 | `RefreshLibrary` | 无 Library → no-op；有 → Tracks/Playlists 从库刷新（队列同步在协调器，CO-14） | 新测（待 Windows 验证） |
| WA-03 | `ImportDirectory` | 有 Library → 导入 + `RefreshLibrary`；无 Library → no-op（数量反馈见 WA-23，T7 已实现） | 新测（待 Windows 验证） |
| WA-04 | `DoSearch` | 空 query → `AllTracks`；非空 → `Search(query)` | 新测（待 Windows 验证） |
| WA-10 | `ResolveAndPlay` 成功 | trim 输入；`AddTrack` 持久化（#39）；`RefreshLibrary()` 从 DB 重载（#139）；`UrlError` 清空；`PlayTrack(saved)` 经协调器 | SpyCoordinator（真 core 解析，无网络） |
| WA-11 | `ResolveAndPlay` 失败（#21） | `UrlError`=错误消息 + `OnUrlError(kind, message)` 回调触发 | SpyCoordinator（invalid_url 真 core 失败） |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WA-12 | `ResolveAndPlay` 空/纯空白输入 | trim 后为空 → 直接返回 | 新测（待 Windows 验证） |
| WA-13 | `ResolveAndPlay` 防重入 | `IsResolvingUrl=true` 期间忽略新调用（以 `OnUrlError` 回调计数为观察面：连续两次失败输入只回调一次） | 新测（待 Windows 验证） |
| WA-14 | `ResolveAndPlay` 无 dispatcher | 后台结果被丢弃、`IsResolvingUrl` 复位（降级模式） | 新测（待 Windows 验证） |
| WA-16 | `Library` 打开失败 | `OpenDatabase(坏路径)` → `Library` 内部 ptr 为 null，后续方法安全 no-op | 新测（待 Windows 验证） |

## 错误路径（P2）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WA-17 | 解析失败各 kind 上报 | `OnUrlError` 收到机器可读 kind（与 macOS 文案体系对齐）（P2 部分顺延：invalid_url 已由 WA-11 覆盖；其余 kind 需 stub yt-dlp 注入，Windows 无 shell stub 设施） | 新测 |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| WA-05 | `PlayTrack` 缺 filePath/sourceUrl 仍置 `CurrentTrack`/`IsPlaying`（无声假播放，#78 同类） | [#81](https://github.com/Teeeeeeerry/Rhythm/issues/81) | 已修复于 T7（#103），红测解禁转绿 |
| WA-07 | `TogglePlayPause` 恢复时从头重播而非 `Resume()` 续播 | [#82](https://github.com/Teeeeeeerry/Rhythm/issues/82) | 已修复于 T7（#103），红测解禁转绿 |

## 功能新增（用户 2026-08-13 决策：与 macOS 对齐，产品代码实现，非红测）

> Wave 4a 范围裁剪（用户 2026-08-14 决策）：T6 只写测试设施与测试、不动产品代码；
> WA-18–23 已于 T7（#90）实现。

| 编号 | 行为 | 断言 | 说明 |
|---|---|---|---|
| WA-23（T7 已实现，待 Windows 验证） | `ImportDirectory` 导入反馈 | 导入数量经状态/回调反馈到 UI（对齐 macOS alert） | 功能新增 |
| WA-26（#173） | M3U8 导入逐条入库 | `ImportM3U8(path)` 解析→逐条 `AddTrack`（URL→direct_url、其余→local）→刷新→统计文案；失败计数与 macOS 一致（原 no-op 修复） | 真 FFI 解析 + 临时 M3U8 文件 |
| WA-27（#175） | 事件驱动渲染（替代定时器） | `finished`/`track_changed`/`progress`/`state`/`error` 事件 → `IsPlaying`/`CurrentTrack`/`Position`/`IsBuffering`/分类文案（自动切歌在核心，CO-25） | SpyCoordinator 事件注入 |
| WA-26（#141，待 Windows 验证） | Windows 文案层 L10n | 全部用户可见文案经 `L10n`（对齐 macOS `L10n` 枚举）：手动覆盖（注册表 `AppLanguage`）优先、否则跟随系统 UI 语言；#120 expired/cdn_rejected 分类含英文分支；导入反馈、解析状态、来源徽标、托盘菜单同层 | 功能新增（macOS 对齐） |
