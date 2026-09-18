# Windows App SDK 接入方式

> 2026-09-18 部分被 ADR-0004 取代：应用改由 MSBuild 构建（下文否决 MSBuild 的结论不再成立）；依赖获取部分（固定版本 + SHA-256、`RhythmWindowsDeps.cmake`）继续有效，MSBuild 工程经它写出的 props 取用同一套依赖。

2026-09-15（#386）：Windows 端的依赖由 CMake 模块 `windows/cmake/RhythmWindowsDeps.cmake` 在配置期按固定版本获取——下载 NuGet 包（Windows App SDK、WebView2、C++/WinRT）与 nlohmann/json 单头文件并校验 SHA-256，用固定版本的 `cppwinrt.exe` 生成投影头，以原有目标名导出为导入目标。应用与测试宿主共用这个模块，构建入口仍是 `python3 scripts/tasks.py build` / `test`。

**Considered Options**

- **应用改由 MSBuild 工程驱动，CMake 只管测试宿主**：否决。引入第二套构建方言，应用与测试宿主各有一套依赖前提。
- **采用上游为 CMake 提供支持的实验版本**：否决。实验版本不稳定，且仍依赖上游包布局，可复现性无保证。
- **在 CMake 里手工引入 SDK 头文件与库**：采纳。前提全部写在仓库里（版本 + 哈希），干净机器只需 MSVC + Windows SDK + 联网即可配置，不引入新方言。

**Consequences**

- 配置命令在干净机器上零退出；首次配置需要联网，之后走 `build/windows-deps/` 缓存。哈希不符即失败，不静默放过。
- 升级依赖只改模块里的版本与哈希；投影头随版本戳自动重新生成。
- XAML 标记编译（`*.xaml` -> `*.g.h`、`XamlTypeInfo`）仍只有 MSBuild 任务提供，本模块不覆盖；应用目标能否完整链接取决于后续票处理。本决定只保证构建推进到编译阶段，编译暴露的代码缺陷由既有各票处理。
