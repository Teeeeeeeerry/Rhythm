# 播放协调器（PlaybackCoordinator）行为清单

- 模块：`rust-core/src/coordinator/mod.rs`（起播、传输、队列同步、有界跳过——双端 UI 的编排规则唯一出处，父 issue #165）；测试：`rust-core/tests/coordinator_behavior.rs`（CO-xx）
- 历史回归：`#51`（先停后播）、`#69`（刷新后队列同步）、`#78`（不可播曲目有界跳过）、`#81`（Windows 无位置守卫）、`#120`（错误分类）
- 接缝需求（最小接缝，已落地）：`PlayerSurface` trait（`play_file`/`play_url`/`pause`/`resume`/`stop`/`seek`/`set_volume`/`volume`/`state`/`position`/`duration`/`error_message`/`error_kind`），生产实现为 `AudioEngine`，测试注入记录调用序列的 Fake（断言 #51 的"先 stop 后 play"顺序）
- 测试途径：`cargo test --test coordinator_behavior`；FFI 往返经 `rhythm_coordinator_*` 直调（结构化结果 JSON 形状）
- 契约：所有操作一次返回结构化结果（`CoordinatorResult`：ok + 分类错误 + current_track），无全局错误槽

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| CO-01 | `start` 本地曲目 | 顺序：`player.stop()` 先于 `play_file(path)`（#51）→ 队列建立并定位到当前曲目（可切下一首/上一首正确）；`current_track` 置位；结果 ok | FakePlayer 调用记录 + 真队列 |
| CO-02 | `start` URL 曲目 | 非 local 来源 → `play_url(source_url)`（不调 play_file） | FakePlayer |
| CO-03 | `start` 无播放位置（#78/#81 守卫） | 缺 `file_path`/`source_url` → 分类错误 `no_playable_location`；不碰 player；不进入播放态（current_track 保持 nil、队列不建立） | FakePlayer |
| CO-04 | `start` 空字符串位置 | 空串 file_path/source_url 视为缺失 → 同上分类错误 | FakePlayer |
| CO-05 | `start` recordPlay 落库 | 带真实库（临时 SQLite）→ `record_play(id)` 使 DB play_count 递增 | 真 Library |
| CO-06 | `start` 队列定位与模式 | 队列来自 queue_tracks 并 `jump_to` 目标 id；Sequential 末位无 next、ListLoop 恒有 next | 真队列 |
| CO-07 | `start` 单曲队列 | 无 next 无 previous | 真队列 |
| CO-08 | `start` 引擎即时失败 | `play_file` 返回 Err → 分类错误 `playback_failed`（stop 仍已执行） | FakePlayer 注入失败 |
| CO-09 | `next` | 队列有下一首：`stop()` 先于分派、`current_track` 更新、结果带新 current | FakePlayer + 真队列 |
| CO-10 | `next` 有界跳过（#78） | 跳过无位置曲目落到下一个可播；全死队列放弃且当前曲继续（结果 current 不变、无引擎调用） | FakePlayer |
| CO-11 | `next` 队列耗尽 | Sequential 末位 → 结果 ok 且 current 不变（no-op） | FakePlayer |
| CO-12 | `previous` | 对称于 next：回退、跳过、头部不再回退（Sequential 头部 previous 返回当前曲目并重放——与旧队列语义一致） | FakePlayer |
| CO-13 | `next`/`previous` 无队列 | 结果 ok 且 current 为空（no-op） | FakePlayer |
| CO-14 | `sync_queue`（#69） | 刷新后 replace + jump_to 当前曲目 id；当前曲被删 → 游标落在新队列头 | FakePlayer + 真队列 |
| CO-15 | `stop` | 引擎 stop；current_track/队列清空；无 next/previous | FakePlayer |
| CO-16 | `set_play_mode` | 模式存储并同步到队列（ListLoop 下末位仍有 next） | 真队列 |
| CO-17 | FFI 结构化结果 | `rhythm_coordinator_start` 无位置曲目 → `{"ok":false,"error_kind":"no_playable_location",...}`；null 句柄/坏 JSON → `invalid_input`；`has_next`/`has_previous`/`current_track`/`get_play_mode`/`get_state` 空态往返 | FFI 直调 |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| CO-18 | 单曲循环模式 next | SingleLoop 下 `next` 返回当前曲目（重放） | FakePlayer |
| CO-19 | sync_queue 坏 JSON（FFI） | 安全 no-op，句柄仍可用 | FFI 直调 |
| CO-20 | 传输可用性导出 | `can_play_next`/`can_play_previous` 与队列游标一致（含模式矩阵） | 真队列 |

## 错误路径（P2）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| CO-21 | 起播失败分类文案来源 | `error_kind` 分类（no_playable_location/playback_failed/invalid_input）由结果一次携带，UI 只做文案映射 | 结构断言 |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| — | 无（守卫 #78/#81、跳过规则、先停后播均由本清单覆盖） | — | — |
