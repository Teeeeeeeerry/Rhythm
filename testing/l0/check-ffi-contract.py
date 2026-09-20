#!/usr/bin/env python3
"""L0: FFI 数据契约生成物一致性（#180/#185）——契约的第一道门。

contracts/ffi-contract.json 是跨 seam 字段/枚举的单一声明；本脚本以
scripts/gen-ffi-bindings.py 重新生成，与提交的
macos/Rhythm/Models/GeneratedCodec.swift、windows/Rhythm/Bridge/GeneratedCodec.h
逐字节比对——漂移即红（人为漂移被拦截）。

这一道比的是文本，按结构就看不见「生成器本身产不出可编译代码」：生成器与提交物
一起错的时候它照样绿（#323）。第二道门补上这一格——生成物由 CMake 目标
RhythmGeneratedCodec 单独编译一次（#369），两道都过才算契约没漂。

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

    # #323：Swift 侧的生成范围是「默认全生成 + 一张具名的退出清单」。清单里留着
    # 契约已经不再声明的对象，就说明有人删了对象却没回来看这张清单——下一个读它的人
    # 会以为那个对象还在契约里、只是走了 macOS 自己的路径。
    stale = gen.SWIFT_SKIP - set(gen.contract_object_keys(schema))
    if stale:
        problems.append(
            "scripts/gen-ffi-bindings.py 的 SWIFT_SKIP 里有契约未声明的对象："
            + "、".join(sorted(stale)))

    if problems:
        print("FFI 契约生成物校验失败：")
        print("\n".join(problems))
        return 1
    print("OK：FFI 契约生成物与 contracts/ffi-contract.json 一致。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
