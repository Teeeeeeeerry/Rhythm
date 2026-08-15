# Resampler 行为清单

- 模块：`rust-core/src/audio/resampler.rs`（线性插值重采样 + 声道映射 + 跨块连续性）
- 历史回归：`#28`（跨块后 src_pos 未回卷导致静音）、`#35`（尾帧相关）
- 测试途径：`cargo test` 单元测试；纯逻辑无 IO，无需接缝。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| RM-01 | 恒等直通 | 同率同声道：`is_identity()` true、样本 1:1 拷贝 | 已有 `test_identity_passthrough` |
| RM-02 | 单声道→立体声 | 单声道样本复制到两声道 | 已有 `test_mono_to_stereo` |
| RM-03 | 降采样 | 48k→24k 半速，输出帧数按比例 | 已有 `test_half_speed_downsample` |
| RM-04 | 线性插值 | 48k→96k 中间帧取均值；末尾钳制到末帧 | 已有 `test_interpolation` |
| RM-05 | 跨块连续性（#28） | 44.1k→48k 多块输出总数 ≈ 理论值（无每块返 0） | 已有 `multi_block_44k_to_48k_continuous_output` |
| RM-06 | 恒等跨块连续 | 恒等多块每块输出完整帧数 | 已有 `multi_block_identity_continuous_output` |
| RM-07 | `reset`（seek 场景） | 重置后 phase 清零，输出与全新实例一致 | 已有 `reset_matches_fresh_instance` |
| RM-08 | 空输入/空输出 | 返回 0 帧 | 已有 `empty_input_or_output_yields_zero` |
| RM-09 | 声道映射 | 输入声道少于输出（如立体声→4 声道）多余声道静音；输入多于输出取前 N 平面 | 已有 `channel_mapping_extra_and_missing_planes` |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| RM-10 | 输出缓冲不足一帧 | 返回 0 帧，输入不丢失（src_pos 不推进） | 已有 `short_output_buffer_consumes_nothing` |
| RM-11 | 输入不足一帧（长度 < 声道数） | 返回 0 帧 | 已有 `short_input_yields_zero` |
| RM-12 | 极端速率比 | 超大升采样（如 8k→192k）不越界、无 NaN | 已有 `extreme_upsampling_no_nan` |

## 错误路径（P2）

（纯内存结构，不设错误路径。）

## 红测登记

（暂空。实现时若发现现状代码与清单不符，测试照写、禁用并挂 issue 编号，在此登记。）
