# Windows RhythmCore（Bridge 封装层）行为清单

- 模块：`windows/Rhythm/Bridge/RhythmCore.h` + `.cpp`（FFI 包装类、Track 模型、Resolver 静态封装、UTF-8/UTF-16 转换；来源徽标与时长文案已迁入视图状态，见 `windows-viewstate.md`）
- #364 起：解析结果与协调器结果的字段一律由生成物解码（`generated::ResolveResultFromJson` / `CoordinatorResultFromJson`），桥接层只保留解码器做不了的调用点校验（无载荷、JSON 不合法、成功却无载荷）
- #367/#368 起：歌单同样由生成物解码（`generated::PlaylistFromJson`），桥接层只走数组；创建与修改时间戳随契约进入编解码，此前从未被解码
- 历史回归：`#21`（解析失败原因）、`#39`（URL 持久化）、F1（来源徽标色双主题——`testing/l1/windows/source_color_test.cpp` 为该修复的验收测试，`#122` 已解除自声明桩；#338 起直测视图状态的 `SourceBadgeOf`）
- 测试设施：Catch2 v3.5.4 header-only（`windows/tests/vendor/`）；测试 main 已 `init_apartment`（Brush 构造可用）；测试文件 `windows/tests/BridgeBehaviorTests.cpp`。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WB-01 | 时长文案（原 `Track::DurationFormatted`） | 已随 #337 迁入视图状态的行时长文案，见 `windows-viewstate.md` VS-18～VS-20 | 已迁移 |
| WB-02 | 来源标记（原 `Track::SourceTag`） | 已随 #338 迁入视图状态，见 `windows-viewstate.md` VS-21 | 已迁移 |
| WB-03 | 来源前景色（原 `Track::SourceColor`） | 已随 #338 迁入视图状态，见 `windows-viewstate.md` VS-22 / VS-23 | 已迁移 |
| WB-04 | 胶囊底色（原 `Track::SourceBackgroundColor`） | 已随 #338 迁入视图状态，见 `windows-viewstate.md` VS-24（未知来源的胶囊底改为回退正文色 @ alpha 38，不再是灰） | 已迁移 |
| WB-05 | `JsonToTrack`/`TrackToJson` 往返 | 各字段保真；null 可选字段 → `nullopt`；缺省字段取默认（#101 已修复：album_artist/genre/file_size/date_added/last_played 全部解析；date_added 由 DB 插入时盖章、last_played 新插入为 NULL） | 新测（待 Windows 验证） |
| WB-06 | `Utf8ToWide`/`WideToUtf8` 往返 | 中文/emoji 标题转换无损坏；空串安全 | 新测 |
| WB-07 | `Library` 空指针防御 | open 失败（坏路径）→ 各方法安全默认（-1/空列表/false/原 track 返回） | 新测 |
| WB-09 | `Resolver::ResolveURL` 成功 | `ok=true`；track 字段解析正确；`sourceUrl` 保留页面 URL（非 CDN 链接） | 新测（真 core 直链） |
| WB-10 | `Resolver::ResolveURL` 失败 | null → `LastResolveFailure`：kind/message 来自 core 的 JSON（#21） | 新测 |
| WB-11 | `LastResolveFailure` 兜底 | 无 payload → kind=internal + 通用英文消息；malformed JSON → 保留通用消息（兜底分支现状不可达：core 失败必先写合法 `{kind,message}`、成功即清空；测试锁定"失败恒携带 core 的 kind/message"） | 新测（待 Windows 验证） |
| WB-12 | `Resolver::StatusText` | checking/verifying/updating/failed 各文案；downloading 有 total 时 `x / y MB`、无 total 时 `x MB`；未知/quiet → 空串（中/英各固定一次） | 新测 |
| WB-13 | `ResolverStatus::IsQuiet` | idle/ready → true；其余 → false | 新测 |
| WB-14 | `Resolver::ClassifyURL` | 返回 "youtube"/"bilibili"/"direct_url"；失败 → 空串 | 新测 |
| WB-20 | 前景色色值（原 `Track::SourceForegroundColor`） | 已随 #338 迁入视图状态，见 `windows-viewstate.md` VS-22 | 已迁移 |
| WB-21 | `ExportM3U8` | 经生成的编码器写出 M3U8（含 `#EXTM3U` 与曲目路径，中文路径不乱码），返回具名结果 `Exported` 与曲目数；目录不存在 → `WriteFailed`、`code == -2`（#428/#351） | 新测 |
| WB-22 | 文件选择面板的扩展名 `kAudioFileTypes` | 每个扩展名都是核心收的格式：同扩展名的非音频文件经 `ImportFile` 只会计为读取失败、从不计为格式不支持；对照 `.txt` 计为不支持（核心是闸门，列表只塑形对话框，#242）；列表为普通字符串，桥接层不带 WinRT 类型（#327） | 新测 |
| WB-17 | `Coordinator` 绑定真实 `Library` 起播 | `Library::Handle()` 交给核心：起播成功则该曲目播放次数加一；无音频设备时结果为核心分类错误 `playback_failed` 且不记录（#416） | 新测 |
| WB-23 | 解析结果由生成物解码（#362） | 直链成功与非法链接失败两种核心载荷，生成的 `ResolveResultFromJson` 与 `Resolver::ResolveURL` 逐字段一致：`ok`、`resolved` 的标题/艺人/时长/来源类型，失败时的 `errorKind`/`errorMessage`。只比对格式合法的核心载荷：`ok` 为真却无 `resolved`、JSON 不合法时报 `internal` 属于调用点校验而非解码，由 `ResolveURL` 保留（#364 改调生成物时不变） | 新测（真 core） |
| WB-24 | 解析结果的契约字段全部被解码（#362） | 每个字段都有值的载荷：`ok`、`resolved` 的七个字段（含 `stream_url`、`thumbnail_url`、`http_headers`）、`error_kind`、`error_message` 逐一还原 | 新测 |
| WB-25 | 协调器结果由生成物解码（#363） | 同一输入分别交给裸核心协调器与 `Coordinator` 包装：无位置曲目（`no_playable_location`）、文件缺失、真实 wav（有设备时成功带当前曲目，无设备时 `playback_failed`）三种起播，外加空闲时的切换、下一首、上一首；生成的 `CoordinatorResultFromJson` 与包装结果逐字段一致（`ok`、`errorKind`、`errorMessage`、`currentTrack` 整条曲目、`playbackActive`）。当前曲目有值只出现在有音频设备时的真实 wav 起播，无设备机器上由 WB-26 覆盖曲目解码。只比对格式合法的核心载荷：空载荷与 JSON 不合法时报 `internal` 属于调用点校验而非解码，由包装保留（#364 改调生成物时不变） | 新测（真 core） |
| WB-26 | 协调器结果的契约字段全部被解码（#363） | 每个字段都有值的载荷：`ok`、`error_kind`、`error_message`、`current_track`（经曲目解码）、`playback_active` 逐一还原；成功载荷的错误两项与当前曲目解码为空（`std::optional`，与契约一致），不是空串 | 新测 |
| WB-27 | 解析出的曲目形状（#364） | 直链解析成功后 `outcome.track` 恰为：`id == -1`（未入库）、来源类型/标题/艺人/时长取自核心、`sourceUrl` 为粘贴的页面 URL、其余取模型缺省（`isAvailable` 为真；`id` 与可用性同 macOS 与核心自建的解析曲目，macOS 另有空标题回退为 URL，Windows 未做）。此前把解析载荷整个当曲目解码，留下 `id 0` 并把曲目标为不可用，入库后数据库里 `is_available = 0` | 新测（真 core，先红后绿） |
| WB-28 | 歌单由生成物解码，含两个时间戳（#367/#368） | 真库建歌单并加一首曲目后 `AllPlaylists`：`id`/`name` 还原，创建与修改时间戳都有值且为 `YYYY-MM-DD HH:MM:SS`（核心写入，此前不在契约里、从未被解码），曲目列表经曲目自己的编解码还原 id 与标题；载荷里没有这两个键时解码为空（`std::optional` 无值），不是空串也不是别的缺省值 | 新测（真 core） |
| WB-29 | 导出返回码映射为具名结果（#351） | `0` → `Exported`（带曲目数）；`-1` → `InvalidTracks`；`-2` → `WriteFailed`；未知负码也算 `WriteFailed` 并保留原码，绝不当成功 | 新测 |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| WB-15 | `ResolveURL` malformed JSON | `ok=false`、kind=internal、消息含 "Malformed resolver response"（分支现状不可达：core 自产 payload 恒可解；测试锁定 core payload 恒解码）；成功却无 `resolved` 报 `internal` 同为调用点校验、现状不可达（#364 起与解码分开，解码走生成物） | 新测（待 Windows 验证） |
| WB-16 | `ParseTrackList` 空/null 输入 | 返回空列表，不崩溃（null 分支不可达：FFI 空库返回 `"[]"` 非 null；经 `AllTracks` 黑盒锁定空库 → 空列表） | 新测（待 Windows 验证） |
| WB-18 | `Coordinator::SyncQueue` 空队列 | 不崩溃；之后起播结果与非空队列一致（队列序列化为合法空数组，#416） | 新测 |
| WB-19 | 协调器绑定打开失败的 `Library` | 句柄为空；起播/传输/同步调用安全返回、不崩溃（#416） | 新测 |

