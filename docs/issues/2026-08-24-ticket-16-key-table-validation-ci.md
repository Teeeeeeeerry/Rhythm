# Ticket: 16 — 键表校验与 run-all 收口

- **日期**: 2026-08-24
- **来源**: 父 issue「L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案」（#167）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #185）
- **涉及平台**: 双端

---

## Parent
L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案（#167）

## What to build
键表校验脚本（键缺失、双端漂移、生成物一致性）纳入 run-all.sh，任一红即非零退出。

## Acceptance criteria
- [ ] 人为引入漂移被校验拦截
- [ ] run-all 汇总各步成败，任一红非零退出

## Blocked by
- Ticket Windows L10n 生成（保留平台差异）（#183）
- Ticket 来源徽标色并入单一来源（#184）
