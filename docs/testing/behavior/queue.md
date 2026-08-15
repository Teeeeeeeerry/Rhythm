# PlayQueue 行为清单

- 模块：`rust-core/src/queue/mod.rs`（播放队列：四种模式 + 游标 + 替换）
- 历史回归：`#66`+`#67`（ForEach ID 碰撞）、`#69`+`#72`（refreshLibrary 队列同步）——本模块被 AppState 依赖，队列语义错误会放大到 UI。
- 测试途径：`cargo test` 单元测试；纯逻辑无 IO，无需接缝。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| Q-01 | Sequential 顺序播放 | 新建队列 current=第一首；next 依次推进；末尾 next → None（耗尽） | 已有 `test_sequential_plays_in_order` |
| Q-02 | ListLoop 回绕 | 末尾 next → 回到第一首 | 已有 `test_list_loop_wraps` |
| Q-03 | SingleLoop 重复 | next 恒返回当前曲 | 已有 `test_single_loop_repeats` |
| Q-04 | Sequential previous | 后退一格；已在开头时 previous 返回当前（游标不动） | 已有 `test_previous_in_sequential` |
| Q-05 | `jump_to` | 命中 id → true 且 current 变为目标；不存在 id → false 且游标不动 | 已有 `test_jump_to` + `test_jump_to_missing_id_leaves_cursor` |
| Q-06 | Shuffle 一轮全覆盖 | 一轮内 20 曲全部出现且不重复 | 已有 `test_shuffle_covers_all` |
| Q-07 | `replace`（#72 依赖） | 替换列表后游标重置到 0、顺序列表重建、current=新首曲 | 已有 `test_replace_resets_cursor` |
| Q-08 | `has_next` 各模式矩阵 | Sequential：末曲 false；ListLoop/Shuffle：非空恒 true；SingleLoop：非空恒 true；空队列 false | 已有 `test_has_next_matrix` |
| Q-09 | `has_previous` 各模式矩阵 | Sequential：开头 false、否则 true；其他模式：非空恒 true | 已有 `test_has_previous_matrix` |
| Q-10 | 空队列 | `new([])`：current/next/previous 均 None；has_next/has_previous false | 已有 `test_empty_queue` |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 状态 |
|---|---|---|---|
| Q-11 | 非 Sequential 模式 previous 回绕 | ListLoop/Shuffle/SingleLoop 在开头 previous → 跳到当前播放序末尾曲（Shuffle 为 shuffle 序末位） | 已有 `test_previous_wraps_in_non_sequential` |
| Q-12 | Sequential 耗尽后 previous | next 耗尽（游标=len）后 previous 返回最后一首 | 已有 `test_previous_after_exhaustion` |
| Q-13 | Shuffle 第二轮重洗 | 一轮耗尽后 shuffle_order 重建，第二轮仍全覆盖 | 已有 `test_shuffle_second_round_reshuffles` |
| Q-14 | `PlayMode::from_i32` 非法值 | 回退 Sequential | 已有 `test_play_mode_from_i32` |
| Q-15 | `jump_to` 在 Shuffle 模式 | 命中后 current 正确（按 shuffle 序定位游标） | 已有 `test_jump_to_in_shuffle` |

## 错误路径（P2）

（纯内存结构，无外部依赖，不设错误路径。）

## 红测登记

（暂空。实现时若发现现状代码与清单不符，测试照写、禁用并挂 issue 编号，在此登记。）
