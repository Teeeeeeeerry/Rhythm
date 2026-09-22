# Windows AppState 行为清单

- 模块：`windows/Rhythm/AppState.h` + `AppState.cpp`（WinUI 前端状态编排）
- 历史回归：`#21`（解析失败原因上报）、`#39`（URL 曲目持久化）
- 编排归属（#173 起）：起播（先停后播 #51、按来源分发、recordPlay、队列建立定位）、toggle/next/previous（有界跳过 #78）、队列同步（#69）、Finished 自动切歌全部在 rust-core 协调器（见 `coordinator.md` CO-xx）；AppState 只渲染状态，事件（progress/state/finished/error/track_changed）替代 500ms 定时器轮询（#172/#173）
- 测试设施（Wave 4a 已落地）：
  - Catch2 v3.5.4 header-only（`windows/tests/vendor/` amalgamated，BSL-1.0）+ CMake 测试 target `RhythmTests` 挂 ctest（`enable_testing()`）
  - 测试 main 用 Catch2 自带的：`AppState` 是普通 C++ 类（#326 去掉 `winrt::implements` 基类），构造不需要 WinRT 单元
  - 链接 `rhythm_core.dll.lib` + 临时 DB 路径注入（`OpenDatabase` 接受路径）
  - `nlohmann/json` 在测试 target 显式声明（`find_package(nlohmann_json CONFIG REQUIRED)`；主构建的隐式依赖已在此登记）
  - `ResolveAndPlay` 与协调器事件经 `UiPost` 回到 UI 线程：应用由 `MainWindow` 把 WinUI DispatcherQueue 包成 `UiPost` 设入（#326，DispatcherQueue 不进行为库），测试用 `SetUiPost` 注入 `UiThread`（`TestHelpers.h`，专属线程队列；DispatcherQueue 是 Windows App Runtime 类，未打包的测试 exe 无法激活，#418），降级路径（未设 UI 线程）直接测
  - 接缝（#173）：AppState 的编排经 `ICoordinator` seam，测试注入 `SpyCoordinator`（`windows/tests/TestHelpers.h`，顺序队列模型镜像协调器契约）；原「无音频设备 SKIP」用例全部转确定性断言（真规则在 rust-core）
  - 测试文件：`windows/tests/AppStateBehaviorTests.cpp`（WA）与 `BridgeBehaviorTests.cpp`（WB）
  - 临时目录夹具 `TempDir`（#455）：按进程号 + 进程内计数命名，已存在的名字跳过，不依赖计时器精度；清理失败即让留下残留的那个用例失败（报出目录与原因）。
    用例里 `TempDir` 要先于 `AppState`/`Library` 声明（后析构），否则库还占着数据库文件、目录删不掉。
    夹具自身的约定见下文「测试夹具」表（`windows/tests/TestFixtureTests.cpp`）

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
| WA-14 | `ResolveAndPlay` 未设 UI 线程 | 后台结果被丢弃、`IsResolvingUrl` 复位（降级模式） | 新测（待 Windows 验证） |
| WA-16 | `Library` 打开失败 | `OpenDatabase(坏路径)` → `Library` 内部 ptr 为 null，后续方法安全 no-op | 新测（待 Windows 验证） |
| WA-31（#432） | 协调器事件载荷非 ASCII | `track_changed` 的中文标题、艺人、路径与 `error` 的中文消息按 UTF-8 往返无损（此前宽窄逐字符截断：字段乱码，或 JSON 非法整条事件被丢弃）；`CurrentTrack`、`OnUrlError` 的 message、`UrlError` 原样还原 | SpyCoordinator 事件注入 |
| WA-32（#339） | `FindPlaylist(id)` | 返回已加载歌单中该 id 的那一个（指向 `Playlists` 内元素）；id 不存在 → null。歌单详情视图与视图状态 `PlaylistRows` 共用这一处查找 | SpyApp 之外直接构造 |
| WA-33（#347） | `CreatePlaylist(name)` | 真库建歌单后 `Playlists` 立即含新歌单（id 与返回值一致），库里也有；未打开库或打开失败 → 返回 -1、列表不变；空名字 → 返回 -1、什么都不建。视图因此不必持有资料库句柄 | 真库（临时路径） |
| WA-34（#349） | `ResolverStatusText()` | 链接解析进行中：各供给阶段（checking/downloading/verifying/updating/failed）返回与 `Resolver::StatusText` 相同的文案；静默阶段（idle/ready）返回空串；无解析在进行 → 空串且不轮询解析器；未打开资料库时照常安全返回。状态来源经 `PollResolverStatus` 注入，默认是真解析器的 FFI 轮询。视图因此不必直接接触解析器 | 注入状态来源 + 真解析器 |
| WA-35（#351） | `ExportPlaylist(id, path)` | 真库歌单导出成功 → 具名结果 `Exported`、`exported` 为曲目数、文件含曲目路径；写不出文件 → `WriteFailed`、`code == -2`（核心把「曲目数据解不开」-1 与「写不出」-2 分开，见 FF-14）；未打开资料库或歌单 id 不存在 → `nullopt`、什么都不写。视图因此不必直接调 FFI 导出层 | 真库（临时路径） |
| WA-36（#352） | 导出反馈文案 | `ExportPlaylist` 成功 → `ShowExportAlert`，标题「导出结果」、正文 `ExportedTracks(n)`；失败 → 标题「导出失败」、正文 `ExportFailed(code)`（键表 `export_failed`，此前无渲染者）；没跑导出（无库/无歌单）→ 不置提示。`DismissAlerts()` 同时清掉导入与导出两个提示 | 真库（临时路径） |

