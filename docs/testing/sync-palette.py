#!/usr/bin/env python3
"""同步 palette.json（单一事实来源）与双端源码，并生成 L1 测试种子。

用法（仓库根执行）：
    python3 docs/testing/sync-palette.py                 # 从源码刷新 tokens/sources 段
    python3 docs/testing/sync-palette.py --check         # CI 模式：源码与 palette.json 漂移即失败
    python3 docs/testing/sync-palette.py --emit-swift-seed   # 生成 l1/macos/PaletteSeed.swift
    python3 docs/testing/sync-palette.py --log PATH          # 覆盖日志路径（默认 logs/sync-palette.log）

palette.json 的 usage / backgrounds / exceptions / whitelist 段为手写决策段，
sync 只刷新 tokens 与 sources（由源码解析所得，构造上不可能与源码不一致）。
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
SWIFT_SEED = "docs/testing/l1/macos/PaletteSeed.swift"

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
        r"Gray",                    # F1/F4 修复前 SourceColor 回退（修复后移除）
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
    """源码值覆盖 tokens/sources；手写决策段保留（缺失时补默认值）。"""
    base: dict = {
        "version": 1,
        "note": "Rhythm 品牌配色单一事实来源。tokens/sources 由 sync-palette.py "
                "从源码自动提取（勿手改）；usage/backgrounds/exceptions/whitelist "
                "为设计决策段（人工维护）。",
        "alphaTolerance": 2,  # alpha 通道比对容差 ±2/255（浮点舍入）
        **source,
    }
    for key, default in (
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
    problems += diff_fields("tokens", source["tokens"], existing.get("tokens", {}))
    problems += diff_fields("sources", source["sources"], existing.get("sources", {}))
    # 新增 token 未进 usage 段 → 需要人工决策
    for t in source["tokens"]:
        if t not in (existing.get("usage") or {}):
            problems.append(f"  usage 段缺少新 token: {t}（请补充背景/阈值决策）")
    return problems


def emit_swift_seed(root: Path, palette: dict) -> None:
    """生成 L1 数据驱动测试种子（paletteRGB / contrast / sourceDistinct 共用）。"""
    lines = [
        "// 自动生成 — 由 docs/testing/sync-palette.py --emit-swift-seed 生成，勿手改。",
        "// 修改 token 后重新生成：python3 docs/testing/sync-palette.py --emit-swift-seed",
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


def main() -> int:
    ap = argparse.ArgumentParser(description="同步 palette.json 与双端源码")
    ap.add_argument("--root", type=Path, default=None, help="仓库根（默认自动探测）")
    ap.add_argument("--check", action="store_true", help="仅校验，不写文件；漂移即退出码 1")
    ap.add_argument("--emit-swift-seed", action="store_true", help="生成 L1 测试种子")
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 docs/testing/logs/sync-palette.log，覆盖写入）")
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
            print("运行 `python3 docs/testing/sync-palette.py` 刷新后提交。")
            return 1
        print("OK：palette.json 与源码一致（tokens/sources/usage 全覆盖）。")
        return 0

    pl.save_palette(merged, ppath)
    print(f"已同步 {ppath.relative_to(root)}")

    if args.emit_swift_seed:
        emit_swift_seed(root, merged)
    return 0


if __name__ == "__main__":
    sys.exit(main())
