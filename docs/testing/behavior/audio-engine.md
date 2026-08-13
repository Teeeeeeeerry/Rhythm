# AudioEngine 行为清单

- 模块：`rust-core/src/audio/mod.rs`（`AudioEngine`、`drive_playback`/`spawn_playback`、`run_playback_loop`、`stream_hint`；测试接缝 `Decoder`/`Sink`、`run_playback_with`、`play_file_with`/`play_url_with`、`new_with_resolver`、`open_resolved_stream`）
- 历史回归：`#35`（Finished 未落地）、`#51`+`#52`（双流叠加）、`#23`（失败状态不可见）、`#28`（尾帧被截）
- 接缝需求（最小接缝）：`run_playback_loop` 目前依赖具体 `AudioDecoder`/`AudioOutput`/`Resampler`。将解码、输出抽象为 trait（`Decoder`/`Sink` 或泛型参数），测试注入假解码器（预制包流、可控包数）与假输出（内存收集），循环逻辑从线程中解耦（`run_playback_loop` 在测试线程直调）。公共 API 的"快速失败"路径（文件不存在等）无需接缝。
- 测试途径：Rust `cargo test`（unit + 集成）。

## 主路径（P0 — 该模块票的合并门槛）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| AE-01 | 引擎初始状态 | `state()==Stopped`、`volume()==1.0`、`duration()==0.0`、`position()==0.0` | 直测，无接缝 |
| AE-02 | `play_file` 不存在的文件 | 返回 `FileNotFound`；状态保持 Stopped；无线程产生 | 直测，无接缝 |
| AE-03 | `play_file` 成功启动播放 | `state()` 可见 `Playing` 且状态回调收到 `Playing`；`duration()` 等于解码器时长 | 接缝 + 真 WAV 文件 |
| AE-04 | `play_url` 先落 Buffering（#23） | resolve/连接/预缓冲期间 `state()` 可见 `Buffering`（回调与状态双写） | 接缝 + stub resolver |
| AE-05 | `play_url` 成功后 Playing | `state()==Playing`；解码器无时长时 `duration()` 回退用 `resolved.duration`（DASH 流兜底） | 接缝 + stub resolver |
| AE-06 | `pause`：Playing → Paused | 状态与回调都可见 `Paused` | 接缝 |
| AE-07 | `resume`：Paused → Playing | 状态与回调都可见 `Playing` | 接缝 |
| AE-08 | `stop` | 状态与回调落 `Stopped`；`current_source`/`desired_position` 清空；旧线程因 generation 失配退出，不再产出、不重复发状态 | 接缝 |
| AE-09 | 自然播完（#35） | 流耗尽 → `output.drain()` 后状态与回调落 `Finished` | 接缝（假解码器返回 `Ok(None)`） |
| AE-10 | `seek` 合法值 | 排队到 `desired_position`；播线程消费后 `position()` 更新、进度回调收到新位置 | 接缝 |
| AE-11 | `seek` 负数 | 返回 `InvalidInput`；状态不变 | 直测，无接缝 |
| AE-12 | `seek` 超过 duration（duration>0） | 返回 `InvalidInput` | 直测（预置 duration 后），无接缝 |
| AE-13 | `set_volume` 钳制 | `set_volume(-0.5)` → `volume()==0.0`；`set_volume(1.5)` → `1.0` | 直测，无接缝 |
| AE-14 | 播放中再次 `play_file`/`play_url`（#51/#52） | 旧线程退出：旧流的解码/输出停止，新流独占输出，无双流叠加 | 接缝（可控节奏的假解码器） |
| AE-15 | 播放失败落 Error（#23） | 任一阶段出错 → `state()` 可见 `Error(message)` 且回调收到；不表现为 idle 0:00 | 接缝 + 可控失败注入 |
| AE-16 | `stream_hint` 扩展名映射 | mp3→mp3；m4a/mp4/mov/m4s→m4a；aac/flac/wav/aiff/aif/ogg/opus 各自正确；未知扩展→None；带 query 的 URL 取 path 部分；大小写不敏感 | 直测纯函数 |
| AE-17 | 进度回调 | 播放中收到 `(pos, dur)` 且 pos 递增 | 接缝 |

## 边界情况（P1 — 同波次内完成）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| AE-18 | `pause` 在非 Playing 状态调用 | 无效果：状态不变、回调不发 | 直测，无接缝 |
| AE-19 | `resume` 在非 Paused 状态调用 | 无效果：状态不变、回调不发 | 直测，无接缝 |
| AE-20 | `duration()==0` 时 `seek` | 上限校验跳过，任意正数接受（DASH 流场景） | 直测，无接缝 |
| AE-21 | 暂停期间的 `seek` | 期望：暂停中 seek 立即生效——消费 `desired_position`、更新 `position`、发进度回调；resume 后从新位置继续。**红测禁用**：现状代码挂起 seek 直到 resume，测试禁用挂 issue #77 | 接缝 |
| AE-22 | 未注册回调 | 播放/暂停/失败不崩溃，回调 emit 为 no-op | 直测 + 接缝 |
| AE-23 | 暂停期间不产出音频 | Paused 后假输出不再收到写入，resume 后继续 | 接缝 |
| AE-24 | `stop` 清空 pending seek | `seek` 排队后 `stop`，resume 时旧 seek 不被应用 | 接缝 |
| AE-25 | 音量在循环中生效 | `volume<1.0` 时假输出收到的样本按比例衰减 | 接缝 |

## 错误路径（P2 — 仅断言"错误被正确上报"，可顺延）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| AE-26 | 解码器打开失败 | `state()==Error(消息非空)` + 回调收到 Error | 接缝（坏文件或假解码器报错） |
| AE-27 | 输出设备初始化失败 | 同 AE-26 | 接缝（假输出构造报错） |
| AE-28 | URL 解析失败 | 同 AE-26（resolve 返回 Err 的场景） | 接缝 + stub resolver 报错 |
| AE-29 | HTTP 流打开/缓冲失败 | 同 AE-26 | 接缝 + stub 流报错 |
| AE-30 | 循环中解码错误 | `next_packet` 返回 Err → 循环终止，同 AE-26 | 接缝（假解码器中途报错） |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| AE-21 | 暂停中 seek 被挂起，直到 resume 后才生效 | [#77](https://github.com/Teeeeeeerry/Rhythm/issues/77) | 已挂接（Wave 1，`ae21_seek_while_paused_applies_immediately` 以 `#[ignore]` 禁用，修复 #77 时解禁） |

## Decoder / HttpStream（已有测试行为对照）

| 编号 | 行为（已有测试） | 出处 |
|---|---|---|
| AE-31 | 本地 WAV 全量解码：采样率/声道/时长正确、帧数完整、产出真实样本 | `streaming.rs::test_decoder_decodes_local_wav` |
| AE-32 | 本地 WAV seek 0.5s 后 position 接近 0.5 且继续产出 | `streaming.rs::test_decoder_seek_local_wav` |
| AE-33 | HttpStream 全量下载字节一致、`byte_len`/`is_seekable` 正确 | `test_http_stream_downloads_all_bytes` |
| AE-34 | 反向 seek 触发 Range 请求且数据一致 | `test_http_stream_seek_backwards_issues_range` |
| AE-35 | 缓冲窗口内正向 seek 无需网络往返 | `test_http_stream_forward_seek_within_buffer` |
| AE-36 | 未显式等初始缓冲时读取仍返回完整数据 | `test_http_stream_blocks_until_initial_buffer` |
| AE-37 | 绝对 seek 后紧跟相对 seek 位置不漂移（#23） | `test_http_stream_relative_seek_after_absolute_seek` |
