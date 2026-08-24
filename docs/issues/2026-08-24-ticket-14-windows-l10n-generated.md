# Ticket: 14 — Windows L10n 生成（保留平台差异）

- **日期**: 2026-08-24
- **来源**: 父 issue「L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案」（#167）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #183）
- **涉及平台**: 双端

---

## Parent
L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案（#167）

## What to build
Windows 文案实现改由同一键表生成，保留系统语言检测与注册表覆盖差异；固定 locale 断言测试全绿。

## Acceptance criteria
- [ ] Windows 与 macOS 同键表，漂移不可能
- [ ] 固定 locale 的 L10n 测试全绿
- [ ] 平台差异字段生效

## Blocked by
- Ticket 文案键表与 macOS L10n 生成（#182）
