#!/usr/bin/env python3
"""构建任务的落点约定与失败传播（#261/#263）。

只锁两条：核心产物的落点与取用点必须是同一处（迁移前四个脚本四种约定，
其中两处即便修好路径也串不起来，#222/#223），以及构建失败必须非零退出
（批处理版「失败还报成功」的形状）。构建本身不在自动化测试范围内。
"""

from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))

import task_build  # noqa: E402
import tasklib  # noqa: E402

WINDOWS_CMAKE = ROOT / "windows" / "CMakeLists.txt"
# 应用构建配置里对核心产物的引用，形如 ${CMAKE_CURRENT_SOURCE_DIR}/../target/release/x
CORE_REF_RE = re.compile(r"\$\{CMAKE_CURRENT_SOURCE_DIR\}/\.\./([^\s)]+)/rhythm_core\.")


class ArtifactLayoutTest(unittest.TestCase):
    def test_core_artifacts_live_under_the_workspace_root(self):
        # 工作区布局：产物在仓库根 target/ 下，不在成员目录 rust-core/target/ 下
        self.assertEqual(task_build.core_artifact_dir(ROOT), ROOT / "target" / "release")

    def test_windows_take_point_matches_the_single_convention(self):
        refs = set(CORE_REF_RE.findall(WINDOWS_CMAKE.read_text(encoding="utf-8")))
        self.assertTrue(refs, "未在 Windows 构建配置里找到核心产物引用")
        expected = task_build.core_artifact_dir(ROOT).relative_to(ROOT).as_posix()
        self.assertEqual(refs, {expected},
                         "Windows 应用构建的取用点与核心产物落点必须是同一处")

    def test_app_artifacts_live_under_the_build_directory(self):
        self.assertEqual(task_build.windows_build_dir(ROOT), ROOT / "build" / "windows")
        self.assertEqual(task_build.windows_app_exe(ROOT).parent.parent,
                         ROOT / "build" / "windows")


class BuildFailurePropagationTest(unittest.TestCase):
    """构建失败必须非零退出，不得继续执行并报成功。"""

    def setUp(self):
        self._run_checked = tasklib.run_checked
        self.addCleanup(setattr, tasklib, "run_checked", self._run_checked)

    def _fail_on(self, needle: str):
        def fake(cmd, **kwargs):
            if needle in " ".join(str(c) for c in cmd):
                raise tasklib.StepFailed([str(c) for c in cmd], 1)

        tasklib.run_checked = fake

    def test_windows_build_returns_non_zero_when_the_core_fails(self):
        self._fail_on("cargo")
        self.assertEqual(task_build.build_windows([]), 1)

    def test_windows_build_returns_non_zero_when_cmake_fails(self):
        self._fail_on("cmake")
        self.assertEqual(task_build.build_windows([]), 1)

    def test_macos_build_returns_non_zero_when_the_core_fails(self):
        self._fail_on("cargo")
        self.assertEqual(task_build.build_macos([]), 1)

    def test_unknown_argument_is_a_usage_error(self):
        self.assertEqual(task_build.build_windows(["--nope"]), tasklib.USAGE_ERROR)
        self.assertEqual(task_build.build_macos(["--nope"]), tasklib.USAGE_ERROR)


if __name__ == "__main__":
    unittest.main()
