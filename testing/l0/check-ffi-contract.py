#!/usr/bin/env python3
"""L0: FFI 数据契约生成物一致性（#180/#185）。

contracts/ffi-contract.json 是跨 seam 字段/枚举的单一声明；本脚本以
scripts/gen-ffi-bindings.py 重新生成，与提交的
macos/Rhythm/Models/GeneratedCodec.swift、windows/Rhythm/Bridge/GeneratedCodec.h
逐字节比对——漂移即红（人为漂移被拦截）。

用法：python3 testing/l0/check-ffi-contract.py
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def _load_script(name: str):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), ROOT / "scripts" / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


def main() -> int:
    gen = _load_script("gen-ffi-bindings")
    schema = gen.load_schema()
    swift = gen.gen_swift(schema)
    cpp = gen.gen_cpp(schema)

    problems: list[str] = []
    for rel, generated in (
        ("macos/Rhythm/Models/GeneratedCodec.swift", swift),
        ("windows/Rhythm/Bridge/GeneratedCodec.h", cpp),
    ):
        path = ROOT / rel
        if path.read_text(encoding="utf-8") != generated:
            problems.append(f"{rel} 与契约漂移——运行 python3 scripts/gen-ffi-bindings.py")

    if problems:
        print("FFI 契约生成物校验失败：")
        print("\n".join(problems))
        return 1
    print("OK：FFI 契约生成物与 contracts/ffi-contract.json 一致。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
