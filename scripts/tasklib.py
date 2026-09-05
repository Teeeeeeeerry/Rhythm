#!/usr/bin/env python3
"""编排层共享实现（零依赖，仅 Python 3 stdlib）。

bash、批处理、PowerShell 三种脚本方言各自实现过同一组编排职责，于是各漂一次
（#221/#222/#223）。本模块把这四项收成单一实现，任务入口与后续迁移的构建、
测试任务都从这里取：

1. 仓库根定位          repo_root / logs_dir / log_path
2. 日志目录与输出转存  Tee / open_log / run 的 log 参数
3. 失败计数与退出码聚合 Failures
4. 子进程调用与错误传播 run / run_checked / StepFailed

日志文件名与落点沿用迁移前的约定：testing/logs/<名字>.log，每次运行覆盖。
CI 收集产物的路径因此不随迁移变化。
"""

from __future__ import annotations

import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

# ---------------------------------------------------------------------------
# 1) 仓库根定位
# ---------------------------------------------------------------------------

# 工作区清单是仓库根的判定标志（成员目录 rust-core 下没有它，不会误判）。
REPO_MARKERS = ("Cargo.toml",)

LOGS_REL = Path("testing") / "logs"

# 环境变量开关的真值写法（与迁移前的 shell 判定一致）
TRUTHY = ("1", "true", "yes", "on")

# 用法错误的退出码（未知任务或未知参数），与迁移前的脚本一致
USAGE_ERROR = 2


def repo_root(start: Path | None = None) -> Path:
    """从给定位置（默认本文件所在目录）向上找仓库根。"""
    cur = (Path(start) if start else Path(__file__).resolve().parent).resolve()
    for parent in (cur, *cur.parents):
        if any((parent / m).exists() for m in REPO_MARKERS):
            return parent
    raise RuntimeError(f"未找到仓库根目录（缺少 {', '.join(REPO_MARKERS)}）: {cur}")


def logs_dir(root: Path | None = None) -> Path:
    """日志目录 testing/logs/，不存在则创建。"""
    d = (root or repo_root()) / LOGS_REL
    d.mkdir(parents=True, exist_ok=True)
    return d


def log_path(name: str, root: Path | None = None) -> Path:
    """单个日志文件 testing/logs/<name>.log（每次运行覆盖写）。"""
    return logs_dir(root) / f"{name}.log"


# ---------------------------------------------------------------------------
# 2) 日志目录与输出转存
# ---------------------------------------------------------------------------

class Tee:
    """文件对象：同一份输出同时写多个流（终端 + 日志文件）。"""

    def __init__(self, *streams):
        self.streams = streams

    def write(self, data: str) -> int:
        for s in self.streams:
            s.write(data)
            s.flush()
        return len(data)

    def flush(self) -> None:
        for s in self.streams:
            s.flush()


def open_log(path: Path) -> Path:
    """把 stdout/stderr 同时写入终端与日志文件，返回日志路径。"""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    handle = open(path, "w", encoding="utf-8")
    sys.stdout = Tee(sys.stdout, handle)
    sys.stderr = Tee(sys.stderr, handle)
    print(f"[log] 完整输出已写入 -> {path}")
    return path


# ---------------------------------------------------------------------------
# 3) 失败计数与退出码聚合
# ---------------------------------------------------------------------------

class Failures:
    """按步累计失败，最后聚合成整体退出码。

    严格模式是默认（#144）：任一步红则非零退出。容错只能由调用方显式打开
    （allow_expected 为真），不会因为「历史上有几步预期红」悄悄变回默认。
    """

    def __init__(self) -> None:
        self.failed: list[str] = []
        self.passed: list[str] = []

    def record(self, step: str, ok: bool) -> bool:
        """登记一步的结果，返回该步是否通过。"""
        (self.passed if ok else self.failed).append(step)
        return ok

    def record_code(self, step: str, code: int) -> bool:
        """按子进程退出码登记一步（零为通过）。"""
        return self.record(step, code == 0)

    @property
    def count(self) -> int:
        return len(self.failed)

    def exit_code(self, allow_expected: bool = False) -> int:
        """整体退出码：有失败即 1；仅在显式豁免时降为 0。"""
        if not self.failed:
            return 0
        return 0 if allow_expected else 1

    def summary(self) -> str:
        if not self.failed:
            return f"===== 全部通过（{len(self.passed)} 步）====="
        return (f"===== {self.count} 步失败 ====="
                + "".join(f"\n  - {s}" for s in self.failed))


