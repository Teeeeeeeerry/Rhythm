#!/usr/bin/env python3
"""check-orchestration-dialects.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一棵被 git 跟踪的文件树，校验通过还是失败、失败时报出哪些路径。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "testing" / "l0" / "check-orchestration-dialects.py"


def make_repo(root: Path, files: dict[str, str]) -> None:
    (root / "Cargo.toml").write_text(
        '[workspace.package]\nversion = "1.2.3"\n', encoding="utf-8")
    for rel, content in files.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)


class OrchestrationDialectTests(unittest.TestCase):
    def run_check(self, root: Path, log: Path | None = None):
        with tempfile.TemporaryDirectory() as log_dir:
            return subprocess.run(
                [sys.executable, str(SCRIPT), "--root", str(root),
                 "--log", str(log or Path(log_dir) / "check.log")],
                capture_output=True, text=True,
            )

    def test_python_only_tree_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {"scripts/tasks.py": "print('hi')\n", "README.md": "x\n"})
            result = self.run_check(root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_a_new_shell_script_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {"scripts/deploy.sh": "#!/bin/sh\necho hi\n"})
            result = self.run_check(root)
            self.assertEqual(result.returncode, 1)
            self.assertIn("scripts/deploy.sh", result.stdout)

    def test_batch_and_powershell_are_reported_too(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_repo(root, {"a.bat": "@echo off\n", "b.ps1": "Write-Host hi\n"})
            result = self.run_check(root)
            self.assertEqual(result.returncode, 1)
            self.assertIn("a.bat", result.stdout)
            self.assertIn("b.ps1", result.stdout)

    def test_the_real_repository_has_no_dialects_left(self):
        result = self.run_check(REPO_ROOT)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