## 已知缺口

| 编号 | 缺口 | 状态 | 重启入口 |
|---|---|---|---|
| WB-G1 | Windows 视图外观回归（L2 截屏 + golden 像素比对）：截屏宿主只有骨架、无 CMake 工程、无 golden；徽标色值表（视图状态 VS-22～VS-24，原 WB-03/04）只锁数值，不覆盖视图实际渲染 | 未实现，已从测试入口移除（#387） | `testing/README.md`「Windows L2 缺口」 |

## 错误路径（P2）

（薄封装层，错误经返回码与 null 传播，不另设。）

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| WB-02 | `SourceTag` 返回英文 "Local"，测试期望「本地」 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：断言依赖机器 UI 语言；改为 `LanguageScope` 固定中/英各断言一遍，已解禁转绿 |
| WB-04 | `SourceBackgroundBrush` 抛未捕获异常 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：画刷是 Windows App Runtime 类，未打包的测试 exe 无法激活；色值拆为纯值 `SourceBackgroundColor(isDark)`（主题作参数），测它，已解禁转绿 |
| WB-12 | `StatusText` 返回英文文案，测试期望中文 | [#418](https://github.com/Teeeeeeeerry/Rhythm/issues/418) | 测试口径：同 WB-02，中/英各断言一遍，已解禁转绿 |
