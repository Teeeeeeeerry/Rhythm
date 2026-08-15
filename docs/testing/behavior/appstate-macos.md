# AppState（macOS）行为清单

- 模块：`macos/Rhythm/AppState.swift`（播放编排、队列同步、导入流程、URL 解析导入）；测试：`macos/Tests/AppStateTests/`（`AppStatePlaybackMainPathTests.swift` AS-01–26、`AppStatePlaybackBoundaryTests.swift` AS-27–39、`Support/PlaybackTestSupport.swift` SpyPlayer/夹具、`AppStateImportTests.swift` 既有导入回归）
- 历史回归：`#24`+`#25`（托盘可用性）、`#32`（单文件导入）、`#33`（删除曲目）、`#38`（后台导入）、`#39`（URL 持久化）、`#51`（先停后播）、`#53`（停止按钮）、`#66`+`#67`（DB 重载一致性）、`#69`+`#72`（队列同步）、`#71`+`#74`（导入不自动播放）、`#73`（seek）、`#21`（解析错误文案）
- 接缝需求（最小接缝，已落地）：
  1. `player` 协议化：`RhythmPlayerProtocol`（playFile/playURL/pause/resume/stop/setVolume/seek + position/duration/state/errorMessage），`AppState.player` 类型为协议、默认 `RhythmPlayer()`。测试注入 SpyPlayer 记录调用序列（断言 #51 的"先 stop 后 play"顺序）。
  2. resolver 注入：`AppState.resolver` 可注入闭包（默认指向全局 `resolveURL`），测试注入 stub。
  3. queue 断言面：经 `canPlayNext`/`canPlayPrevious` 与真 `RhythmQueue`（链接真 core，无 IO）间接断言。
  4. 辅助测试面：`openDatabase(at:)`（临时库路径）、`isPollingResolverStatus`（轮询 Timer 生命周期的只读镜像）。
- 测试途径：XCTest，沿用现有模式（真 rust-core 静态库 + 临时 SQLite 库注入 `library`）；异步路径（importURLs/resolveAndImport）用 expectation 等待主线程回调。

