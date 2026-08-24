# Issue: 播放期重解析策略收敛为单一入口——#120 修复面从 8 文件 3 语言收拢

- **日期**: 2026-08-24
- **来源**: 架构审查（architecture-review-20260825-094221.html 候选 4）
- **状态**: 已立项（GitHub issue #168）
- **涉及平台**: Rust 核心（双端 UI 消费其错误分类）

---

## Problem Statement

「播放遇到坏 CDN 链接怎么办」的策略散落在多个模块：缓存 TTL 与淘汰在 resolver，绕过缓存重试在音频引擎，错误分类横跨错误类型定义、导出层与双端文案层。一次修复（#120，YouTube 403 误报）横跨 8 个文件 3 种语言。resolver 模块本身 1783 行，同时装下 yt-dlp 发现、安装、stderr 分类、URL 分类与缓存，内部职责没有边界，缓存策略与解析流程交错。

## Solution

在核心内新增「播放期解析」具名策略入口：命中缓存、淘汰并重试一次、错误分类一次完成；音频引擎只调用该入口。resolver 内部按职责拆分子模块（内部 seam，对外 interface 不变）。

## User Stories

1. As a 维护者, I want 缓存命中/淘汰/重试规则在同一入口, so that #120 这类修复只落一处
2. As a 维护者, I want 错误分类由该入口统一产出, so that 两端不再各自映射
3. As a 维护者, I want resolver 内部职责有边界, so that 1783 行大模块可独立演进
4. As a 测试者, I want 策略级测试覆盖「命中/淘汰/重试一次」, so that 回归测试聚焦策略本身
5. As a 维护者, I want 新错误分类只改一处, so that 文案映射同步更新不再散落
6. As a 维护者, I want 重试次数与缓存 TTL 可配置, so that 策略调整不动调用方

## Implementation Decisions

- 新增播放期解析入口（resolve-for-playback）：入参页面 URL，出参可播放信息 + 分类错误；内部完成缓存命中、淘汰、新鲜重试一次
- 音频引擎的恢复逻辑改为调用该入口，不再自行组合原子操作
- resolver 内部拆分：yt-dlp 发现与安装、stderr 分类、URL 分类、缓存为独立子模块；对外 interface（解析、分类、诊断）保持不变
- 错误分类结果（expired / cdn_rejected / other）由该入口统一产出，双端只做文案映射
- 重试次数与缓存 TTL 作为配置项，默认值与现状一致（重试一次、TTL 1 小时）

## Testing Decisions

- 测试面 = 播放期解析入口（策略级）：缓存命中、失效淘汰、重试一次仍败、错误分类
- 先例：现有 resolver 行为测试（端到端 stub、路径失败、流式）与 #120 回归测试
- 内部子模块沿用各自现有测试，不新增跨内部 seam 的测试
- 行为清单 resolver.md 更新：播放期解析策略条目

## Out of Scope

- FFI 契约重构（另立 issue #166）
- 播放编排下沉到核心（另立 issue #165）
- yt-dlp 二进制管理流程本身

## Further Notes

- #120 修复面清单与图示见架构审查报告候选 4（architecture-review-20260825-094221.html）
- 与 FFI 契约加深（#166）协同：错误分类的契约形状由该入口定义
