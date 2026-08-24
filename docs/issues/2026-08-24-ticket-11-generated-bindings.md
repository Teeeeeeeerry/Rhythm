# Ticket: 11 — 单一契约声明与双端绑定生成（expand-contract）

- **日期**: 2026-08-24
- **来源**: 父 issue「FFI 层 52 个直通导出加深为结构化契约——错误随结果返回、状态走事件、绑定生成」（#166）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #180）
- **涉及平台**: 双端 + Rust 核心

---

## Parent
FFI 层 52 个直通导出加深为结构化契约——错误随结果返回、状态走事件、绑定生成（#166）

## What to build
schema 声明一次，生成 Swift 与 C++ 编解码绑定。先与手写绑定共存（expand），按模块迁移调用方（migrate，每批保持绿），最后删除手写绑定（contract）。

## Acceptance criteria
- [ ] 生成绑定与手写绑定产物对比一致
- [ ] 每批迁移后构建与测试全绿
- [ ] 手写编解码删除

## Blocked by
- Ticket M3U8 条目具名结构（#177）
- Ticket 状态事件通道（替代轮询）（#178）
- Ticket 状态与模式枚举具名化（#179）
