#!/usr/bin/env python3
"""check-palette.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：产物与配色文件一致时通过，人为改动任一处产物即报红并指出是哪一处。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "testing" / "l0" / "check-palette.py"

GENERATED = (
    "macos/RhythmTheme/Theme.swift",
    "windows/Rhythm/Themes/Colors.xaml",
    "windows/Rhythm/Bridge/RhythmCore.h",
)


def make_tree(root: Path) -> None:
    """把配色文件与三处产物拷进一棵最小仓库树。"""
    (root / "Cargo.toml").write_text(
        '[workspace.package]\nversion = "1.2.3"\n', encoding="utf-8")
    for rel in ("testing/palette.json", *GENERATED):
        dest = root / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REPO_ROOT / rel, dest)


class CheckPaletteTests(unittest.TestCase):
    def run_check(self, root: Path) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as log_dir:
            return subprocess.run(
                [sys.executable, str(SCRIPT), "--root", str(root),
                 "--log", str(Path(log_dir) / "check.log")],
                capture_output=True, text=True,
            )

    def test_untouched_outputs_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_tree(root)
            result = self.run_check(root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_a_hand_edit_in_any_output_is_reported(self):
        for rel in GENERATED:
            with self.subTest(output=rel):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    make_tree(root)
                    path = root / rel
                    # 必须改在生成区间内：区间外的改动本就不该报红
                    text = path.read_text(encoding="utf-8")
                    start = text.index("BEGIN GENERATED")
                    head, tail = text[:start], text[start:]
                    self.assertIn("D4", tail)
                    path.write_text(head + tail.replace("D4", "D5", 1), encoding="utf-8")
                    result = self.run_check(root)
                    self.assertEqual(result.returncode, 1, result.stdout)
                    self.assertIn(rel, result.stdout)

    def test_a_translucent_declaration_that_stops_matching_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_tree(root)
            palette = root / "testing" / "palette.json"
            text = palette.read_text(encoding="utf-8")
            palette.write_text(text.replace('"opacity": 0.15', '"opacity": 0.16'),
                               encoding="utf-8")
            result = self.run_check(root)
            self.assertEqual(result.returncode, 1, result.stdout)

    def test_a_hand_edit_outside_the_generated_region_is_not_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_tree(root)
            path = root / "macos/RhythmTheme/Theme.swift"
            text = path.read_text(encoding="utf-8")
            path.write_text("// 区间外的手写注释\n" + text, encoding="utf-8")
            result = self.run_check(root)
            self.assertEqual(result.returncode, 0, result.stdout)

    def test_the_real_repository_is_in_sync(self):
        result = self.run_check(REPO_ROOT)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
