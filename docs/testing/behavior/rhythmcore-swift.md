# RhythmCore（Swift 封装层）行为清单

- 模块：`macos/Rhythm/Models/RhythmCore.swift`（FFI 包装类、`PlayMode`、`Track` 编解码、resolver 分派与辅助函数）
- 历史回归：无直接记录（本层零行为测试是当前缺口本身）
- 测试途径：XCTest；链接真 rust-core 静态库（现有模式）；无需接缝（薄包装 + 纯函数）。

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 |
|---|---|---|
| SW-01 | `encodeJSON`/`decodeJSON` snake_case 往返 | `Track`/`Playlist` 各字段经 Rust core 往返保真（`source_type` 等转换正确） |
| SW-02 | `decodeJSON` 非法输入 | 返回 nil，不崩溃 |
| SW-03 | `PlayMode.next()` | 0→1→2→3→0 循环 |
| SW-04 | `ResolverStatus.isQuiet` | idle/ready → true；其余 phase → false |
| SW-05 | `resolveURL` 成功分派 | core 返回 JSON → `.success(ResolvedInfo)`（snake_case 解码） |
| SW-06 | `resolveURL` 失败回退 | core 返回 null → `.failure(lastResolveError() ?? .unknown)` |
| SW-07 | `resolveURL` malformed 响应 | 非空但解码失败 → `.failure(kind: "internal")`（现状：`ResolvedUrl` 多余键被解码器忽略，分支防御性不可达；`testSW07_MalformedResponseBranchIsDefensive` 锁定可观察分派） |
| SW-08 | `RhythmLibrary` 打开失败 | `init?` 返回 nil；ptr 为 nil 时各方法返回安全默认（-1/[]/false/nil）（ptr-nil 分支现状不可达：`init?` 失败即 nil，不会交出实例——与 SW-09 同类的防御分支） |
| SW-09 | `RhythmPlayer` 空指针防御 | ptr 为 nil 时 `state == -1`、`position/duration == 0` 等 |
| SW-10 | `RhythmQueue` 空曲目列表 | `queue_create("[]")` 合法 → 句柄非空；`current() == nil` |
| SW-11 | `RhythmLibrary.addTrack` | 成功返回带 DB id 的 `Track`；core 返回 null → nil（core-null 分支现状不可达：模型均为纯 Codable，`encodeJSON` 恒产出合法 JSON——与 SW-07/SW-13 同类的防御分支） |

## 边界情况（P1）

| 编号 | 行为 | 断言 |
|---|---|---|
| SW-12 | `Track` 可选字段缺省 | 缺失字段解码为 nil，不失败 |
| SW-13 | `encodeJSON` 编码失败兜底 | 返回 `"[]"` 而非崩溃（现状防御：模型均为纯 Codable，失败分支不可达；`testSW13_EncodeJSONWellFormedValuesAlwaysEncode` 锁定） |
| SW-14 | `RhythmLibrary.removeTrack` 不存在的 id | core 返回 -1 → false（现状返回 true → #98，红测 `testSW14_RemoveTrackMissingIdReturnsFalse` 条件 XCTSkip 禁用中） |

## 错误路径（P2）

| 编号 | 行为 | 断言 |
|---|---|---|
| SW-15 | core 返回畸形 JSON 的各类入口 | 全部走 nil 回退，不崩溃 |

## 红测登记

| 编号 | 缺陷 | 测试 | 状态 |
|---|---|---|---|
| SW-14 | `remove_track` 不检查受影响行数，0 行 DELETE 仍返回成功 → Swift `removeTrack(999) == true` | `testSW14_RemoveTrackMissingIdReturnsFalse`（条件 XCTSkip） | #98 待修 |

## 错误路径状态

SW-15（core 畸形 JSON 入口）与 SW-07/SW-13 同类：防御分支在公共 API 不可达，
无法黑盒触发；成功/失败分派已由 SW-05/SW-06/SW-11 锁定，无需顺延。
