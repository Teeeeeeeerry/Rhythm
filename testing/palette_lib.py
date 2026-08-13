#!/usr/bin/env python3
"""Rhythm 品牌配色测试共享库（零依赖，仅 Python 3 stdlib）。

职责（供 testing/l0/*.py 与 sync-palette.py 复用）：
1. 定位仓库根目录、读取 palette.json（单一事实来源）。
2. 解析双端源码中的品牌 token：
   - macOS  `macos/RhythmTheme/Theme.swift`（dark/light 三元组 + alpha；P2 后独立 target）
   - Windows `windows/Rhythm/Themes/Colors.xaml`（Default/Light 字典，#RRGGBB / #AARRGGBB）
   - Windows `windows/Rhythm/Bridge/RhythmCore.h`（SourceColor / SourceBackgroundBrush）
3. WCAG 2.1 相对亮度、对比度、alpha 合成计算。

色值内部一律以 (r, g, b, a) 0-255 元组表示；hex 序列化统一走 to_hex / from_hex。
"""

from __future__ import annotations

import json
import math
import re
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# 路径定位
# ---------------------------------------------------------------------------

REPO_MARKERS = ("Cargo.toml", "Package.swift", "CMakeLists.txt")


def find_repo_root(start: Path | None = None) -> Path:
    """从脚本所在位置向上找仓库根（含 Cargo.toml / Package.swift 的目录）。"""
    cur = (start or Path(__file__).resolve().parent).resolve()
    for parent in (cur, *cur.parents):
        if any((parent / m).exists() for m in REPO_MARKERS):
            return parent
    raise RuntimeError(f"未找到仓库根目录（缺少 {', '.join(REPO_MARKERS)}）: {cur}")


def default_repo_root() -> Path:
    return find_repo_root()


def palette_path(repo_root: Path) -> Path:
    return repo_root / "testing" / "palette.json"


# ---------------------------------------------------------------------------
# 测试日志（终端 + 文件双写）
# ---------------------------------------------------------------------------

class Tee:
    """文件对象：把同一份输出同时写入多个流（终端 + 日志文件）。"""

    def __init__(self, *streams):
        self.streams = streams

    def write(self, data: str) -> int:
        for s in self.streams:
            s.write(data)
            s.flush()
        return len(data)

    def flush(self) -> None:
        for s in self.streams:
            s.flush()


def default_log_path(script_name: str, repo_root: Path | None = None) -> Path:
    """默认日志位置 testing/logs/<脚本名>.log（每次运行覆盖）。"""
    root = repo_root or find_repo_root()
    return root / "testing" / "logs" / f"{script_name}.log"


def open_log(path: Path) -> Path:
    """把 stdout/stderr 同时写入终端与日志文件，返回日志路径。

    所有测试脚本统一走它：无论怎么跑（单脚本 / run-all / CI），
    结束后 testing/logs/ 下必有完整输出可查。
    """
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    f = open(path, "w", encoding="utf-8")
    sys.stdout = Tee(sys.stdout, f)
    sys.stderr = Tee(sys.stderr, f)
    print(f"[log] 完整输出已写入 → {path}")
    return path


# ---------------------------------------------------------------------------
# 色值表示与转换
# ---------------------------------------------------------------------------

Color = tuple[int, int, int, int]  # (r, g, b, a) 各 0-255


def from_hex(s: str) -> Color:
    """'#RRGGBB' / '#AARRGGBB' / 无 # 变体 → (r,g,b,a)。"""
    h = s.strip().lstrip("#")
    if len(h) == 6:
        return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)
    if len(h) == 8:
        return (int(h[2:4], 16), int(h[4:6], 16), int(h[6:8], 16), int(h[0:2], 16))
    raise ValueError(f"无法解析颜色 hex: {s!r}")


def to_hex(c: Color) -> str:
    r, g, b, a = c
    return f"#{r:02X}{g:02X}{b:02X}" if a == 255 else f"#{a:02X}{r:02X}{g:02X}{b:02X}"


def blend(fg: Color, bg: Color) -> Color:
    """把半透明 fg 合成到不透明 bg 上（背景按不透明处理）。"""
    fr, fg_r, fb, fa = fg
    br, bg_r, bb, _ = bg
    a = fa / 255.0
    return (
        round(fr * a + br * (1 - a)),
        round(fg_r * a + bg_r * (1 - a)),
        round(fb * a + bb * (1 - a)),
        255,
    )


