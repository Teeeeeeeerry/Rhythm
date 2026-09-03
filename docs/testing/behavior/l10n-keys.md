# L10n 文案键表行为清单

- 模块：`contracts/l10n-keys.json`（单一事实来源，中英文案 + platform 差异字段）+ `scripts/gen-l10n.py`（生成器）+ 双端适配层（macOS `L10n.swift`/`L10nKeys.swift`、Windows `L10n.h`/`L10nKeys.h`）；测试：macOS `RhythmCoreSwiftBehaviorTests`（SW-17）、Windows `L10nTests.cpp`（LK-01~06）
- 历史回归：`#141`（Windows 文案层）、`#145`（macOS 文案层收敛）、`#120`（播放失败分类文案）、`#142`（固定 locale 确定性断言）
- 教义（#167 组）：文案键只定义一次；双端实现由同一键表生成，漂移结构性不可能；平台差异（安装命令、语言检测机制）保留为键表字段/适配层差异
- 键名对齐（#225）：同一处文案双端查同一个键——资料库空态与导入提示曾各查一个键，已对齐为 `library_empty` + `import_hint`，重复键删除
- 校验：`testing/l0/check-l10n-keys.py`（#185）——键表结构、双端生成物一致性、Windows 映射覆盖，任一红即 run-all 非零退出

## 主路径（P0 — 合并门槛）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| LK-01 | 静态文案双语言 | 键表每键 zh/en 齐全；固定 locale 下取对应语言（macOS UserDefaults 覆盖、Windows 注册表覆盖） | 固定 locale 断言 |
| LK-02 | 播放失败分类文案（#120） | `expired` → 保留"重新粘贴"建议；`cdn_rejected` → 换网络且**不**建议重贴；其它 → 泛化"播放失败"；中英分支同分类 | 双端固定 locale |
| LK-03 | 解析失败文案 | 中文 headline（各 kind）+ 英文原始 detail；未识别 kind → 原文 | Windows LK-04/LK-05 同源 |
| LK-04 | 解析器状态文案 | checking/verifying/updating/failed 各文案；downloading 有 total → `x / y MB`、无 → `x MB`；未知/quiet → 空串 | 双端 |
| LK-05 | 来源徽标与托盘文案 | tag local/youtube/bilibili/direct_url、托盘播放/暂停/停止/上下首 | 双端 |
| LK-06 | 键表覆盖（#182/#185） | 键表在 zh/en 两语言下均有可用取值；生成物与键表一致；Windows `Key()` 映射覆盖全部 windows 键 | SW-17 + L0 校验 |

## 边界情况（P1）

| 编号 | 行为 | 断言 | 测试途径 |
|---|---|---|---|
| LK-07 | 平台差异字段 | yt-dlp 安装命令（macOS brew / Windows winget）只出现在对应平台生成物中 | L0 生成物比对 |
| LK-08 | 语言检测差异（适配层） | macOS 跟随系统 Locale + AppLanguage；Windows 系统 UI 语言 + 注册表覆盖——检测机制不入键表 | 双端固定 locale |

## 红测登记

| 编号 | 缺陷 | issue | 状态 |
|---|---|---|---|
| — | 无（#141/#145/#120 均已修复并有固定 locale 测试锁定） | — | — |
