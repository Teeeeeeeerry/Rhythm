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

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

import task_build  # noqa: E402
import task_test  # noqa: E402
import tasklib  # noqa: E402

USAGE_ERROR = tasklib.USAGE_ERROR


# ---------------------------------------------------------------------------
# 步骤与退出码聚合（实现在 scripts/tasklib.py，入口只声明约定）
# ---------------------------------------------------------------------------

Step = tasklib.Step
select_steps = tasklib.select_steps
run_steps = tasklib.run_steps
TestFlags = tasklib.TestFlags
parse_test_flags = tasklib.parse_test_flags

# ---------------------------------------------------------------------------
# 任务注册表
# ---------------------------------------------------------------------------

@dataclass
class Task:
    name: str
    summary: str
    handler: Callable[[list[str]], int]
    platform: str = "all"


def _run_module(rel: str, argv: list[str]) -> int:
    """按路径加载一个校验模块并跑它的 main（模块名可导入，不再需要命令行外壳）。"""
    import importlib.util

    path = tasklib.repo_root() / rel
    spec = importlib.util.spec_from_file_location(path.stem, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[path.stem] = module
    sys.path.insert(0, str(path.parent))
    spec.loader.exec_module(module)
    saved, sys.argv = sys.argv, [str(path), *argv]
    try:
        return module.main()
    finally:
        sys.argv = saved


TASKS: list[Task] = [
    Task("test", "全量测试：macOS 为 L0 静态分析 + L1 单元测试，"
                 "Windows 为 L1 单元 + L2 截屏比对 + L3 冒烟（--smoke）",
         lambda argv: task_test.run_full_suite(argv)),
    Task("build", "构建本平台应用：macOS 为 build/Rhythm.app，"
                  "Windows 为 build/windows/Release/Rhythm.exe",
         lambda argv: task_build.build_app(argv)),
    Task("check-no-emoji", "零 emoji 校验（硬性约定，覆盖被跟踪的全部文件）",
         lambda argv: _run_module("scripts/check_no_emoji.py", argv)),
    Task("compare-screenshots", "L2 截屏与 golden 的像素比对",
         lambda argv: _run_module("testing/l2/windows/compare_screenshots.py", argv)),
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
