#!/usr/bin/env python3
"""L0: 文档色值漂移检查。

扫描仓库文档（README.md / README.en.md / docs/*.md / 问题报告.md 等）中出现的
hex 色值：必须属于 palette.json tokens/sources/backgrounds 色值集合。
文档出现 palette 之外的色值即失败（防止文档与 token 漂移）。

用法：python3 docs/testing/l0/check-doc-drift.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl

HEX_RE = re.compile(r"#(?:[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})\b")
DOC_PATTERNS = ("*.md",)


def collect_docs(root: Path) -> list[Path]:
    docs = list((root / "docs").glob("*.md"))
    for fname in ("README.md", "README.en.md", "问题报告.md"):
        p = root / fname
        if p.exists():
            docs.append(p)
    return sorted(set(docs))


def known_colors(palette: dict) -> set[str]:
    out = set()
    for group in ("tokens", "sources"):
        for v in palette.get(group, {}).values():
            for c in v.values():
                if c:
                    out.add(c.upper())
    for v in palette.get("backgrounds", {}).values():
        for appearance in v.values():
            for c in appearance.values():
                out.add(c.upper())
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 docs/testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-doc-drift", root))
    palette = pl.load_palette(repo_root=root)
    known = known_colors(palette)

    problems: list[str] = []
    seen: set[str] = set()
    for doc in collect_docs(root):
        text = doc.read_text(encoding="utf-8")
        for m in HEX_RE.finditer(text):
            hex_upper = m.group(0).upper()
            if hex_upper in known:
                continue
            line_start = text.rfind("\n", 0, m.start()) + 1
            line_end = text.find("\n", m.end())
            line = text[line_start : line_end if line_end != -1 else len(text)].strip()
            problems.append(f"  {doc.relative_to(root)}: {m.group(0)}"
                            f"（palette 未收录）{line[:60]}")

    if problems:
        print("FAIL — 文档出现 palette.json 之外的色值（与 token 漂移）：")
        print("\n".join(problems))
        print("请改为文档内引用品牌 token 名，或确认后加入 palette.json。")
        return 1
    print(f"OK：{len(collect_docs(root))} 份文档中的色值全部属于 palette.json 集合。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
