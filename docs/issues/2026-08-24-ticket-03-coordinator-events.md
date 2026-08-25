# Ticket: 03 — 协调器事件驱动（Finished 自动切歌与错误事件）

- **日期**: 2026-08-24
- **来源**: 父 issue「播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块」（#165）的 tracer-bullet 切片
- **状态**: 已合入（GitHub issue #172）
- **涉及平台**: 双端 + Rust 核心

---

## Parent
播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块（#165）

## What to build
协调器发布「播完、错误、进度、状态变化」事件；macOS 订阅事件替代进度轮询驱动，播完自动切歌与失败分类提示改由事件触发。

## Acceptance criteria
- [ ] macOS 播完自动切歌由事件驱动，轮询驱动代码删除
- [ ] 失败提示基于事件携带的结构化分类
- [ ] 暂停期间 UI 状态与引擎一致（无在途残留）

## Blocked by
- Ticket macOS 传输操作迁移（toggle/next/previous/stop 与跳过规则）（#171）

## Coordination
错误分类的契约形状与「结构化结果契约」组（#166 组）对齐
