#!/usr/bin/env python3
"""L0: 编排层方言回归拦截（#221）。

编排层的三种脚本方言（bash、批处理、PowerShell）已在 #258-#266 全部退场，
构建与测试只走 `python3 scripts/tasks.py <任务>`。但「不要再加一份 shell」
是一条只写在文档里的约定，文档拦不住新增文件——本脚本把它变成会报红的检查。

新增一个 .sh / .bat / .ps1 即失败。确有必要的例外写进 ALLOWED 并附理由，
让豁免留痕而不是靠没人注意。

用法：python3 testing/l0/check-orchestration-dialects.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl

# 被收编的方言：出现即报红。
DIALECT_SUFFIXES = (".sh", ".bash", ".zsh", ".bat", ".cmd", ".ps1", ".psm1")

# 例外登记：{相对路径: 理由}。留空表示当前一处例外都没有。
ALLOWED: dict[str, str] = {}

ENTRY = "python3 scripts/tasks.py"


def offenders(root: Path) -> list[str]:
    return [f for f in pl.tracked_files(root, DIALECT_SUFFIXES) if f not in ALLOWED]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-orchestration-dialects", root))

    bad = offenders(root)
    if bad:
        print("FAIL — 出现已收编的脚本方言（编排层只用 Python，#221）：")
        for f in bad:
            print(f"  {f}")
        print(f"请改写为 Python 并挂到任务入口（{ENTRY}）；")
        print("确有必要的例外写进本脚本的 ALLOWED 并附理由。")
        return 1

    scanned = len(pl.tracked_files(root))
    note = f"，另有 {len(ALLOWED)} 处登记例外" if ALLOWED else ""
    print(f"OK：{scanned} 个被跟踪文件中无 bash / 批处理 / PowerShell 脚本{note}。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