# ---------------------------------------------------------------------------
# WCAG 2.1
# ---------------------------------------------------------------------------

def _linearize(c: int) -> float:
    c = c / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def relative_luminance(c: Color) -> float:
    r, g, b, _ = c
    return 0.2126 * _linearize(r) + 0.7152 * _linearize(g) + 0.0722 * _linearize(b)


def contrast_ratio(fg: Color, bg: Color) -> float:
    """WCAG 2.1 对比度。fg 需已合成到 bg 上（调用方负责 blend）。"""
    l1 = relative_luminance(fg)
    l2 = relative_luminance(bg)
    hi, lo = max(l1, l2), min(l1, l2)
    return (hi + 0.05) / (lo + 0.05)


def alpha_ratio(token: Color, bg: Color) -> float:
    """token（可能带 alpha）在给定背景上的最终对比度。"""
    return contrast_ratio(blend(token, bg), bg)


# ---------------------------------------------------------------------------
# palette.json
# ---------------------------------------------------------------------------

def load_palette(path: Path | None = None, repo_root: Path | None = None) -> dict:
    path = path or palette_path(repo_root or default_repo_root())
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def save_palette(data: dict, path: Path) -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")


# ---------------------------------------------------------------------------
# macOS Theme.swift 解析
# ---------------------------------------------------------------------------

_SWIFT_VAR = re.compile(r"static var (\w+): Color \{", re.S)
_SWIFT_NSCOLOR = re.compile(
    r"NSColor\(red:\s*(0x[0-9A-Fa-f]+)\s*/\s*255\.0,\s*"
    r"green:\s*(0x[0-9A-Fa-f]+)\s*/\s*255\.0,\s*"
    r"blue:\s*(0x[0-9A-Fa-f]+)\s*/\s*255\.0,\s*"
    r"alpha:\s*([\d.]+)\s*\)"
)
_SWIFT_WHITE = re.compile(r"NSColor\.white")


def _swift_ns_to_color(expr: str) -> Color | None:
    m = _SWIFT_NSCOLOR.search(expr)
    if m:
        return (
            int(m.group(1), 16), int(m.group(2), 16), int(m.group(3), 16),
            round(float(m.group(4)) * 255),
        )
    if _SWIFT_WHITE.search(expr):
        return (255, 255, 255, 255)
    return None


def _swift_colors_in_block(block: str) -> list[Color]:
    """块内所有 NSColor 按源码出现顺序。"""
    colors: list[Color] = []
    for m in _SWIFT_NSCOLOR.finditer(block):
        colors.append(
            (
                int(m.group(1), 16), int(m.group(2), 16), int(m.group(3), 16),
                round(float(m.group(4)) * 255),
            )
        )
    for m in _SWIFT_WHITE.finditer(block):
        colors.append((255, 255, 255, 255))
    return colors


def parse_swift_theme(path: Path) -> dict[str, dict[str, Color]]:
    """→ {token: {"dark": Color, "light": Color}}。

    约定：每个 token 块内恰好两个 NSColor，按源码顺序为
    `isDark(appearance) ? <dark> : <light>`（本仓库 Theme.swift 的固定结构）。
    """
    text = path.read_text(encoding="utf-8")
    result: dict[str, dict[str, Color]] = {}
    pos = 0
    while True:
        var = _SWIFT_VAR.search(text, pos)
        if not var:
            break
        name, start = var.group(1), var.end()
        nxt = _SWIFT_VAR.search(text, start)
        block = text[start : nxt.start() if nxt else len(text)]
        colors = _swift_colors_in_block(block)
        if len(colors) == 2:
            result[name] = {"dark": colors[0], "light": colors[1]}
        pos = nxt.start() if nxt else len(text)
    return result


# ---------------------------------------------------------------------------
# Windows Colors.xaml 解析
# ---------------------------------------------------------------------------

_XAML_DICT_START = re.compile(r'ResourceDictionary x:Key="(\w+)"')
_XAML_BRUSH = re.compile(
    r'<SolidColorBrush x:Key="Rhythm(\w+)Brush" Color="#([0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})"'
)