## 测试夹具（P1，#455）

夹具出错时红灯落在与被测行为无关的用例上，夹具自身的约定单独锁定（`windows/tests/TestFixtureTests.cpp`）。

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| TF-01 | `TempDir` 每个实例唯一 | 相邻两次构造（同一计时器 tick 内）得到两个不同、存在且为空的目录（此前按进程号 + `GetTickCount64` 命名，相邻用例共用目录、读到上一个用例残留的数据库，WA-03 偶发 `2 == 1`） | 新测 |
| TF-02 | 目录先于库声明时清理干净 | `TempDir` 先于 `AppState` 声明、导入一首后离开作用域，目录不复存在（库先关闭再删目录）；反序声明时清理失败让该用例失败，不再静默留下 `test.db` | 新测 |

## 错误路径（P2）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WA-17 | 解析失败各 kind 上报 | `OnUrlError` 收到机器可读 kind（与 macOS 文案体系对齐）（P2 部分顺延：invalid_url 已由 WA-11 覆盖；其余 kind 需 stub yt-dlp 注入，Windows 无 shell stub 设施） | 新测 |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| WA-05 | `PlayTrack` 缺 filePath/sourceUrl 仍置 `CurrentTrack`/`IsPlaying`（无声假播放，#78 同类） | [#81](https://github.com/Teeeeeeerry/Rhythm/issues/81) | 已修复于 T7（#103），红测解禁转绿 |
| WA-07 | `TogglePlayPause` 恢复时从头重播而非 `Resume()` 续播 | [#82](https://github.com/Teeeeeeerry/Rhythm/issues/82) | 已修复于 T7（#103），红测解禁转绿 |
| WA-05 | 起播调用的队列为空（期望 1 首） | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：用例加曲后没 `RefreshLibrary`，队列（已加载列表）为空；补上加载，已解禁转绿 |
| WA-10 | `ResolveAndPlay` 成功路径抛未捕获异常 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：`DispatcherQueueController` 是 Windows App Runtime 类，未打包的测试 exe 无法激活；AppState 新增 `SetUiPost` 接缝，测试用 `UiThread`（专属线程队列），已解禁转绿 |
| WA-11 | `ResolveAndPlay` 失败路径抛未捕获异常 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：同 WA-10，已解禁转绿 |
| WA-13 | `ResolveAndPlay` 重入保护用例抛未捕获异常 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：同 WA-10，已解禁转绿 |
| WA-24 | `ResolveAndPlay` 重载资料库用例抛未捕获异常 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：同 WA-10；另列表按标题排序，夹具曲名改为排在解析曲目之后的 "Z"，已解禁转绿 |
| WA-25 | 自动下一首后 `CurrentTrack` 仍是上一首 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：用例手拼事件 JSON，Windows 路径的反斜杠未转义，事件解析失败被忽略；改用 JSON 库构造，已解禁转绿 |

## 功能新增（用户 2026-08-13 决策：与 macOS 对齐，产品代码实现，非红测）

> Wave 4a 范围裁剪（用户 2026-08-14 决策）：T6 只写测试设施与测试、不动产品代码；
> WA-18–23 已于 T7（#90）实现。

| 编号 | 行为 | 断言 | 说明 |
|---|---|---|---|
| WA-23（#241） | `ImportDirectory` 导入反馈 | 断言基于具名结果 `{imported, unsupported, failed}`：`imported>0` → 已导入 N 首 + 刷新；`failed>0` → 导入失败；三项全 0 → 未找到音频文件（对齐 macOS alert，本端魔数三分支删除） | 功能新增 |
| WA-26（#236） | M3U8 导入结果渲染 | `ImportM3U8(path)` 调核心解析并入库入口 → 按具名结果 `{imported, failed}` 选提示语 → 从数据库重载列表；入库策略断言归属核心（PL-17 至 PL-24），列表不可读时不弹提示 | 真 FFI + 临时 M3U8 文件 |
| WA-29（#242，新能力 P0） | `ImportFile` 单文件导入 | 支持格式 → 已导入 1 首 + 从数据库重载列表；扩展名不支持 → 不支持的音频格式；支持但读不出 → 导入失败；无 Library → no-op。文件选择面板按核心支持的扩展名过滤 | 功能新增（macOS 一直有，Windows 此前完全缺失） |
| WA-30（#243，新能力 P0） | `ImportPaths` 批量导入与部分成功 | 目录与文件混合批量，分派与聚合来自核心；四种文案与 macOS 逐字相同：全成 → 已导入 N 首；部分成 → 已导入 N 首、M 个失败；全败 → 全部导入失败；只有不支持格式 → 未找到支持的音频文件 | 功能新增（macOS 一直有，Windows 此前完全缺失） |
| WA-28（#186） | 文案断言引用合并清单 | Windows 文案断言（原 WA-26 文案层）随 `l10n-keys.md`（LK-01~06）——固定 locale、键表驱动 | L10nTests.cpp |
| WA-27（#175） | 事件驱动渲染（替代定时器） | `finished`/`track_changed`/`progress`/`state`/`error` 事件 → `IsPlaying`/`CurrentTrack`/`Position`/`IsBuffering`/分类文案（自动切歌在核心，CO-25） | SpyCoordinator 事件注入 |
| WA-26（#141，待 Windows 验证） | Windows 文案层 L10n | 全部用户可见文案经 `L10n`（对齐 macOS `L10n` 枚举）：手动覆盖（注册表 `AppLanguage`）优先、否则跟随系统 UI 语言；#120 expired/cdn_rejected 分类含英文分支；导入反馈、解析状态、来源徽标、托盘菜单同层 | 功能新增（macOS 对齐） |
