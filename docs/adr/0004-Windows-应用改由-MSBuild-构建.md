# Windows 应用改由 MSBuild 构建

2026-09-18（#428）：Windows 应用（XAML 外壳）改由 MSBuild 工程 `windows/Rhythm/Rhythm.vcxproj` 构建；CMake 只保留**行为库** `RhythmBehavior`（AppState、核心桥接、L10n 层）与测试宿主。取代 ADR-0003 中「否决 MSBuild」的结论；ADR-0003 的依赖获取部分（固定版本 + SHA-256）继续有效。

**Considered Options**

- **在 CMake 里模拟 XamlCompiler 与 MIDL**：否决。WinUI 3 桌面应用没有官方 CMake 的 XAML 编译支持，手工复刻两遍编译的输入输出维护负担重、随 SDK 升级易坏。
- **应用改由 MSBuild 构建，CMake 保留行为库与测试宿主**：采纳。XAML 标记编译（`*.xaml` -> `*.g.h`、`XamlTypeInfo`）只有 MSBuild 任务提供——ADR-0003 自己在 Consequences 里承认了这个缺口；#317 拆分后 exe 只剩 XAML 外壳，正是 MSBuild 擅长而 CMake 最难的部分。

**Consequences**

- **一套依赖前提**：vcxproj 不声明任何版本。CMake 配置期由 `RhythmWindowsDeps.cmake` 写出 `build/windows-deps/RhythmWindowsDeps.props`（已下载并校验的包路径），`windows/CMakeLists.txt` 再写出 `build/windows/<配置>/RhythmBehavior.props`（行为库、核心导入库、包含目录），vcxproj 只导入后者。升级依赖仍只改模块里的版本与哈希。
- **源文件只声明一处**：行为代码只在 CMake 的 `BEHAVIOR_SOURCES` 登记；vcxproj 只登记 XAML 外壳（视图、行模型、托盘、IDL），链接 CMake 产出的 `RhythmBehavior.lib`。编译选项（C++20、`/utf-8`、`NOMINMAX`、`/MD`）两边一致。
- **构建入口不变**：`python scripts/tasks.py build` 依次跑 Rust 核心、CMake 配置、CMake 构建行为库、MSBuild 构建应用（MSBuild 经 vswhere 定位），产物仍是 `build/windows/Release/Rhythm.exe`。`tasks.py test` 不变，只构建测试宿主。
- **前提仍只有 Build Tools**：Build Tools 不带 Visual Studio「WinUI/UWP C++」工作负载里把 XAML 编译器接入 C++ 构建的胶水。vcxproj 自己接上：`MarkupCompilePass1` 挂在 MIDL 前、`MarkupCompilePass2` 挂在编译前，喂给 Pass2 C++/WinRT 已解析的 winmd 引用，生成的 `XamlTypeInfo*.g.cpp` 显式编译；PRI 用 SDK 自带的 MSIX 工具（`EnablePriGenTooling=false`）。换 SDK 版本时这几处钩子是首先要复查的地方。
- **非打包、自包含**：`WindowsPackageType=None` + `WindowsAppSDKSelfContained=true`，运行时随 exe 放在输出目录，机器上不需要装 Windows App Runtime。未打包进程没有 `ApplicationData`，资料库放 `%LOCALAPPDATA%\Rhythm\library.db`。
