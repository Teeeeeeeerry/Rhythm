# L10n 文案键表行为清单

- 模块：`contracts/l10n-keys.json`（单一事实来源，中英文案 + platform 差异字段）+ `scripts/gen-l10n.py`（生成器）+ 双端适配层（macOS `L10n.swift`/`L10nKeys.swift`、Windows `L10n.h`/`L10nKeys.h`）；测试：macOS `RhythmCoreSwiftBehaviorTests`（SW-17）、Windows `L10nTests.cpp`（LK-01~06）
- 历史回归：`#141`（Windows 文案层）、`#145`（macOS 文案层收敛）、`#120`（播放失败分类文案）、`#142`（固定 locale 确定性断言）
- 教义（#167 组）：文案键只定义一次；双端实现由同一键表生成，漂移结构性不可能；平台差异（安装命令、语言检测机制）保留为键表字段/适配层差异
- 键名对齐（#225）：同一处文案双端查同一个键——资料库空态与导入提示曾各查一个键，已对齐为 `library_empty` + `import_hint`，重复键删除
- 接缝值原样（#226）：播放失败分类跨接缝传核心原始值（`expired` / `cdn_rejected` / `other`，非 HTTP 失败为空），UI 侧不加前缀、对端不剥前缀
- 分派归核心（#216 组收尾）：三条分派（播放失败、解析失败、解析器状态）全部下沉到 `rust-core/src/message/`，双端适配层只剩按键取模板与填占位符；新增分类只改核心与键表
- 校验：`testing/l0/check-l10n-keys.py`（#185）——键表结构、双端生成物一致性、Windows 映射覆盖，任一红即 run-all 非零退出；
  #370 起再比对**访问器**：Windows 代码与测试调用的每个 `L10n::X()` 必须有定义，每个 windows 键必须至少被一个访问器取用
  （检验面从键名表扩到调用面；自测 `testing/l0/tests/test_check_l10n_keys.py`）。#370 落地时曾报红（`TrayQuit` 被调用却不存在，
  另有一批键无访问器），#371 的访问器生成合入后已转绿

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| LK-01 | 静态文案双语言 | 键表每键 zh/en 齐全；固定 locale 下取对应语言（macOS UserDefaults 覆盖、Windows 注册表覆盖） | 固定 locale 断言 |
| LK-02 | 播放失败分类文案（#120） | `expired` → 保留"重新粘贴"建议；`cdn_rejected` → 换网络且**不**建议重贴；其它 → 泛化"播放失败"；中英分支同分类 | 核心表驱动（MS-01/MS-02，#228 起分派在核心） |
| LK-03 | 解析失败文案 | 中文 headline（各 kind）+ 英文原始 detail；未识别 kind → 原文 | 核心表驱动（MS-04/MS-05，#230 起分派在核心） |
| LK-04 | 解析器状态文案 | checking/verifying/updating/failed 各文案；downloading 有 total → `x / y MB`、无 → `x MB`；未知/quiet → 空串 | 核心表驱动（MS-06~08，#232 起分派与换算在核心） |
| LK-05 | 来源徽标与托盘文案 | tag local/youtube/bilibili/direct_url、托盘播放/暂停/停止/上下首 | 双端 |
| LK-06 | 访问器与键表对应（#182/#185/#370/#371） | 键表在 zh/en 两语言下均有可用取值；生成物与键表一致；Windows `Key()` 映射覆盖全部 windows 键；Windows 具名访问器由键表生成（`L10nAccessors.h`），每个 windows 键恰有一个访问器、调用的访问器都有定义；托盘文案（含 `TrayQuit`）中英断言原样通过 | SW-17 + L0 校验 + `L10nTests.cpp` |

## 核心消息规格（P0 — 分派归核心，#216 组）

