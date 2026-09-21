# Rhythm 深度测试套件（L0–L4）

> 依据 [deep-testing-plan.md](deep-testing-plan.md) v2 实现。
> 目标：**一切可机检的皆自动化**，手工只留 8 项主观审美判断。

## 分层一览

| 层 | 目录 | 内容 | 运行时机 |
|---|---|---|---|
| 数据源 | `palette.json` | 品牌配色的单一声明（人工维护）：tokens/sources 色值、translucent 基色 + 不透明度、docs token 文档块、sourceBadge 胶囊底不透明度、决策段 | — |
| 生成器 | `../scripts/gen-palette.py` | 配色文件写回三处源码标记区间（macOS 主色 token #247、Windows 主题字典 #248、来源徽标色 #246、徽标胶囊底与未知来源回退 #249/#219）；`--emit-swift-seed` 顺带刷新 L1 种子；与文案、契约两个生成器同构 | 改色后 |
| L0 静态 | `l0/` | 9 个零依赖 Python 脚本（palette/contrast/forbidden/coverage/doc-drift/ffi-contract/l10n-keys/version-drift/orchestration-dialects） | `tasks.py test` 双端共享前缀 |
| L0 静态 | `../scripts/check_no_emoji.py` | 零 emoji 硬性约定校验：范围是 git 跟踪的全部文件减排除清单（第三方 vendor 目录、依赖锁文件、构建产物），二进制按内容探测跳过（#224/#257） | 提交前 / `tasks.py test` 双端共享前缀 |
| L0 自测 | `l0/tests/` | L0 校验脚本自身的行为测试（stdlib unittest，临时文件树夹具） | `tasks.py test` 双端共享前缀 |
| L1 单元 | `l1/macos/` | PaletteSeed + 五组 Swift 测试（isDark/RGB/对比度/语义/互异） | `swift test` |
| L1 单元 | `l1/windows/` | 来源徽标色 assert 测试 exe（直测视图状态的徽标 `SourceBadgeOf`，链接行为库，#121/#122/#338） | ctest |
| L2 快照 | `l2/macos/` | swift-snapshot-testing 模板（8 视图 × 状态 × 外观 × 语言） | visual CI |
| L2 快照 | `l2/windows/` | **缺口，未实现**：`capture_views.cpp` 是骨架、没有 CMake 工程、没有 golden；已从测试入口移除（#387，见下文「Windows L2 缺口」）。像素比对工具 `compare_screenshots.py` 本身可用 | — |
| L3 UI | `l3/macos/` | XcodeGen project.yml + 4 组 XCUITest（外观切换/键盘/a11y/新建弹窗） | visual CI |
| L3 UI | `l3/windows/` | WinAppDriver 兼容性验证 + 主题切换脚本（stdlib 直调 REST） | visual CI |
| L4 手工 | `l4/` | 8 项主观烟测清单（唯一人工环节） | PR 合并前 |
| 编排 | `tasks/tests/` | 任务入口与共享实现的行为测试（退出码聚合、开关语义、产物落点约定） | `tasks.py test` 双端共享前缀 |
| CI | `ci/` | ci.yml（L0+L1）、visual.yml（L2/L3/Nightly）**未部署的模板**：仓库没有 `.github/workflows/`，表中各项目前只在本地执行（#346） | 拷贝部署后 |

## 快速开始（本地全量）

