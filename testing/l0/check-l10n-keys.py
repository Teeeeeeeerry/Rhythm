#!/usr/bin/env python3
"""L0: L10n 文案键表校验（#185）。

1. 键表结构：contracts/l10n-keys.json 每个键必须同时有 zh 与 en 字段
   （platform 差异条目同；文案为空是合法设计，如英文分支回退原始 detail）。
2. 生成物一致性：以 scripts/gen-l10n.py 重新生成，与提交的
   macos/Rhythm/Models/L10nKeys.swift、windows/Rhythm/Bridge/L10nKeys.h
   逐字节比对——漂移即红（人为漂移被拦截）。
3. Windows L10n.h 的 Key() 映射表（L10N_ENTRY 列表）必须恰好覆盖
   windows 平台键集——漏键即红。

用法：python3 testing/l0/check-l10n-keys.py [--root PATH]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import importlib.util

_SCRIPTS = Path(__file__).resolve().parents[2] / "scripts"


def _load_script(name: str):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), _SCRIPTS / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


gen_l10n = _load_script("gen-l10n")

SWIFT_OUT = "macos/Rhythm/Models/L10nKeys.swift"
CPP_OUT = "windows/Rhythm/Bridge/L10nKeys.h"
WINDOWS_L10N_H = "windows/Rhythm/L10n.h"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    args = ap.parse_args()

    root = Path(__file__).resolve().parent.parent.parent
    problems: list[str] = []

    table = gen_l10n.load()

    # 1) 键表结构
    for key, entry in table["keys"].items():
        for field in ("zh", "en"):
            if field not in entry:
                problems.append(f"键 {key} 缺 {field} 字段")

    # 2) 生成物一致性（重新生成后比对）
    swift = gen_l10n.gen_swift(table)
    cpp = gen_l10n.gen_cpp(table)
    swift_path = root / SWIFT_OUT
    cpp_path = root / CPP_OUT
    if swift_path.read_text(encoding="utf-8") != swift:
        problems.append(f"L10nKeys.swift 与键表漂移——运行 python3 scripts/gen-l10n.py")
    if cpp_path.read_text(encoding="utf-8") != cpp:
        problems.append(f"L10nKeys.h 与键表漂移——运行 python3 scripts/gen-l10n.py")

    # 3) Windows L10n.h 的 Key() 映射覆盖 windows 平台键
    l10n_h = (root / WINDOWS_L10N_H).read_text(encoding="utf-8")
    listed = set(re.findall(r"^\s+L10N_ENTRY\(([a-z0-9_]+)\)", l10n_h, re.M))
    expected = set(gen_l10n.entries_for(table, "windows").keys())
    missing = expected - listed
    extra = listed - expected
    if missing:
        problems.append(f"L10n.h Key() 映射缺 {len(missing)} 个键: {sorted(missing)[:5]}...")
    if extra:
        problems.append(f"L10n.h Key() 映射多出 {len(extra)} 个非 windows 键: {sorted(extra)[:5]}...")

    if problems:
        print("L10n 键表校验失败：")
        print("\n".join(problems))
        return 1
    print(f"OK：键表结构完整、双端生成物一致、Windows 映射覆盖 "
          f"（{len(table['keys'])} 键 / windows {len(expected)} 键）。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
