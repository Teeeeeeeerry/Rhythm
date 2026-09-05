#!/usr/bin/env python3
"""L2: golden 像素比对（零依赖 PNG 解码 + diff）。

比较实际截屏与 golden 目录：
- 尺寸不一致 → 失败
- 逐像素比较，差异像素占比 > 阈值（默认 0.1%，抗锯齿容忍）→ 失败
- 输出差异热图（可选，--heatmap 写出差异像素红点图，便于 review）

用法：
    python3 testing/l2/windows/compare_screenshots.py \
        --actual build/artifacts --golden testing/l2/windows/golden \
        [--threshold 0.001] [--heatmap build/heatmap] [--log PATH]

golden 维护：外观改动后人工确认截图 → 拷入 golden 目录（git 提交）；
CI 比对失败即拦（见 ci/visual.yml）。
"""

from __future__ import annotations

import argparse
import struct
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))
from palette_lib import default_log_path, open_log  # noqa: E402


class PNG:
    """最小 PNG 解码器：8-bit RGB/RGBA（WinUI RenderTargetBitmap 输出即此）。"""

    def __init__(self, data: bytes):
        assert data[:8] == b"\x89PNG\r\n\x1a\n", "非 PNG 文件"
        self.bit_depth = 8
        self.color_type = 6  # RGBA
        self.interlace = 0
        idat = b""
        pos = 8
        while pos < len(data):
            length = struct.unpack(">I", data[pos:pos + 4])[0]
            kind = data[pos + 4:pos + 8]
            chunk = data[pos + 8:pos + 8 + length]
            if kind == b"IHDR":
                (self.width, self.height, self.bit_depth, self.color_type,
                 _, _, self.interlace) = struct.unpack(">IIBBBBB", chunk)
                assert self.bit_depth == 8 and self.color_type in (2, 6)
                assert self.interlace == 0, "不支持 interlaced PNG"
            elif kind == b"IDAT":
                idat += chunk
            elif kind == b"IEND":
                break
            pos += 12 + length
        self.channels = 4 if self.color_type == 6 else 3
        self.stride = self.width * self.channels
        # IDAT 是逐行 filter 编码的 —— 必须先 unfilter 才能得到真实像素
        self.raw = unfilter(zlib.decompress(idat), self.stride,
                            self.height, self.channels)

    def pixel(self, x: int, y: int) -> tuple[int, ...]:
        off = y * (self.stride + 1) + 1 + x * self.channels
        return tuple(self.raw[off:off + self.channels])


def paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    return a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)


def unfilter(raw: bytes, stride: int, height: int, channels: int) -> bytes:
    """PNG scanline filter 反转（0-4）。直接解码到 out 原位。"""
    out = bytearray(raw)
    prev = bytearray(stride)  # 上一行已解码像素
    for y in range(height):
        row = y * (stride + 1)
        f = raw[row]
        for i in range(stride):
            idx = row + 1 + i
            a = out[idx - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
            v = out[idx]
            if f == 1:
                out[idx] = (v + a) & 0xFF
            elif f == 2:
                out[idx] = (v + b) & 0xFF
            elif f == 3:
                out[idx] = (v + (a + b) // 2) & 0xFF
            elif f == 4:
                out[idx] = (v + paeth(a, b, c)) & 0xFF
        prev = out[row + 1:row + 1 + stride]
    return bytes(out)


def diff_pixels(actual: PNG, golden: PNG) -> tuple[int, int]:
    """返回 (差异像素数, 总像素数)。"""
    if (actual.width, actual.height) != (golden.width, golden.height):
        return -1, -1
    total = actual.width * actual.height
    diff = 0
    for y in range(actual.height):
        for x in range(actual.width):
            if actual.pixel(x, y)[:3] != golden.pixel(x, y)[:3]:
                diff += 1
    return diff, total


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--actual", required=True, type=Path, help="本次截屏输出目录")
    ap.add_argument("--golden", required=True, type=Path, help="golden 目录")
    ap.add_argument("--threshold", type=float, default=0.001,
                    help="差异像素占比上限（默认 0.001 = 0.1 个百分点；"
                         "argparse 帮助文本中百分号须写两个）")
    ap.add_argument("--heatmap", type=Path, default=None,
                    help="输出差异热图目录（PNG 白底红点）")
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/compare-screenshots.log，覆盖写入）")
    args = ap.parse_args()

    open_log(args.log or default_log_path("compare-screenshots"))

    golden_names = {p.name for p in args.golden.glob("*.png")}
    actual_names = {p.name for p in args.actual.glob("*.png")}
    if not golden_names:
        print(f"FAIL — golden 目录为空: {args.golden}")
        return 1

    # 双向：actual 多出的截图 = 新增视图未入库 golden；缺的 = 本次没截到
    names = sorted(golden_names | actual_names)
    failures: list[str] = []
    total_checked = 0
    for name in names:
        golden = args.golden / name
        actual = args.actual / name
        if not actual.exists():
            failures.append(f"缺截图: {name}")
            continue
        if not golden.exists():
            failures.append(f"缺 golden: {name}（新增截图必须人工确认后入库）")
            continue
        total_checked += 1
        with open(golden, "rb") as f:
            g = PNG(f.read())
        with open(actual, "rb") as f:
            a = PNG(f.read())
        diff, total = diff_pixels(a, g)
        if diff < 0:
            failures.append(f"{golden.name}: 尺寸不符 "
                            f"(actual {a.width}x{a.height} vs golden {g.width}x{g.height})")
            continue
        ratio = diff / total if total else 0
        status = "OK " if ratio <= args.threshold else "FAIL"
        if ratio > args.threshold:
            failures.append(f"{golden.name}: {ratio:.4%} 像素差异 > {args.threshold:.2%}")
        print(f"  [{status}] {golden.name}: diff {diff}/{total} ({ratio:.4%})")

        if args.heatmap and ratio > 0:
            args.heatmap.mkdir(parents=True, exist_ok=True)
            write_heatmap(args.heatmap / f"{golden.stem}.png", a, g)

    if failures:
        print("\nFAIL — 像素回归：")
        print("\n".join(f"  {f}" for f in failures))
        return 1
    print(f"\nOK：{total_checked} 张截图全部匹配（阈值 {args.threshold:.2%}）。")
    return 0


def write_heatmap(path: Path, actual: PNG, golden: PNG) -> None:
    """白底 + 差异像素红点（简单 PPM→PNG 输出用 zlib 手写）。"""
    import struct as st
    w, h = actual.width, actual.height
    stride = w * 3
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            if actual.pixel(x, y)[:3] != golden.pixel(x, y)[:3]:
                raw += bytes((255, 0, 0))
            else:
                raw += bytes((255, 255, 255))
    data = b"\x89PNG\r\n\x1a\n"
    def chunk(kind: bytes, payload: bytes) -> bytes:
        return (st.pack(">I", len(payload)) + kind + payload
                + st.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF))
    data += chunk(b"IHDR", st.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(bytes(raw)))
    data += chunk(b"IEND", b"")
    path.write_bytes(data)


if __name__ == "__main__":
    sys.exit(main())
