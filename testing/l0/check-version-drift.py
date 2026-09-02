#!/usr/bin/env python3
"""L0: 版本号漂移检查。

工作区清单 Cargo.toml 的 [workspace.package] version 是版本号的唯一出处；
仓库里另有六处副本（依赖锁文件、两份 README 的版本行、macOS 应用包版本字段、
Windows 构建配置的项目版本、测试基础设施说明的状态表）。任一处与出处不一致即失败，
输出指出位置与两个值。

用法：python3 testing/l0/check-version-drift.py [--root PATH] [--log PATH]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import palette_lib as pl

VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")

# 唯一出处：工作区清单的 [workspace.package] version。
SOURCE_FILE = "Cargo.toml"
SOURCE_RE = re.compile(
    r"^\[workspace\.package\]$.*?^version\s*=\s*\"([^\"]+)\"",
    re.MULTILINE | re.DOTALL,
)

# 六处副本：(相对路径, 说明, 提取正则)。正则第一组即该处的版本值。
COPIES: list[tuple[str, str, re.Pattern[str]]] = [
    ("Cargo.lock", "依赖锁文件 rhythm-core 版本",
     re.compile(r"^name\s*=\s*\"rhythm-core\"$\s*^version\s*=\s*\"([^\"]+)\"",
                re.MULTILINE)),
    ("README.md", "中文 README 版本行",
     re.compile(r"当前版本\s*\*\*v([0-9][^\s\"]*)")),
    ("README.en.md", "英文 README 版本行",
     re.compile(r"Current version:\s*\*\*v([0-9][^\s\"]*)")),
    ("macos/Rhythm/Resources/Info.plist", "macOS 应用包 CFBundleShortVersionString",
     re.compile(r"<key>CFBundleShortVersionString</key>\s*<string>([^<]+)</string>")),
    ("windows/CMakeLists.txt", "Windows 构建配置 project VERSION",
     re.compile(r"^project\s*\([^)]*?\bVERSION\s+(\S+)", re.MULTILINE)),
    ("testing/README.md", "测试基础设施说明状态表版本",
     re.compile(r"^##\s*当前状态（main，v(\S+?)）", re.MULTILINE)),
]


def read_version(path: Path, pattern: re.Pattern[str]) -> tuple[str | None, str | None]:
    """返回 (版本值, 问题描述)。取不到时版本值为 None。"""
    if not path.exists():
        return None, "文件缺失"
    m = pattern.search(path.read_text(encoding="utf-8"))
    if not m:
        return None, "未找到版本字段"
    value = m.group(1).strip()
    if not VERSION_RE.match(value):
        return None, f"版本格式异常（{value}）"
    return value, None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, default=None)
    ap.add_argument("--log", type=Path, default=None,
                    help="日志文件（默认 testing/logs/<脚本名>.log，覆盖写入）")
    args = ap.parse_args()

    root = pl.find_repo_root(args.root)
    pl.open_log(args.log or pl.default_log_path("check-version-drift", root))

    source, problem = read_version(root / SOURCE_FILE, SOURCE_RE)
    if source is None:
        print(f"FAIL — 版本号唯一出处不可用：{SOURCE_FILE} [workspace.package] version：{problem}")
        return 1

    problems: list[str] = []
    checked = 0
    for rel, label, pattern in COPIES:
        value, problem = read_version(root / rel, pattern)
        if value is None:
            problems.append(f"  {rel}（{label}）：{problem}；出处 {SOURCE_FILE} = {source}")
            continue
        checked += 1
        if value != source:
            problems.append(f"  {rel}（{label}）：{value} != {source}"
                            f"（出处 {SOURCE_FILE}）")

    if problems:
        print(f"FAIL — 版本号与唯一出处漂移（{SOURCE_FILE} = {source}）：")
        print("\n".join(problems))
        print(f"请把上列位置同步到 {source}（版本号只改 {SOURCE_FILE}，其余是副本）。")
        return 1
    print(f"OK：{checked} 处版本副本与 {SOURCE_FILE} 一致（{source}）。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
