# L3 Windows：WinAppDriver 兼容性验证方案

> 状态：**未验证**（方案 §8 最大成本项）。本文件是验证协议 —— 先跑通
> 冒烟脚本再写正式用例；若不可行，按降级路径执行，**不得静默跳过**。

## 1. 冒烟验证（30 分钟，决定走 WinAppDriver 还是降级）

1. 安装 WinAppDriver（https://github.com/microsoft/WinAppDriver/releases，≥ 1.2.1）。
2. 启动 `WinAppDriver.exe`（默认 127.0.0.1:4723）。
3. 以管理员身份跑冒烟：
   ```bash
   python3 testing/l3/windows/theme_switch.py --smoke
   ```
   冒烟断言：
   - `/session` 创建成功（WinAppDriver 能 attach 到 WinUI 3 进程）；
   - `Appium` 元素树中能枚举出至少 1 个控件（`TextBlock`/`Button`）；
   - 截图接口返回非空 PNG。

**判定**：
- 全过 → 走 WinAppDriver 正式用例（§3）。
- 元素树空 / attach 失败 → 记录失败现象，进入降级（§4）。

已知风险（先查再测）：
- WinUI 3（Windows App SDK）的 XAML 控件走 WinRT 自动化树，WinAppDriver
  的 UIA 桥对部分控件（Pivot、ListView 模板内元素）支持不稳定。
- 截图接口对 GPU 合成窗口可能返回黑图 —— 若黑图，改用
  `RenderTargetBitmap`（L2 capture_views.cpp）补位。

## 2. 主题切换机制（正式用例依赖）

WinUI 3 的 `ThemeResource` 跟随系统主题：
- dark：`reg add HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize /v AppsUseLightTheme /t REG_DWORD /d 0 /f`
- light：同上，`/d 1`
- 改后窗口需触发重载（`SetTheme` 广播或重启应用进程）。

theme_switch.py 已封装（`--dark / --light` 两模式 + 截图 + 像素断言）。

## 3. 正式用例清单（冒烟通过后按此展开）

| 用例 | 操作 | 断言 |
|---|---|---|
| 主题切换 | `theme_switch.py --dark` 截图 vs `--light` 截图 | 窗口背景 ≈ #011F26 / #FFFFFF（中心像素抽样） |
| 徽标双外观 | 导入 4 来源曲目，dark/light 各截图 | 徽标区前景色与 palette.json sources 双端一致（区域抽样） |
| 键盘 Tab 顺序 | Tab 循环 | 焦点元素序列 = 预期顺序（F2 修复后含 Sidebar） |

断言颜色：WinAppDriver 截图（base64 PNG）→ 复用
`testing/l2/windows/compare_screenshots.py` 的 PNG 解码器做区域抽样。

## 4. 降级路径（WinAppDriver 不可行时 —— 必须执行，不是放弃）

1. **像素 golden 兜底**（主）：L2 capture_views.cpp 渲染 5 视图 × 2 主题，
   与 golden diff（compare_screenshots.py）。颜色回归 100% 覆盖。
2. **MsixTest + 截图比对**：用 Windows App SDK 的 MsixTest（UITest 框架）
   做进程级冒烟（启动/退出/主题切换重启），交互断言降级为截图像素断言。
3. 键盘顺序降级：L1 层断言 XAML 逻辑树中的 TabIndex 序（静态检查脚本，
   挂 check-token-coverage.py 同款位置扫描）。

## 5. 决策记录

验证结果与降级选择必须写回本文件（P4 验收项），并同步 ci/visual.yml 的
windows 任务。
