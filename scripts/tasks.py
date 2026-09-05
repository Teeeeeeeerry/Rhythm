#!/usr/bin/env python3
"""Rhythm 跨平台任务入口（#221）。

一个入口列出全部可跑的任务，任务名两个平台相同，文档里的命令不必分平台写两遍。
编排职责（路径、日志、失败计数、子进程）取自 scripts/tasklib.py。

用法：
    python3 scripts/tasks.py                 # 列出全部任务
    python3 scripts/tasks.py <任务> [参数...]

退出码约定：
    0   全部步骤通过（或失败但已显式豁免）
    1   有步骤失败（严格模式，默认）
    2   用法错误（未知任务或未知参数）

严格模式是默认（#144）：任一步红则整体非零退出。容错只能显式打开——
命令行 --allow-expected-failures 或环境变量 ALLOW_EXPECTED_FAILURES=1。
"""

from __future__ import annotations

import os
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

import task_build  # noqa: E402
import tasklib  # noqa: E402

USAGE_ERROR = 2

TRUTHY = ("1", "true", "yes", "on")


# ---------------------------------------------------------------------------
# 步骤与退出码聚合
# ---------------------------------------------------------------------------

@dataclass
class Step:
    """一个可执行步骤。action 返回退出码（零为通过）。

    static_analysis 标记该步是否属于静态分析段——只跑静态分析时保留的正是这些。
    """

    name: str
    action: Callable[[], int]
    static_analysis: bool = True


def select_steps(steps: list[Step], *, l0_only: bool) -> list[Step]:
    """只跑静态分析时过滤掉非静态分析步骤。"""
    return [s for s in steps if s.static_analysis or not l0_only]


def run_steps(steps: list[Step], *, l0_only: bool = False,
              allow_expected: bool = False) -> int:
    """按顺序跑步骤，聚合成整体退出码。

    不因某步失败而提前中断——全部跑完才知道到底红了几处；但只要有红，
    默认就以非零退出（#144：曾经绿着吞掉红灯）。
    """
    tally = tasklib.Failures()
    for step in select_steps(steps, l0_only=l0_only):
        print(f"\n----- {step.name} -----")
        tally.record_code(step.name, step.action())
    print()
    print(tally.summary())
    code = tally.exit_code(allow_expected)
    if tally.count and code == 0:
        print("（失败已按 --allow-expected-failures / ALLOW_EXPECTED_FAILURES 显式豁免）")
    elif code:
        print("存在失败步骤，以非零退出码结束"
              "（预期失败请用 --allow-expected-failures 或 "
              "ALLOW_EXPECTED_FAILURES=1 显式豁免）", file=sys.stderr)
    return code


@dataclass
class TestFlags:
    l0_only: bool = False
    allow_expected: bool = False


def parse_test_flags(argv: list[str], env: dict[str, str] | None = None) -> TestFlags:
    """解析全量测试入口的两个开关（命令行与环境变量两种形式）。

    语义与迁移前的 run-all.sh 一致：ALLOW_EXPECTED_FAILURES=1 等价于
    --allow-expected-failures；未知参数按用法错误处理。
    """
    env = os.environ if env is None else env
    flags = TestFlags(
        allow_expected=str(env.get("ALLOW_EXPECTED_FAILURES", "0")).lower() in TRUTHY
    )
    for arg in argv:
        if arg == "--l0-only":
            flags.l0_only = True
        elif arg == "--allow-expected-failures":
            flags.allow_expected = True
        else:
            raise SystemExit(
                f"未知参数: {arg}（支持 --l0-only / --allow-expected-failures）"
            )
    return flags


# ---------------------------------------------------------------------------
# 任务注册表
# ---------------------------------------------------------------------------

@dataclass
class Task:
    name: str
    summary: str
    handler: Callable[[list[str]], int]
    platform: str = "all"


def pending(ticket: str, legacy: str) -> Callable[[list[str]], int]:
    """尚未迁移的任务：明说去向，不假装能跑（退出码 2 = 用法错误）。"""

    def handler(_argv: list[str]) -> int:
        print(f"该任务尚未迁移到任务入口（{ticket}）；当前仍用 {legacy}",
              file=sys.stderr)
        return USAGE_ERROR

    return handler


def task_test(argv: list[str]) -> int:
    """全量测试（L0 静态分析 + L1 单元测试）。步骤集合由 #262 迁入。"""
    flags = parse_test_flags(argv)
    return run_steps(test_steps(), l0_only=flags.l0_only,
                     allow_expected=flags.allow_expected)


def test_steps(root: Path | None = None) -> list[Step]:
    """全量测试的步骤表。骨架阶段为空——迁移票（#262）逐步填入。"""
    return []


TASKS: list[Task] = [
    Task("test", "全量测试：L0 静态分析 + L1 单元测试"
                 "（--l0-only / --allow-expected-failures）", task_test),
    Task("build-macos", "构建 macOS 应用包（build/Rhythm.app）",
         lambda argv: task_build.build_macos(argv), platform="macos"),
    Task("build-windows", "构建 Windows 应用",
         pending("#263", "scripts/build-windows.bat"), platform="windows"),
    Task("test-windows", "Windows 测试：L1 单元 + L2 截屏比对 + L3 冒烟",
         pending("#264", "powershell -File testing/run-windows.ps1"),
         platform="windows"),
    Task("check-no-emoji", "零 emoji 校验（硬性约定，覆盖被跟踪的全部文件）",
         pending("#265", "python3 scripts/check-no-emoji.py")),
    Task("compare-screenshots", "L2 截屏与 golden 的像素比对",
         pending("#265", "python3 testing/l2/windows/compare-screenshots.py")),
]


def find_task(name: str) -> Task | None:
    return next((t for t in TASKS if t.name == name), None)


def print_tasks() -> None:
    print("Rhythm 任务入口 — python3 scripts/tasks.py <任务> [参数...]")
    print()
    print("可用任务：")
    width = max(len(t.name) for t in TASKS)
    for t in TASKS:
        scope = "" if t.platform == "all" else f"  [{t.platform}]"
        print(f"  {t.name.ljust(width)}  {t.summary}{scope}")
    print()
    print("退出码：0 全绿 / 1 有步骤失败 / 2 用法错误")


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help", "help", "list"):
        print_tasks()
        return 0
    name, rest = argv[0], argv[1:]
    task = find_task(name)
    if task is None:
        print(f"未知任务: {name}", file=sys.stderr)
        print_tasks()
        return USAGE_ERROR
    return task.handler(rest)


if __name__ == "__main__":
    sys.exit(main())
