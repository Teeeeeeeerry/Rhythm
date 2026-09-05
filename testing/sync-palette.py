#!/usr/bin/env python3
"""同步 palette.json（单一事实来源）与双端源码，并生成 L1 测试种子。

用法（仓库根执行）：
    python3 testing/sync-palette.py                 # 刷新 tokens + 把 sources 写回双端生成物
    python3 testing/sync-palette.py --check         # CI 模式：双端生成物与 palette.json 漂移即失败
    python3 testing/sync-palette.py --emit-swift-seed   # 生成 l1/macos/PaletteSeed.swift
    python3 testing/sync-palette.py --log PATH          # 覆盖日志路径（默认 logs/sync-palette.log）

palette.json 的 usage / backgrounds / exceptions / whitelist 段为手写决策段。
tokens 由源码解析刷新；sources（来源徽标色，#184）以 palette.json 为单一事实来源，
sync 时写回 macOS Theme.swift 的 rhythmSource* 与 Windows RhythmCore.h 的 kTable
（标记区间内生成，改色只改 palette.json）。
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import palette_lib as pl

SWIFT_THEME = "macos/RhythmTheme/Theme.swift"  # P2 重构后独立 target 目录
XAML_COLORS = "windows/Rhythm/Themes/Colors.xaml"
CPP_CORE = "windows/Rhythm/Bridge/RhythmCore.h"
SWIFT_SEED = "testing/l1/macos/PaletteSeed.swift"

# 已知设计决策段（sync 不触碰；新 token 出现时需人工补 usage 项）
DEFAULT_USAGE: dict = {
    "rhythmAccent": {
        "background": "window", "contrastThreshold": 4.5,
        "purpose": "强调/交互/选中态文字",
    },
    "rhythmTextPrimary": {
        "background": "window", "contrastThreshold": 4.5,
        "purpose": "正文",
    },
    "rhythmTextSecondary": {
        "background": "window", "contrastThreshold": 3.0,
        "purpose": "次要文本（AA 大文本线，F8 例外登记中）",
    },
    "rhythmTextTertiary": {
        "background": "window", "contrastThreshold": 3.0,
        "purpose": "提示文本（F8 例外登记中）",
    },
    "rhythmBorder": {
        "background": "window", "contrastThreshold": 3.0,
        "purpose": "分隔描边（装饰性，参考阈值）",
    },
    "rhythmSurface": {
        "background": "window", "contrastThreshold": 0.0,
        "purpose": "背景（不参与对比度断言）",
    },
    "rhythmElevated": {
        "background": "window", "contrastThreshold": 0.0,
        "purpose": "卡片/占位背景（不参与对比度断言）",
    },
    "rhythmSourceLocal": {
        "background": "window", "contrastThreshold": 4.5,
        "purpose": "来源徽标前景",
    },
    "rhythmSourceYoutube": {
        "background": "window", "contrastThreshold": 4.5,
        "purpose": "来源徽标前景",
    },
    "rhythmSourceBilibili": {
        "background": "window", "contrastThreshold": 4.5,
        "purpose": "来源徽标前景",
    },
    "rhythmSourceUrl": {
        "background": "window", "contrastThreshold": 4.5,
        "purpose": "来源徽标前景",
    },
}

# 渲染背景映射（§2.3）：macOS light 下 List .inset 的行背景为系统浅灰
# （非纯白），dark 下行背景为 surface/elevated；Windows 行背景即窗口背景。
DEFAULT_BACKGROUNDS: dict = {
    "macos": {
        "light": {"window": "#FFFFFF", "row": "#F5F5F7"},
        "dark": {"window": "#011F26", "row": "#011F26"},
    },
    "windows": {
        "light": {"window": "#FFFFFF", "row": "#FFFFFF"},
        "dark": {"window": "#011F26", "row": "#0D464D"},
    },
}

# 已登记例外（对比度不达标但已批准；check-contrast.py 只允许这些）
# measured 为 sync 时 check-contrast 实测值，漂移超 0.1 会提示核对。
DEFAULT_EXCEPTIONS: list = [
    {
        "token": "rhythmTextSecondary", "appearance": "light",
        "background": "window", "measured": 3.45,
        "reason": "F8：light 下 0.6 alpha 合成后 ~3.5:1，低于 AA 正文 4.5:1；"
                  "阈值按 AA 大文本 3.0 登记，后续决策调整 alpha 或色值",
        "approved": True,
    },
    {
        "token": "rhythmTextTertiary", "appearance": "light",
        "background": "window", "measured": 2.15,
        "reason": "F8：light 下 0.4 alpha 合成后 ~2.2:1；提示文本（空状态说明），"
                  "阈值按 3.0 登记（暂低于线），后续决策调整",
        "approved": True,
    },
    {
        "token": "rhythmBorder", "appearance": "light",
        "background": "window", "measured": 1.17,
        "reason": "装饰性面板分隔线（WCAG 1.4.11 非文本对比度不适用于纯装饰描边）；"
                  "Theme.swift 注释明确依赖边框与其他面板的配合而非自身对比度",
        "approved": True,
    },
    {
        "token": "rhythmBorder", "appearance": "dark",
        "background": "window", "measured": 1.37,
        "reason": "同 light：装饰性分隔线，1.63:1 的色块邻接靠边框区分，"
                  "不适用文本对比度要求",
        "approved": True,
    },
]

# 半透明 token 的设计意图声明（#245）：基色 + 不透明度。
# 八位十六进制是它的算出物，不是出处——两端各自手算过一次，同一个边框色因此
# 差了一个数值（Windows 侧 0x4D vs macOS 侧 0x4C），只能靠 alphaTolerance 兜着。
# 声明落地后具体数值可由生成器一次算出（#246 起）。
DEFAULT_TRANSLUCENT: dict = {
    "rhythmTextSecondary": {
        "dark": {"base": "#ABC8D4", "opacity": 0.7},
        "light": {"base": "#0D464D", "opacity": 0.6},
    },
    "rhythmTextTertiary": {
        "dark": {"base": "#ABC8D4", "opacity": 0.55},
        "light": {"base": "#0D464D", "opacity": 0.4},
    },
    "rhythmBorder": {
        "dark": {"base": "#ABC8D4", "opacity": 0.15},
        "light": {"base": "#ABC8D4", "opacity": 0.3},
    },
}

DEFAULT_WHITELIST: dict = {
    "macos": [
        # 合法系统组件与功能性颜色（非品牌外观色）。
        # 注意：.gray/.white/.black 等裸系统色不豁免 —— F4 的 .gray 回退必须
        # 被 check-forbidden-colors 报出，修复为 .rhythmTextTertiary 后自然消失。
        r"\.clear\b",               # 透明（选中态空白背景）
        r"\.red\b", r"\.orange\b", r"\.yellow\b", r"\.green\b", r"\.blue\b",
        r"\.purple\b", r"\.pink\b", r"\.secondary", r"\.tertiary",
        r"Color\(nsColor:",         # Theme.swift 内部的 NSColor 构造（token 定义处）
        r"opacity\(",               # 品牌 token 的透明度派生，非裸色
    ],
    "windows": [
        r"Transparent",
        r"Red|Orange|Yellow|Green|Blue|Purple|Pink",
        r"\{ThemeResource",         # 主题资源引用（品牌 token 或系统资源）
    ],
}


def extract_from_source(root: Path) -> dict:
    swift = pl.parse_swift_theme(root / SWIFT_THEME)
    xaml = pl.parse_xaml_colors(root / XAML_COLORS)
    cpp = pl.parse_cpp_sources(root / CPP_CORE)

    tokens = {
        name: {"dark": pl.to_hex(v["dark"]), "light": pl.to_hex(v["light"])}
        for name, v in sorted(swift.items())
        if name in DEFAULT_USAGE  # 只收 palette 成员；视图内其它 static var（如未知）忽略
    }
    sources = {
        name: {"dark": pl.to_hex(v["dark"]), "light": pl.to_hex(v["light"]) if v["light"] else None}
        for name, v in sorted(cpp["sources"].items())
    }
    # xaml 仅用于内部比对（parity 脚本），不持久化进 palette.json
    return {"tokens": tokens, "sources": sources}


def merge_palette(source: dict, existing: dict | None) -> dict:
    """tokens 由源码提取覆盖；sources（#184）以 palette.json 为单一事实来源，
    保留既有值（sync 时写回双端生成物）；手写决策段保留（缺失时补默认值）。"""
    base: dict = {
        "version": 1,
        "note": "Rhythm 品牌配色单一事实来源。tokens 由 sync-palette.py 从源码提取；"
                "sources（来源徽标色，#184）以本文件为准，sync 时写回双端生成物；"
                "translucent（半透明 token 的基色 + 不透明度声明，#245）与 "
                "usage/backgrounds/exceptions/whitelist 为设计决策段（人工维护）。",
        "alphaTolerance": 2,  # alpha 通道比对容差 ±2/255（浮点舍入）
        **source,
    }
    if existing and existing.get("sources"):
        base["sources"] = existing["sources"]
    # rhythmSource* token 与 sources 段保持同值（种子/对比测试仍在 tokens 中引用）
    source_token_map = {
        "rhythmSourceLocal": "local",
        "rhythmSourceYoutube": "youtube",
        "rhythmSourceBilibili": "bilibili",
        "rhythmSourceUrl": "direct_url",
    }
    for token, source_key in source_token_map.items():
        if source_key in base["sources"]:
            base["tokens"][token] = base["sources"][source_key]
    for key, default in (
        ("translucent", DEFAULT_TRANSLUCENT),
        ("usage", DEFAULT_USAGE),
        ("backgrounds", DEFAULT_BACKGROUNDS),
        ("exceptions", DEFAULT_EXCEPTIONS),
        ("whitelist", DEFAULT_WHITELIST),
    ):
        base[key] = (existing or {}).get(key) or default
    return base


def diff_fields(name: str, new, old) -> list[str]:
    out = []
    for k in sorted(set(new) | set(old)):
        if new.get(k) != old.get(k):
            out.append(f"  {name}.{k}: {old.get(k)} → {new.get(k)}")
    return out


def check_parity(existing: dict, source: dict) -> list[str]:
    problems = []
    source_tokens = {
        k: v for k, v in source["tokens"].items()
        if not k.startswith("rhythmSource")
    }
    existing_tokens = {
        k: v for k, v in existing.get("tokens", {}).items()
        if not k.startswith("rhythmSource")
    }
    problems += diff_fields("tokens", source_tokens, existing_tokens)
    problems += diff_fields("sources", source["sources"], existing.get("sources", {}))
    # 新增 token 未进 usage 段 → 需要人工决策
    for t in source["tokens"]:
        if t not in (existing.get("usage") or {}):
            problems.append(f"  usage 段缺少新 token: {t}（请补充背景/阈值决策）")
    return problems


def emit_swift_seed(root: Path, palette: dict) -> None:
    """生成 L1 数据驱动测试种子（paletteRGB / contrast / sourceDistinct 共用）。"""
    lines = [
        "// 自动生成 — 由 testing/sync-palette.py --emit-swift-seed 生成，勿手改。",
        "// 修改 token 后重新生成：python3 testing/sync-palette.py --emit-swift-seed",
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
            c = pl.from_hex(v[appearance])
            parts.append(
                f'"{appearance}": RGB(r: {c[0]}, g: {c[1]}, b: {c[2]}, a: {c[3]})'
            )
        lines.append(f'        "{name}": [{" , ".join(parts)}],')
    lines += [
        "    ]",
        "",
        "    static let sources: [String: [String: RGB]] = [",
    ]
    for name, v in sorted(palette["sources"].items()):
        parts = []
        for appearance in ("dark", "light"):
            if not v[appearance]:
                continue
            c = pl.from_hex(v[appearance])
            parts.append(
                f'"{appearance}": RGB(r: {c[0]}, g: {c[1]}, b: {c[2]}, a: {c[3]})'
            )
        lines.append(f'        "{name}": [{" , ".join(parts)}],')
    lines += [
        "    ]",
        "}",
        "",
    ]
    out = root / SWIFT_SEED
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"已生成 {out.relative_to(root)}（{len(palette['tokens'])} token + "
          f"{len(palette['sources'])} source）")




# ─── 来源徽标色写回双端生成物（#184）────────────────────────────────

SWIFT_SOURCE_MARK_BEGIN = "    // BEGIN GENERATED SOURCE COLORS (#184)"
SWIFT_SOURCE_MARK_END = "    // END GENERATED SOURCE COLORS (#184)"
CPP_SOURCE_MARK_BEGIN = "        // BEGIN GENERATED SOURCE TABLE (#184)"
CPP_SOURCE_MARK_END = "        // END GENERATED SOURCE TABLE (#184)"

SOURCE_SWIFT_NAMES = {
    "local": "rhythmSourceLocal",
    "youtube": "rhythmSourceYoutube",
    "bilibili": "rhythmSourceBilibili",
    "direct_url": "rhythmSourceUrl",
}

def emit_source_colors(root: Path, palette: dict) -> None:
    """把 palette.json 的 sources 写回双端生成物（改色只改 palette.json）。

    - macos/RhythmTheme/Theme.swift：rhythmSource* 属性（标记区间）
    - windows/Rhythm/Bridge/RhythmCore.h：SourceColorRGB kTable（标记区间）
    """
    sources = palette.get("sources", {})
    if not sources:
        return

    swift_path = root / SWIFT_THEME
    swift = swift_path.read_text(encoding="utf-8")
    swift_lines = [
        "    // BEGIN GENERATED SOURCE COLORS (#184) — 由 testing/sync-palette.py 生成，勿手改",
    ]
    for name in sources:
        v = sources[name]
        prop = SOURCE_SWIFT_NAMES[name]
        dark = pl.from_hex(v["dark"])
        light = pl.from_hex(v["light"])
        swift_lines += [
            f"    /// 来源徽标色（生成自 testing/palette.json，#184）。",
            f"    /// Dark: {v['dark'].upper()}   Light: {v['light'].upper()}",
            f"    public static var {prop}: Color {{",
            "        Color(nsColor: NSColor(name: nil) { appearance in",
            "            isDark(appearance)",
            f"                ? NSColor(red: 0x{dark[0]:02X} / 255.0, green: 0x{dark[1]:02X} / 255.0, blue: 0x{dark[2]:02X} / 255.0, alpha: 1.0)",
            f"                : NSColor(red: 0x{light[0]:02X} / 255.0, green: 0x{light[1]:02X} / 255.0, blue: 0x{light[2]:02X} / 255.0, alpha: 1.0)",
            "        })",
            "    }",
        ]
    swift_lines.append("    // END GENERATED SOURCE COLORS (#184)")
    swift = replace_region(swift, SWIFT_SOURCE_MARK_BEGIN, SWIFT_SOURCE_MARK_END, swift_lines)
    swift_path.write_text(swift, encoding="utf-8")

    cpp_path = root / CPP_CORE
    cpp = cpp_path.read_text(encoding="utf-8")
    cpp_lines = [
        "        // BEGIN GENERATED SOURCE TABLE (#184) — 由 testing/sync-palette.py 生成，勿手改",
        "        static constexpr Entry kTable[] = {",
    ]
    for name in sources:
        v = sources[name]
        dark = pl.from_hex(v["dark"])
        light = pl.from_hex(v["light"])
        dark_hex = f"0x{dark[0]:02X}, 0x{dark[1]:02X}, 0x{dark[2]:02X}"
        light_hex = f"0x{light[0]:02X}, 0x{light[1]:02X}, 0x{light[2]:02X}"
        cpp_lines.append(f'            {{L"{name}", {{{dark_hex}}}, {{{light_hex}}}}},')
    cpp_lines += [
        "        };",
        "        // END GENERATED SOURCE TABLE (#184)",
    ]
    cpp = replace_region(cpp, CPP_SOURCE_MARK_BEGIN, CPP_SOURCE_MARK_END, cpp_lines)
    cpp_path.write_text(cpp, encoding="utf-8")

    print(f"已写回来源徽标色：{swift_path.relative_to(root)} + {cpp_path.relative_to(root)}"
          f"（{len(sources)} 个来源）")


def replace_region(text: str, begin_mark: str, end_mark: str, new_lines: list[str]) -> str:
    """替换 begin_mark..end_mark 之间的内容（含标记行本身）。"""
    lines = text.splitlines(keepends=True)
    begin = next((i for i, l in enumerate(lines) if begin_mark in l), None)
    end = next((i for i, l in enumerate(lines) if end_mark in l), None)
    if begin is None or end is None or end <= begin:
        raise SystemExit(f"生成标记缺失：{begin_mark} .. {end_mark}")
    body = "\n".join(new_lines) + "\n"
    return "".join(lines[:begin]) + body + "".join(lines[end + 1:])


def main() -> int:
    ap = argparse.ArgumentParser(description="同步 palette.json 与双端源码")
    ap.add_argument("--root", type=Path, default=None, help="仓库根（默认自动探测）")
    ap.add_argument("--check", action="store_true", help="仅校验，不写文件；漂移即退出码 1")
    ap.add_argument("--emit-swift-seed", action="store_true", help="生成 L1 测试种子")
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/sync-palette.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("sync-palette", root))
    source = extract_from_source(root)
    existing = None
    ppath = pl.palette_path(root)
    if ppath.exists():
        existing = json.loads(ppath.read_text(encoding="utf-8"))

    merged = merge_palette(source, existing)

    if args.check:
        problems = check_parity(existing or {}, source)
        if problems:
            print("palette.json 与源码漂移：")
            print("\n".join(problems))
            print("运行 `python3 testing/sync-palette.py` 刷新后提交。")
            return 1
        print("OK：palette.json 与源码一致（tokens/sources/usage 全覆盖）。")
        return 0

    pl.save_palette(merged, ppath)
    print(f"已同步 {ppath.relative_to(root)}")

    # #184: sources 以 palette.json 为单一事实来源，写回双端生成物。
    emit_source_colors(root, merged)

    if args.emit_swift_seed:
        emit_swift_seed(root, merged)
    return 0


if __name__ == "__main__":
    sys.exit(main())