核心把具名分类翻译成「文案键 + 参数」（`rust-core/src/message/`），双端适配层只按键取模板、按参数填占位符。测试：`rust-core/tests/message_spec_behavior.rs`。

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| MS-01 | 播放失败分类到文案键（#227） | `expired` / `cdn_rejected` 各自的键；`other` 与非 HTTP 失败走泛化标题；键段不带参数 | 核心表驱动 |
| MS-02 | 中英拼装形状（#227） | 中文为标题 + 空行 + 详情前缀 + 详情，英文为标题 + 空行 + 详情；同一分类两语言选中同一个键（#135） | 核心表驱动 |
| MS-03 | 详情为空（#227） | 只留标题一段，两语言同 | 核心表驱动 |
| MS-04 | 解析失败分类到文案键（#229） | 七种分类各自的键；`internal` 与未识别分类回退引擎原文；英文一律返回引擎原文；中文保持 headline 加详情形状 | 核心表驱动 |
| MS-05 | 平台差异键（#229） | yt-dlp 缺失/过旧两条按平台标记选 macOS 与 Windows 各自的键；无差异的分类两平台同键 | 核心表驱动 |
| MS-06 | 解析器阶段到文案键（#231） | 检查中/下载中/校验中/更新中/失败各自的键；空闲与就绪产出空规格 | 核心表驱动 |
| MS-07 | 下载进度两种形态（#231） | 有总量 → `resolver_status_downloading` 带 `received`/`total` 两个参数；无总量（缺失或 0）→ `resolver_status_downloading_unknown_total` 只带 `received` | 核心表驱动 |
| MS-08 | 字节到 MB 换算（#231） | 除以 1048576 保留一位小数，换算与位数由核心决定 | 核心表驱动 |
| MS-09 | 目录导入结果分类到文案键（#375） | 有导入（无视失败与否）→ `imported_tracks` 带 `count`/`s` 参数；无导入且有失败 → `import_dir_failed`；无导入无失败 → `import_dir_empty`；失败/空两键不带参数 | 核心表驱动 |
| MS-10 | 单文件导入结果分类到文案键（#377） | 有导入 → `imported_tracks` 带 `count`/`s` 参数（优先于不支持计数）；无导入且格式不支持 → `import_file_unsupported`；无导入且非不支持（读取失败）→ `import_file_failed`；两键不同，不折成一种失败 | 核心表驱动 |
| MS-11 | 批量导入结果分类到文案键（#379） | 全部成功（有导入无失败）→ `imported_tracks` 带 `count`/`s` 参数；部分成功 → `import_some_failed` 恰好带 `imported`/`failed` 两个参数（两个数字在同一句话里）；无导入有失败 → `import_all_failed`；两者皆 0 → `import_none_found`；后两键不带参数 | 核心表驱动 |
| MS-12 | M3U8 导入结果分类到文案键（#381） | 有失败 → `import_some_failed` 恰好带 `imported`/`failed` 两个参数（成功为 0 也照报，沿用双端原措辞）；无失败有导入 → `imported_tracks` 带 `count`/`s` 参数；两者皆 0（列表里没有可读条目）→ 确定的 `import_none_found`，不返回空规格；与目录/单文件/批量三条路径同一种返回形状 | 核心表驱动 |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| LK-07 | 平台差异字段 | yt-dlp 安装命令（macOS brew / Windows winget）只出现在对应平台生成物中——含 Windows 访问器（#372）；校验器按 platform 字段对两端期望键集分别计算，并直接从提交的 Swift 取值表、Windows 取值与访问器生成物读出实际键集比对，不经过生成器；选键由核心按构建目标决定（MS-05） | L0 生成物比对 + 核心表驱动 |
| LK-08 | 语言检测差异（适配层） | macOS 跟随系统 Locale + AppLanguage；Windows 系统 UI 语言 + 注册表覆盖——检测机制不入键表 | 双端固定 locale |
| LK-09 | 适配层模板填充（#228） | 按键取模板、按参数填 `{占位符}`、按顺序拼接核心规格的各段；未知键回退（macOS 回退键名、Windows 回退空串） | 双端固定 locale |
| LK-10 | 播放模式提示接线（#373） | Windows 播放栏新增播放模式控件（点击循环 `CyclePlayMode`，图标取视图状态 `PlayerBarState().playModeIcon`，不含 WinUI 类型的 `rhythm::view::Icon`：顺序 List / 随机 Shuffle / 单曲循环 RepeatOne / 列表循环 RepeatAll，由播放栏换成 XAML Symbol；映射自 #335 起在 `ViewState.cpp`，#329），提示文案取键表 `play_mode_tooltip`（生成访问器 `PlayModeTooltip()`）；中英两语言均能取到 | Windows 固定 locale（`L10nTests.cpp`：文案与四种模式图标）；点击循环后图标随模式（`AppStateBehaviorTests.cpp`，#411）；两条均断言视图状态的返回值（#335） |
| LK-11 | 链接解析失败兜底文案（#374） | 解析失败却没有可显示内容（核心无返回、无分类无详情、`internal` 无详情）时显示键表 `url_resolve_failed`（生成访问器 `UrlResolveFailed()`），不弹空白对话框；有引擎详情时仍显示详情；Windows 解析桥接的无返回分支留空消息，兜底只在 `L10n::UrlResolveError` 一处（#412） | Windows 固定 locale（`L10nTests.cpp`） |
| LK-12 | 导出反馈文案（#352） | `ExportedTracks(n)` 取键表 `exported_tracks`（中文「已导出 n 首歌曲」，英文单复数同导入）；`ExportFailed(code)` 填 `export_failed` 的 `{code}`；曲目数据解不开时 `ExportInvalidTracks(code)` 取 `export_invalid_tracks`（#321 评审）；标题 `export_result_title` / `export_failed_title`。三个新键仅 Windows（macOS 导出成功不提示） | Windows 固定 locale（`L10nTests.cpp`） |
| LK-13（#359） | 歌单详情空态文案（#359） | `NoPlaylistSelected()` 取键表 `no_playlist_selected`（中文「未选中歌单」，英文 "No playlist selected"）；未选中歌单时由视图状态 `PlaylistDetailOf` 取，视图不写死字符串。该键仅 Windows | Windows 固定 locale（`L10nTests.cpp`） |
| LK-14（#376） | 目录导入结果文案 | 双端 `ImportDirectory`/`importDirectory` 不再自选文案键，改传具名计数 `{imported, failed}` 给核心入口渲染；同一组计数两端渲染同一句话 | 核心表驱动（MS-09，#376 起分派在核心）；Windows `AppStateBehaviorTests.cpp`（WA-23）+ macOS `AppStatePlaybackMainPathTests.swift`（AS-24）行为回归原样通过 |
| LK-15（#378） | 单文件导入结果文案 | 双端 `ImportFile`/`importFile` 不再自选文案键，改传具名计数 `{imported, unsupported}` 给核心入口渲染；「不支持」与「读取失败」在双端仍显示为两句不同的话 | 核心表驱动（MS-10，#378 起分派在核心）；Windows `AppStateBehaviorTests.cpp`（WA-29）+ macOS `AppStatePlaybackMainPathTests.swift`（AS-24）行为回归原样通过 |
| LK-16（#380） | 批量导入结果文案 | 双端 `ImportPaths`/`importURLs` 不再自选文案键、不再自行拼接数字，改传具名计数 `{imported, failed}` 给核心入口渲染；部分成功时两端显示同一句话（两个数字在同一句里） | 核心表驱动（MS-11，#380 起分派在核心）；Windows `AppStateBehaviorTests.cpp`（WA-30）+ macOS `AppStatePlaybackMainPathTests.swift`（AS-23）行为回归原样通过 |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| — | 无（#141/#145/#120 均已修复并有固定 locale 测试锁定） | — | — |
