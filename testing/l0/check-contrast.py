#!/usr/bin/env python3
"""L0: WCAG 2.1 对比度全矩阵检查。

矩阵 = token × 渲染背景 × 外观（dark/light）× 平台（macOS/windows）。
- 背景由 palette.json usage[token].background + backgrounds 段决定（§2.3）。
- token 先 alpha 合成到背景再算对比度。
- 未达 usage[token].contrastThreshold 且未在 palette.json exceptions 段登记的
  组合即失败（退出码 1）；已登记的组合即使不达标也放行（决策留痕）。

用法：python3 testing/l0/check-contrast.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl

TEXT_ROLES = ("textPrimary", "textSecondary", "textTertiary", "source")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-contrast", root))
    palette = pl.load_palette(repo_root=root)
    tokens: dict = palette["tokens"]
    usage: dict = palette.get("usage", {})
    backgrounds: dict = palette.get("backgrounds", {})
    exceptions: list = palette.get("exceptions", [])

    registered = {
        (e["token"], e["appearance"], e["background"]): e
        for e in exceptions if e.get("approved")
    }

    failures: list[str] = []
    rows: list[tuple] = []  # 汇总表

    for platform, bg_map in backgrounds.items():
        for appearance, bg_names in bg_map.items():
            for bg_name, bg_hex in bg_names.items():
                bg = pl.from_hex(bg_hex)
                for token, variants in tokens.items():
                    cfg = usage.get(token, {})
                    if cfg.get("contrastThreshold", 0) <= 0:
                        continue  # 背景类 token 不参与
                    if cfg.get("background") != bg_name:
                        continue
                    fg = pl.from_hex(variants[appearance])
                    ratio = round(pl.alpha_ratio(fg, bg), 2)
                    threshold = float(cfg["contrastThreshold"])
                    key = (token, appearance, bg_name)
                    status = "PASS"
                    if ratio < threshold:
                        if key in registered:
                            status = "例外(已批准)"
                        else:
                            status = "FAIL"
                            failures.append(
                                f"{platform}.{appearance} {token} on {bg_name} "
                                f"= {ratio}:1 < {threshold}:1"
                            )
                    rows.append(
                        (platform, appearance, token, bg_name, ratio, threshold, status)
                    )

    # 汇总表（CI 贴图/人工速查）
    print(f"{'平台':<9}{'外观':<7}{'token':<22}{'背景':<8}{'比值':<7}{'阈值':<6}状态")
    for platform, appearance, token, bg_name, ratio, threshold, status in rows:
        print(f"{platform:<9}{appearance:<7}{token:<22}{bg_name:<8}"
              f"{ratio:<7}{threshold:<6.1f}{status}")

    if failures:
        print("\nFAIL — 未登记的低对比度组合（需修复或登记例外）：")
        print("\n".join(f"  {f}" for f in failures))
        print("登记方式：palette.json exceptions 段追加"
              " {token, appearance, background, measured, reason, approved: true}。")
        return 1

    # 已登记例外的实测值漂移提示（不阻断，供维护）
    for key, exc in registered.items():
        token, appearance, bg_name = key
        bg_hex = None
        for bg_map in backgrounds.values():
            if bg_name in bg_map.get(appearance, {}):
                bg_hex = bg_map[appearance][bg_name]
        if not bg_hex:
            continue
        ratio = round(pl.alpha_ratio(pl.from_hex(tokens[token][appearance]),
                                     pl.from_hex(bg_hex)), 2)
        if abs(ratio - float(exc.get("measured", ratio))) > 0.1:
            print(f"提示：例外 {token}.{appearance}@{bg_name} 实测 {ratio}:1，"
                  f"登记值 {exc.get('measured')}:1（已漂移，请核对）")

    print(f"\nOK：{len(rows)} 个组合全部达标或已登记例外。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
