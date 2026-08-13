#!/usr/bin/env python3
"""CLI 壳：实现与 import 入口在 compare_screenshots.py（下划线模块名，
便于 theme_switch.py 复用其 PNG 解码器；连字符文件名无法被 import）。

用法见 compare_screenshots.py 的 docstring：
    python3 testing/l2/windows/compare-screenshots.py \
        --actual build/artifacts --golden testing/l2/windows/golden
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from compare_screenshots import main  # noqa: E402

if __name__ == "__main__":
    sys.exit(main())
