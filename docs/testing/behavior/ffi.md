# FFI 层行为清单

- 模块：`rust-core/src/ffi/mod.rs`（C ABI 导出：opaque 句柄 + JSON 交换 + 空指针防御 + 错误码契约）
- 历史回归：`#21`（解析失败只有 null、无原因）
- 测试途径：`cargo test` 集成测试（现有 `player_ffi.rs` 模式扩展）；library/queue/resolver 部分链接真实现 + 临时库；无需接缝。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 |
|---|---|---|
| FF-01 | library open/close 往返 | 合法路径 → 非空句柄；close 后不泄漏崩溃；close(null) 安全 |
| FF-02 | library open 失败 | 不可写路径 → null |
| FF-03 | `rhythm_library_import` | 成功返回导入数；错误/空指针 → -1 |
| FF-04 | `rhythm_library_import_file` 返回值契约 | 成功 1、不支持格式 0、错误 -1（#79 修复后启用） |
| FF-05 | 字符串内存契约 | `get_all_tracks` 等返回 JSON 需 `rhythm_free_string` 释放；空指针入参 → null/-1 安全默认 |
| FF-06 | `rhythm_library_add_track` JSON 往返 | 合法 JSON → 带 DB id 的 JSON；非法 JSON → null |
| FF-07 | player create/destroy | create 非空；destroy 停播并释放；destroy(null) 安全 |
| FF-08 | player 状态码映射 | 0–5 对应 Stopped/Playing/Paused/Buffering/Error/Finished；空指针 -1（Buffering(3) 仅 `play_url` 网络流可触发，无网络依赖的测试不可达，未断言——其余 0/1/2/4/5/-1 全绿） |
| FF-09 | `rhythm_player_error` | Error 状态返回消息字符串；其他状态 null |
| FF-10 | `rhythm_player_seek` | 空指针 -1；负数/超范围 -1；合法 0 |
| FF-11 | queue 全链路 | create/current/next/previous/jump_to/set_mode/has_next/has_previous 经 FFI 往返行为正确 |
| FF-12 | resolve 与 last_error（#21） | 成功 → JSON 且 last_error 清空；失败 → null 且 `rhythm_last_error` 返回 `{kind, message}`；成功后再读 → null |
| FF-13 | `rhythm_classify_url` | 返回 "youtube"/"bilibili"/"direct_url" 字符串；失败 → null + last_error |
| FF-14 | M3U8 FFI | export 成功 0/失败 -1；import 成功 JSON/失败 null |
| FF-15 | `rhythm_free_string` | 空指针安全 |
| FF-16 | metadata FFI | `rhythm_metadata_extract`/`scan`/`extract_artwork`：成功 JSON/路径，失败 null |

## 边界情况（P1）

| 编号 | 行为 | 断言 |
|---|---|---|
| FF-17 | 空字符串路径入参 | 不崩溃（现状：open("") 触发 SQLite 临时库语义 → 非空句柄可正常 close，`ff17_empty_path_inputs_do_not_crash` 锁定"不崩溃"为断言核心） |
| FF-18 | `rhythm_queue_create` 非法 JSON | null |
| FF-19 | `rhythm_queue_replace` 非法 JSON | 原队列保持不变 |
| FF-20 | 各错误码函数空指针 | 全部返回安全默认（-1/0/null/false） |

## 错误路径（P2）

| 编号 | 行为 | 断言 |
|---|---|---|
| FF-21 | 大 JSON 往返（数百曲目） | 无截断、无 panic，编解码一致（已随 Wave 3b 覆盖 `ff21_large_json_roundtrip`，无需顺延） |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| FF-04 | `rhythm_library_import_file` 从不返回 0：不支持格式映射为 -1，Swift 侧"不支持的音频格式"分支死代码 | [#79](https://github.com/Teeeeeeerry/Rhythm/issues/79) | 已修复（#108） |
