# Rhythm 品牌配色体系 — 深度测试方案 v2（全面自动化）

> 针对 `fix/43-macos-brand-colors` 分支（macOS Theme.swift 11 token + Windows Colors.xaml 补充）。
> 现状：全仓库仅 1 个测试文件（`rust-core/tests/streaming.rs`），UI 端零测试、零 CI。
> v2 目标：**一切可机检的皆自动化，手工只留主观审美判断**；
> 覆盖矩阵按 token × 使用点 × 状态 × 外观 × 平台 × 版本 × 语言全展开。

---

## 1. 设计原则：自动化优先

| 原则 | 内容 | 消除的风险 |
|---|---|---|
| **单一事实来源** | 新建 `design/palette.json`（全部 token 的语义名、dark/light/高对比色值、alpha、适用背景、对比度阈值、例外说明）→ 构建期**代码生成** Swift 扩展 / XAML 资源 / C++ 头 / 文档 / 测试种子 | 双端 21 处 hex 手工同步漂移（构造上不可能不一致） |
| **数据驱动测试** | 单测/静态检查全部由 palette.json 生成——新增 token **自动获得** parity、contrast、RGB 全套测试 | 测试随 token 体系同步过期 |
| **禁止裸色** | L0 静态扫描全仓源码：任何视图代码中非 token 的 `Color.*`/hex/`NSColor`/Brush 引用即 CI 失败（白名单含合法系统组件） | 回归到硬编码系统色 |
| **手工最小化** | L3 UI 自动化接管外观切换/键盘/可访问性/像素比对，L4 手工烟测只保留 8 项主观项 | "看起来对了"= 单人肉眼，不可复现 |

## 2. 覆盖矩阵（全面性定义）

### 2.1 维度

| 维度 | 取值 | 说明 |
|---|---|---|
| Token | 11 个（accent / surface / elevated / textPrimary / textSecondary / textTertiary / border / source×4） | 每个 token 的 dark + light 变体（high-contrast 沿用同值，待 F7 决策） |
| 使用点 | macOS **38 处**（8 视图）/ Windows **15 处**（4 XAML）+ 1 处待品牌化 | 清单见 2.2，由 L0 扫描器从源码生成，不手维护 |
| 交互状态 | 未选中 / 选中 / 悬停 / 按下 / 聚焦（键盘）/ 播放中 | 每状态的颜色断言 + 快照 |
| 外观 | dark / light / 高对比 dark / 高对比 light（macOS）+ dark/light/HC（Windows） | 运行时切换用例见 L3 |
| 平台×版本 | macOS 13/14/15 × arm64+x86_64；Windows 10/11 | CI 矩阵见 §6 |
| 语言 | 中文 / 英文（L10n） | 快照与 UI 测试双语言跑 |
| 数据 | 空库 / 1 条 / 多曲目 / 无封面 / 超长标题 / 4 来源混排 | UI 测试夹具库 |

### 2.2 使用点清单（L0 扫描器输出，当前快照）

**macOS（38 处）**：
- `SidebarView.swift`：accent@15% 选中底、accent 选中字、textPrimary 未选字、surface 底
- `ArtistAlbumView.swift`：textPrimary 标题、textSecondary 艺术家/时长、accent@12% 选中底、elevated 占位底、source×4 徽标（前景+15% 底）
- `LibraryView.swift`：textSecondary×2、textTertiary
- `AlphabeticalView.swift`：textSecondary
- `PlayerBarView.swift`：border、textSecondary×6、accent（tint+播放模式）、surface、elevated
- `PlaylistListView.swift`：textSecondary×3、surface（新建弹窗）
- `PlaylistDetailView.swift`：textSecondary×2、textTertiary

**Windows（15 处）**：
- `LibraryView.xaml`：textPrimary、textSecondary×2
- `PlaylistDetailView.xaml`：textPrimary、textSecondary×2
- `PlaylistListView.xaml`：textSecondary×2
- `PlayerBarView.xaml`：surface、border、accent（ProgressBar）、elevated、textSecondary×2
- `SidebarView.xaml`：**0 处** → F2 未品牌化，L0 扫描器按"覆盖率缺口"报警

### 2.3 对比度背景映射（L0 脚本内置，按实际渲染背景）

macOS `List .inset` 在 light 下使用**系统浅灰 row 背景**（非纯白）→ 正文色在行背景上的
对比度必须与"on white"分开计算；dark 下 row 背景为 surface/elevated。映射表由
palette.json 的 `usage` 段驱动，全矩阵（token × 背景 × 外观）自动生成。

---

## 3. 自动化测试层级 L0–L4

### L0 静态分析（CI，秒级，全部零依赖 Python 3 stdlib）

