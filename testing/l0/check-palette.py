#!/usr/bin/env python3
"""L0: 配色生成物一致性（#219 组）。

testing/palette.json 是品牌配色的单一声明；本脚本以 scripts/gen-palette.py
重新生成，与提交的三处产物逐字节比对——漂移即红（人为改动被拦截）：

- macos/RhythmTheme/Theme.swift（主色 token + 来源徽标色）
- windows/Rhythm/Themes/Colors.xaml（深浅两套画刷）
- windows/Rhythm/Bridge/RhythmCore.h（来源徽标色表 + 胶囊底 alpha）

与 check-l10n-keys.py、check-ffi-contract.py 同一形状：导入生成器模块、
重新生成、逐字节比对。「漂移」在本仓库因此只有一种含义。

另外校验半透明 token 的「基色 + 不透明度」声明与 tokens 段记录的八位值一致
（#245，过渡期两者并存）。

用法：python3 testing/l0/check-palette.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl

_SCRIPTS = Path(__file__).resolve().parents[2] / "scripts"


def _load_generator():
    spec = importlib.util.spec_from_file_location("gen_palette", _SCRIPTS / "gen-palette.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def check_translucent(palette: dict, gen) -> list[str]:
    """半透明声明与八位值并存校验（#245）。"""
    problems: list[str] = []
    for token, variants in sorted(palette.get("translucent", {}).items()):
        recorded = palette.get("tokens", {}).get(token)
        if recorded is None:
            problems.append(f"translucent 段的 {token} 不在 tokens 段中")
            continue
        for appearance in ("dark", "light"):
            decl = variants.get(appearance)
            if not decl:
                problems.append(f"translucent {token}.{appearance} 缺声明")
                continue
            r, g, b = gen.rgb(decl["base"])
            computed = (r, g, b, gen.alpha_from_opacity(decl["opacity"]))
            expected = pl.from_hex(recorded[appearance])
            if computed != expected:
                problems.append(
                    f"translucent {token}.{appearance}: 声明算出 {pl.to_hex(computed)}"
                    f"（{decl['base']} @ {decl['opacity']}）"
                    f" != tokens 段 {pl.to_hex(expected)}"
                )
    return problems


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-palette", root))

    gen = _load_generator()
    palette = gen.load(str(pl.palette_path(root)))

    problems: list[str] = []
    generated = gen.generate(palette, str(root))
    for rel, expected in generated.items():
        if (root / rel).read_text(encoding="utf-8") != expected:
            problems.append(f"{rel} 与 palette.json 漂移——运行 python3 scripts/gen-palette.py")
    problems += check_translucent(palette, gen)

    if problems:
        print("FAIL — 配色生成物校验失败：")
        print("\n".join(f"  {p}" for p in problems))
        return 1
    print(f"OK：{len(generated)} 处配色生成物与 testing/palette.json 一致；"
          f"{len(palette.get('translucent', {}))} 个半透明 token 的声明与八位值一致。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
