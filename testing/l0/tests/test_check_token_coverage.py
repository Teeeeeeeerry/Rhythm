#!/usr/bin/env python3
"""check-token-coverage.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一棵视图文件树，校验返回零还是非零、失败时报出哪个视图。
锁住 #322 的结论：校验器没有按文件名的例外——无人可达的视图应被删除，
而不是在校验器里给它排一个永久的缺口分支。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from script_runner import run_script

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "testing" / "l0" / "check-token-coverage.py"

BRANDED = '<UserControl>\n  <Grid Background="{ThemeResource RhythmSurfaceBrush}" />\n</UserControl>\n'
UNBRANDED = "<UserControl>\n  <Grid />\n</UserControl>\n"


def build_tree(root: Path, views: dict[str, str]) -> None:
    """最小可识别的仓库树：仓库根标志 + Windows 视图目录。"""
    (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
    (root / "macos" / "Rhythm" / "Views").mkdir(parents=True)
    view_dir = root / "windows" / "Rhythm" / "Views"
    view_dir.mkdir(parents=True)
    for name, text in views.items():
        (view_dir / name).write_text(text, encoding="utf-8")


class TokenCoverageTests(unittest.TestCase):
    def run_check(self, root: Path) -> subprocess.CompletedProcess[str]:
        return run_script(str(SCRIPT), "--root", str(root), "--log", str(root / "out.log"))

    def test_branded_views_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), {"LibraryView.xaml": BRANDED, "PlayerBarView.xaml": BRANDED})
            result = self.run_check(Path(tmp))
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("2 个 Windows 视图", result.stdout)

    def test_unbranded_view_fails_and_is_named(self):
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), {"LibraryView.xaml": BRANDED, "NewView.xaml": UNBRANDED})
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("NewView.xaml", result.stdout)

    def test_no_filename_gets_a_gap_exception(self):
        # 曾经为 SidebarView.xaml 写死的「F2 缺口」分支已随 #383/#384 删除：
        # 同名文件没有特殊待遇，按通用规则报红。
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), {"SidebarView.xaml": UNBRANDED})
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SidebarView.xaml", result.stdout)
        self.assertIn("新增视图必须引用品牌色", result.stdout)
        self.assertNotIn("F2", result.stdout)


if __name__ == "__main__":
    unittest.main()