| 脚本 | 作用 | 失败条件 |
|---|---|---|
| `check-color-parity.py` | 解析生成的 Swift/XAML/C++ 源码，逐 token × 外观比对双端 RGB（alpha 容差 ±2/255） | 任一端漂移 |
| `check-contrast.py` | WCAG 2.1 相对亮度 + alpha 合成，全矩阵（§2.3）；未达标项必须在 palette.json 例外段登记 | 新低对比度未登记 |
| `check-forbidden-colors.py` | 扫描双端源码（排除生成文件/白名单）：非 token 颜色引用即失败 | 出现 `Color.blue`/hex/裸 Brush |
| `check-token-coverage.py` | 每个受品牌化视图至少引用 1 个 token（视图级覆盖率，含 F2 缺口报警） | 新增视图无 token |
| `check-doc-drift.py` | 文档（本文件、README、PR 模板）中出现的色值与 palette.json 比对 | 文档与 token 漂移 |

### L1 单元测试（数据驱动，分钟级）

前置重构（必须）：`macos/Package.swift` 拆出 library target `RhythmTheme`
（SwiftPM 禁止测试 target import executable 目标）+ `RhythmThemeTests`。

测试全部由 palette.json 生成用例：

| 测试组 | 内容 | 生成方式 |
|---|---|---|
| `isDarkMatrix` | ≥6 种 appearance（darkAqua/aqua/HC×2/vibrantDark/未知）的判定钉住 | 固定表 |
| `paletteRGB` | 每 token × 每外观，`NSAppearance.current` 强制后 sRGB 解析逐通道断言 | token 循环生成 |
| `contrast` | 每 token × 每映射背景 × 每外观，复刻 L0 数学 | token 循环生成 |
| `semanticEquality` | light 下 elevated==surface；dark 下不等 | 固定 |
| `sourceDistinct` | 4 source 色互异且 ≠ accent | token 组合 |
| 视图级单元（后续） | 选中态计算、SourceTagView sourceType 映射表 | 数据驱动 |

Windows：`windows/tests/` 零依赖 assert 测试 exe（CMake `enable_testing()`），
F1 已修复（#121）：`Track::SourceColor(sourceType, isDarkTheme)` 双端值 + alpha==38 + 未知类型回退由 `source_color_test.cpp`（#122 解除桩）覆盖。

### L2 快照与像素回归（必做，非可选）

- macOS：`swift-snapshot-testing`，用例 = §2.2 清单 × 选中/未选中 × dark/light × zh/en；
  测试内 `NSAppearance.current` 强制外观；golden 入库，CI 比对失败即拦；
  维护约定：外观改动必附快照更新，review 看 diff。
- Windows：XAML 渲染截屏 — 用 WinUI 3 离屏渲染 + `RenderTargetBitmap` 导出 PNG，
  与 golden 像素 diff（阈值容忍抗锯齿，如 0.1%）。
- 首帧闪烁检测：录首帧序列，断言无"系统色→品牌色"跳变（L3 实现，见下）。

### L3 UI 自动化（取代手工大头）

| 平台 | 手段 | 用例 |
|---|---|---|
| macOS | XCUITest（需 XcodeGen 从 Package.swift 生成工程） | ① 运行时外观切换：`defaults write -g AppleInterfaceStyle Dark` 后逐视图断言 accessibility 颜色值/截图比对，无需重启 ② 侧边栏键盘导航（方向键移动选中）③ VoiceOver 选中语义（`accessibilityValue`/traits 断言）④ 新建播放列表弹窗全程截图 ⑤ 首帧闪烁检查 |
| Windows | WinAppDriver/Appium（需先验证 WinUI 3 兼容性，不支持则退化为 MsixTest + 截图比对） | ① 主题切换（`ThemeResource` 响应）② 徽标双外观截图 ③ 键盘 Tab 顺序 |

UI 自动化断言颜色：macOS 用 `XCUIElement` 的 `value` + 窗口截图像素抽样；
不依赖内部实现，锁 UI 行为。

### L4 手工烟测（仅主观项，8 项）

1. 深色下 teal 系是否"耐看"（无廉价感） 2. 4 个徽标色和谐度 3. Light 下 elevated/surface
同色时卡片是否靠阴影/边框可辨 4. 选中高亮（15% vs 12%）观感是否统一 5. 长标题截断观感
6. 空状态排版 7. 外接显示器（不同 ColorSync profile）颜色是否仍协调 8. 高对比度模式整体观感。
每项 3 分钟内完成，作为 PR 合并前的**唯一人工环节**。

---

## 4. 运行时自检（DEBUG 内建）

`#if DEBUG` 隐藏入口（设置菜单）：**Token 检验页** —— 每 token 按规范背景渲染色块 +
实测对比度数值叠加显示 + 与 palette.json 期望值自动比对（绿/红标注）。
价值：开发者/测试者 30 秒目检全部 22 组配色；UI 测试可驱动该页做全量截图（替代逐视图截屏）。

---

## 5. CI 矩阵设计（新增 `.github/workflows/ci.yml` + `visual.yml`）

