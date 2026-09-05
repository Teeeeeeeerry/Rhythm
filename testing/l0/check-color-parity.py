#!/usr/bin/env python3
"""L0: 双端品牌色 parity 检查。

解析 macOS Theme.swift 与 Windows Colors.xaml / RhythmCore.h，
逐 token × 外观比对双端 RGB（alpha 容差 ±2/255，见 palette.json alphaTolerance）。
任一漂移即失败（退出码 1）。

- macOS 的 7 个基础 token 必须与 Windows Colors.xaml 完全一致（dark + light）。
- 4 个 source 徽标色：macOS（Theme.swift rhythmSource*）必须与 Windows
  RhythmCore.h 的 dark 值一致；C++ 缺 light 变体（F1）按缺口报告。
- C++ 徽标背景 alpha（A=38 ≈ 15 %）必须与 macOS `color.opacity(0.15)` 一致。
- 半透明 token 的「基色 + 不透明度」声明（#245）算出的八位值必须等于 tokens 段记录值。

用法：python3 testing/l0/check-color-parity.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-color-parity", root))
    palette = pl.load_palette(repo_root=root)
    alpha_tol = int(palette.get("alphaTolerance", 2))

    swift = pl.parse_swift_theme(root / "macos/RhythmTheme/Theme.swift")  # P2 后独立 target
    xaml = pl.parse_xaml_colors(root / "windows/Rhythm/Themes/Colors.xaml")
    cpp = pl.parse_cpp_sources(root / "windows/Rhythm/Bridge/RhythmCore.h")

    problems: list[str] = []

    # 1) 基础 token：Swift vs XAML（以 palette.json 的 token 集合为准）
    shared = sorted(set(swift) & set(xaml))
    if not shared:
        problems.append("未找到双端公共 token（解析可能失效）")
    for token in sorted(palette["tokens"]):
        if token not in xaml:
            continue  # source token 在 XAML 无对应，走第 2 步
        for appearance in ("dark", "light"):
            s, x = swift[token][appearance], xaml[token][appearance]
            if abs(s[0] - x[0]) > 0 or abs(s[1] - x[1]) > 0 or abs(s[2] - x[2]) > 0 \
                    or abs(s[3] - x[3]) > alpha_tol:
                problems.append(
                    f"token {token}.{appearance}: "
                    f"Swift={pl.to_hex(s)} ≠ XAML={pl.to_hex(x)}"
                )

    # 2) source 色：Swift rhythmSource* vs C++ SourceColor（dark 必须一致）
    source_map = {
        "local": "rhythmSourceLocal", "youtube": "rhythmSourceYoutube",
        "bilibili": "rhythmSourceBilibili", "direct_url": "rhythmSourceUrl",
    }
    for cpp_name, swift_name in source_map.items():
        if cpp_name not in cpp["sources"]:
            problems.append(f"C++ 缺少 source: {cpp_name}")
            continue
        c = cpp["sources"][cpp_name]["dark"]
        s = swift.get(swift_name, {}).get("dark")
        if s is None:
            problems.append(f"Swift 缺少 source token: {swift_name}")
            continue
        if c[:3] != s[:3]:
            problems.append(
                f"source {cpp_name}.dark: C++={pl.to_hex(c)} ≠ Swift={pl.to_hex(s)}"
            )
        if cpp["sources"][cpp_name]["light"] is None:
            problems.append(
                f"source {cpp_name}: C++ 缺 light 变体（F1，需补 theme 感知签名）"
            )

    # 3) C++ 徽标背景 alpha 必须 ≈ 15 %（A=38/255），与 macOS opacity(0.15) 约定一致
    cpp_alpha = cpp["alpha"]
    if not (35 <= cpp_alpha <= 41):
        problems.append(f"C++ SourceBackgroundBrush alpha={cpp_alpha}，期望 ≈ 38 (15%)")

    # 4) palette.json 与源码的 token 值必须一致（sync 已保证，此处防手改 palette）
    for token, variants in swift.items():
        if token in palette.get("tokens", {}):
            for appearance in ("dark", "light"):
                expected = pl.from_hex(palette["tokens"][token][appearance])
                actual = variants[appearance]
                if expected != actual:
                    problems.append(
                        f"palette.json {token}.{appearance} 与源码不符: "
                        f"{pl.to_hex(expected)} ≠ {pl.to_hex(actual)}"
                    )

    # 5) 半透明声明与八位值并存校验（#245）：由「基色 + 不透明度」算出的值
    #    必须等于 tokens 段记录的八位十六进制。过渡期两者并存，声明是设计意图，
    #    八位值是算出物；对不上说明有人只改了一边。
    translucent = palette.get("translucent", {})
    for token, variants in sorted(translucent.items()):
        recorded = palette.get("tokens", {}).get(token)
        if recorded is None:
            problems.append(f"translucent 段的 {token} 不在 tokens 段中")
            continue
        for appearance in ("dark", "light"):
            decl = variants.get(appearance)
            if not decl:
                problems.append(f"translucent {token}.{appearance} 缺声明")
                continue
            computed = pl.from_base_opacity(decl["base"], decl["opacity"])
            expected = pl.from_hex(recorded[appearance])
            if computed != expected:
                problems.append(
                    f"translucent {token}.{appearance}: 声明算出 {pl.to_hex(computed)}"
                    f"（{decl['base']} @ {decl['opacity']}）"
                    f" != tokens 段 {pl.to_hex(expected)}"
                )

    if problems:
        print("FAIL — 品牌色 parity 漂移：")
        print("\n".join(f"  {p}" for p in problems))
        return 1
    print(f"OK：{len(shared)} 个双端 token + 4 个 source 色全部一致"
          f"（alpha 容差 ±{alpha_tol}/255）；"
          f"{len(translucent)} 个半透明 token 的声明与八位值一致。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
