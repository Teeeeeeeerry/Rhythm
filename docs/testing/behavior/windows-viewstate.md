# Windows 视图状态行为清单

- 模块：`windows/Rhythm/ViewState.h` + `ViewState.cpp`（行为库，#317 组）
- 职责：「界面该渲染什么」——纯函数，输入 `AppState`，输出普通结构体，不含任何 UI 框架类型（#331）。XAML 壳只把结构体灌进控件，不再自己做渲染决定；壳本身不测（被有意做薄）
- 测试设施：`windows/tests/ViewStateTests.cpp`（Catch2，链接行为库；构造 `AppState` 直接设字段，不需要 XAML 与 WinRT）
- 教义：`docs/adr/0001-行为清单制测试教义.md`

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| VS-01（#331） | 资料库行 `LibraryRows` | 空曲库 → 空行列表；每首已加载曲目 → 一行，行内带着该曲目（点击播放用）与其标题 | 新测 |
| VS-14（#336） | 按艺人/专辑排序 `LibraryRows(state, ArtistAlbum)` | 艺人优先，其次专辑，其次音轨号（#503 起即分组结构自上而下读出的顺序，见 VS-51） | 新测 |
| VS-15（#336） | 按首字母排序 `LibraryRows(state, Alphabetical)` | 标题升序（#504 起即首字母分节自上而下读出的顺序，见 VS-55） | 新测 |
| VS-18（#337，原 WB-01） | 曲目行时长文案 `TrackRow::durationText` | 分:秒，秒补零（65 秒 → 1:05，5 秒 → 0:05，不足一秒舍去） | 新测 |
| VS-21（#338，原 WB-02） | 来源标记 `SourceBadgeOf().tag` | local → 本地/Local、youtube → YT、bilibili → B站/Bili、direct_url → 链接/Link、未知 → 空串（中英各一次） | 新测 |
| VS-22（#338，原 WB-03/WB-20） | 来源前景色 `SourceBadgeOf().foreground` | 四种来源 dark/light 双端色值与 palette.json 一致（#121），不透明 | 新测 |
| VS-24（#338，原 WB-04） | 胶囊底色 `SourceBadgeOf().background` | 前景色 @ 声明不透明度（alpha 38，与 macOS `.background(color.opacity(0.15))` 一致），五种来源（含未知）× 两个主题 | 新测 |
| VS-25（#338） | 资料库行带徽标 `TrackRow::badge` | 行的徽标即该曲目来源在所给主题下的徽标（主题由壳传入，#342） | 新测 |
| VS-26（#339） | 行的艺人 `TrackRow::artist` | 取曲目艺人，没有则为空串 | 新测 |
| VS-41（#320） | 歌单列表 `PlaylistListState(state)` | 每个已加载且有标识的歌单一行，按列表顺序，行带标识、名字、曲目数文案；行的标识就是 `AppState::SelectPlaylist` 的入参——列表与详情靠标识对应，不靠位置 | 新测 |
| VS-38（#358） | 歌单详情 `PlaylistDetailOf(state, isDark)` | 输入只有编排状态与主题：有当前歌单 → `hasPlaylist` 为真、标题为歌单名、行按歌单内顺序；没有 → `hasPlaylist` 为假、标题与行都为空；刷新（导入 M3U8 会触发）后的内容在下一次渲染即可见。行与资料库同一种（标题、时长文案、徽标、点击载荷），按歌单内顺序。详情页因此不持有歌单指针或标识（#358 起取代按 id 取行的 VS-27/VS-28） | 新测 |
| VS-39（#359） | 未选中歌单的空态 `PlaylistDetailOf(state, isDark).emptyMessage` | 没有当前歌单 → 空态文案取自键表 `no_playlist_selected`（中英各一次），标题与行为空；有当前歌单 → 空态文案为空。壳据此显示空态文案、隐藏曲目列表与两个歌单动作按钮，返回按钮保留 | 新测 |
| VS-29（#340） | 托盘菜单文案 `TrayMenuState()` | 播放暂停、显示窗口、退出三项文案，跟随语言层的当前语言（中英各一次）；托盘只按此构造原生菜单 | 新测 |
| VS-30（#341） | 空曲库时托盘「播放暂停」不可用 `TrayMenuState().playPauseEnabled` | 无曲目、无当前曲目 → 不可用（原生菜单置灰，点击命令同样以此为准） | 新测 |
| VS-31（#341） | 有当前曲目时托盘「播放暂停」可用 | 起播后 → 可用 | 新测 |
| VS-34（#317） | 解析中且解析器无事可报 `PlayerBarState(state).urlStatusText` | 解析器状态为 idle/ready → 「解析中」文案（中英各一次）；状态经 AppState 的 `PollResolverStatus` 注入（#350，播放条不再直接查询解析器） | 新测 |
| VS-35（#317） | 解析中且首次下载 yt-dlp | 返回下载进度文案（与 `Resolver::StatusText` 同一出处），读作进度而非卡死 | 新测 |
| VS-36（#352） | 导入导出反馈弹窗 `PendingAlert(state)` | 有待显示的导出提示 → 导出自己的标题与正文；否则有待显示的导入提示 → 标题取 `ImportResultTitle`（中英各一次）、正文为导入文案。歌单详情在导入 M3U8 与导出之后、主窗口在工具栏导入开始与结束时（`OnImportingChanged`，#533，见 windows-appstate.md WA-45）按此弹窗，弹出即 `DismissAlerts` | 新测 |
| VS-02（#332） | 播放条进度百分比 `PlayerBarState().progressPercent` | 时长已知 → 位置 / 时长 × 100 | 新测 |
| VS-43（#498） | 进度条可拖动 `PlayerBarState().seekable` | 有当前曲目且时长已知 → 可拖动；无当前曲目，或时长未知（为 0）→ 不可拖动。播放栏据此启用或禁用进度条，进度值仍取 `progressPercent`（VS-02～04） | 新测 |
| VS-44（#498） | 进度条值换算成位置 `SeekPosition(state, percent)` | 0 到 100 的进度条值按时长换算成秒（50 → 时长一半）；越界的值夹进范围（140 → 时长，-10 → 0）；时长未知 → 0。播放栏松手时把它交给 `AppState::Seek` | 新测 |
| VS-05（#333） | 缓冲中的时间文案 `PlayerBarState().timeText` | 缓冲中 → 缓冲文案（取 `L10n::Buffering`，中英各一次），不显示时钟（#137：链接起播要等一会儿，0:00 / 0:00 像是死机） | 新测 |
| VS-06（#333） | 正常的时间文案 | 不缓冲 → 「位置 / 时长」，分:秒，秒补零（65 秒 → 1:05，5.9 秒 → 0:05，不足一秒舍去） | 新测 |
| VS-08（#334） | 播放条标题与艺人 `PlayerBarState().title/artist` | 有当前曲目 → 其标题；有艺人 → 其艺人 | 新测 |
| VS-11（#335） | 播放条音量 `PlayerBarState().volumePercent` | 返回当前音量（滑块刻度 0 到 100：0.35 → 35） | 新测 |
| VS-12（#335） | 播放按钮图标 `PlayerBarState().playIcon` | 播放中 → 暂停图标；非播放中 → 播放图标。播放模式图标 `playModeIcon` 同在此结构（覆盖见 LK-10，`l10n-keys.md`） | 新测 |
| VS-45（#501） | 常驻侧栏条目 `SidebarState(state)` | 两项，按 macOS 顺序：资料库、播放列表；每项同时带图标与文案（文案取键表 `library_tab` / `playlists_tab`，中英各一次），没有只剩图标的中间态。壳把它灌进常驻展开的侧栏，不再用汉堡菜单与浮层 | 新测 |
| VS-46（#501） | 侧栏选中项 `SidebarEntry::selected` | 恰好 `AppState::SelectedView` 那一项为选中：启动时资料库选中；切到播放列表后只有播放列表选中。壳据此套用选中样式（底色 `rhythmSelection`、前景 `rhythmAccent`，取自 palette.json，与 macOS 侧栏一致）。#516 起侧栏条目是同组单选按钮，选中与否仍取此字段（`IsChecked`），UI Automation 经 SelectionItem 读出当前选中项；悬停与按下底色为 `rhythmHover` / `rhythmPressed`（palette.json），由 L2 `MainWindow_Pointer_*` 强制视觉状态拍下 | 新测 |
| VS-47（#502） | 资料库唯一的视图切换 `LibraryViewSwitch(state)` | 两段，按 macOS 分段控件的顺序与标签：按艺人/专辑（`ByArtistAlbum`）、按首字母（`ByLetter`），中英各一次；每段带它选中的排序。壳把它灌进资料库页顶部居中的分段控件，主窗口工具栏不再有视图下拉框 | 新测 |
| VS-48（#502） | 切换视图后选中段与列表顺序 `AppState::LibraryOrder` + `LibraryRows(state, isDark)` | 启动时按艺人/专辑、第一段选中；把 `LibraryOrder` 设为第二段的排序后，只有第二段选中，`LibraryRows(state, isDark)` 改为按首字母。所选排序是状态而非页面私有值，资料库页随刷新重建后仍保持 | 新测 |
| VS-57（#516） | 工具栏导入控件 `ImportControlsState(state)` | 与 macOS 工具栏一致：资料库页且未在导入 → 显示两个导入按钮、不显示进度；资料库页导入进行中（`AppState::IsImporting`）→ 隐藏按钮、显示进度指示，其提示与无障碍名称取键表 `importing`（中英各一次）；播放列表页无论是否在导入 → 两者都不显示。视图切换两段同 VS-46 改为同组单选按钮（选中取 `selected`，悬停/按下同色） | 新测 |
| VS-49（#503） | 按艺人/专辑分组 `ArtistAlbumSections(state, isDark)` | 艺人分节按艺人名排序，节内按专辑名分组；组内曲目按碟号、音轨号排序（缺失记 0），相同时保持曲库原顺序；与 macOS `groupByArtistAlbum` 一致 | 新测 |
| VS-50（#503） | 未知艺人/专辑的归组 | 缺艺人归入「未知艺人」节、缺专辑归入「未知专辑」组，文案取键表 `unknown_artist` / `unknown_album`（中英各一次），按当前语言的文案与其他名字一起排序（中文下「未知艺人」排在拉丁字母艺人之后；macOS 同规则见 appstate-macos.md AS-45，#518）；同名专辑分属两位艺人时仍是两组 | 新测 |
| VS-51（#503） | 平铺的艺人/专辑顺序即分组顺序 | `LibraryRows(state, ArtistAlbum)` 等于 `ArtistAlbumSections` 自上而下读出的曲目顺序，艺人/专辑的规则只有一处 | 新测 |
| VS-52（#503） | 资料库列表逐行 `LibraryLines(state, isDark)` | 按艺人/专辑：每节先是艺人标题行，每组先是专辑标题行、随后是组内曲目行（标记在组内）；按首字母：每节先是字母标题行、随后是节内曲目行（不在组内，#504）；空曲库无行。页面只把每行变成列表项：标题行渲染为不可点、不可聚焦的标题，专辑标题旁带封面占位（`rhythmElevated`），组内曲目缩进到与专辑标题对齐 | 新测 |
| VS-53（#504） | 按首字母分节 `LetterSections(state, isDark)` | 节标题为标题（先合成为 NFC，分解写法的重音字母与预组合写法同节）首字符完整转大写（大小写归同一节）；首字符（按码位，BMP 以外的字母同样）带 Unicode Alphabetic 属性才自成一节（含中日韩文字，与 macOS `Character.isLetter` 一致；分类与大小写取系统 ICU，运行时载入，缺失时退回 Win32），数字、标点、表情符号与空标题归入 `#`；节按码位排序（与 Swift 字符串比较一致），`#` 在字母之前、中日韩文字在拉丁字母之后；与 macOS `groupByFirstLetter` 一致 | 新测 |
| VS-54（#504） | 节内顺序 | 按标题忽略大小写、按用户区域设置排序（对应 macOS `localizedCaseInsensitiveCompare`），标点按字符串排序计入（`a-z` 在 `ab` 前，不按词排序忽略连字符），相同时保持曲库原顺序 | 新测 |
| VS-55（#504） | 平铺的首字母顺序即分节顺序 | `LibraryRows(state, Alphabetical)` 等于 `LetterSections` 自上而下读出的顺序；`LibraryLines` 按首字母时为每节的字母标题行加节内曲目行，曲目不在组内 | 新测 |
| VS-58（#520） | 按首字母的字母索引条 `LetterIndex(state)` | 按首字母时条目即 `LetterSections` 的节标题、顺序与分节一致（`#` 在前，不另设规则），每项带其节标题行在 `LibraryLines` 中的位置（该行是标题行、标题相同）；按艺人/专辑或空曲库 → 无条目（与 macOS 一致）。资料库页在列表右侧逐项渲染为纯文字按钮（前景 `rhythmAccent`），点击把对应节标题行滚到顶部；无条目时整列折叠 | 新测 |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| VS-03（#332） | 时长未知时的进度 | 时长为零 → 0，不除零、不出无穷；从已知进度变为时长未知时同样回到 0，不沿用上一次的值（视图原先在时长为零时跳过更新） | 新测 |
| VS-04（#332） | 位置越界时的进度 | 位置超出时长 → 100；位置为负 → 0 | 新测 |
| VS-07（#333） | 位置与时长都为零 | 返回 0:00 / 0:00；位置为负同样读作 0:00，不出现负号 | 新测 |
| VS-09（#334） | 当前曲目没有艺人 | 艺人为空串 | 新测 |
| VS-10（#334） | 没有当前曲目 | 标题为「未在播放」文案、艺人为空串，不沿用上一首的值（视图原先只在有当前曲目时更新，清空后残留旧值）；中英各一次 | 新测 |
| VS-56（#512） | 是否显示艺人行 `PlayerBarState().showsArtist` | 只有当前曲目带非空艺人时为真；没有当前曲目、曲目没有艺人或艺人为空串时为假（与 macOS `if let artist` 一致）。壳据此折叠艺人行，未播放时「未在播放」标题与封面占位垂直居中 | 新测 |
| VS-13（#335） | 缓冲中的播放按钮 | 缓冲中视为播放中 → 暂停图标（即便 `IsPlaying` 尚未置位） | 新测 |
| VS-16（#336，#503 修订） | 缺少艺人的曲目 | 不丢失；缺艺人归入「未知艺人」组、按该文案与其他艺人名一起排序（#503 起与 macOS 一致，原先按空名排在最前），彼此之间保持曲库原顺序（稳定排序，原先 `std::sort` 对相等键的顺序不确定） | 新测 |
| VS-17（#336） | 排序不修改传入的状态 | 两种排序各跑一次后 `AppState::Tracks` 顺序不变（行列表是副本） | 新测 |
| VS-19（#337） | 时长为零 | 返回 0:00 | 新测 |
| VS-20（#337） | 时长超过一小时 | 分钟继续累加、不进位成小时（3725 秒 → 62:05），与 macOS `Track.durationFormatted` 一致 | 新测 |
| VS-23（#338） | 未知来源的前景色 | 回退正文色（dark `#ABC8D4` / light `#0D464D`），绝不返回系统灰（F4） | 新测 |
| VS-42（#320） | 空歌单列表与无标识歌单 | 没有歌单 → 行为空、空态文案取自键表 `no_playlists`（中英各一次）；歌单没有标识（未入库）→ 不渲染该行 | 新测 |
| VS-40（#359） | 选择被刷新清空后的详情 | 当前歌单被删除并刷新后再渲染 → 空态（`hasPlaylist` 为假、行为空、有空态文案），不是一个失效的指向 | 新测 |
| VS-32（#341） | 托盘可用性与协调器一致 | 空、有曲库无当前曲目（协调器可空闲起播）、清空曲库三种状态下都等于 `Coordinator->CanTogglePlayback()` | 新测 |
| VS-33（#317） | 没有在解析链接 | 链接状态行为空串，即便解析器仍报告下载进度 | 新测 |
| VS-37（#352） | 没有待显示的提示 | 两个提示都未置位 → 无弹窗，即便留有旧文案 | 新测 |

## 错误路径（P2）

（纯函数，输入是已加载的状态，不设错误路径。）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|

## 红测登记

（暂无。）

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