```bash
cd /Users/home-folder/GitHub/Rhythm

# 1. 数据源生成与自检
python3 scripts/gen-palette.py --emit-swift-seed   # 从 palette.json 写回三处产物 + 刷新 L1 种子
python3 testing/l0/check-palette.py                # 产物与 palette.json 逐字节一致？

# 2. L0 静态分析（现状全绿；F1/F2/F4 已分别由 #121/#124/#125 修复）
#    每个脚本结束自动把完整输出写入 testing/logs/<脚本名>.log（--log 可覆盖）
python3 testing/l0/check-palette.py
python3 testing/l0/check-contrast.py
python3 testing/l0/check-forbidden-colors.py
python3 testing/l0/check-token-coverage.py
python3 testing/l0/check-doc-drift.py
python3 testing/l0/check-ffi-contract.py           # 契约的第一道门：生成物与契约文本一致？
python3 testing/l0/check-l10n-keys.py
python3 testing/l0/check-version-drift.py
python3 testing/l0/check-orchestration-dialects.py
python3 scripts/check_no_emoji.py
# L0 校验脚本自身的测试：
python3 -m unittest discover -s testing/l0/tests
# 编排层自测：
python3 -m unittest discover -s testing/tasks/tests
# 或一键全量（日志统一落盘）。两个平台先跑同一组静态分析前缀（L0 九项 + 零 emoji + 两组自测），
# 再跑平台段：macOS 为 L1 swift test + ASan，Windows 为 L1 ctest（L2 未实现，#387）。
# Windows 段里的「L1b 契约生成物编译门」是契约的第二道门（#369）：生成物由 CMake 目标
# RhythmGeneratedCodec 单独编译一次，文本比对看不见的「生成器产不出可编译代码」在这里报红；
# 门本身有牙齿由 ctest 用例 GeneratedCodecTypeErrorFailsTheBuild 证明（构建一个故意写错类型的
# 翻译单元，构建必须失败）
python3 scripts/tasks.py test
# Windows 侧（任务名相同，--smoke 追加 L3 冒烟）：
python3 scripts/tasks.py test --smoke

# 3. L1（P2 重构已完成：Theme.swift 拆为 RhythmTheme target）
python3 scripts/gen-palette.py --emit-swift-seed    # 刷新测试种子
cp testing/l1/macos/*.swift macos/Tests/RhythmThemeTests/  # 拷贝挂载（与 CI 一致）
cd macos && swift build && swift test

# 4. L2 Windows 像素工具自测
python3 - <<'EOF'
import sys; sys.path.insert(0, "testing/l2/windows")
from compare_screenshots import PNG
print("PNG 解码器可用")
EOF
```

## 当前状态（main，v0.5.206）

| 检查 | 现状 | 含义 |
|---|---|---|
| `check-palette.py` | PASS | 三处配色生成物与 `palette.json` 逐字节一致（#249）；3 个半透明 token 的「基色 + 不透明度」声明与八位值一致（#245） |
| `check-contrast.py` | PASS | 36 组合全达标或已登记例外（F8 两项 + border 装饰线 + source 徽标 4.84 已登记） |
| `check-forbidden-colors.py` | PASS | 9 个 Swift 视图 + 4 个 XAML 视图无裸色（F4 已修复：#125/#128） |
| `check-token-coverage.py` | PASS | 7 个 macOS 视图 + 4 个 Windows 视图全部引用 token，校验器无按文件名的例外分支（F2 随 #383 删除无人可达的 SidebarView 关闭，#384） |
| `check-doc-drift.py` | PASS | 文档色值全部收录于 palette.json |
| `check-version-drift.py` | PASS | 四处人工版本副本与 `Cargo.toml` 一致；macOS 应用包版本与 Windows 项目版本改构建期派生，源文件写死版本即报红（版本号只改 `Cargo.toml`，#251/#252/#253/#254/#255） |
| `check-orchestration-dialects.py` | PASS | 被跟踪文件中无 bash / 批处理 / PowerShell 脚本（编排层只用 Python，#221） |
| `check_no_emoji.py` | PASS | 203 个被跟踪文件零 emoji；范围由扩展名白名单翻转为排除清单，此前漏检的 43 个文件（Windows UI 实现层、L0 脚本、界面标记文件）自此纳入（#224/#257） |

L0 已全绿，P0（F1–F5，F5 于 #147 删除死代码）完成。合并门槛见 deep-testing-plan.md §7；
状态表随每次改色/改视图核对，方式是重跑 `python3 scripts/tasks.py test`（严格模式，任一红即非零退出，#144）。

## 关键约定

1. **改色流程**：改 `palette.json` → `python3 scripts/gen-palette.py --emit-swift-seed`（写回三处产物）
   （`--emit-swift-seed` 顺带刷新测试种子）→ L0 脚本 → `swift test`。
   源码的标记区间是生成物，不手改；漂移由 `check-palette.py` 逐字节比对拦截（#249）。
   任何一步红都要解释，禁止 `|| true` 静默吞掉。
2. **palette.json 字段**：`tokens`/`sources` 是色值声明，`translucent` 是半透明 token 的
   「基色 + 不透明度」（八位值由生成器算出，不再手写），`docs` 是 token 文档块，
   `sourceBadge` 是徽标胶囊底的不透明度；`usage`/`backgrounds`/`exceptions`/`whitelist`
   是 L0 检查的"立法"：低对比度要么修复要么登记例外，两者都留痕。
   透明度容差字段已随 #250 移除——生成方向确立后，任何不一致都必须报红。
   三个文件里的品牌色字面量一律落在生成标记区间内：区间外的手写副本不被逐字节比对覆盖，
   等于重新开一条漂移通道（#219 收尾把最后一处——未知来源的回退色——收了进来）。
