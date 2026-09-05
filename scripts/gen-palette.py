#!/usr/bin/env python3
"""Generate the platform colour definitions from testing/palette.json (#219 组).

配色文件是单一事实来源；双端源码的标记区间是生成物。与既有的文案生成器
（gen-l10n.py）、契约绑定生成器（gen-ffi-bindings.py）同构：读声明、产出全文、
由 L0 逐字节比对拦截漂移。

接管的产物：macOS 主题模块的主色 token 定义与来源徽标色（#246/#247），
Windows 主题字典的画刷定义（#248）。

Run: python3 scripts/gen-palette.py [--emit-swift-seed]

--emit-swift-seed 顺带刷新 L1 数据驱动测试的种子。生成顺序在此固定：
先写三处源码产物，再出种子，种子因此不会基于旧产物（#219）。
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from decimal import Decimal

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PALETTE = os.path.join(ROOT, "testing", "palette.json")

SWIFT_SEED = "testing/l1/macos/PaletteSeed.swift"
SWIFT_THEME = "macos/RhythmTheme/Theme.swift"
XAML_COLORS = "windows/Rhythm/Themes/Colors.xaml"
CPP_CORE = "windows/Rhythm/Bridge/RhythmCore.h"

SWIFT_TOKENS_BEGIN = "    // BEGIN GENERATED TOKENS (#247)"
SWIFT_TOKENS_END = "    // END GENERATED TOKENS (#247)"
SWIFT_SOURCE_BEGIN = "    // BEGIN GENERATED SOURCE COLORS (#184)"
SWIFT_SOURCE_END = "    // END GENERATED SOURCE COLORS (#184)"
CPP_SOURCE_BEGIN = "        // BEGIN GENERATED SOURCE TABLE (#184)"
CPP_SOURCE_END = "        // END GENERATED SOURCE TABLE (#184)"
CPP_BADGE_BEGIN = "    // BEGIN GENERATED BADGE BACKGROUND (#249)"
CPP_BADGE_END = "    // END GENERATED BADGE BACKGROUND (#249)"
CPP_FALLBACK_BEGIN = "    // BEGIN GENERATED SOURCE FALLBACK (#219)"
CPP_FALLBACK_END = "    // END GENERATED SOURCE FALLBACK (#219)"

# 未知来源徽标回退到正文色（绝不返回系统 Gray，F4/#121）
FALLBACK_TOKEN = "rhythmTextPrimary"

# Windows 主题字典：Default 即 dark，Light 即 light。两段各有自己的标记。
XAML_DICTS = (("Default", "dark"), ("Light", "light"))


def xaml_begin(dict_name: str) -> str:
    return f"            <!-- BEGIN GENERATED BRUSHES: {dict_name} (#248) -->"


def xaml_end(dict_name: str) -> str:
    return f"            <!-- END GENERATED BRUSHES: {dict_name} (#248) -->"

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


def alpha_from_opacity(opacity: float) -> int:
    """不透明度（0-1）-> alpha 通道（0-255）。恰好 .5 时进位。

    这条规则是按平台实测定的：AppKit 把 0.7 量化成 179、0.30 量化成 77
    （L1 取值测试逐字节断言的就是这个值）。用二进制浮点乘会因 0.7 不可精确
    表示而少算一位，所以走 Decimal 按十进制字面量算（#250）。
    """
    return int(Decimal(str(opacity)) * 255 + Decimal("0.5"))


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
# Windows 主题字典（#248）
# ---------------------------------------------------------------------------

def brush_name(token: str) -> str:
    """token 名 -> 画刷资源名（rhythmTextPrimary -> RhythmTextPrimaryBrush）。"""
    return "Rhythm" + token[len("rhythm"):] + "Brush"


def xaml_color(palette: dict, token: str, appearance: str) -> str:
    """画刷色值：不透明写 #RRGGBB，半透明写 #AARRGGBB（alpha 由不透明度算出）。"""
    r, g, b = rgb(base_of(palette, token, appearance))
    opacity = opacity_of(palette, token, appearance)
    if opacity >= 1.0:
        return f"#{r:02X}{g:02X}{b:02X}"
    return f"#{alpha_from_opacity(opacity):02X}{r:02X}{g:02X}{b:02X}"


def xaml_brush_lines(palette: dict, dict_name: str, appearance: str) -> list[str]:
    lines = [xaml_begin(dict_name)]
    for token, doc in palette.get("docs", {}).items():
        opacity = opacity_of(palette, token, appearance)
        if opacity < 1.0:
            # 半透明画刷的八位值是算出物，注明它由什么算来，别人才不会去手改
            purpose = doc["lines"][0].rstrip(".")
            purpose = purpose[:1].lower() + purpose[1:]
            base = base_of(palette, token, appearance).upper()
            lines.append(f"            <!-- {base} @ {opacity:g} for {purpose} -->")
        lines.append(
            f'            <SolidColorBrush x:Key="{brush_name(token)}" '
            f'Color="{xaml_color(palette, token, appearance)}" />')
    lines.append(xaml_end(dict_name))
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


