# Ticket: 05 — 队列同步与可用性判断收口（双端）

- **日期**: 2026-08-24
- **来源**: 父 issue「播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块」（#165）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #174）
- **涉及平台**: 双端 + Rust 核心

---

## Parent
播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块（#165）

## What to build
资料库刷新后的队列同步（替换队列并跳到当前曲目）与传输可用性判断从双端 AppState 移入协调器；双端删除各自实现。

## Acceptance criteria
- [ ] 双端导入/删除/刷新后队列与当前曲目状态一致
- [ ] 双端可用性判断均来自协调器
- [ ] 双端 AppState 不再含队列同步代码

## Blocked by
- Ticket macOS 传输操作迁移（toggle/next/previous/stop 与跳过规则）（#171）
- Ticket Windows 迁移与 Player 最小接缝（含 M3U8 no-op 修复）（#173）