## 主路径（P0 — 该模块票的合并门槛）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| AS-01 | `openDatabase` | 创建 `RhythmLibrary(dbURL)`；`tracks`/`playlists` 从库刷新 | 真库（临时路径） |
| AS-02 | `refreshLibrary` 队列同步（#69/#72） | `queue.replace(tracks)` 被调用；`currentTrack` 存在且 `id>=0` 时 `jumpTo(current.id)` | 真库 + 真 queue 经可用性断言 |
| AS-03 | `playTrack` 本地曲目 | 顺序：`currentTrack` 置位 → `player.stop()` 先于 `playFile`（#51）→ `playFile(filePath)` → `isPlaying=true` → `recordPlay(id)` → 队列建立（`RhythmQueue(tracks:)`+`setMode(playMode)`+`jumpTo(track.id)`） | SpyPlayer + 真库/真 queue |
| AS-04 | `playTrack` URL 曲目 | 非 local 来源 → `playURL(sourceUrl)`（不调 playFile） | SpyPlayer |
| AS-05 | `playTrack(_:queueTracks:)` 自定义队列 | 队列由 `queueTracks` 构造（playlist 场景），jumpTo 目标不变 | 真 queue 断言 |
| AS-06 | `togglePlayPause` 播放中 | `player.pause()`、`isPlaying=false`、`isBuffering=false` | SpyPlayer |
| AS-07 | `togglePlayPause` 暂停恢复 | 有 `currentTrack` 且非播放 → `player.resume()`、`isPlaying=true` | SpyPlayer |
| AS-08 | `togglePlayPause` 空闲启动 | 无 `currentTrack` 且曲库非空 → `playTrack(tracks.first)` | SpyPlayer + 真库 |
| AS-09 | `playNext` | 队列有 next：`currentTrack` 更新、`player.stop()` 先于分派、按 sourceType 调 playFile/playURL、`recordPlay`；无 next：no-op | SpyPlayer + 真 queue |
| AS-10 | `playPrevious` | 对称于 AS-09 | 同上 |
| AS-11 | `stop()` | `player.stop()`、`isPlaying=false`、`isBuffering=false`、`currentTrack=nil`、`queue=nil`、`position=0`、`duration=0` | SpyPlayer + 真 queue |
| AS-12 | `updatePlaybackProgress` 正常播放 | `position`/`duration` 从 player 同步；`isBuffering=(state==3)` | SpyPlayer（预置状态） |
| AS-13 | `updatePlaybackProgress` 播完连播 | `state==5`（Finished）且有 next → 自动 `playNext` | SpyPlayer + 真 queue |
| AS-14 | `updatePlaybackProgress` 播完终止 | Finished 且无 next → `isPlaying=false` | 同上 |
| AS-15 | `updatePlaybackProgress` 播放失败（#23 类） | `state==4`（Error）→ `isPlaying=false`、`urlError=L10n.playbackFailed(detail)` 非空 | SpyPlayer（预置 Error+消息） |
| AS-16 | `seek(to:)`（#73） | `player.seek(seconds)` 被调；`position` 乐观更新为秒数 | SpyPlayer |
| AS-17 | `cyclePlayMode` | `playMode` 循环至下一模式；`queue.setMode` 同步 | 真 queue 断言 |
| AS-18 | 传输可用性（#24/#25） | `canTogglePlayback=currentTrack!=nil || !tracks.isEmpty`；`canPlayNext/Previous=queue?.hasNext/hasPrevious`；`canStop=isPlaying`——与传输方法实际行为镜像 | SpyPlayer + 真 queue |
| AS-19 | `resolveAndImport` 成功（#74） | trim 输入；成功后 `importResolved(track)`；**不启动播放**；`isResolvingURL` 复位；URL 曲目 `sourceUrl` 存页面 URL（非 CDN 链接） | stub resolver + SpyPlayer（断言无播放调用） |
| AS-20 | `resolveAndImport` 失败（#21） | `urlError=L10n.urlResolveError(kind, detail)` 非空；不弹导入 alert | stub resolver 报错 |
| AS-21 | `importResolved`（#71） | `addTrack` 持久化 → `refreshLibrary`（#66）→ `urlInput=""` → 导入 alert；不播放 | 真库 + SpyPlayer |
| AS-22 | `playResolved`（#39/#66） | `addTrack` → `refreshLibrary` → `player.stop()` 先于 `playURL(saved.sourceUrl)` → `isPlaying=true` → 队列建立并 `jumpTo` 真实 DB id | 真库 + SpyPlayer + 真 queue |
| AS-23 | `importURLs` 批量导入（#38） | `isImporting` 防重入；后台执行；目录/文件分派；成功才 `refreshLibrary`；四种统计文案（全成/部分成/全败/无支持） | 真库（临时目录夹具）+ expectation | 备注：**现状三种可达**（全成/部分成/全败），"未找到支持的音频文件"分支因 #79（importFile 对不支持格式返回 -1）不可达，测试锁定现状 || AS-24 | `importDirectory`/`importFile` 单路径 | `>0` 成功文案+刷新；`==0` "未找到/格式不支持"；`<0` 失败文案 | 真库 + 夹具目录 | 备注：`==0` 分支因 #79（不支持格式走 Err → -1）当前不可达，测试锁定现状（坏文件落失败文案） |
| AS-25 | `confirmDeleteTrack` 删除当前播放曲目 | `player.stop()`、`isPlaying=false`、`currentTrack=nil`、`queue=nil`；`removeTrack` + `refreshLibrary`（#33） | SpyPlayer + 真库 |
| AS-26 | `search` | 空 query → `allTracks()`；非空 → `lib.search(query)` | 真库 |