def parse_xaml_colors(path: Path) -> dict[str, dict[str, Color]]:
    """→ {token: {"dark": Color, "light": Color}}。Default 字典 = dark。"""
    text = path.read_text(encoding="utf-8")
    raw: dict[str, dict[str, Color]] = {}
    m = _XAML_DICT_START.search(text)
    while m:
        dict_name = m.group(1)
        nxt = _XAML_DICT_START.search(text, m.end())
        section_end = nxt.start() if nxt else len(text)
        section = text[m.end() : section_end]
        for brush in _XAML_BRUSH.finditer(section):
            token = "rhythm" + brush.group(1)
            raw.setdefault(token, {})[dict_name] = from_hex("#" + brush.group(2))
        m = nxt
    # 统一键名：Default → dark，Light → light
    return {
        token: {"dark": variants["Default"], "light": variants["Light"]}
        for token, variants in raw.items()
        if "Default" in variants and "Light" in variants
    }


# ---------------------------------------------------------------------------
# Windows RhythmCore.h 解析（source 徽标色）
# ---------------------------------------------------------------------------

_CPP_SOURCE_RET = re.compile(r'if \(sourceType == L"(\w+)"\) return L"#([0-9A-Fa-f]{6})";')
_CPP_SOURCE_RGB = re.compile(
    r'if \(sourceType == L"(\w+)"\)\s*\{ r = (0x[0-9A-Fa-f]+); g = (0x[0-9A-Fa-f]+); b = (0x[0-9A-Fa-f]+); \}'
)
_CPP_SOURCE_ALPHA = re.compile(r"winrt::Windows::UI::Color\{(\d+),")
_CPP_FALLBACK_GRAY = re.compile(r"return L\"Gray\";")


def parse_cpp_sources(path: Path) -> dict:
    """→ {"sources": {type: {"dark": Color, "light": Color|None}}, "alpha": int, "gray_fallback": bool}。

    F1 未修复前 light 变体为 None（parity / coverage 脚本据此报缺口）。
    """
    text = path.read_text(encoding="utf-8")
    sources: dict[str, dict] = {}
    for m in _CPP_SOURCE_RET.finditer(text):
        sources[m.group(1)] = {"dark": from_hex("#" + m.group(2)), "light": None}
    alpha = 255
    am = _CPP_SOURCE_ALPHA.search(text)
    if am:
        alpha = int(am.group(1))
    return {
        "sources": sources,
        "alpha": alpha,
        "gray_fallback": bool(_CPP_FALLBACK_GRAY.search(text)),
    }


# ---------------------------------------------------------------------------
# 视图文件定位（coverage / forbidden 共用）
# ---------------------------------------------------------------------------

MACOS_VIEWS = ("macos/Rhythm/Views",)
WINDOWS_VIEWS = ("windows/Rhythm/Views",)


def swift_view_files(repo_root: Path) -> list[Path]:
    return sorted((repo_root / "macos" / "Rhythm" / "Views").rglob("*.swift"))


def xaml_view_files(repo_root: Path) -> list[Path]:
    return sorted((repo_root / "windows" / "Rhythm" / "Views").rglob("*.xaml"))


if __name__ == "__main__":
    root = find_repo_root()
    print(f"repo root: {root}")
    swift = parse_swift_theme(root / "macos" / "RhythmTheme" / "Theme.swift")
    print(f"macOS tokens: {len(swift)}")
    for name, v in sorted(swift.items()):
        print(f"  {name}: dark={to_hex(v['dark'])} light={to_hex(v['light'])}")
    xaml = parse_xaml_colors(root / "windows" / "Rhythm" / "Themes" / "Colors.xaml")
    print(f"Windows tokens: {len(xaml)}")
    for name, v in sorted(xaml.items()):
        print(f"  {name}: dark={to_hex(v['dark'])} light={to_hex(v['light'])}")
    cpp = parse_cpp_sources(root / "windows" / "Rhythm" / "Bridge" / "RhythmCore.h")
    print(f"C++ sources: {len(cpp['sources'])} (alpha={cpp['alpha']}, gray_fallback={cpp['gray_fallback']})")
    for name, v in sorted(cpp["sources"].items()):
        print(f"  {name}: dark={to_hex(v['dark'])} light={v['light']}")
