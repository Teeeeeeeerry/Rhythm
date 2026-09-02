#!/usr/bin/env python3
"""check-no-emoji.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一棵被 git 跟踪的文件树，校验通过还是失败、失败时报出哪些位置。
不断言内部遍历顺序。夹具里的 emoji 一律用转义写法，避免测试源码自身违反零 emoji 约定。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "scripts" / "check-no-emoji.py"

GRINNING = "\U0001F600"  # 属于 pictographs 段，判定为 emoji
ARROWS = "-> <- => |-- --|"  # 普通符号，不算 emoji


def make_repo(root: Path, files: dict[str, bytes]) -> None:
    """在 root 建一个 git 仓库并跟踪 files（键为相对路径，值为字节内容）。"""
    (root / "Cargo.toml").write_text(
        '[workspace.package]\nversion = "1.2.3"\n', encoding="utf-8")
    for rel, content in files.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)


class CheckNoEmojiTests(unittest.TestCase):
    def run_check(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root),
             "--log", str(root / "check.log")],
            capture_output=True, text=True,
        )

    def test_clean_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {
                "src/main.rs": b"fn main() {}\n",
                "windows/Rhythm/Views/LibraryView.xaml": b"<Grid/>\n",
                "scripts/gen.py": b"print('ok')\n",
            })
            result = self.run_check(root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("OK: no emoji found", result.stdout)

    def test_emoji_reported_with_position(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {
                "windows/Rhythm/AppState.cpp":
                    f"// line one\n// line two {GRINNING}\n".encode("utf-8"),
            })
            result = self.run_check(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("windows/Rhythm/AppState.cpp:2:", result.stdout)
            self.assertIn("U+1F600", result.stdout)
            self.assertIn("FAIL: 1 emoji found", result.stderr)

    def test_previously_uncovered_extensions_are_checked(self) -> None:
        """扩展名白名单时代漏掉的类型（.cpp/.hpp/.xaml/.py/.ps1）现在都被检查。"""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            names = ["a.cpp", "b.hpp", "c.xaml", "d.py", "e.ps1", "f.newlang"]
            make_repo(root, {n: f"x {GRINNING}\n".encode("utf-8") for n in names})
            result = self.run_check(root)
            self.assertNotEqual(result.returncode, 0)
            for n in names:
                self.assertIn(f"{n}:1:", result.stdout)

    def test_excluded_paths_not_reported(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {
                "windows/tests/vendor/catch_amalgamated.cpp":
                    f"// {GRINNING}\n".encode("utf-8"),
                "Cargo.lock": f"# {GRINNING}\n".encode("utf-8"),
                "src/main.rs": b"fn main() {}\n",
            })
            result = self.run_check(root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertNotIn("catch_amalgamated", result.stdout)
            self.assertNotIn("Cargo.lock", result.stdout)

    def test_binary_file_does_not_abort(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {
                "assets/icon.bin": bytes([0x89, 0x50, 0x00, 0xFF, 0xFE, 0x00]),
                "assets/blob": b"\x00\x01\x02\x03" + GRINNING.encode("utf-8"),
                "src/main.rs": b"fn main() {}\n",
            })
            result = self.run_check(root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_plain_symbols_not_flagged(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {
                "docs/notes.md": f"流程：A {ARROWS} B\n".encode("utf-8"),
                "src/main.rs": b"// a -> b\n",
            })
            result = self.run_check(root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_non_ascii_filename_is_checked(self) -> None:
        """非 ASCII 路径此前被 git ls-files 的引号转义漏掉，现在必须被检查。"""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {"docs/adr/0001-行为清单.md": f"# {GRINNING}\n".encode("utf-8")})
            result = self.run_check(root)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("0001-行为清单.md:1:", result.stdout)


if __name__ == "__main__":
    unittest.main()
