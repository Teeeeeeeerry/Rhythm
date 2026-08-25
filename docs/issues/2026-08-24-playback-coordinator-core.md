# Issue: 播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块

> 已合入（#170~#175 六个 ticket 全部落地）：协调器 `rust-core/src/coordinator/mod.rs`
> + FFI `rhythm_coordinator_*`；双端起播/传输/自动切歌/队列同步/可用性全部经协调器；
> 行为清单合并为 `docs/testing/behavior/coordinator.md`（CO-01~28）；事件替代双端轮询。

- **日期**: 2026-08-24
- **来源**: 架构审查（architecture-review-20260825-094221.html 候选 1）
- **状态**: 已合入（GitHub issue #165）
- **涉及平台**: 双端（macOS + Windows）+ Rust 核心

---

## Problem Statement

macOS 与 Windows 的播放编排（停止旧曲目、按 sourceType 分发播放、队列重建、暂停/恢复、播完自动切歌、错误处理、队列与资料库同步）在两端 UI 层各实现一遍：macOS 的 AppState 与 Windows 的 AppState 是两份近乎逐行的镜像。同一套规则被维护两次，且已经实际分叉：

- 跳过不可播曲目：macOS 用有界循环跳过（#78），Windows 遇到第一个不可播曲目直接返回
- 起播候选选择：macOS 选第一个「可播」曲目，Windows 直接取列表首项
- 播完自动切歌：macOS 在 AppState 内驱动，Windows 在视图层定时器里驱动（不可测试）
- M3U8 导入：Windows 解析结果未入库（no-op），macOS 正常逐条入库（#136）
- 播放失败错误分类词汇两端不一致（macOS 直通核心分类，Windows 自行加前缀再剥掉）

每次编排缺陷修复（#51、#78、#81、#111、#137、#138、#147）都要在两个平台各落一遍；行为清单与测试也各维护一套（AS-xx / WA-xx），同一规格测试两遍、两套接口、两种语言。

## Solution

在 Rust 核心内新增播放协调器模块：把传输状态机从两端 UI 收进一个小 interface（启动、下一首、上一首、暂停/恢复、停止、事件回调），macOS 与 Windows UI 变成只渲染状态的薄 adapter。行为清单与测试从两套并一套。

## User Stories

1. As a 用户, I want 双端播放行为完全一致（跳过策略、自动切歌、暂停恢复）, so that 换平台不换体验
2. As a 维护者, I want 传输状态机只存在一处, so that 一个修复不再落两遍
3. As a 维护者, I want 跳过不可播曲目的策略在核心内, so that 两端分叉结构性消失
4. As a 维护者, I want 播完自动切歌由核心驱动, so that Windows 端该路径可被测试
5. As a 维护者, I want Windows M3U8 导入与 macOS 行为一致, so that 漂移实例被消灭
6. As a 测试者, I want 一套行为清单覆盖双端编排, so that 清单不再按平台重复
7. As a 测试者, I want 编排测试不依赖音频设备, so that 无设备环境也能全量运行
8. As a 维护者, I want 新平台客户端只需实现薄 adapter, so that 编排规则零成本复用
9. As a 用户, I want 删除正在播放的曲目时双端行为一致, so that 不会一端清队列一端残留
10. As a 维护者, I want 队列与资料库同步规则在核心内, so that 刷新列表不再双端各自实现
11. As a 用户, I want 播放失败提示分类一致, so that 双端看到同一套归因
12. As a 维护者, I want 传输可用性判断（可切下一首/上一首/可暂停）由协调器导出, so that 托盘菜单校验不再双端镜像

## Implementation Decisions

- 新增播放协调器模块，位于 Rust 核心，作为现有音频引擎与播放队列之上的组合层；音频引擎与队列的现有 interface 不变
- 协调器 interface：启动（曲目 + 队列）、下一首、上一首、暂停/恢复、停止、播放模式切换、事件回调（播完、错误、进度、状态变化）
- 「不可播曲目」判定与有界跳过规则移入协调器；起播候选选择规则（选第一个可播）由协调器统一
- 队列同步（资料库刷新时替换队列并跳到当前曲目）从两端 AppState 移入协调器
- 传输可用性（canPlayNext / canPlayPrevious / canTogglePlayback）由协调器导出，双端 UI 直接消费
- 播放失败的结构化分类（expired / cdn_rejected / other）由协调器事件携带，UI 只做文案映射
- FFI 层新增协调器导出；过渡期内保留现有导出，双端迁移完成后再清理
- UI 状态（选中项、弹窗、搜索框、URL 输入）留在 AppState，不进入协调器

## Testing Decisions

- 遵守行为清单制测试教义（ADR-0001）：先写协调器行为清单，每条主路径/边界/错误路径都必须有自动化测试；行覆盖率只作参考
- 测试面 = 协调器的 interface：行为清单中 appstate-macos 与 windows-appstate 的编排条目合并为一份协调器清单，双端只保留薄层 UI 测试
- 先例：rust-core 的队列与音频引擎行为测试、macOS AppStateTests（真实临时数据库，不用 mock）、Windows Catch2 测试
- Windows 侧需要为播放器引入最小接缝（当前为具体成员，无音频设备时相关用例 SKIP）——接缝与测试同批提交，不改变产品行为
- 红测登记：因已知缺陷无法通过的测试照写并禁用（XCTSkip / Catch2 SKIP），禁用原因挂本 issue 链接
- 只测外部行为，不测实现细节：通过协调器 interface 驱动，不直接断言内部字段

## Out of Scope

- 播放引擎内部（解码、输出、重采样）的改动
- L10n 文案与来源徽标色的单一事实来源（另立 issue #167）
- FFI 契约整体重构（另立 issue #166）
- UI 视觉与交互样式

## Further Notes

- 分叉现状的详细证据见架构审查报告候选 1（architecture-review-20260825-094221.html）
- 行为清单 appstate-macos.md 与 windows-appstate.md 合并是本 issue 验收的一部分
- 本 issue 与「FFI 契约加深」（#166）可并行推进；若先做本 issue，新导出直接采用新契约形状
