# Ticket: 13 — 文案键表与 macOS L10n 生成

- **日期**: 2026-08-24
- **来源**: 父 issue「L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案」（#167）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #182）
- **涉及平台**: 双端

---

## Parent
L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案（#167）

## What to build
文案键表（中英文案 + 平台差异字段）成为单一事实来源；macOS L10n 实现改由键表生成，行为不变。

## Acceptance criteria
- [ ] macOS 构建产物来自键表
- [ ] 现有 macOS 文案行为测试全绿
- [ ] 新增文案只需改键表

## Blocked by
None — can start immediately
