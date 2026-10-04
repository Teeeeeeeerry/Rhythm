#!/usr/bin/env python3
"""check-forbidden-colors.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一棵视图源码树，校验通过还是失败、失败时报出哪个文件。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path

from script_runner import run_script

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "testing" / "l0" / "check-forbidden-colors.py"

MAC_VIEWS = "macos/Rhythm/Views"


def make_repo(root: Path, files: dict[str, str]) -> None:
    (root / "Cargo.toml").write_text(
        '[workspace.package]\nversion = "1.2.3"\n', encoding="utf-8")
    palette = root / "testing" / "palette.json"
    palette.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(REPO_ROOT / "testing" / "palette.json", palette)
    for rel, content in files.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")


class ForbiddenColorsTests(unittest.TestCase):
    def check_tree(self, files: dict[str, str]):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, files)
            return run_script(str(SCRIPT), "--root", str(root),
                              "--log", str(root / "check.log"))

    def test_views_that_use_translucent_tokens_pass(self):
        result = self.check_tree({
            f"{MAC_VIEWS}/Sidebar/SidebarView.swift":
                "        .fill(selected\n"
                "            ? AnyShapeStyle(.rhythmSelection)\n"
                "            : AnyShapeStyle(.clear))\n"
                "        .opacity(track.isAvailable ? 1 : 0.35)\n",
        })
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_hand_computed_opacity_on_a_brand_token_is_reported(self):
        result = self.check_tree({
            f"{MAC_VIEWS}/Sidebar/SidebarView.swift":
                "        .fill(selected\n"
                "            ? AnyShapeStyle(.rhythmAccent.opacity(0.15))\n"
                "            : AnyShapeStyle(.clear))\n",
        })
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("SidebarView.swift", result.stdout)
        self.assertIn(".rhythmAccent.opacity(0.15)", result.stdout)

    def test_hand_computed_opacity_in_a_comment_is_ignored(self):
        result = self.check_tree({
            f"{MAC_VIEWS}/Sidebar/SidebarView.swift":
                "        // 曾经写作 .rhythmAccent.opacity(0.15)\n"
                "        .fill(AnyShapeStyle(.rhythmSelection))\n",
        })
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
