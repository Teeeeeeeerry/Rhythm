# AudioOutput 行为清单

- 模块：`rust-core/src/audio/output.rs`（cpal 输出封装 + `PcmPump` 块打包/静音填充/排空）
- 历史回归：`#23`（大块被截断 + 位置推进失真）、`#28`（尾帧被截）
- 测试途径：`PcmPump` 为纯逻辑，`cargo test` 直测（现有模式）；`AudioOutput` 的 `new()`/`write()`/`drain()` 依赖真实音频设备，无法单测——该部分行为经 Wave 1 接缝（假输出）在 AudioEngine 层间接锁定，此处登记为"设备依赖、不可直测"。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| AO-01 | 大块跨回调完整 | 2230 样本经 512 样本回调全量输出不丢 | 已有 `block_larger_than_callback_survives_intact` |
| AO-02 | 小块打包 | 多个小块在单次回调内连续打包 | 已有 `blocks_smaller_than_callback_are_packed` |
| AO-03 | 无公因数尺寸零丢失 | 377 样本块 × 149 样本回调，字节级一致 | 已有 `ragged_sizes_lose_nothing` |
| AO-04 | 饥饿补静音 | 通道耗尽时输出尾部补零（不残留脏数据） | 已有 `starved_channel_fills_silence` |
| AO-05 | 断开排空剩余 | 发送端断开后剩余块仍完整输出 | 已有 `disconnect_drains_remaining_block` |
| AO-06 | 首个样本等待语义 | 空缓冲时 `fill` 等待至多 timeout 拉首个块；一旦产出过样本，后续饥饿立即返回 | 已有 `first_sample_waits_but_starvation_is_instant` |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| AO-07 | 恰好等于缓冲大小的块 | 单次回调整块输出，游标归零 | 已有 `exact_size_block_fills_in_one_call` |
| AO-08 | 空块入队 | 空 `Vec` 块不产出、不 panic | 已有 `empty_blocks_are_skipped` |

## 错误路径（P2）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| AO-09 | 无输出设备 | `AudioOutput::new()` → `Output("No output device found")` | 设备依赖，不可直测（登记） |
| AO-10 | 不支持采样格式 | `new()` → `Output` 错误 | 设备依赖，不可直测（登记） |
| AO-11 | 通道关闭后 `write` | → `Output("Audio output channel closed")` | 经 Wave 1 接缝的假输出锁定（登记） |

## 红测登记

（暂空。实现时若发现现状代码与清单不符，测试照写、禁用并挂 issue 编号，在此登记。）
