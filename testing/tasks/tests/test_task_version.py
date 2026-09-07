#!/usr/bin/env python3
"""版本提升任务的外部行为（#220 收尾）。

只锁三条：改的位置与 L0 校验读的位置同源（两边不可能各漂一次）、
只动版本值不动正文里的其他数字、非法版本号按用法错误处理。
依赖锁文件交给包管理器，不在本测试范围内。
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))

import task_version  # noqa: E402
import tasklib  # noqa: E402

CHECK = task_version.load_drift_check(ROOT)

# 夹具：出处与三处人工副本的最小可识别形状。构建号 45 与最低系统版本 13.0
# 是「正文里的其他数字」，必须原样保留。
FIXTURE = {
    "Cargo.toml": '[workspace]\nmembers = ["rust-core"]\n\n'
                  '[workspace.package]\nversion = "{v}"\nedition = "2021"\n',
    "README.md": '初步开发完成。当前版本 **v{v} "Motif"**（与 `Cargo.toml` 同步）。\n',
    "README.en.md": 'Initial development is complete. '
                    'Current version: **v{v} "Motif"**.\n',
    "testing/README.md": '# 测试套件\n\n## 当前状态（main，v{v}）\n\n'
                         '| 检查 | 现状 |\n\n构建号 45，最低系统版本 13.0。\n',
}


def build_tree(root: Path, base: str = "1.2.3") -> None:
    for rel, template in FIXTURE.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(template.format(v=base), encoding="utf-8")


class VersionBumpTest(unittest.TestCase):
    def test_next_patch_increments_the_last_component(self):
        self.assertEqual(task_version.next_patch("0.5.139"), "0.5.140")
        self.assertEqual(task_version.next_patch("0.5.9"), "0.5.10")

    def test_apply_version_moves_the_source_and_every_manual_copy(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root)
            changed = task_version.apply_version(root, "1.2.4", check=CHECK)
            self.assertEqual(set(changed), set(FIXTURE))
            for rel in FIXTURE:
                text = (root / rel).read_text(encoding="utf-8")
                self.assertIn("1.2.4", text, rel)
                self.assertNotIn("1.2.3", text, rel)

    def test_apply_version_leaves_other_numbers_alone(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root)
            task_version.apply_version(root, "1.2.4", check=CHECK)
            text = (root / "testing/README.md").read_text(encoding="utf-8")
            self.assertIn("构建号 45，最低系统版本 13.0。", text)

    def test_apply_version_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root)
            task_version.apply_version(root, "1.2.4", check=CHECK)
            self.assertEqual(task_version.apply_version(root, "1.2.4", check=CHECK), [])

    def test_missing_copy_fails_loudly(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            build_tree(root)
            (root / "README.en.md").unlink()
            with self.assertRaises(tasklib.StepFailed):
                task_version.apply_version(root, "1.2.4", check=CHECK)

    def test_written_positions_come_from_the_drift_check(self):
        # 写的位置 = 校验读的位置减去依赖锁文件（那份由包管理器同步）
        expected = {CHECK.SOURCE_FILE} | {rel for rel, _, _ in CHECK.COPIES
                                          if rel != task_version.LOCK_FILE}
        self.assertEqual(set(FIXTURE), expected)

    def test_malformed_version_is_a_usage_error(self):
        self.assertEqual(task_version.bump_version(["0.5"]), tasklib.USAGE_ERROR)
        self.assertEqual(task_version.bump_version(["--nope"]), tasklib.USAGE_ERROR)
        self.assertEqual(task_version.bump_version(["1.2.3", "4.5.6"]),
                         tasklib.USAGE_ERROR)


if __name__ == "__main__":
    unittest.main()
