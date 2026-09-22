#!/usr/bin/env python3
"""check-view-seams.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一棵视图源码树，校验通过还是失败、失败时报出哪个文件哪一行。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from script_runner import run_script

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "testing" / "l0" / "check-view-seams.py"

VIEWS = "windows/Rhythm/Views"


def make_repo(root: Path, files: dict[str, str]) -> None:
    (root / "Cargo.toml").write_text(
        '[workspace.package]\nversion = "1.2.3"\n', encoding="utf-8")
    (root / VIEWS).mkdir(parents=True, exist_ok=True)
    for rel, content in files.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")


class ViewSeamTests(unittest.TestCase):
    def run_check(self, root: Path):
        with tempfile.TemporaryDirectory() as log_dir:
            return run_script(str(SCRIPT), "--root", str(root),
                              "--log", str(Path(log_dir) / "check.log"))

    def check_tree(self, files: dict[str, str]):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, files)
            return self.run_check(root)

    def test_views_that_only_call_app_state_pass(self):
        result = self.check_tree({
            f"{VIEWS}/PlaylistDetailView.xaml.cpp":
                "appState_->ExportPlaylist(*playlistId_, path);\n"
                "btnExport().Content(box(rhythm::L10n::ExportM3U8()));\n",
        })
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_the_ffi_export_layer_is_reported(self):
        result = self.check_tree({
            f"{VIEWS}/PlaylistDetailView.xaml.cpp":
                "int ok = 1;\n"
                "rhythm::ExportM3U8(file.Path().c_str(), current->tracks);\n",
        })
        self.assertEqual(result.returncode, 1)
        self.assertIn("PlaylistDetailView.xaml.cpp:2", result.stdout)

    def test_raw_ffi_calls_are_reported(self):
        result = self.check_tree({
            f"{VIEWS}/A.xaml.cpp": "rhythm_export_m3u8(p, json);\n",
        })
        self.assertEqual(result.returncode, 1)
        self.assertIn("A.xaml.cpp:1", result.stdout)

    def test_resolver_and_lower_handles_are_reported(self):
        result = self.check_tree({
            f"{VIEWS}/B.xaml.cpp":
                "auto s = rhythm::Resolver::Status();\n"
                "appState_->Library->CreatePlaylist(name);\n"
                "appState_->Coordinator->Stop();\n",
        })
        self.assertEqual(result.returncode, 1)
        for line in ("B.xaml.cpp:1", "B.xaml.cpp:2", "B.xaml.cpp:3"):
            self.assertIn(line, result.stdout)

    def test_including_a_bridge_header_is_reported(self):
        result = self.check_tree({
            f"{VIEWS}/D.xaml.cpp":
                '#include "AppState.h"\n'
                '#include "Bridge/RhythmCore.h"\n'
                "#include <rhythm_core.h>\n",
        })
        self.assertEqual(result.returncode, 1)
        self.assertNotIn("D.xaml.cpp:1", result.stdout)
        self.assertIn("D.xaml.cpp:2", result.stdout)
        self.assertIn("D.xaml.cpp:3", result.stdout)

    def test_comments_do_not_count(self):
        result = self.check_tree({
            f"{VIEWS}/C.xaml.cpp":
                "// the view used to call rhythm::ExportM3U8 directly (#353)\n"
                "/// never Resolver::Status() here\n",
        })
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_block_comments_do_not_count_and_keep_line_numbers(self):
        result = self.check_tree({
            f"{VIEWS}/E.xaml.cpp":
                "/* the old path:\n"
                "   rhythm::ExportM3U8(path, tracks); */\n"
                "rhythm::Resolver::Status();\n",
        })
        self.assertEqual(result.returncode, 1)
        self.assertNotIn("E.xaml.cpp:2", result.stdout)
        self.assertIn("E.xaml.cpp:3", result.stdout)

    def test_files_outside_the_views_are_not_scanned(self):
        result = self.check_tree({
            "windows/Rhythm/AppState.cpp": "return ExportM3U8(path, playlist->tracks);\n",
        })
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
