# Ticket: 19 — resolve-for-playback 策略入口

- **日期**: 2026-08-24
- **来源**: 父 issue「播放期重解析策略收敛为单一入口——#120 修复面从 8 文件 3 语言收拢」（#168）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #188）
- **涉及平台**: Rust 核心

---

## Parent
播放期重解析策略收敛为单一入口——#120 修复面从 8 文件 3 语言收拢（#168）

## What to build
核心新增「播放期解析」入口：缓存命中、失效淘汰、新鲜重试一次、错误分类一次完成；策略级测试覆盖。

## Acceptance criteria
- [ ] 策略级测试：命中/淘汰/重试一次仍败/分类
- [ ] 默认行为与现状一致（重试一次、TTL 1 小时）

## Blocked by
- Ticket resolver 内部职责拆分（prefactor）（#187）
