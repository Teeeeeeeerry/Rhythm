#!/usr/bin/env python3
"""零 emoji 校验（硬性约定，见 CONTEXT.md 工作约定）。

检查范围是「被 git 跟踪的全部文件减去排除清单」：新增语言或文件类型默认被覆盖，
不需要有人记得回来补白名单（#224/#257，此前的扩展名白名单漏掉 43 个文件）。
跳过两类：EXCLUDED 列出的路径（有意声明的豁免）与内容探测判定的二进制文件。
发现 emoji 即报错退出（exit 1）。

用法：python3 scripts/tasks.py check-no-emoji
     python3 scripts/check_no_emoji.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "testing"))
import palette_lib as pl

# 匹配真实 emoji：pictographs / dingbats / misc symbols / 星星 / 变体选择符 /
# ZWJ 序列 / 播放控制符。箭头（->）等普通符号不在此列。
EMOJI_RE = re.compile(
    "[\U0001F000-\U0001FAFF\u2700-\u27BF\u2B00-\u2BFF\u2600-\u26FF\uFE0F\u200D"
    "\u25B6\u25C0\u23E9-\u23FA\u23F0\u23F3\u2B50\u2728\u2705\u274C\u274E"
    "\u2753\u2754\u2755\u2757\u2763\u2764\u2795\u2796\u2797\u2714\u2716\u271D"
    "\u2721\u3030\u303D\u3297\u3299]"
)

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-no-emoji", root))

    bad = 0
    scanned = 0
    for f in pl.tracked_files(root):
        path = root / f
        if pl.is_binary(path):
            continue
        try:
            with open(path, encoding="utf-8", errors="strict") as fh:
                for lineno, line in enumerate(fh, 1):
                    m = EMOJI_RE.search(line)
                    if m:
                        bad += 1
                        print(f"{f}:{lineno}: emoji U+{ord(m.group()):04X}: {line.rstrip()}")
        except (UnicodeDecodeError, OSError):
            # 非 UTF-8 或不可读文件跳过：校验不因编码错误中断。
            continue
        scanned += 1

    if bad:
        print(f"FAIL: {bad} emoji found (CONTEXT.md 工作约定: 零 emoji, 必须修掉)",
              file=sys.stderr)
        return 1
    print(f"OK: no emoji found ({scanned} files scanned)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
