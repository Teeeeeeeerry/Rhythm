# Windows AppState 行为清单

- 模块：`windows/Rhythm/AppState.h` + `AppState.cpp`（WinUI 前端状态编排）
- 历史回归：`#21`（解析失败原因上报）、`#39`（URL 曲目持久化）
- 测试设施（Wave 4a 已落地）：
  - Catch2 v3.5.4 header-only（`windows/tests/vendor/` amalgamated，BSL-1.0）+ CMake 测试 target `RhythmTests` 挂 ctest（`enable_testing()`）
  - 测试 main（`windows/tests/TestMain.cpp`）调一次 `winrt::init_apartment(apartment_type::single_threaded)` 以构造 `AppState`
  - 链接 `rhythm_core.dll.lib` + 临时 DB 路径注入（`OpenDatabase` 接受路径）
  - `nlohmann/json` 在测试 target 显式声明（`find_package(nlohmann_json CONFIG REQUIRED)`；主构建的隐式依赖已在此登记）
  - `ResolveAndPlay` 的 dispatcher 注入经 `SetDispatcherQueue`；完整链路测试用 `DispatcherQueueController::CreateOnDedicatedThread()`（专属线程自带消息循环），降级路径（无 dispatcher）直接测
  - 测试文件：`windows/tests/AppStateBehaviorTests.cpp`（WA）与 `BridgeBehaviorTests.cpp`（WB）

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WA-01 | `OpenDatabase` | `Library` 创建 + `Tracks`/`Playlists` 填充 | 新测（待 Windows 验证） |
| WA-02 | `RefreshLibrary` | 无 Library → no-op；有 → Tracks/Playlists 从库刷新 | 新测（待 Windows 验证） |
| WA-03 | `ImportDirectory` | 有 Library → 导入 + `RefreshLibrary`；无 Library → no-op（数量反馈见 WA-23，T7 已实现） | 新测（待 Windows 验证） |
| WA-04 | `DoSearch` | 空 query → `AllTracks`；非空 → `Search(query)` | 新测（待 Windows 验证） |
| WA-05 | `PlayTrack` 分派 | 期望：filePath → `PlayFile`；sourceUrl → `PlayURL`；`RecordPlay(id)`；缺两者时不进入播放状态（#81 已修复于 T7，红测解禁转绿） | 新测（待 Windows 验证） |
| WA-06 | `TogglePlayPause` 播放中 | `Pause()` + `IsPlaying=false` | 新测（待 Windows 验证） |
| WA-07 | `TogglePlayPause` 恢复 | 期望：`Player->Resume()` 续播（#82 已修复于 T7，红测解禁转绿；无音频设备时环境 SKIP） | 新测（待 Windows 验证） |
| WA-08 | `TogglePlayPause` 空闲启动 | 无 CurrentTrack 且 Tracks 非空 → `PlayTrack(Tracks[0])` | 新测（待 Windows 验证） |
| WA-09 | `SetVolume` | `Volume` 状态更新 + `Player->SetVolume(float)` | 新测（待 Windows 验证） |
| WA-10 | `ResolveAndPlay` 成功 | trim 输入；`AddTrack` 持久化（#39）；saved 插入 Tracks 头部；`UrlError` 清空；`PlayTrack(saved)` | 新测（真 core 直链，无网络），已绿（待 Windows 验证） |
| WA-11 | `ResolveAndPlay` 失败（#21） | `UrlError`=错误消息 + `OnUrlError(kind, message)` 回调触发 | 新测（invalid_url 真 core 失败），已绿（待 Windows 验证） |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WA-12 | `ResolveAndPlay` 空/纯空白输入 | trim 后为空 → 直接返回 | 新测（待 Windows 验证） |
| WA-13 | `ResolveAndPlay` 防重入 | `IsResolvingUrl=true` 期间忽略新调用（以 `OnUrlError` 回调计数为观察面：连续两次失败输入只回调一次） | 新测（待 Windows 验证） |
| WA-14 | `ResolveAndPlay` 无 dispatcher | 后台结果被丢弃、`IsResolvingUrl` 复位（降级模式） | 新测（待 Windows 验证） |
| WA-15 | `TogglePlayPause` 无曲目可播 | 无 CurrentTrack 且 Tracks 空 → no-op | 新测（待 Windows 验证） |
| WA-16 | `Library` 打开失败 | `OpenDatabase(坏路径)` → `Library` 内部 ptr 为 null，后续方法安全 no-op | 新测（待 Windows 验证） |

## 错误路径（P2）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WA-17 | 解析失败各 kind 上报 | `OnUrlError` 收到机器可读 kind（与 macOS 文案体系对齐）（P2 部分顺延：invalid_url 已由 WA-11 覆盖；其余 kind 需 stub yt-dlp 注入，Windows 无 shell stub 设施） | 新测 |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| WA-05 | `PlayTrack` 缺 filePath/sourceUrl 仍置 `CurrentTrack`/`IsPlaying`（无声假播放，#78 同类） | [#81](https://github.com/Teeeeeeerry/Rhythm/issues/81) | 已修复于 T7（#103），红测解禁转绿 |
| WA-07 | `TogglePlayPause` 恢复时从头重播而非 `Resume()` 续播 | [#82](https://github.com/Teeeeeeerry/Rhythm/issues/82) | 待实现票挂接（测试禁用） |

## 功能新增（用户 2026-08-13 决策：与 macOS 对齐，产品代码实现，非红测）

> Wave 4a 范围裁剪（用户 2026-08-14 决策）：T6 只写测试设施与测试、不动产品代码；
> WA-18–23 已于 T7（#90）实现。

| 编号 | 行为 | 断言 | 说明 |
|---|---|---|---|
| WA-18（T7 已实现，待 Windows 验证） | `PlayTrack` 先停后播 | 分派前先 `Player->Stop()`（macOS #51 模式），防止新旧流叠加 | 随 WA-05/WA-07 同一票实现 |
| WA-19（T7 已实现，待 Windows 验证） | 播放队列：`playNext`/`playPrevious` | 引入 `RhythmQueue`（FFI 已具备）；next/previous 分派、队列耗尽 no-op | 功能新增（macOS 对齐） |
| WA-20（T7 已实现，待 Windows 验证） | `RefreshLibrary` 队列同步 | 队列随曲库刷新 replace + jumpTo 当前曲（macOS #69/#72 对齐） | 功能新增 |
| WA-21（T7 已实现，待 Windows 验证） | 播放模式循环 | `PlayMode` 四种模式 + `cyclePlayMode` 同步到队列 | 功能新增 |
| WA-22（T7 已实现，待 Windows 验证） | 传输可用性 | `canPlayNext`/`canPlayPrevious` 等可用性属性（macOS #24/#25 对齐） | 功能新增 |
| WA-23（T7 已实现，待 Windows 验证） | `ImportDirectory` 导入反馈 | 导入数量经状态/回调反馈到 UI（对齐 macOS alert） | 功能新增 |
