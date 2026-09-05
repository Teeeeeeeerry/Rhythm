#!/usr/bin/env python3
"""Generate the platform colour definitions from testing/palette.json (#219 组).

配色文件是单一事实来源；双端源码的标记区间是生成物。与既有的文案生成器
（gen-l10n.py）、契约绑定生成器（gen-ffi-bindings.py）同构：读声明、产出全文、
由 L0 逐字节比对拦截漂移。

接管的产物：macOS 主题模块的主色 token 定义与来源徽标色（#246/#247），
Windows 主题字典的画刷定义（#248）。

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

SWIFT_TOKENS_BEGIN = "    // BEGIN GENERATED TOKENS (#247)"
SWIFT_TOKENS_END = "    // END GENERATED TOKENS (#247)"
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
# 主色 token（#247）
# ---------------------------------------------------------------------------

def opacity_of(palette: dict, token: str, appearance: str) -> float:
    """token 在该外观下的不透明度；不在 translucent 段即完全不透明。"""
    decl = palette.get("translucent", {}).get(token, {}).get(appearance)
    return float(decl["opacity"]) if decl else 1.0


def base_of(palette: dict, token: str, appearance: str) -> str:
    """token 在该外观下的基色；半透明的取声明基色，其余取 tokens 段色值。"""
    decl = palette.get("translucent", {}).get(token, {}).get(appearance)
    if decl:
        return decl["base"]
    return palette["tokens"][token][appearance]


def values_line(palette: dict, token: str) -> str:
    """文档里的取值行：不透明的写色值，半透明的写「基色 @ 不透明度」。"""
    parts = []
    for appearance, label in (("dark", "Dark"), ("light", "Light")):
        base = base_of(palette, token, appearance).upper()
        opacity = opacity_of(palette, token, appearance)
        parts.append(f"{label}: {base}" if opacity == 1.0
                     else f"{label}: {base} @ {opacity:g}")
    return "   ".join(parts)


def swift_alpha(opacity: float) -> str:
    """Swift 侧的 alpha 字面量：去掉多余零，但整数保留一位小数（1 -> 1.0）。"""
    text = f"{opacity:g}"
    return text if "." in text else f"{text}.0"


def _ns_color(palette: dict, token: str, appearance: str) -> str:
    r, g, b = rgb(base_of(palette, token, appearance))
    opacity = opacity_of(palette, token, appearance)
    return (f"NSColor(red: 0x{r:02X} / 255.0, green: 0x{g:02X} / 255.0, "
            f"blue: 0x{b:02X} / 255.0, alpha: {swift_alpha(opacity)})")


def swift_token_lines(palette: dict) -> list[str]:
    lines = [
        "    // BEGIN GENERATED TOKENS (#247) — 由 scripts/gen-palette.py 生成，勿手改",
    ]
    for token, doc in palette.get("docs", {}).items():
        group = doc.get("group")
        if group:
            lines += ["", f"    // MARK: {group}"]
        lines.append("")
        for line in doc["lines"]:
            text = line.replace("{values}", values_line(palette, token))
            lines.append(f"    /// {text}" if text else "    ///")
        lines += [
            f"    public static var {token}: Color {{",
            "        Color(nsColor: NSColor(name: nil) { appearance in",
            "            isDark(appearance)",
            f"                ? {_ns_color(palette, token, 'dark')}",
            f"                : {_ns_color(palette, token, 'light')}",
            "        })",
            "    }",
        ]
    lines += ["", SWIFT_TOKENS_END]
    return lines


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
    swift = replace_region(
        swift, SWIFT_TOKENS_BEGIN, SWIFT_TOKENS_END, swift_token_lines(palette))
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
