#!/usr/bin/env python3
"""L0: Windows 视图只经 AppState 取能力（#353，#321 收尾）。

#321 把视图绕过 AppState 的几条路径逐一收拢成 AppState 的方法：创建歌单
（#347/#348）、解析器状态文案（#349/#350）、歌单导出（#351~#353）。收拢之后
「视图只依赖 AppState」仍只是一条写在文档里的约定——本脚本把它变成会报红的检查：
windows/Rhythm/Views/ 下的源码一旦直接调用下层模块即失败。

被拦下的调用与包含：
  - 下层模块的头文件（Bridge/ 下的任何头、rhythm_core.h；#321：视图的包含列表里只有 AppState）
  - FFI 裸函数（rhythm_xxx(...)）
  - Bridge 的导出层（ExportM3U8(...)，文案访问器 L10n::ExportM3U8() 除外）
  - 解析器（Resolver::...）
  - AppState 持有的下层句柄（->Library / .Library / ->Coordinator / .Coordinator）

注释不计。确有必要的例外写进 ALLOWED 并附理由，让豁免留痕。

用法：python3 testing/l0/check-view-seams.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl

VIEWS_DIR = Path("windows") / "Rhythm" / "Views"
SOURCE_SUFFIXES = (".cpp", ".h")

# (理由, 正则)：每条对应 #321 收拢过的一类绕过路径。
RULES: list[tuple[str, re.Pattern[str]]] = [
    ("下层模块头文件（只包含 AppState.h）",
     re.compile(r'#\s*include\s*[<"](?:Bridge/|rhythm_core\.h)')),
    ("FFI 裸函数", re.compile(r"\brhythm_[a-z0-9_]+\s*\(")),
    ("FFI 导出层（改调 AppState::ExportPlaylist）",
     re.compile(r"(?<!L10n::)\bExportM3U8\s*\(\s*[^)\s]")),
    ("解析器（改调 AppState::ResolverStatusText）", re.compile(r"\bResolver::")),
    ("资料库句柄（改调 AppState 的方法）", re.compile(r"(->|\.)Library\b")),
    ("协调器句柄（改调 AppState 的方法）", re.compile(r"(->|\.)Coordinator\b")),
]

# 例外登记：{"相对路径:行号": 理由}。留空表示当前一处例外都没有。
ALLOWED: dict[str, str] = {}

LINE_COMMENT = re.compile(r"//.*$")


def view_sources(root: Path) -> list[Path]:
    base = root / VIEWS_DIR
    if not base.is_dir():
        return []
    return sorted(p for p in base.rglob("*") if p.suffix in SOURCE_SUFFIXES)


def offenders(root: Path) -> list[str]:
    found = []
    for path in view_sources(root):
        rel = path.relative_to(root).as_posix()
        text = path.read_text(encoding="utf-8", errors="replace")
        for number, line in enumerate(text.splitlines(), start=1):
            code = LINE_COMMENT.sub("", line)
            for reason, pattern in RULES:
                where = f"{rel}:{number}"
                if pattern.search(code) and where not in ALLOWED:
                    found.append(f"{where}  {reason}：{line.strip()}")
    return found


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-view-seams", root))

    bad = offenders(root)
    if bad:
        print("FAIL — Windows 视图绕过 AppState 直接调用下层模块（#321/#353）：")
        for item in bad:
            print(f"  {item}")
        print("请给 AppState 加方法、视图改调它；确有必要的例外写进本脚本的 ALLOWED 并附理由。")
        return 1

    scanned = len(view_sources(root))
    note = f"，另有 {len(ALLOWED)} 处登记例外" if ALLOWED else ""
    print(f"OK：{scanned} 个 Windows 视图源文件只经 AppState 取能力{note}。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
