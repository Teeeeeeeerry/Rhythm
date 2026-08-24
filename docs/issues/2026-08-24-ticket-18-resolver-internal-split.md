# Ticket: 18 — resolver 内部职责拆分（prefactor）

- **日期**: 2026-08-24
- **来源**: 父 issue「播放期重解析策略收敛为单一入口——#120 修复面从 8 文件 3 语言收拢」（#168）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #187）
- **涉及平台**: Rust 核心

---

## Parent
播放期重解析策略收敛为单一入口——#120 修复面从 8 文件 3 语言收拢（#168）

## What to build
解析模块内部按职责拆分子模块：yt-dlp 发现与安装、stderr 分类、URL 分类、缓存；对外 interface 不变，纯重构。

## Acceptance criteria
- [ ] 拆分后现有 resolver 测试全绿
- [ ] 对外 interface 不变

## Blocked by
None — can start immediately
