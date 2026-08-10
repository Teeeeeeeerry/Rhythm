#!/usr/bin/env python3
"""L0: 视图级 token 覆盖率检查。

每个受品牌化视图至少引用 1 个品牌 token；新增视图无 token 即失败。
已知缺口报警：
- F2: Windows SidebarView.xaml 零品牌化（当前视为缺口列出，修复后自动消失）。

另输出逐视图使用点统计（替代 §2.2 手工维护清单，由扫描器生成）。

用法：python3 docs/testing/l0/check-token-coverage.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl

# 品牌 token 形态：Swift 为 .rhythmXxx，XAML 为 ThemeResource RhythmXxxBrush。
# 注意排除 `RhythmCore`（import RhythmCore 模块名，非 token）。
TOKEN_RE = re.compile(r"rhythm[A-Z]\w+|Rhythm[A-Z]\w*Brush")


def count_tokens(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    return sorted(set(TOKEN_RE.findall(text)))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 docs/testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-token-coverage", root))
    # 受品牌化视图：排除 token 定义（Theme.swift）与非 UI 文件（TrayManager）
    mac_views = [p for p in pl.swift_view_files(root)
                 if p.name not in ("Theme.swift", "ContentView.swift",
                                   "TrayManager.swift")]
    win_views = pl.xaml_view_files(root)

    failures: list[str] = []
    print("== macOS ==")
    for p in mac_views:
        tokens = count_tokens(p)
        rel = p.relative_to(root)
        if not tokens:
            failures.append(f"{rel}: 0 个 token（新增视图必须引用品牌色）")
        else:
            print(f"  {rel}: {len(tokens)} 个 → {', '.join(tokens)}")

    print("== Windows ==")
    for p in win_views:
        tokens = count_tokens(p)
        rel = p.relative_to(root)
        if not tokens:
            if p.name == "SidebarView.xaml":
                failures.append(f"{rel}: 0 个 token（F2 — 覆盖率缺口，待品牌化）")
            else:
                failures.append(f"{rel}: 0 个 token（新增视图必须引用品牌色）")
        else:
            print(f"  {rel}: {len(tokens)} 个 → {', '.join(tokens)}")

    if failures:
        print("\nFAIL — 存在覆盖率缺口：")
        print("\n".join(f"  {f}" for f in failures))
        return 1
    print(f"\nOK：{len(mac_views)} 个 macOS 视图 + {len(win_views)} 个 Windows 视图"
          "全部引用品牌 token。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
