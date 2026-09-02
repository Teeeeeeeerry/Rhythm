#!/usr/bin/env python3
"""check-version-drift.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一棵文件树，校验返回零还是非零、失败时报出哪一处与哪两个值。
不断言脚本内部用什么正则。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "testing" / "l0" / "check-version-drift.py"

# 夹具：七处版本副本的最小可识别形状（内容与仓库真实文件一致的片段）。
FIXTURE = {
    "Cargo.toml": '[workspace]\nmembers = ["rust-core"]\n\n'
                  '[workspace.package]\nversion = "{v}"\nedition = "2021"\n',
    "Cargo.lock": 'version = 3\n\n[[package]]\nname = "blake3"\nversion = "1.5.0"\n\n'
                  '[[package]]\nname = "rhythm-core"\nversion = "{v}"\n',
    "README.md": '## 开发状态\n\n初步开发完成。当前版本 **v{v} "Motif"**（与 `Cargo.toml` 同步）。\n',
    "README.en.md": '## Development Status\n\n'
                    'Initial development is complete. Current version: **v{v} "Motif"**.\n',
    "macos/Rhythm/Resources/Info.plist":
        '<plist version="1.0">\n<dict>\n'
        '    <key>CFBundleShortVersionString</key>\n    <string>{v}</string>\n'
        '    <key>CFBundleVersion</key>\n    <string>45</string>\n</dict>\n</plist>\n',
    "windows/CMakeLists.txt":
        'cmake_minimum_required(VERSION 3.20)\n'
        'project(Rhythm VERSION {v} LANGUAGES CXX)\n',
    "testing/README.md": '# 测试套件\n\n## 当前状态（main，v{v}）\n\n| 检查 | 现状 |\n',
}


def build_tree(root: Path, versions: dict[str, str] | None = None,
               base: str = "1.2.3", drop: tuple[str, ...] = ()) -> None:
    """写出一棵夹具树；versions 覆盖单个文件的版本值，drop 删除指定文件。"""
    overrides = versions or {}
    for rel, template in FIXTURE.items():
        if rel in drop:
            continue
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(template.format(v=overrides.get(rel, base)), encoding="utf-8")


class CheckVersionDriftTests(unittest.TestCase):
    def run_check(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root),
             "--log", str(root / "check.log")],
            capture_output=True, text=True,
        )

    def test_all_consistent_returns_zero(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root)
            result = self.run_check(root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("OK", result.stdout)

    def test_single_drift_returns_nonzero_and_names_both_values(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root, versions={"windows/CMakeLists.txt": "1.2.0"})
            result = self.run_check(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("windows/CMakeLists.txt", result.stdout)
            self.assertIn("1.2.0", result.stdout)
            self.assertIn("1.2.3", result.stdout)

    def test_multiple_drifts_all_reported(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root, versions={"windows/CMakeLists.txt": "1.2.0",
                                       "testing/README.md": "1.1.9"})
            result = self.run_check(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("windows/CMakeLists.txt", result.stdout)
            self.assertIn("testing/README.md", result.stdout)

    def test_missing_copy_returns_nonzero(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root, drop=("README.en.md",))
            result = self.run_check(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("README.en.md", result.stdout)

    def test_malformed_version_returns_nonzero(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root, versions={"testing/README.md": "not-a-version"})
            result = self.run_check(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("testing/README.md", result.stdout)

    def test_malformed_source_version_returns_nonzero(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root)
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.5"\n', encoding="utf-8")
            result = self.run_check(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Cargo.toml", result.stdout)


if __name__ == "__main__":
    unittest.main()
