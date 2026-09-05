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


def macos_steps(root: Path) -> list[tasklib.Step]:
    """全量测试的步骤表（顺序与迁移前的 run-all.sh 一致）。"""
    env = developer_dir_override()
    steps: list[tasklib.Step] = [
        _script_step("L0-0 sync-palette --check（palette.json 与源码一致性）",
                     ["testing/sync-palette.py", "--check"], root),
    ]
    # 含版本号漂移校验 check-version-drift.py（#253）：版本号只改 Cargo.toml，
    # 其余六处副本漂移在此报红，不必等到发布后才发现。
    for script in sorted((root / "testing" / "l0").glob("check-*.py")):
        rel = script.relative_to(root).as_posix()
        steps.append(_script_step(f"L0 静态分析 {rel}", [rel], root))
    steps += [
        _script_step("L0 零 emoji（硬性约定，覆盖 git 跟踪的全部文件减排除清单）",
                     ["scripts/check-no-emoji.py"], root),
        _unittest_step("L0 校验脚本自测（testing/l0/tests/）",
                       "testing/l0/tests", "l0-script-tests", root),
        _unittest_step("编排层自测（testing/tasks/tests/：退出码聚合与共享实现，#259/#260）",
                       "testing/tasks/tests", "tasks-tests", root),
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


def run_full_suite(argv: list[str] | None = None) -> int:
    """全量测试入口（L0 + L1）。"""
    flags = tasklib.parse_test_flags(list(argv or []))
    root = tasklib.repo_root()
    log_dir = tasklib.logs_dir(root)
    print(f"===== Rhythm 全量测试 {_now()} =====")
    code = tasklib.run_steps(macos_steps(root), l0_only=flags.l0_only,
                             allow_expected=flags.allow_expected)
    print(f"全部日志见 {log_dir}/")
    for name in sorted(p.name for p in log_dir.glob("*.log")):
        print(name)
    return code


def _now() -> str:
    from datetime import datetime

    return datetime.now().strftime("%Y-%m-%d %H:%M:%S")
