#!/usr/bin/env python3
"""L0: L10n 文案键表校验（#185）。

1. 键表结构：contracts/l10n-keys.json 每个键必须同时有 zh 与 en 字段
   （platform 差异条目同；文案为空是合法设计，如英文分支回退原始 detail）。
2. 生成物一致性：以 scripts/gen-l10n.py 重新生成，与提交的
   macos/Rhythm/Models/L10nKeys.swift、windows/Rhythm/Bridge/L10nKeys.h
   逐字节比对——漂移即红（人为漂移被拦截）。
3. Windows L10n.h 的 Key() 映射表（L10N_ENTRY 列表）必须恰好覆盖
   windows 平台键集——漏键即红。
4. 访问器与调用面（#370）：调用方依赖的是具名访问器，而不是键名表。
   #371 起访问器由键表生成（windows/Rhythm/L10nAccessors.h），与重新生成的结果逐字节比对；
   a) Windows 代码与测试里调用的每个 `L10n::X(...)` 都必须有定义——
      被引用却不存在的访问器（如曾被删掉的 TrayQuit）在此报红，不必等编译器。
      「有定义」只认 L10n.h 与 L10nAccessors.h 的 `namespace L10n` 内的 inline 函数，
      别处的同名函数不能冒充访问器（#410）；
   b) 每个 windows 键都必须出现在至少一个访问器的函数体里——有文案却没有任何
      访问器取值的键在此报红。它不检查访问器是否被调用：无人调用的访问器照样通过
      （#410）。

5. 平台差异键分端（#372）：两端的期望键集按 platform 字段分别计算，并直接从提交的
   生成物里读出实际键集比对——macOS 专用键不得出现在 Windows 的取值与访问器里，
   反之亦然。逐字节比对只能证明「生成物等于生成器的产出」，生成器自己漏筛时两边
   一起错；这一项不经过生成器，独立兜底。

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

# 生成物路径只在生成器声明（#410）。
SCHEMA = gen_l10n.SCHEMA_REL
SWIFT_OUT = gen_l10n.SWIFT_OUT_REL
CPP_OUT = gen_l10n.CPP_OUT_REL
ACCESSORS_OUT = gen_l10n.ACCESSORS_OUT_REL
WINDOWS_L10N_H = "windows/Rhythm/L10n.h"

# Windows 侧的访问器定义与调用都在这两处（第三方 vendor 目录除外）。
WINDOWS_SOURCE_DIRS = ("windows/Rhythm", "windows/tests")
VENDOR_PARTS = {"vendor"}

# L10n.h 里的基础设施函数：语言解析、取值、填充、渲染。它们不是访问器——
# 取值表 Key() 的函数体里列着全部键，算进去会让「键无访问器」永远查不出来。
INFRASTRUCTURE = {
    "isChineseComputed", "OverrideLanguage", "SetOverrideLanguage", "IsChinese",
    "Key", "Fill", "RenderMessageSpec",
}

DEFINITION_RE = re.compile(r"^inline\s+[^\n(;]*?\b([A-Za-z_]\w*)\s*\(", re.M)
CALL_RE = re.compile(r"\bL10n::([A-Za-z_]\w*)\s*\(")
KEY_LITERAL_RE = re.compile(r'\bKey\("([a-z0-9_]+)"\)')


def windows_sources(root: Path) -> list[Path]:
    files: list[Path] = []
    for rel in WINDOWS_SOURCE_DIRS:
        base = root / rel
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if path.suffix in (".h", ".cpp") and not VENDOR_PARTS & set(path.parts):
                files.append(path)
    return files


L10N_NAMESPACE_RE = re.compile(r"^namespace L10n \{$(.*?)^\} // namespace L10n$", re.M | re.S)


def l10n_namespace_text(header: str) -> str:
    """头文件里全部 `namespace L10n { ... }` 块的正文（#410）。"""
    return "\n".join(L10N_NAMESPACE_RE.findall(header))


def accessor_bodies(header: str) -> dict[str, str]:
    """L10n.h 里每个 inline 函数名 -> 它的定义文本（到下一个 inline 定义为止）。"""
    matches = list(DEFINITION_RE.finditer(header))
    bodies: dict[str, str] = {}
    for i, m in enumerate(matches):
        end = matches[i + 1].start() if i + 1 < len(matches) else len(header)
        bodies[m.group(1)] = bodies.get(m.group(1), "") + header[m.start():end]
    return bodies


