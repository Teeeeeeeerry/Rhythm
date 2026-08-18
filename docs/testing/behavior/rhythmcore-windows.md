# Windows RhythmCore（Bridge 封装层）行为清单

- 模块：`windows/Rhythm/Bridge/RhythmCore.h` + `.cpp`（FFI 包装类、Track 模型与纯函数、Resolver 静态封装、UTF-8/UTF-16 转换）
- 历史回归：`#21`（解析失败原因）、`#39`（URL 持久化）、F1（来源徽标色双主题——`docs/testing/l1/windows/source_color_test.cpp` 为该修复的验收测试，当前测的是自声明桩而非真实代码）
- 测试设施：Catch2 v3.5.4 header-only（`windows/tests/vendor/`）；测试 main 已 `init_apartment`（Brush 构造可用）；测试文件 `windows/tests/BridgeBehaviorTests.cpp`。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WB-01 | `Track::DurationFormatted` | 秒 → `m:ss` 格式（秒位零填充） | 新测 |
| WB-02 | `Track::SourceTag` | local→本地、youtube→YT、bilibili→B站、direct_url→链接、未知→空串 | 新测 |
| WB-03 | `Track::SourceColor(sourceType, isDarkTheme)` | 四种来源 dark/light 双端色值（与 macOS Theme.swift 一致，#121）；未知来源回退 teal 文字色（dark `#ABC8D4` / light `#0D464D`），非系统 Gray（F4） | 新测 |
| WB-04 | `Track::SourceBackgroundBrush` | A=38 的 SolidColorBrush、RGB 与 SourceColor 一致、未知回退灰 | 新测（apartment） |
| WB-05 | `JsonToTrack`/`TrackToJson` 往返 | 各字段保真；null 可选字段 → `nullopt`；缺省字段取默认（#101 已修复：album_artist/genre/file_size/date_added/last_played 全部解析；date_added 由 DB 插入时盖章、last_played 新插入为 NULL） | 新测（待 Windows 验证） |
| WB-06 | `Utf8ToWide`/`WideToUtf8` 往返 | 中文/emoji 标题转换无损坏；空串安全 | 新测 |
| WB-07 | `Library` 空指针防御 | open 失败（坏路径）→ 各方法安全默认（-1/空列表/false/原 track 返回） | 新测 |
| WB-08 | `Player` 空指针防御 | `State()==-1`、`Position()/Duration()==0`、`ErrorMessage()` 空（ptr-null 分支不可构造：构造器必 `rhythm_player_create`；测试锁定 fresh 默认 + 停播态方法安全） | 新测（待 Windows 验证） |
| WB-09 | `Resolver::ResolveURL` 成功 | `ok=true`；track 字段解析正确；`sourceUrl` 保留页面 URL（非 CDN 链接） | 新测（真 core 直链） |
| WB-10 | `Resolver::ResolveURL` 失败 | null → `LastResolveFailure`：kind/message 来自 core 的 JSON（#21） | 新测 |
| WB-11 | `LastResolveFailure` 兜底 | 无 payload → kind=internal + 通用英文消息；malformed JSON → 保留通用消息（兜底分支现状不可达：core 失败必先写合法 `{kind,message}`、成功即清空；测试锁定"失败恒携带 core 的 kind/message"） | 新测（待 Windows 验证） |
| WB-12 | `Resolver::StatusText` | checking/verifying/updating/failed 各文案；downloading 有 total 时 `x / y MB`、无 total 时 `x MB`；未知/quiet → 空串 | 新测 |
| WB-13 | `ResolverStatus::IsQuiet` | idle/ready → true；其余 → false | 新测 |
| WB-14 | `Resolver::ClassifyURL` | 返回 "youtube"/"bilibili"/"direct_url"；失败 → 空串 | 新测 |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WB-15 | `ResolveURL` malformed JSON | `ok=false`、kind=internal、消息含 "Malformed resolver response"（分支现状不可达：core 自产 payload 恒可解；测试锁定 core payload 恒解码） | 新测（待 Windows 验证） |
| WB-16 | `ParseTrackList` 空/null 输入 | 返回空列表，不崩溃（null 分支不可达：FFI 空库返回 `"[]"` 非 null；经 `AllTracks` 黑盒锁定空库 → 空列表） | 新测（待 Windows 验证） |

## 错误路径（P2）

（薄封装层，错误经返回码与 null 传播，不另设。）

## 红测登记

（暂空。实现时若发现现状代码与清单不符，测试照写、禁用并挂 issue 编号，在此登记。）