3. **禁止裸色**是硬约束：视图代码只准出现 `.rhythm*` token；新视图必须有 token。
4. **快照维护**：外观改动必附 golden 更新，review 看 diff；CI 快照红 = 真回归。
5. **手工最小化**：合并前 L4 八项勾选（l4/manual-smoke-checklist.md），
   每项 ≤ 3 分钟。
6. **日志留痕**：任何测试结束后必有日志 —— Python 脚本自动把完整输出双写
   终端与 `testing/logs/<脚本名>.log`（`--log` 可覆盖路径）；
   `swift test` / `ctest` / `xcodebuild` 的输出由任务入口转存（xcodebuild 另有
   `.xcresult` 结构化日志）。
   `testing/logs/` 已 gitignore；CI 模板部署后每次运行作为 artifact 上传（模板目前未部署，#346）。
7. **一键入口**：`python3 scripts/tasks.py test`。任务名两个平台相同；静态分析前缀两个平台一致，
   平台段各跑本机支持的层级（两者并不等价，见下文任务表），日志齐后看 `testing/logs/`。
8. **改版本流程**：版本号只改 `Cargo.toml` 的 `[workspace.package] version`，
   发布时跑 `python3 scripts/tasks.py bump-version`（不带参数末位加一，也可指定版本），
   它把出处与三处文档副本一起推到新值、同步依赖锁文件，再跑一次 `check-version-drift.py` 自校验。
   两处构建配置不参与——macOS 应用包版本在组装时写入、Windows 项目版本在 cmake 配置期派生（#254/#255），
   源文件里再写死版本值即报红。写的位置取自校验的副本清单，两边不可能各漂一次（#220 收尾）。

## Windows L2 缺口（#387）

Windows 端**没有视图外观回归防线**。测试入口曾登记三段 L2 步骤（截屏宿主 cmake 配置与构建、截屏、golden 像素比对），
但它们指向的东西从未提交：`testing/l2/windows/` 下没有 CMake 工程文件，`golden/` 目录不存在，`capture_views.cpp`
只是注释写着「P3」的骨架。这三步在 Windows 上注定失败，却让文档与 CI 模板以为防线存在，因此已从入口移除。

重启这项工作时需要补齐的产物（缺一不可，补齐后再把步骤加回 `scripts/task_test.py` 的 `windows_steps`）：

1. `testing/l2/windows/CMakeLists.txt`：截屏宿主工程，依赖走 `windows/cmake/RhythmWindowsDeps.cmake`（与应用一致，#386）
2. `capture_views.cpp` 实现骨架里的集成步骤：初始化 WinRT 与 DispatcherQueue，各受管视图 x {Default, Light} 渲染，
   输出 `<视图名>_<Default|Light>.png`（比对工具依赖此命名）
3. `testing/l2/windows/golden/`：首批基准由人工确认截图后提交。`compare_screenshots.py` 在 golden 为空时直接失败，
   新增截图缺基准也失败，所以首次建立基准不会被误判为通过
4. 验收：改一个受管视图的配色后比对非零退出；比对失败时用 `--heatmap` 输出差异热图

## 任务入口（#221）

编排层只有一种语言。构建与测试都走 `python3 scripts/tasks.py <任务>`，
CI 模板调用的是同名命令（模板在 `testing/ci/`，尚未部署到 `.github/workflows/`，#346）。

| 任务 | 内容 |
|---|---|
| `build` | 构建本平台应用（macOS `build/Rhythm.app`；Windows `build/windows/Release/Rhythm.exe`） |
| `test` | 本平台全量测试：双端共享静态分析前缀（L0 九项 + 零 emoji + 两组自测，#344/#345），再接平台段——macOS L1（swift test + ASan）；Windows L1（ctest）外加契约生成物编译门（#369），`--smoke` 追加 L3；Windows L2 未实现（#387） |
| `bump-version` | 提升版本号（不带参数末位加一），同步三处文档副本与依赖锁文件后自校验 |
| `check-no-emoji` | 零 emoji 硬性约定校验 |
| `compare-screenshots` | L2 截屏与 golden 的像素比对 |

退出码：`0` 全绿 / `1` 有步骤失败 / `2` 用法错误（未知任务或未知参数）。
筛选后一步都没跑也以 `1` 退出并提示「没有步骤被执行」，显式豁免救不回零步（#343）。

开关（`test`）：`--l0-only` 只跑静态分析；`--allow-expected-failures` 与
`ALLOW_EXPECTED_FAILURES=1` 显式豁免预期失败，默认严格模式不容错（#144）；
`--smoke` 只在有冒烟段的平台上被接受。

共享实现在 `scripts/tasklib.py`：仓库根定位、日志目录与输出转存、失败计数与
退出码聚合、子进程调用与错误传播。日志一律落 `testing/logs/<名字>.log`。
