# Issue: FFI 层 52 个直通导出加深为结构化契约——错误随结果返回、状态走事件、绑定生成

> 已合入（#176~#181 六个 ticket 全部落地）：resolve/classify/install 与协调器全部
> 一次返回结构化结果；状态/进度走事件通道；M3U8 条目与状态/模式具名化；
> `contracts/ffi-contract.json` 单一声明 + 生成器产出双端编解码绑定；
> 旧 player 直通导出与全局错误槽删除。

- **日期**: 2026-08-24
- **来源**: 架构审查（architecture-review-20260825-094221.html 候选 2）
- **状态**: 已合入（GitHub issue #166）
- **涉及平台**: 双端（macOS + Windows）+ Rust 核心

---

## Problem Statement

FFI 层目前是 52 个直通导出，每个都是「空指针检查 + 一次核心调用 + JSON/哨兵值返回」，interface 宽度几乎等于实现宽度。调用方要学的 interface 知识散落各处：

- 魔法整数：播放状态 0-5、播放模式 0-3，macOS 与 Windows 各自硬编码（播放模式甚至存在 4 份拷贝）
- 错误约定随函数而异：null 指针、-1、0 在不同函数里含义不同（成功 / 不支持格式 / 假值）
- 进程级全局错误槽：解析失败后必须再查一次全局错误，两步协议且并发解析共享同一槽
- 位置元组跨 seam：M3U8 条目以无名字段数组传输，调用方靠下标记忆字段顺序，核心内文档注释已与实际顺序漂移
- JSON schema（snake_case 键）由 Swift 与 C++ 各声明一遍；核心已有的转换逻辑（resolved_to_track）未导出，两端各自手写 Track 与 JSON 互转
- 状态只能轮询：核心引擎已有事件回调，seam 把它们拍平成 getter

后果即 CONTEXT.md 记载的「三层套路」（#70 教训）：每加一个引擎能力要改导出层、Swift 封装、C++ 封装再加各自视图，schema 每次都要在两端同步，测试也随之按平台重复（rhythmcore-swift / rhythmcore-windows 行为清单）。

## Solution

把 seam 的契约加深：结构化结果（成功载荷 + 分类错误一次返回）、事件驱动状态（替代轮询）、具名结构替代位置元组、单一 schema 声明生成双端绑定。

## User Stories

1. As a 开发者, I want 每个调用一次返回结果与错误, so that 不再有「先 null 再查全局槽」的两步协议
2. As a 开发者, I want 错误语义随函数明确, so that 不再出现 0 兼作成功/不支持/假值
3. As a 开发者, I want 状态通过事件推送, so that 两端不再各自实现轮询
4. As a 开发者, I want M3U8 条目是具名字段, so that 字段顺序不再靠下标记忆
5. As a 开发者, I want 播放状态与播放模式是具名枚举, so that 魔法整数消失
6. As a 开发者, I want schema 声明一次、双端绑定生成, so that 编解码不再手写两遍
7. As a 开发者, I want 核心的转换逻辑通过契约暴露, so that 两端不再各自实现
8. As a 测试者, I want 契约测试聚焦形状而非逐函数直通, so that 测试数量随能力增长是亚线性的
9. As a 维护者, I want 新增引擎能力只改一处契约声明, so that 三层套路成本下降
10. As a 开发者, I want 并发解析不共享全局状态, so that 跨调用竞态消失

## Implementation Decisions

- 统一结果类型：每个导出返回「成功载荷 + 分类错误」的结构化结果；废除进程级全局错误槽与两步查询协议
- 状态事件通道：新增事件回调导出（播放状态变化、进度、解析进度），UI 订阅而非轮询
- 具名结构替代位置元组：M3U8 条目（标题/艺术家/位置）改为具名字段对象
- 状态与模式枚举具名化，双端不再硬编码数值
- 单一契约声明（schema 文件或代码生成器），Swift 与 C++ 绑定由此生成
- 核心已有转换逻辑通过契约暴露，删除两端手写副本
- 兼容过渡：旧导出保留至双端迁移完成，迁移按模块分批进行

## Testing Decisions

- 测试面 = FFI 导出契约：现有 ffi 行为测试（23 例）与双端 rhythmcore 行为清单；契约形状测试优先（结果结构、错误分类、事件序列）
- 事件通道测试先例：核心音频引擎状态机行为测试
- 错误分类映射测试收拢到契约测试，双端只测薄文案映射（固定 locale，先例 #142）
- 行为清单 ffi.md 更新为新契约；rhythmcore-swift / rhythmcore-windows 合并或瘦身为绑定生成物测试

## Out of Scope

- 播放编排下沉到核心（另立 issue #165）
- 播放引擎内部实现
- 文案单一事实来源（另立 issue #167）

## Further Notes

- 证据与图示见架构审查报告候选 2（architecture-review-20260825-094221.html）
- 与协调器下沉（#165）协同：新导出直接采用新契约形状
