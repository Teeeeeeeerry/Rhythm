# Ticket: 02 — macOS 传输操作迁移（toggle/next/previous/stop 与跳过规则）

- **日期**: 2026-08-24
- **来源**: 父 issue「播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块」（#165）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #171）
- **涉及平台**: 双端 + Rust 核心

---

## Parent
播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块（#165）

## What to build
暂停/恢复（仅 Paused 可恢复）、停止、下一首/上一首（有界跳过不可播曲目）与传输可用性判断（可切下一首/上一首/可暂停/可停止）全部由协调器驱动；macOS UI 改调新 interface，删除 AppState 内对应编排代码。

## Acceptance criteria
- [ ] macOS 上 toggle/next/previous/stop 行为与旧版一致，AS 相关条目测试全绿
- [ ] 有界跳过规则只存在于协调器
- [ ] 可用性判断由协调器导出，UI 直接消费

## Blocked by
- Ticket 协调器启动路径落地（core + FFI + macOS 接入）（#170）