```yaml
# ci.yml — 每次 push/PR（macOS runner）
#   python3 scripts/check-color-parity.py
#   python3 scripts/check-contrast.py
#   python3 scripts/check-forbidden-colors.py
#   python3 scripts/check-token-coverage.py
#   python3 scripts/check-doc-drift.py
#   cargo test -p rhythm-core
#   swift test                    # RhythmThemeTests（L1）
#   swift test --sanitize=address # 测试自身无内存问题

# visual.yml — PR label "visual-review" 或 main push（并行矩阵）
#   macOS:     [13, 14, 15] × [arm64, x86_64]  → 快照比对（L2）
#   macOS:     XCUITest 套件（L3）
#   windows:   windows-latest                   → 像素 golden + WinAppDriver（L3）
```

- 快照 golden 变更 = 视觉 diff，通过 PR comment 贴图；对比度矩阵表每 PR 自动贴出。
- **Nightly**：全矩阵刷新 golden 基线 + 用最新系统版本跑一遍（捕获 OS 升级导致的 NSColor 行为漂移）。

---

## 6. 先行修复项（升级为必须，按依赖序）

| # | 问题 | 位置 | 阻断谁 | 处置 |
|---|---|---|---|---|
| **F1** | Windows Source 色仅 dark 变体，Light 徽标对比度 ~3:1 | `RhythmCore.h:47-66` | parity/contrast/L1/L3 全链 | 补 light 变体 + theme 感知签名 |
| **F2** | Windows Sidebar 零品牌化（覆盖率为 0） | `SidebarView.xaml` | token-coverage | 品牌化 + 键盘/Tab 语义 |
| **F3** | macOS Sidebar 移除 `List(selection:)` 失键盘导航/VoiceOver | `SidebarView.swift:6-30` | L3 a11y 用例 | 恢复语义（`.accessibilityAddTraits(.isSelected)` 或回归 selection 绑定+自定义 tint） |
| **F4** | `SourceTagView` 未知类型回退 `.gray` | `ArtistAlbumView.swift:140` | forbidden-color 扫描 | 回退改 `.rhythmTextTertiary` |
| **F5** | `Track.swift:48` 遗留 `sourceColor` 死代码 | `Track.swift` | 文档漂移 | 删除 |
| **F6** | `isDark()` 未知 appearance 静默落 light | `Theme.swift:7-13` | L1 isDark 矩阵 | 决策 fallback + 测试钉住 |
| **F7** | 高对比度模式沿用普通色值 | 全 token | contrast 矩阵新增 HC 列 | 决策：登记例外 or 提供 HC 变体 |
| **F8** | **已发现缺陷**：Light textSecondary 合成后 ~3.4:1、textTertiary ~2.1:1（低于 AA） | 全仓使用点 | contrast 脚本必报 | 决策：调 alpha / 改色值 / 登记例外，三选一并记录在 palette.json |

> F1–F5 阻断 L0 全绿，**本分支合并前必须完成**；F6–F8 需设计决策（登记例外即可先绿）。

---

## 7. 分阶段执行计划

| 阶段 | 内容 | 规模 | 验收标准 |
|---|---|---|---|
| **P0** | F1–F5 修复 | 1 天 | 双端 Light 徽标达标；Sidebar 双端品牌化；无死代码 |
| **P1** | `palette.json` + 代码生成器（Swift/XAML/C++/文档/测试种子）+ L0 五脚本 + CI | 1–1.5 天 | `ci.yml` 全绿；从 palette.json 改一个色值 → 全端自动更新 |
| **P2** | `RhythmTheme` 重构 + L1 数据驱动测试（含 F6/F7/F8 决策落地） | 1 天 | `swift test` 全绿；加 1 个新 token → 自动获得 3 组测试 |
| **P3** | L2 快照（macOS golden 入库 + Windows RenderTargetBitmap） | 1–1.5 天 | 双端 golden 齐备，改色必红 |
| **P4** | L3 UI 自动化（XcodeGen + XCUITest + WinAppDriver 验证） | 2 天 | 运行时切换/键盘/a11y/首帧全自动 |
| **P5** | DEBUG 自检页 + L4 清单文档化 + Nightly | 0.5 天 | 手工只剩 8 项主观判断 |

**本分支合并门槛**：P0 完成 + P1 `ci.yml` 在本次 12 个改动文件上全绿 + L4 八项勾选；
P2–P5 作为紧接的自动化基建 PR 跟进（若分支已合入，F 项必须挂 issue 编号跟踪，不得静默消失）。

## 8. 风险与成本标注

- **最大成本项**：P4 的 UI 自动化（XcodeGen 工程化 + WinAppDriver 对 WinUI 3 的兼容性未验证）。
  缓解：P4 与 L2 快照互为备份，若 WinAppDriver 不可行，Windows 侧由像素 golden + L1 兜底。
- palette.json 代码生成需要保证**生成产物与手写时代码等价**（生成器自身要有 golden 测试）。
- XCUITest 对 `NSAppearance` 的运行时切换依赖系统 `defaults` 生效时机，用例需容忍异步（轮询等待）。
- 快照测试对字体渲染/抗锯齿敏感：golden 绑定 macOS 13+ 固定字体，Windows diff 阈值放宽至 0.1%。