def accessor_problems(root: Path, expected_keys: set[str]) -> list[str]:
    """访问器集合与调用面、键表的差额（#370）。"""
    problems: list[str] = []
    # 访问器定义在两处：生成的 L10nAccessors.h（#371）与 L10n.h 里带参数的手写函数。
    header = (root / WINDOWS_L10N_H).read_text(encoding="utf-8")
    accessors = root / ACCESSORS_OUT
    if accessors.is_file():
        header += "\n" + accessors.read_text(encoding="utf-8")

    # 定义只认两份头文件的 namespace L10n 内（#410）：别处的同名 inline 函数
    # 不能掩盖缺失的访问器。
    defined = set(DEFINITION_RE.findall(l10n_namespace_text(header)))
    called: dict[str, str] = {}
    for path in windows_sources(root):
        text = path.read_text(encoding="utf-8", errors="replace")
        rel = path.relative_to(root).as_posix()
        for name in CALL_RE.findall(text):
            called.setdefault(name, rel)

    for name in sorted(set(called) - defined):
        problems.append(f"访问器 L10n::{name}() 被 {called[name]} 调用，但没有定义")

    reachable: set[str] = set()
    for name, body in accessor_bodies(header).items():
        if name not in INFRASTRUCTURE:
            reachable.update(KEY_LITERAL_RE.findall(body))
    orphans = sorted(expected_keys - reachable)
    if orphans:
        problems.append(f"{len(orphans)} 个 windows 键没有任何访问器取用（到不了界面）: "
                        + ", ".join(orphans))
    return problems


SWIFT_KEY_RE = re.compile(r'^\s+"([a-z0-9_]+)": \(zh:', re.M)
CPP_VALUE_KEY_RE = re.compile(r"\bL10nKeys_zh_([a-z0-9_]+)\(")


def platform_key_problems(root: Path, table: dict) -> list[str]:
    """两端生成物里的实际键集与各自期望键集比对（#372）。"""
    expected = {p: set(gen_l10n.entries_for(table, p)) for p in ("macos", "windows")}
    other = {"macos": "windows", "windows": "macos"}
    label = {"macos": "macOS", "windows": "Windows"}

    def read(rel: str) -> str:
        path = root / rel
        return path.read_text(encoding="utf-8") if path.is_file() else ""

    actual = {
        (SWIFT_OUT, "macos"): set(SWIFT_KEY_RE.findall(read(SWIFT_OUT))),
        (CPP_OUT, "windows"): set(CPP_VALUE_KEY_RE.findall(read(CPP_OUT))),
        (ACCESSORS_OUT, "windows"): set(KEY_LITERAL_RE.findall(read(ACCESSORS_OUT))),
    }
    problems: list[str] = []
    for (rel, platform), keys in actual.items():
        foreign = sorted(keys & (expected[other[platform]] - expected[platform]))
        if foreign:
            problems.append(f"{rel} 含 {label[other[platform]]} 专用键（只应出现在"
                            f" {label[other[platform]]} 生成物）: " + ", ".join(foreign))
        missing = sorted(expected[platform] - keys)
        if missing:
            problems.append(f"{rel} 缺 {label[platform]} 键: " + ", ".join(missing))
        unknown = sorted(keys - expected["macos"] - expected["windows"])
        if unknown:
            problems.append(f"{rel} 含键表里不存在的键: " + ", ".join(unknown))
    return problems


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    args = ap.parse_args()

    root = (args.root or Path(__file__).resolve().parent.parent.parent).resolve()
    problems: list[str] = []

    # 与生成器共用同一个键表解析入口（#319）。
    table = gen_l10n.load(root / SCHEMA)

    # 1) 键表结构
    for key, entry in table["keys"].items():
        for field in ("zh", "en"):
            if field not in entry:
                problems.append(f"键 {key} 缺 {field} 字段")

    # 2) 生成物一致性（重新生成后比对）
    swift = gen_l10n.gen_swift(table)
    cpp = gen_l10n.gen_cpp(table)
    if (root / SWIFT_OUT).read_text(encoding="utf-8") != swift:
        problems.append("L10nKeys.swift 与键表漂移——运行 python3 scripts/gen-l10n.py")
    if (root / CPP_OUT).read_text(encoding="utf-8") != cpp:
        problems.append("L10nKeys.h 与键表漂移——运行 python3 scripts/gen-l10n.py")
    accessors_path = root / ACCESSORS_OUT
    if (not accessors_path.is_file()
            or accessors_path.read_text(encoding="utf-8") != gen_l10n.gen_cpp_accessors(table)):
        problems.append("L10nAccessors.h 与键表漂移——运行 python3 scripts/gen-l10n.py")

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

    # 4) 访问器与调用面、键表对应（#370）
    problems += accessor_problems(root, expected)

    # 5) 平台差异键分端（#372）
    problems += platform_key_problems(root, table)

    if problems:
        print("L10n 键表校验失败：")
        print("\n".join(problems))
        return 1
    print(f"OK：键表结构完整、双端生成物一致、Windows 映射覆盖、访问器与键表对应 "
          f"（{len(table['keys'])} 键 / windows {len(expected)} 键）。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
