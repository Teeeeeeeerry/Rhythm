# Issue: L10n 文案与来源徽标色收敛为单一事实来源——palette.json 机制扩展到文案

> 已合入（#182~#186 五个 ticket 全部落地）：`contracts/l10n-keys.json` 键表 + 生成器
> 产出双端 L10n（平台差异保留为字段）；来源徽标色以 palette.json 为唯一来源写回双端；
> L0 校验（check-l10n-keys/check-ffi-contract）纳入 run-all；行为清单合并为 l10n-keys.md。

- **日期**: 2026-08-24
- **来源**: 架构审查（architecture-review-20260825-094221.html 候选 3）
- **状态**: 已合入（GitHub issue #167）
- **涉及平台**: 双端（macOS + Windows）

---

## Problem Statement

每句用户可见文案在 macOS 与 Windows 各实现一遍（两端各约 240 行），来源徽标色表同样双端各一份（macOS 主题 token 与 C++ 色表映射，色值逐字相同）。同一处漂移被修两次甚至四次：

- #141 收敛 Windows 文案层、#145 收敛 macOS 文案层——同一违规成对修复
- #121、#147 把来源徽标色（含 dark/light 双端值）修了四遍
- 行为清单随平台重复（rhythmcore-swift / rhythmcore-windows），L10n 相关条目各写一份

## Solution

文案键表成为单一事实来源，两端生成各自实现；来源徽标色沿用已有单一来源机制（testing/palette.json + L0 校验脚本）并扩展覆盖。

## User Stories

1. As a 维护者, I want 文案键只定义一次, so that 双端不再各自维护
2. As a 维护者, I want 新增文案只改一处, so that 不再出现成对的 L10n 修复
3. As a 用户, I want 双端文案一致（含错误分类文案）, so that 同一行为看到同一提示
4. As a 维护者, I want 来源徽标色在单一来源表, so that 改色只改一处
5. As a 测试者, I want 键表缺失/漂移自动报错, so that 漂移在提交时被拦截
6. As a 测试者, I want 双端 L10n 测试共用同一套键表断言, so that 清单不再按平台重复
7. As a 维护者, I want 平台差异（如安装命令、语言检测机制）作为字段保留, so that 单一来源不抹平合理差异

## Implementation Decisions

- 新增文案键表（单一事实来源）：中英文案 + 平台差异字段（如 yt-dlp 安装命令、系统语言检测方式）
- macOS 与 Windows 的 L10n 实现由键表生成；现有 L10n 枚举/头文件成为生成产物
- 来源徽标色：palette.json 扩展或新增来源色表，生成 macOS 主题 token 与 C++ 色表映射；L0 校验脚本（check-color-parity.py）扩展覆盖
- 语言检测（macOS 系统语言 vs Windows 系统语言/注册表覆盖）作为 adapter 差异保留在各自实现，不并入键表
- 行为清单中 L10n 相关条目合并为一份键表清单

## Testing Decisions

- 测试面 = 键表契约 + 两端生成产物
- 先例：Windows L10nTests.cpp（固定 locale 断言，#142 契约）；macOS 补对应测试
- L0-L4 校验设施扩展：新增键表校验脚本并纳入 run-all.sh，任一红即非零退出（#144 教训）
- 生成物一致性校验：macOS 与 Windows 产物必须来自同一键表（比对脚本，类似 check-color-parity.py）

## Out of Scope

- 翻译文本质量与本地化流程本身
- 语言检测机制的改动
- FFI 契约重构（另立 issue #166）

## Further Notes

- palette.json 单一事实来源 + L0 校验是既有先例，本 issue 是把同一机制扩展到文案
- 证据见架构审查报告候选 3（architecture-review-20260825-094221.html）
