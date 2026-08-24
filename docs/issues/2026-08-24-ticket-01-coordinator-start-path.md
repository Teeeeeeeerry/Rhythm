# Ticket: 01 — 协调器启动路径落地（core + FFI + macOS 接入）

- **日期**: 2026-08-24
- **来源**: 父 issue「播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块」（#165）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #170）
- **涉及平台**: 双端 + Rust 核心

---

## Parent
播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块（#165）

## What to build
核心新增播放协调器模块，interface 提供「以曲目与队列启动播放」：停止旧播放、按来源类型分发、登记播放、建立队列并定位当前曲目。macOS 的双击与 URL 播放改走协调器，现有起播守卫（无播放位置不进入播放态、先停旧播放）行为不回归；Windows 暂走旧路径，双轨共存。

## Acceptance criteria
- [ ] 协调器启动接口经 FFI 暴露，返回结构化结果
- [ ] macOS 双击曲目与 URL 播放经协调器起播，现有 macOS 编排测试全绿
- [ ] 无播放位置的曲目不进入播放态（守卫迁移）
- [ ] 队列在启动时建立并定位到当前曲目

## Blocked by
None — can start immediately
