# Windows RhythmCore（Bridge 封装层）行为清单

- 模块：`windows/Rhythm/Bridge/RhythmCore.h` + `.cpp`（FFI 包装类、Track 模型与纯函数、Resolver 静态封装、UTF-8/UTF-16 转换）
- 历史回归：`#21`（解析失败原因）、`#39`（URL 持久化）、F1（来源徽标色双主题——`testing/l1/windows/source_color_test.cpp` 为该修复的验收测试，`#122` 已解除自声明桩、直测真实 `RhythmCore.h`）
- 测试设施：Catch2 v3.5.4 header-only（`windows/tests/vendor/`）；测试 main 已 `init_apartment`（Brush 构造可用）；测试文件 `windows/tests/BridgeBehaviorTests.cpp`。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WB-01 | `Track::DurationFormatted` | 秒 → `m:ss` 格式（秒位零填充） | 新测 |
| WB-02 | `Track::SourceTag` | local→本地/Local、youtube→YT、bilibili→B站/Bili、direct_url→链接/Link、未知→空串（中/英各固定一次） | 新测 |
| WB-03 | `Track::SourceColor(sourceType, isDarkTheme)` | 四种来源 dark/light 双端色值（与 macOS Theme.swift 一致，#121）；未知来源回退 teal 文字色（dark `#ABC8D4` / light `#0D464D`），非系统 Gray（F4）；#147 起前景与胶囊底共用 `SourceColorRGB` 单一表映射 | 新测 |
| WB-04 | `Track::SourceBackgroundColor` | A=38、RGB 与 SourceColor 一致（dark/light 各一次）、未知回退灰；返回普通结构 `rhythm::Color`，画刷由壳的 `TrackItem` 包装（#328） | 新测 |
| WB-05 | `JsonToTrack`/`TrackToJson` 往返 | 各字段保真；null 可选字段 → `nullopt`；缺省字段取默认（#101 已修复：album_artist/genre/file_size/date_added/last_played 全部解析；date_added 由 DB 插入时盖章、last_played 新插入为 NULL） | 新测（待 Windows 验证） |
| WB-06 | `Utf8ToWide`/`WideToUtf8` 往返 | 中文/emoji 标题转换无损坏；空串安全 | 新测 |
| WB-07 | `Library` 空指针防御 | open 失败（坏路径）→ 各方法安全默认（-1/空列表/false/原 track 返回） | 新测 |
| WB-09 | `Resolver::ResolveURL` 成功 | `ok=true`；track 字段解析正确；`sourceUrl` 保留页面 URL（非 CDN 链接） | 新测（真 core 直链） |
| WB-10 | `Resolver::ResolveURL` 失败 | null → `LastResolveFailure`：kind/message 来自 core 的 JSON（#21） | 新测 |
| WB-11 | `LastResolveFailure` 兜底 | 无 payload → kind=internal + 通用英文消息；malformed JSON → 保留通用消息（兜底分支现状不可达：core 失败必先写合法 `{kind,message}`、成功即清空；测试锁定"失败恒携带 core 的 kind/message"） | 新测（待 Windows 验证） |
| WB-12 | `Resolver::StatusText` | checking/verifying/updating/failed 各文案；downloading 有 total 时 `x / y MB`、无 total 时 `x MB`；未知/quiet → 空串（中/英各固定一次） | 新测 |
| WB-13 | `ResolverStatus::IsQuiet` | idle/ready → true；其余 → false | 新测 |
| WB-14 | `Resolver::ClassifyURL` | 返回 "youtube"/"bilibili"/"direct_url"；失败 → 空串 | 新测 |
| WB-20 | `Track::SourceForegroundColor` | 不透明，RGB 与 `SourceColor` 同一张表、同一未知回退（dark/light 各一次）；返回普通结构 `rhythm::Color`，视图绑定的画刷由壳的 `TrackItem` 包装（#428/#328） | 新测 |
| WB-21 | `ExportM3U8` | 经生成的编码器写出 M3U8（含 `#EXTM3U` 与曲目路径，中文路径不乱码）；目录不存在 → `false`（#428） | 新测 |
| WB-22 | 文件选择面板的扩展名 `kAudioFileTypes` | 每个扩展名都是核心收的格式：同扩展名的非音频文件经 `ImportFile` 只会计为读取失败、从不计为格式不支持；对照 `.txt` 计为不支持（核心是闸门，列表只塑形对话框，#242）；列表为普通字符串，桥接层不带 WinRT 类型（#327） | 新测 |
| WB-17 | `Coordinator` 绑定真实 `Library` 起播 | `Library::Handle()` 交给核心：起播成功则该曲目播放次数加一；无音频设备时结果为核心分类错误 `playback_failed` 且不记录（#416） | 新测 |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WB-15 | `ResolveURL` malformed JSON | `ok=false`、kind=internal、消息含 "Malformed resolver response"（分支现状不可达：core 自产 payload 恒可解；测试锁定 core payload 恒解码） | 新测（待 Windows 验证） |
| WB-16 | `ParseTrackList` 空/null 输入 | 返回空列表，不崩溃（null 分支不可达：FFI 空库返回 `"[]"` 非 null；经 `AllTracks` 黑盒锁定空库 → 空列表） | 新测（待 Windows 验证） |
| WB-18 | `Coordinator::SyncQueue` 空队列 | 不崩溃；之后起播结果与非空队列一致（队列序列化为合法空数组，#416） | 新测 |
| WB-19 | 协调器绑定打开失败的 `Library` | 句柄为空；起播/传输/同步调用安全返回、不崩溃（#416） | 新测 |

## 已知缺口

| 编号 | 缺口 | 状态 | 重启入口 |
|---|---|---|---|
| WB-G1 | Windows 视图外观回归（L2 截屏 + golden 像素比对）：截屏宿主只有骨架、无 CMake 工程、无 golden；WB-03/04 只锁色值表，不覆盖视图实际渲染 | 未实现，已从测试入口移除（#387） | `testing/README.md`「Windows L2 缺口」 |

## 错误路径（P2）

（薄封装层，错误经返回码与 null 传播，不另设。）

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| WB-02 | `SourceTag` 返回英文 "Local"，测试期望「本地」 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：断言依赖机器 UI 语言；改为 `LanguageScope` 固定中/英各断言一遍，已解禁转绿 |
| WB-04 | `SourceBackgroundBrush` 抛未捕获异常 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：画刷是 Windows App Runtime 类，未打包的测试 exe 无法激活；色值拆为纯值 `SourceBackgroundColor(isDark)`（主题作参数），测它，已解禁转绿 |
| WB-12 | `StatusText` 返回英文文案，测试期望中文 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：同 WB-02，中/英各断言一遍，已解禁转绿 |
