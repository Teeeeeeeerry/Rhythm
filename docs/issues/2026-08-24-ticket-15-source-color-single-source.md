# Ticket: 15 — 来源徽标色并入单一来源

- **日期**: 2026-08-24
- **来源**: 父 issue「L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案」（#167）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #184）
- **涉及平台**: 双端

---

## Parent
L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案（#167）

## What to build
来源徽标色（dark/light 双端值）并入 palette.json 机制，生成 macOS 主题 token 与 Windows 色表映射；L0 校验脚本覆盖。

## Acceptance criteria
- [ ] 改色只改单一来源
- [ ] 双端生成物一致，校验脚本通过

## Blocked by
- Ticket 文案键表与 macOS L10n 生成（#182）