# ---------------------------------------------------------------------------
# 4) 子进程调用与错误传播
# ---------------------------------------------------------------------------

class StepFailed(RuntimeError):
    """子进程非零退出（或无法启动）。code 为退出码，命令保留在 cmd。"""

    def __init__(self, cmd: list[str], code: int, message: str | None = None):
        self.cmd = list(cmd)
        self.code = code
        super().__init__(message or f"命令失败（退出码 {code}）: {' '.join(self.cmd)}")


def run(
    cmd: list[str],
    *,
    cwd: Path | None = None,
    log: Path | None = None,
    env: dict[str, str] | None = None,
    echo: bool = True,
) -> int:
    """跑一条命令，返回退出码；输出同时写终端与 log（若给）。

    失败不抛异常——由调用方交给 Failures 累计。可执行文件缺失同样折算成
    非零退出码（127），不让 FileNotFoundError 穿透到入口。
    """
    if echo:
        print(f">>> {' '.join(str(c) for c in cmd)}"
              + (f"  (cwd={cwd})" if cwd else ""))
    merged_env = {**os.environ, **env} if env else None
    sink = open(log, "w", encoding="utf-8") if log else None
    try:
        try:
            proc = subprocess.Popen(
                [str(c) for c in cmd],
                cwd=str(cwd) if cwd else None,
                env=merged_env,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                encoding="utf-8",
                errors="replace",
                bufsize=1,
            )
        except OSError as exc:
            message = f"无法执行 {cmd[0]}: {exc}"
            print(message, file=sys.stderr)
            if sink:
                sink.write(message + "\n")
            return 127
        with proc:
            assert proc.stdout is not None
            for line in proc.stdout:
                sys.stdout.write(line)
                sys.stdout.flush()
                if sink:
                    sink.write(line)
        return proc.returncode
    finally:
        if sink:
            sink.close()


def run_checked(cmd: list[str], **kwargs) -> None:
    """跑一条命令，非零退出即抛 StepFailed（失败必须传播，不静默吞掉）。"""
    code = run(cmd, **kwargs)
    if code != 0:
        raise StepFailed([str(c) for c in cmd], code)


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
    tally = Failures()
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
    smoke: bool = False


def parse_test_flags(argv: list[str], env: dict[str, str] | None = None,
                     allow_smoke: bool = False) -> TestFlags:
    """解析测试入口的开关（命令行与环境变量两种形式）。

    语义与迁移前的脚本一致：ALLOW_EXPECTED_FAILURES=1 等价于
    --allow-expected-failures；--smoke 只在有冒烟段的平台上可用；
    未知参数按用法错误处理。
    """
    env = os.environ if env is None else env
    flags = TestFlags(
        allow_expected=str(env.get("ALLOW_EXPECTED_FAILURES", "0")).lower() in TRUTHY
    )
    supported = ["--l0-only", "--allow-expected-failures"]
    if allow_smoke:
        supported.append("--smoke")
    for arg in argv:
        if arg == "--l0-only":
            flags.l0_only = True
        elif arg == "--allow-expected-failures":
            flags.allow_expected = True
        elif arg == "--smoke" and allow_smoke:
            flags.smoke = True
        else:
            print(f"未知参数: {arg}（支持 {' / '.join(supported)}）", file=sys.stderr)
            raise SystemExit(USAGE_ERROR)
    return flags
