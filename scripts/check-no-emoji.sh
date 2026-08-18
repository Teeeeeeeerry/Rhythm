#!/bin/bash
# 零 emoji 校验（硬性约定，见 CONTEXT.md 工作约定）：
# 扫描所有被 git 跟踪的文本文件，发现 emoji 即报错退出（exit 1）。
# 用法：scripts/check-no-emoji.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

python3 - <<'PY'
import re
import subprocess
import sys

# 匹配真实 emoji：pictographs / dingbats / misc symbols / 星星 / 变体选择符 /
# ZWJ 序列 / 播放控制符。箭头（->）等普通符号不在此列。
pat = re.compile(
    "[\U0001F000-\U0001FAFF\u2700-\u27BF\u2B00-\u2BFF\u2600-\u26FF\uFE0F\u200D"
    "\u25B6\u25C0\u23E9-\u23FA\u23F0\u23F3\u2B50\u2728\u2705\u274C\u274E"
    "\u2753\u2754\u2755\u2757\u2763\u2764\u2795\u2796\u2797\u2714\u2716\u271D"
    "\u2721\u3030\u303D\u3297\u3299]"
)
exts = (
    ".md", ".rs", ".swift", ".txt", ".toml", ".plist", ".m", ".h", ".js",
    ".ts", ".json", ".bat", ".sh", ".yml", ".yaml", ".css", ".html",
    ".xcconfig", ".strings",
)
files = subprocess.check_output(["git", "ls-files"], text=True).splitlines()
files = [f for f in files if f.endswith(exts)]

bad = 0
for f in files:
    try:
        with open(f, encoding="utf-8", errors="strict") as fh:
            for lineno, line in enumerate(fh, 1):
                m = pat.search(line)
                if m:
                    bad += 1
                    print(f"{f}:{lineno}: emoji U+{ord(m.group()):04X}: {line.rstrip()}")
    except (UnicodeDecodeError, OSError):
        # 二进制或不可读文件跳过（git ls-files 已排除构建产物）。
        pass

if bad:
    print(f"FAIL: {bad} emoji found (CONTEXT.md 工作约定: 零 emoji, 必须修掉)", file=sys.stderr)
    raise SystemExit(1)
print("OK: no emoji found")
PY
