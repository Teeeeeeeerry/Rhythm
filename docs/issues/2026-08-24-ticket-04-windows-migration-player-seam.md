# Ticket: 04 — Windows 迁移与 Player 最小接缝（含 M3U8 no-op 修复）

- **日期**: 2026-08-24
- **来源**: 父 issue「播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块」（#165）的 tracer-bullet 切片
- **状态**: 已立项（GitHub issue #173）
- **涉及平台**: Windows + Rust 核心

---

## Parent
播放编排下沉到 Rust 核心——双端 AppState 状态机收敛为一个深模块（#165）

## What to build
Windows 侧引入可注入的播放器接缝（与测试同批提交，不改变产品行为），随后把起播、暂停恢复、下一首/上一首、播完自动切歌改走协调器；同步修复 M3U8 导入 no-op（解析结果不入库），使该漂移实例消失。

## Acceptance criteria
- [ ] 播放器接缝存在，无音频设备时相关用例不再 SKIP
- [ ] Windows 起播/传输/自动切歌经协调器，与 macOS 行为一致
- [ ] M3U8 导入逐条入库并统计失败数
- [ ] 无播放位置守卫在 Windows 生效

## Blocked by
- Ticket 协调器启动路径落地（core + FFI + macOS 接入）（#170）
