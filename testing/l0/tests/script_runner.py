#!/usr/bin/env python3
"""L0 自测启动被测脚本的唯一入口（#433）。

被测脚本打印中文。子进程的标准流默认跟随系统区域（如 cp1252），不约定编码时
子进程一打印就抛 UnicodeEncodeError，自测读到的是空输出。约定取自编排层
scripts/tasklib.py：读管道的一方按 UTF-8 解码，也要求子进程按 UTF-8 写。
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))

from tasklib import CHILD_ENCODING_ENV  # noqa: E402


def run_script(*argv: str) -> subprocess.CompletedProcess[str]:
    """用当前解释器跑一个脚本，捕获输出（UTF-8 两端一致）。"""
    return subprocess.run(
        [sys.executable, *argv],
        capture_output=True, text=True, encoding="utf-8",
        env={**os.environ, **CHILD_ENCODING_ENV},
    )