def cpp_badge_lines(palette: dict) -> list[str]:
    """徽标胶囊底的 alpha：与 macOS `.background(color.opacity(...))` 同一声明。"""
    opacity = float(palette.get("sourceBadge", {}).get("backgroundOpacity", 0.15))
    alpha = alpha_from_opacity(opacity)
    return [
        "    // BEGIN GENERATED BADGE BACKGROUND (#249) — 由 scripts/gen-palette.py 生成，勿手改",
        f"    // 胶囊底 = 徽标前景色 @ {opacity:g}"
        f"（与 macOS `.background(color.opacity({opacity:g}))` 同一声明）",
        f"    static constexpr uint8_t kSourceBadgeBackgroundAlpha = {alpha};",
        CPP_BADGE_END,
    ]


def cpp_fallback_lines(palette: dict) -> list[str]:
    """未知来源徽标的回退色：取正文色，与 macOS 侧同一出处。

    这是三处产物里最后一处手写的品牌色字面量（#219）；纳入生成后，
    改它必须改配色文件，手改会被逐字节比对拦下。
    """
    dark = base_of(palette, FALLBACK_TOKEN, "dark").upper()
    light = base_of(palette, FALLBACK_TOKEN, "light").upper()
    return [
        "    // BEGIN GENERATED SOURCE FALLBACK (#219) — 由 scripts/gen-palette.py 生成，勿手改",
        f"    // 未知来源回退到正文色（{FALLBACK_TOKEN}），绝不返回系统 Gray（F4）",
        f'    static constexpr const wchar_t* kUnknownSourceDark = L"{dark}";',
        f'    static constexpr const wchar_t* kUnknownSourceLight = L"{light}";',
        CPP_FALLBACK_END,
    ]


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

    xaml_path = os.path.join(root, XAML_COLORS)
    with open(xaml_path, encoding="utf-8") as f:
        xaml = f.read()
    for dict_name, appearance in XAML_DICTS:
        xaml = replace_region(xaml, xaml_begin(dict_name), xaml_end(dict_name),
                              xaml_brush_lines(palette, dict_name, appearance))
    out[XAML_COLORS] = xaml

    cpp_path = os.path.join(root, CPP_CORE)
    with open(cpp_path, encoding="utf-8") as f:
        cpp = f.read()
    cpp = replace_region(
        cpp, CPP_SOURCE_BEGIN, CPP_SOURCE_END, cpp_source_lines(palette))
    cpp = replace_region(
        cpp, CPP_BADGE_BEGIN, CPP_BADGE_END, cpp_badge_lines(palette))
    out[CPP_CORE] = replace_region(
        cpp, CPP_FALLBACK_BEGIN, CPP_FALLBACK_END, cpp_fallback_lines(palette))

    return out


# ---------------------------------------------------------------------------
# L1 测试种子
# ---------------------------------------------------------------------------

def swift_seed(palette: dict) -> str:
    """L1 数据驱动测试的种子（paletteRGB / contrast / sourceDistinct 共用）。"""
    lines = [
        "// 自动生成 — 由 scripts/gen-palette.py --emit-swift-seed 生成，勿手改。",
        "// 修改配色后重新生成：python3 scripts/gen-palette.py --emit-swift-seed",
        "import Foundation",
        "",
        "/// palette.json 的 Swift 种子：L1 测试断言的实际期望值。",
        "enum PaletteSeed {",
        "    struct RGB { let r, g, b, a: Int }",
        "",
        "    static let tokens: [String: [String: RGB]] = [",
    ]
    for name, v in sorted(palette["tokens"].items()):
        parts = []
        for appearance in ("dark", "light"):
            r, g, b = rgb(v[appearance])
            a = alpha_from_opacity(opacity_of(palette, name, appearance))
            parts.append(f'"{appearance}": RGB(r: {r}, g: {g}, b: {b}, a: {a})')
        lines.append(f'        "{name}": [{" , ".join(parts)}],')
    lines += [
        "    ]",
        "",
        "    static let sources: [String: [String: RGB]] = [",
    ]
    for name, v in sorted(palette["sources"].items()):
        parts = []
        for appearance in ("dark", "light"):
            if not v.get(appearance):
                continue
            r, g, b = rgb(v[appearance])
            parts.append(f'"{appearance}": RGB(r: {r}, g: {g}, b: {b}, a: 255)')
        lines.append(f'        "{name}": [{" , ".join(parts)}],')
    lines += [
        "    ]",
        "}",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser(description="从 testing/palette.json 生成双端配色定义")
    ap.add_argument("--emit-swift-seed", action="store_true",
                    help="顺带刷新 L1 测试种子（在三处产物之后生成）")
    args = ap.parse_args()

    palette = load()
    for rel, text in generate(palette).items():
        path = os.path.join(ROOT, rel)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        print(f"wrote {rel}")
    if args.emit_swift_seed:
        path = os.path.join(ROOT, SWIFT_SEED)
        with open(path, "w", encoding="utf-8") as f:
            f.write(swift_seed(palette))
        print(f"wrote {SWIFT_SEED}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
