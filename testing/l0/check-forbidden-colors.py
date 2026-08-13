#!/usr/bin/env python3
"""L0: 禁止裸色扫描。

扫描双端视图代码（macOS Views/*.swift、Windows Views/*.xaml）：
任何非 token 的颜色引用（hex、`Color.*`、`NSColor.*`、裸 Brush 色值）即失败，
除非命中 palette.json whitelist 段（合法系统组件/功能性颜色）。

Token 定义处（macos/.../Theme.swift、windows/.../Colors.xaml、
Bridge/RhythmCore.h）自动豁免——它们就是 token 的来源。

用法：python3 testing/l0/check-forbidden-colors.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl

# 品牌 token 形态（Swift 为 .rhythmXxx，XAML 为 ThemeResource RhythmXxxBrush）
TOKEN_RE = re.compile(r"[rR]hythm[A-Z]\w+")

# macOS Swift 视图：裸色形态
MACOS_PATTERNS: list[tuple[str, re.Pattern]] = [
    ("hex 字面量", re.compile(r"#(?:[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})\b")),
    ("Color(...) 构造", re.compile(r"\bColor\s*\(")),
    ("NSColor 引用", re.compile(r"\bNSColor\.[a-zA-Z]+")),
    ("系统色名", re.compile(
        r"\.(?:red|orange|yellow|green|blue|purple|pink|gray|black|"
        r"white|clear|secondary|tertiary|primary|accentColor|system[A-Z]\w*)\b"
    )),
]

# Windows XAML 视图：裸色形态
WINDOWS_PATTERNS: list[tuple[str, re.Pattern]] = [
    ("hex 字面量", re.compile(r"#(?:[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})\b")),
    ("命名颜色/裸 Brush", re.compile(
        r"(?:Background|Foreground|Fill|Stroke|Color)\s*=\s*\"[A-Za-z]+\""
    )),
    ("SolidColorBrush 定义", re.compile(r"<SolidColorBrush[^>]*Color=")),
]

SKIP_DIRS = ("Views/", "Themes/", "Bridge/", "Views/../")  # 保留结构


def scan_text(path: Path, patterns, whitelist_regexes: list[re.Pattern],
              name: str) -> list[str]:
    text = path.read_text(encoding="utf-8")
    issues: list[str] = []
    for label, pattern in patterns:
        for m in pattern.finditer(text):
            # 行内截取上下文（Swift 注释 `//` 之后为文档性内容，不算代码引用）
            line_start = text.rfind("\n", 0, m.start()) + 1
            line_end = text.find("\n", m.end())
            line = text[line_start : line_end if line_end != -1 else len(text)]
            code = line.split("//", 1)[0]
            if not code.strip():
                continue  # 纯注释行
            if TOKEN_RE.search(code):
                continue  # 同一行出现品牌 token 的属不属于裸色（如 .rhythmX 会被系统色名误捕时）
            # 白名单：命中任一豁免正则即跳过
            if any(w.search(code) for w in whitelist_regexes):
                continue
            issues.append(f"  {path}（{name}）: {label} → {code.strip()[:100]}")
    return issues


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-forbidden-colors", root))
    palette = pl.load_palette(repo_root=root)
    whitelist = palette.get("whitelist", {})
    mac_whitelist = [re.compile(p) for p in whitelist.get("macos", [])]
    win_whitelist = [re.compile(p) for p in whitelist.get("windows", [])]

    # macOS：Views 下所有 .swift（排除子目录里的非视图? Views 全部是视图）
    mac_views = [p for p in pl.swift_view_files(root)
                 if p.name not in ("Theme.swift",)]
    win_views = pl.xaml_view_files(root)

    issues: list[str] = []
    for p in mac_views:
        issues += scan_text(p, MACOS_PATTERNS, mac_whitelist, "macOS")
    for p in win_views:
        issues += scan_text(p, WINDOWS_PATTERNS, win_whitelist, "Windows")

    if issues:
        print("FAIL — 视图代码出现非 token 颜色引用（裸色）：")
        print("\n".join(issues))
        print("请改用品牌 token（.rhythm*）；确属系统组件请在 "
              "palette.json whitelist 段登记豁免。")
        return 1
    print(f"OK：扫描 {len(mac_views)} 个 Swift 视图 + {len(win_views)} 个 XAML 视图，"
          "无非 token 颜色引用。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
