#!/usr/bin/env python3
"""测试任务的实现（#221 编排层收敛）。

全量测试的步骤集合、两个开关的语义、日志文件名与落点都在这里，两个平台共用
同一套退出码聚合（scripts/tasklib.py）。#144 确立的严格模式原样保留：
任一步红则整体非零退出，容错只能显式开启。
"""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

import task_build  # noqa: E402
import tasklib  # noqa: E402

PYTHON = sys.executable or "python3"

# 工具链兜底（本机已知坑）：XCTest 只随完整 Xcode 提供，Command Line Tools 的
# Swift 没有。开发者目录指向精简工具而 Xcode.app 在场时，局部切到完整工具链。
XCODE_APP = Path("/Applications/Xcode.app")
COMMAND_LINE_TOOLS = "/Library/Developer/CommandLineTools"


def developer_dir_override() -> dict[str, str]:
    """需要切换工具链时返回要覆盖的环境变量，否则空字典。"""
    if not XCODE_APP.is_dir():
        return {}
    import subprocess

    try:
        current = subprocess.run(["xcode-select", "-p"], capture_output=True,
                                 text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return {}
    if current != COMMAND_LINE_TOOLS:
        return {}
    print("[env] xcode-select 指向 CLT，局部切到 Xcode 工具链（DEVELOPER_DIR）以提供 XCTest")
    return {"DEVELOPER_DIR": str(XCODE_APP)}


def _script_step(name: str, argv: list[str], root: Path,
                 static_analysis: bool = True) -> tasklib.Step:
    """跑一个自带默认日志的 Python 校验脚本（日志由脚本自己写）。"""
    return tasklib.Step(name, lambda: tasklib.run([PYTHON, *argv], cwd=root),
                        static_analysis)


def _unittest_step(name: str, start_dir: str, log_name: str, root: Path,
                   static_analysis: bool = True) -> tasklib.Step:
    """跑一个 unittest 目录，输出转存到 testing/logs/<log_name>.log。"""
    return tasklib.Step(
        name,
        lambda: tasklib.run([PYTHON, "-m", "unittest", "discover", "-s", start_dir],
                            cwd=root, log=tasklib.log_path(log_name, root)),
        static_analysis,
    )


def copy_l1_sources(root: Path) -> int:
    """把 L1 测试拷进 SwiftPM 测试目录（与 CI 一致，保证种子最新）。"""
    dest = root / "macos" / "Tests" / "RhythmThemeTests"
    dest.mkdir(parents=True, exist_ok=True)
    for src in sorted((root / "testing" / "l1" / "macos").glob("*.swift")):
        shutil.copy2(src, dest / src.name)
        print(f"  {src.name}")
    return 0


def static_analysis_steps(root: Path) -> list[tasklib.Step]:
    """两个平台共享的静态分析前缀：按名字排序的全部 testing/l0/check-*.py（#344），
    接零 emoji 校验、L0 校验脚本自测、编排层自测（#345）。

    清单只在这里维护一处——新增一个校验脚本，两个平台自动纳入，不必改两张步骤表。
    配色一致性由 check-palette.py 覆盖（重新生成加逐字节比对，#249）；版本号漂移由
    check-version-drift.py 拦截（#253）。
    """
    steps: list[tasklib.Step] = []
    for script in sorted((root / "testing" / "l0").glob("check-*.py")):
        rel = script.relative_to(root).as_posix()
        steps.append(_script_step(f"L0 静态分析 {rel}", [rel], root))
    steps += [
        _script_step("L0 零 emoji（硬性约定，覆盖 git 跟踪的全部文件减排除清单）",
                     ["scripts/check_no_emoji.py"], root),
        _unittest_step("L0 校验脚本自测（testing/l0/tests/）",
                       "testing/l0/tests", "l0-script-tests", root),
        _unittest_step("编排层自测（testing/tasks/tests/：退出码聚合与共享实现，#259/#260）",
                       "testing/tasks/tests", "tasks-tests", root),
    ]
    return steps


def macos_steps(root: Path) -> list[tasklib.Step]:
    """全量测试的步骤表（顺序与迁移前的 run-all.sh 一致）：共享前缀 + macOS 段。"""
    env = developer_dir_override()
    steps = static_analysis_steps(root)
    steps += [
        tasklib.Step("拷贝 L1 测试到 SwiftPM 目录（与 CI 一致，保证种子最新）",
                     lambda: copy_l1_sources(root), static_analysis=False),
        tasklib.Step(
            "L1 macOS swift test",
            lambda: tasklib.run(["swift", "test"], cwd=root / "macos", env=env,
                                log=tasklib.log_path("l1-macos-swift-test", root)),
            static_analysis=False),
        tasklib.Step(
            "L1 macOS 内存卫生（ASan）",
            lambda: tasklib.run(["swift", "test", "--sanitize=address"],
                                cwd=root / "macos", env=env,
                                log=tasklib.log_path("l1-macos-asan", root)),
            static_analysis=False),
    ]
    return steps


# ---------------------------------------------------------------------------
# Windows 测试（#264）
# ---------------------------------------------------------------------------

# CMake 构建目录随应用产物一起收进仓库根 build/（#263 的单一约定）；
# 日志文件名沿用迁移前的约定，CI 收集路径不变。
#
# L2（截屏宿主构建、截屏、golden 像素比对）已移除（#387）：截屏宿主没有工程文件、
# golden 目录不存在，三步注定失败却让人以为外观回归有防线。重启所需的产物清单见
# testing/README.md「Windows L2 缺口」，补齐后再把步骤加回这里。


def _cmake_step(name: str, args: list[str], root: Path, log_name: str, *,
                project: str | None = None) -> tasklib.Step:
    """cmake 步骤的唯一构造入口（#415）。

    给出 project（配置步骤的源目录）时先检查工程文件：源目录里没有 CMakeLists.txt
    就直接指出缺的是哪个文件（#387），否则失败信息是构建工具的一句笼统报错，
    看不出步骤指向了空目录。
    """
    def action() -> int:
        if project is not None and not (root / project / "CMakeLists.txt").is_file():
            print(f"! 缺工程文件：{project}/CMakeLists.txt（「{name}」指向的目录里没有 CMake 工程）")
            return 1
        return tasklib.run(["cmake", *args], cwd=root,
                           log=tasklib.log_path(log_name, root))

    return tasklib.Step(name, action, static_analysis=False)


def _ctest_step(name: str, build_dir: Path, root: Path, log_name: str) -> tasklib.Step:
    """ctest 步骤：-C 与构建步骤的 --config 同一配置（#427）。

    Visual Studio 是多配置生成器，不点名配置时 ctest 找不到该配置的可执行文件，
    用例全部报 Not Run。
    """
    return tasklib.Step(
        name,
        lambda: tasklib.run(["ctest", "--test-dir", str(build_dir),
                             "-C", task_build.WINDOWS_CONFIG, "--output-on-failure"],
                            cwd=root, log=tasklib.log_path(log_name, root)),
        static_analysis=False)


def _cmake_configure_args(source: str, build_dir: Path) -> list[str]:
    return ["-S", source, "-B", str(build_dir)]


def windows_steps(root: Path, smoke: bool = False) -> list[tasklib.Step]:
    """Windows 测试的步骤表：共享静态分析前缀（#344）+ L1 单元 + L3 冒烟（L2 未实现，#387）。

    平台段只有构建与运行。每段的调用与日志文件名沿用迁移前的 PowerShell 入口；
    失败处理改为统一的退出码聚合——旧入口对 ctest 只打印警告后继续，红灯会被吞掉。
    """
    l1_dir = root / "build" / "windows" / "l1"
    app_dir = task_build.windows_build_dir(root)
    steps = static_analysis_steps(root) + [
        _cmake_step("L1 颜色测试 cmake 配置",
                    _cmake_configure_args("testing/l1/windows", l1_dir),
                    root, "l1-windows-cmake", project="testing/l1/windows"),
        _cmake_step("L1 颜色测试 cmake 构建",
                    ["--build", str(l1_dir), "--config", task_build.WINDOWS_CONFIG],
                    root, "l1-windows-cmake"),
        _ctest_step("L1 颜色测试 ctest", l1_dir, root, "l1-windows-ctest"),
        _cmake_step("L1b 应用工程测试 cmake 配置", _cmake_configure_args("windows", app_dir),
                    root, "l1-windows-rhythmtests", project="windows"),
        _cmake_step("L1b 应用工程测试 cmake 构建",
                    ["--build", str(app_dir), "--target", "RhythmTests",
                     "--config", task_build.WINDOWS_CONFIG],
                    root, "l1-windows-rhythmtests"),
        _ctest_step("L1b 应用工程测试 ctest", app_dir, root, "l1-windows-rhythmtests"),
    ]
    if smoke:
        steps.append(tasklib.Step(
            "L3 WinAppDriver 冒烟",
            lambda: tasklib.run(
                [PYTHON, "testing/l3/windows/theme_switch.py", "--smoke",
                 "--app", str(task_build.windows_app_exe(root))], cwd=root,
                log=tasklib.log_path("l3-windows-smoke", root)),
            static_analysis=False))
    return steps


# ---------------------------------------------------------------------------
# 平台分派
# ---------------------------------------------------------------------------

def is_windows() -> bool:
    return sys.platform == "win32"


def run_full_suite(argv: list[str] | None = None) -> int:
    """全量测试入口。任务名两个平台相同，步骤集合按平台分派。"""
    windows = is_windows()
    flags = tasklib.parse_test_flags(list(argv or []), allow_smoke=windows)
    root = tasklib.repo_root()
    log_dir = tasklib.logs_dir(root)
    label = "Windows 测试" if windows else "全量测试"
    print(f"===== Rhythm {label} {_now()} =====")
    steps = (windows_steps(root, smoke=flags.smoke) if windows
             else macos_steps(root))
    code = tasklib.run_steps(steps, l0_only=flags.l0_only,
                             allow_expected=flags.allow_expected)
    print(f"全部日志见 {log_dir}/")
    for name in sorted(p.name for p in log_dir.glob("*.log")):
        print(name)
    return code


def _now() -> str:
    from datetime import datetime

    return datetime.now().strftime("%Y-%m-%d %H:%M:%S")
