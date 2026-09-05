#!/usr/bin/env python3
"""Generate the platform colour definitions from testing/palette.json (#219 组).

配色文件是单一事实来源；双端源码的标记区间是生成物。与既有的文案生成器
（gen-l10n.py）、契约绑定生成器（gen-ffi-bindings.py）同构：读声明、产出全文、
由 L0 逐字节比对拦截漂移。

本票（#246）先只接管已经是正向管道的那一段——来源徽标色表。主色 token 由
#247/#248 迁入。

Run: python3 scripts/gen-palette.py
"""

from __future__ import annotations

import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PALETTE = os.path.join(ROOT, "testing", "palette.json")

SWIFT_THEME = "macos/RhythmTheme/Theme.swift"
CPP_CORE = "windows/Rhythm/Bridge/RhythmCore.h"

SWIFT_SOURCE_BEGIN = "    // BEGIN GENERATED SOURCE COLORS (#184)"
SWIFT_SOURCE_END = "    // END GENERATED SOURCE COLORS (#184)"
CPP_SOURCE_BEGIN = "        // BEGIN GENERATED SOURCE TABLE (#184)"
CPP_SOURCE_END = "        // END GENERATED SOURCE TABLE (#184)"

# 来源类型 -> macOS 主题属性名
SOURCE_SWIFT_NAMES = {
    "local": "rhythmSourceLocal",
    "youtube": "rhythmSourceYoutube",
    "bilibili": "rhythmSourceBilibili",
    "direct_url": "rhythmSourceUrl",
}


def load(path: str = PALETTE) -> dict:
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def rgb(hex_value: str) -> tuple[int, int, int]:
    h = hex_value.strip().lstrip("#")
    if len(h) == 8:  # #AARRGGBB
        h = h[2:]
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


def replace_region(text: str, begin_mark: str, end_mark: str,
                   new_lines: list[str]) -> str:
    """替换 begin_mark..end_mark 之间的内容（含标记行本身）。"""
    lines = text.splitlines(keepends=True)
    begin = next((i for i, l in enumerate(lines) if begin_mark in l), None)
    end = next((i for i, l in enumerate(lines) if end_mark in l), None)
    if begin is None or end is None or end <= begin:
        raise SystemExit(f"生成标记缺失：{begin_mark} .. {end_mark}")
    body = "\n".join(new_lines) + "\n"
    return "".join(lines[:begin]) + body + "".join(lines[end + 1:])


# ---------------------------------------------------------------------------
# 来源徽标色（#184）
# ---------------------------------------------------------------------------

def swift_source_lines(palette: dict) -> list[str]:
    lines = [
        "    // BEGIN GENERATED SOURCE COLORS (#184) — 由 scripts/gen-palette.py 生成，勿手改",
    ]
    for name, v in palette.get("sources", {}).items():
        prop = SOURCE_SWIFT_NAMES[name]
        dark, light = rgb(v["dark"]), rgb(v["light"])
        lines += [
            "    /// 来源徽标色（生成自 testing/palette.json，#184）。",
            f"    /// Dark: {v['dark'].upper()}   Light: {v['light'].upper()}",
            f"    public static var {prop}: Color {{",
            "        Color(nsColor: NSColor(name: nil) { appearance in",
            "            isDark(appearance)",
            f"                ? NSColor(red: 0x{dark[0]:02X} / 255.0, green: 0x{dark[1]:02X} / 255.0, blue: 0x{dark[2]:02X} / 255.0, alpha: 1.0)",
            f"                : NSColor(red: 0x{light[0]:02X} / 255.0, green: 0x{light[1]:02X} / 255.0, blue: 0x{light[2]:02X} / 255.0, alpha: 1.0)",
            "        })",
            "    }",
        ]
    lines.append(SWIFT_SOURCE_END)
    return lines


def cpp_source_lines(palette: dict) -> list[str]:
    lines = [
        "        // BEGIN GENERATED SOURCE TABLE (#184) — 由 scripts/gen-palette.py 生成，勿手改",
        "        static constexpr Entry kTable[] = {",
    ]
    for name, v in palette.get("sources", {}).items():
        dark, light = rgb(v["dark"]), rgb(v["light"])
        dark_hex = f"0x{dark[0]:02X}, 0x{dark[1]:02X}, 0x{dark[2]:02X}"
        light_hex = f"0x{light[0]:02X}, 0x{light[1]:02X}, 0x{light[2]:02X}"
        lines.append(f'            {{L"{name}", {{{dark_hex}}}, {{{light_hex}}}}},')
    lines += [
        "        };",
        CPP_SOURCE_END,
    ]
    return lines


# ---------------------------------------------------------------------------
# 产物
# ---------------------------------------------------------------------------

def generate(palette: dict, root: str = ROOT) -> dict[str, str]:
    """→ {相对路径: 生成后的文件全文}。

    返回全文而不是直接写盘：L0 校验重新生成后与提交内容逐字节比对，
    与文案键表、FFI 契约两条管道同一形状（#249）。
    """
    out: dict[str, str] = {}

    swift_path = os.path.join(root, SWIFT_THEME)
    with open(swift_path, encoding="utf-8") as f:
        swift = f.read()
    out[SWIFT_THEME] = replace_region(
        swift, SWIFT_SOURCE_BEGIN, SWIFT_SOURCE_END, swift_source_lines(palette))

    cpp_path = os.path.join(root, CPP_CORE)
    with open(cpp_path, encoding="utf-8") as f:
        cpp = f.read()
    out[CPP_CORE] = replace_region(
        cpp, CPP_SOURCE_BEGIN, CPP_SOURCE_END, cpp_source_lines(palette))

    return out


def main() -> int:
    palette = load()
    for rel, text in generate(palette).items():
        path = os.path.join(ROOT, rel)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        print(f"wrote {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
