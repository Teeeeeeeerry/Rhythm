# Ticket: 06 — 编排行为清单合并与旧导出清理

- **日期**: 2026-08-24
- **来源**: 父 issue「播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块」（#165）的 tracer-bullet 切片
- **状态**: 已合入（GitHub issue #175）
- **涉及平台**: 双端 + Rust 核心

---

## Parent
播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块（#165）

## What to build
appstate-macos 与 windows-appstate 行为清单的编排条目合并为一份协调器清单；旧编排 FFI 导出删除；红测登记核对。

## Acceptance criteria
- [ ] 一份编排行为清单覆盖双端
- [ ] 旧编排导出删除后双端构建与测试全绿
- [ ] 清单每项有自动化测试

## Blocked by
- Ticket 协调器事件驱动（Finished 自动切歌与错误事件）（#172）
- Ticket 队列同步与可用性判断收口（双端）（#174）

## Coordination
旧导出删除与「旧导出收缩」组（#166 组）协调，避免重复清理