## 边界情况（P1 — 同波次内完成）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| AS-27 | `togglePlayPause` 无曲目可播 | 无 `currentTrack` 且曲库空 → no-op（不调 player） | SpyPlayer |
| AS-28 | `playTrack` 缺路径/URL | 缺 `filePath`/`sourceUrl` 时不进入播放状态：不置 `currentTrack`/`isPlaying`、不碰 player；playNext/playPrevious 同类分派同样跳过 | SpyPlayer |
| AS-29 | `resolveAndImport` 并发防重入 | `isResolvingURL=true` 期间新调用被忽略；空/纯空白输入被忽略 | stub resolver + expectation |
| AS-30 | resolver 状态轮询生命周期 | resolve 开始启动轮询、结束停止；`isQuiet` 时 `urlStatus=""` | stub resolver + `isPollingResolverStatus` 断言 |
| AS-31 | `deleteSelectedTrack` 无匹配 | `selectedTrackID` 无对应 track → no-op | 真库 |
| AS-32 | `importURLs` 期间再次调用 | `isImporting=true` 时忽略 | 真库 |
| AS-33 | `updatePlaybackProgress` 非播放状态 | `isPlaying=false` 时 no-op（不读 player） | SpyPlayer |
| AS-34 | `playNext`/`playPrevious` 无队列 | `queue==nil` → no-op | SpyPlayer |
| AS-35 | `refreshLibrary` 无当前曲目 | `currentTrack==nil` → 不触碰 queue | 真库 + 真 queue |
| AS-36 | `confirmDeleteTrack` 删除非当前曲目 | 播放器/队列不动，仅 `removeTrack`+`refreshLibrary` | SpyPlayer + 真库 |
| AS-37 | `playResolved` 持久化失败 | `addTrack` 返回 nil → `saved=track`（id=-1）、仍尝试播放、`jumpTo` 因 `id<0` 跳过 | 真库 + SpyPlayer |
| AS-38 | `importResolved` 库未打开 | `addTrack` 返回 nil → `saved=track`、`refreshLibrary` no-op、alert 仍弹 | SpyPlayer（library=nil） |
| AS-39 | `seek` 乐观更新 | `position` 立即更新为请求秒数，不等 core 回报 | SpyPlayer |

## 错误路径（P2 — 仅断言"错误被正确上报"，可顺延）

| 编号 | 行为 | 断言 | 测试途径 | 状态 |
|---|---|---|---|---|
| AS-40 | 解析失败 kind→文案映射（#21） | 各 kind（yt_dlp_missing/timeout/network/unavailable/no_audio_stream/yt_dlp_outdated/internal/invalid_url）均产生非空 `urlError`，不崩溃 | stub resolver 逐个 kind 报错 | **顺延至 Wave 3**（#85 验收范围外；kind→文案纯函数随 `rhythmcore-swift.md` 一并测试） |
| AS-41 | 播放失败信息可见 | Error 状态下 `urlError` 含 core 的错误详情 | SpyPlayer（预置 Error） | 已覆盖（AS-15 已断言含 core 详情，见主路径表；此处仅登记出处） |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| AS-28 | 缺 filePath/sourceUrl 仍置播放中（无声假播放） | [#78](https://github.com/Teeeeeeerry/Rhythm/issues/78) | 已修复（`testPlayTrack_MissingPath_DoesNotEnterPlaying` 解禁，三处分派置位前校验 playableLocation） |

## 附注：RhythmCore 封装层

`RhythmCore.swift` 的包装与辅助函数行为（`PlayMode.next()` 循环、`ResolverStatus.isQuiet`、`decodeJSON`/`encodeJSON` snake_case 转换、`resolveURL` 分派与 malformed 响应处理）不在此清单，随 Wave 3 的 `ffi.md` 姊妹文件 `rhythmcore-swift.md` 一并起草。
