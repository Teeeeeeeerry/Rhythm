# Rhythm 深度测试套件（L0–L4）

> 依据 [deep-testing-plan.md](deep-testing-plan.md) v2 实现。
> 目标：**一切可机检的皆自动化**，手工只留 8 项主观审美判断。

## 分层一览

| 层 | 目录 | 内容 | 运行时机 |
|---|---|---|---|
| 数据源 | `palette.json` | 单一事实来源（tokens/sources 由源码自动提取，决策段人工维护） | — |
| 数据源 | `sync-palette.py` | 源码 ↔ palette.json 同步；`--check` CI 校验；`--emit-swift-seed` 生成 L1 种子 | 改色后 |
| L0 静态 | `l0/` | 8 个零依赖 Python 脚本（parity/contrast/forbidden/coverage/doc-drift/ffi-contract/l10n-keys/version-drift） | 每次 push/PR |
| L0 自测 | `l0/tests/` | L0 校验脚本自身的行为测试（stdlib unittest，临时文件树夹具） | 每次 push/PR |
| L1 单元 | `l1/macos/` | PaletteSeed + 五组 Swift 测试（isDark/RGB/对比度/语义/互异） | `swift test` |
| L1 单元 | `l1/windows/` | 来源徽标色 assert 测试 exe（直测 `RhythmCore.h`，#121/#122） | ctest |
| L2 快照 | `l2/macos/` | swift-snapshot-testing 模板（8 视图 × 状态 × 外观 × 语言） | visual CI |
| L2 快照 | `l2/windows/` | WinUI 3 离屏截屏（capture_views.cpp）+ 零依赖像素 diff | visual CI |
| L3 UI | `l3/macos/` | XcodeGen project.yml + 4 组 XCUITest（外观切换/键盘/a11y/新建弹窗） | visual CI |
| L3 UI | `l3/windows/` | WinAppDriver 兼容性验证 + 主题切换脚本（stdlib 直调 REST） | visual CI |
| L4 手工 | `l4/` | 8 项主观烟测清单（唯一人工环节） | PR 合并前 |
| CI | `ci/` | ci.yml（L0+L1）、visual.yml（L2/L3/Nightly）模板 | 拷贝部署 |

## 快速开始（本地全量）

```bash
cd /Users/home-folder/GitHub/Rhythm

# 1. 数据源同步与自检
python3 testing/sync-palette.py --check        # palette.json 与源码一致？
python3 testing/sync-palette.py --emit-swift-seed  # 刷新 L1 测试种子

# 2. L0 静态分析（现状全绿；F1/F2/F4 已分别由 #121/#124/#125 修复）
#    每个脚本结束自动把完整输出写入 testing/logs/<脚本名>.log（--log 可覆盖）
python3 testing/l0/check-color-parity.py
python3 testing/l0/check-contrast.py
python3 testing/l0/check-forbidden-colors.py
python3 testing/l0/check-token-coverage.py
python3 testing/l0/check-doc-drift.py
python3 testing/l0/check-ffi-contract.py
python3 testing/l0/check-l10n-keys.py
python3 testing/l0/check-version-drift.py
# L0 校验脚本自身的测试：
python3 -m unittest discover -s testing/l0/tests
# 或一键全量（含 sync 校验 + L1 swift test，日志统一落盘）：
bash testing/run-all.sh
# Windows 侧：
powershell -File testing/run-windows.ps1 -Smoke

# 3. L1（P2 重构已完成：Theme.swift 拆为 RhythmTheme target）
python3 testing/sync-palette.py --emit-swift-seed   # 刷新测试种子
cp testing/l1/macos/*.swift macos/Tests/RhythmThemeTests/  # 拷贝挂载（与 CI 一致）
cd macos && swift build && swift test

# 4. L2 Windows 像素工具自测
python3 - <<'EOF'
import sys; sys.path.insert(0, "testing/l2/windows")
from compare_screenshots import PNG
print("PNG 解码器可用")
EOF
```

## 当前状态（main，v0.5.92）

| 检查 | 现状 | 含义 |
|---|---|---|
| `sync-palette.py --check` | PASS | palette.json 与源码一致（tokens/sources/usage 全覆盖） |
| `check-color-parity.py` | PASS | 7 个双端 token + 4 个 source 色一致（F1 已修复：#121/#123） |
| `check-contrast.py` | PASS | 36 组合全达标或已登记例外（F8 两项 + border 装饰线 + source 徽标 4.84 已登记） |
| `check-forbidden-colors.py` | PASS | 9 个 Swift 视图 + 5 个 XAML 视图无裸色（F4 已修复：#125/#128） |
| `check-token-coverage.py` | PASS | 7 个 macOS 视图 + 5 个 Windows 视图全部引用 token（F2 已修复：#124/#133） |
| `check-doc-drift.py` | PASS | 文档色值全部收录于 palette.json |
| `check-version-drift.py` | PASS | 六处版本副本与 `Cargo.toml` 一致（版本号只改 `Cargo.toml`，#251/#252/#253） |

L0 已全绿，P0（F1–F5，F5 于 #147 删除死代码）完成。合并门槛见 deep-testing-plan.md §7；
状态表随每次改色/改视图核对，方式是重跑 `bash testing/run-all.sh`（严格模式，任一红即非零退出，#144）。

## 关键约定

1. **改色流程**：改 `Theme.swift` / `Colors.xaml` / `RhythmCore.h` →
   `sync-palette.py`（刷新 palette.json + 测试种子）→ L0 脚本 → `swift test`。
   任何一步红都要解释，禁止 `|| true` 静默吞掉。
2. **palette.json 决策段**（usage/backgrounds/exceptions/whitelist）人工维护，
   是 L0 检查的"立法"：低对比度要么修复要么登记例外，两者都留痕。
3. **禁止裸色**是硬约束：视图代码只准出现 `.rhythm*` token；新视图必须有 token。
4. **快照维护**：外观改动必附 golden 更新，review 看 diff；CI 快照红 = 真回归。
5. **手工最小化**：合并前 L4 八项勾选（l4/manual-smoke-checklist.md），
   每项 ≤ 3 分钟。
6. **日志留痕**：任何测试结束后必有日志 —— Python 脚本自动把完整输出双写
   终端与 `testing/logs/<脚本名>.log`（`--log` 可覆盖路径）；
   `swift test` / `ctest` / `xcodebuild` 由 `run-all.sh` / `run-windows.ps1`
   或 CI tee 落盘（xcodebuild 另有 `.xcresult` 结构化日志）。
   `testing/logs/` 已 gitignore，CI 每次运行作为 artifact 上传。
7. **一键入口**：`bash testing/run-all.sh`（macOS）、
   `powershell -File testing/run-windows.ps1 -Smoke`（Windows）
   跑完本机支持的全部层级，日志齐后看 `testing/logs/`。
