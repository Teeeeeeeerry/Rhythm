#!/usr/bin/env python3
"""Rhythm 品牌配色测试共享库（零依赖，仅 Python 3 stdlib）。

职责（供 testing/l0/*.py 复用）：
1. 定位仓库根目录、日志双写、读取 palette.json（单一事实来源）。
2. 色值表示与转换（hex、基色 + 不透明度、alpha 合成）。
3. WCAG 2.1 相对亮度与对比度计算。
4. 仓库文件遍历（跟踪文件减排除清单）与视图文件定位。

配色管道是单向的：palette.json -> scripts/gen-palette.py -> 双端源码标记区间。
本库不再从源码语法反解色值——三套语言正则解析器已随 #250 删除，
校验改为「重新生成加逐字节比对」（testing/l0/check-palette.py）。

色值内部一律以 (r, g, b, a) 0-255 元组表示；hex 序列化统一走 to_hex / from_hex。
"""

from __future__ import annotations

import json
import subprocess
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



# ---------------------------------------------------------------------------
# 仓库文件遍历（零 emoji 校验与 L0 脚本共用，#265）
# ---------------------------------------------------------------------------

# 排除清单：这些地方不受仓库文本约定管辖，一眼可读。
# 目录前缀（含结尾斜杠）与完整路径分开列，改动时只动这一处。
EXCLUDED_PREFIXES = (
    "windows/tests/vendor/",  # 第三方单头测试框架（Catch2 amalgamated），非本仓库文本
    "build/",                 # 构建产物
    "target/",                # Rust 构建产物
)
EXCLUDED_PATHS = (
    "Cargo.lock",             # 依赖锁文件，由包管理器生成
)
# 二进制资源的扩展名兜底（内容探测之外的快速跳过）。
EXCLUDED_SUFFIXES = (
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".icns", ".webp", ".pdf",
    ".zip", ".gz", ".tar", ".xz", ".ttf", ".otf", ".woff", ".woff2",
    ".mp3", ".mp4", ".wav", ".flac", ".car", ".pyc",
)

BINARY_PROBE_BYTES = 8192


def is_excluded(rel: str) -> bool:
    """是否在排除清单内（有意声明的豁免，不受仓库文本约定管辖）。"""
    return (rel in EXCLUDED_PATHS
            or rel.startswith(EXCLUDED_PREFIXES)
            or rel.endswith(EXCLUDED_SUFFIXES))


def is_binary(path: Path) -> bool:
    """按内容探测二进制：前 8KB 出现 NUL 字节即判定为二进制。"""
    try:
        with open(path, "rb") as fh:
            return b"\x00" in fh.read(BINARY_PROBE_BYTES)
    except OSError:
        return True


def tracked_files(root: Path, suffixes: tuple[str, ...] | None = None) -> list[str]:
    """git 跟踪的全部文件减去排除清单（可再按扩展名收窄）。

    只有一处遍历实现：扩展名清单不会再漏掉一整类文件（#224/#257 的教训——
    此前零 emoji 校验按扩展名白名单收集，43 个文件从未被检查过）。
    用 -z 取原始路径：默认输出会对非 ASCII 路径加引号转义，那样的路径既匹配
    不上排除清单也打不开。
    """
    out = subprocess.check_output(["git", "ls-files", "-z"], cwd=root)
    rels = [f for f in out.decode("utf-8").split("\0") if f]
    picked = (f for f in rels if not is_excluded(f))
    if suffixes:
        picked = (f for f in picked if f.endswith(suffixes))
    return sorted(picked)


# ---------------------------------------------------------------------------
# 视图文件定位（coverage / forbidden 共用）
# ---------------------------------------------------------------------------

MACOS_VIEWS = ("macos/Rhythm/Views",)
WINDOWS_VIEWS = ("windows/Rhythm/Views",)


def swift_view_files(repo_root: Path) -> list[Path]:
    return sorted((repo_root / "macos" / "Rhythm" / "Views").rglob("*.swift"))


def xaml_view_files(repo_root: Path) -> list[Path]:
    return sorted((repo_root / "windows" / "Rhythm" / "Views").rglob("*.xaml"))
