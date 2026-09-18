#!/usr/bin/env python3
"""构建任务的落点约定、版本写入与失败传播（#261/#263/#254/#255）。

只锁三条：核心产物的落点与取用点必须是同一处（迁移前四个脚本四种约定，
其中两处即便修好路径也串不起来，#222/#223），两个平台的版本字段都来自工作区清单
（#164 的漂移不得复发），以及构建失败必须非零退出（批处理版「失败还报成功」的
形状）。构建本身不在自动化测试范围内。

Windows 侧的派生发生在 cmake 配置期，本机没有 cmake 时只能锁住写法；
实际产出的版本值由 CI 的 windows runner 验收。
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


class BundleVersionTest(unittest.TestCase):
    """应用包的版本字段由工作区清单写入，模板里不留人工副本。"""

    def test_placeholders_are_replaced_with_the_workspace_version(self):
        filled = task_build.fill_bundle_plist(
            "<string>$(EXECUTABLE_NAME)</string>"
            "<string>$(MARKETING_VERSION)</string>", "9.8.7")
        self.assertIn("<string>Rhythm</string>", filled)
        self.assertIn("<string>9.8.7</string>", filled)
        self.assertNotIn("$(", filled)

    def test_missing_version_placeholder_fails(self):
        with self.assertRaises(tasklib.StepFailed):
            task_build.fill_bundle_plist(
                "<string>$(EXECUTABLE_NAME)</string><string>1.2.3</string>", "9.8.7")

    def test_windows_config_derives_the_version_from_the_manifest(self):
        text = WINDOWS_CMAKE.read_text(encoding="utf-8")
        self.assertIn("../Cargo.toml", text,
                      "Windows 构建配置必须从工作区清单读版本")
        self.assertRegex(text, r"project\s*\([^)]*VERSION\s+\$\{RHYTHM_VERSION\}",
                         "project() 的版本必须用派生出来的变量，不写字面量")

    def test_repo_template_carries_the_version_placeholder(self):
        template = (ROOT / "macos" / "Rhythm" / "Resources" / "Info.plist").read_text(
            encoding="utf-8")
        filled = task_build.fill_bundle_plist(template,
                                              tasklib.workspace_version(ROOT))
        self.assertIn(f"<string>{tasklib.workspace_version(ROOT)}</string>", filled)


class WindowsBuildOrderTest(unittest.TestCase):
    """ADR-0004（#428）：CMake 出核心与行为库，MSBuild 出应用，入口仍是一条命令。"""

    def setUp(self):
        self.calls: list[list[str]] = []
        self._run_checked = tasklib.run_checked
        self.addCleanup(setattr, tasklib, "run_checked", self._run_checked)
        tasklib.run_checked = lambda cmd, **_: self.calls.append([str(c) for c in cmd])

    def test_cmake_builds_the_behavior_library_before_msbuild_builds_the_app(self):
        task_build.build_windows([])
        tools = [Path(cmd[0]).name.lower() for cmd in self.calls]
        self.assertEqual(tools[0], "cargo")
        cmake_build = next(i for i, cmd in enumerate(self.calls)
                           if cmd[:2] == ["cmake", "--build"])
        msbuild = next(i for i, name in enumerate(tools) if name.startswith("msbuild"))
        self.assertIn("RhythmBehavior", self.calls[cmake_build])
        self.assertLess(cmake_build, msbuild)
        self.assertEqual(msbuild, len(self.calls) - 1, "应用是最后一步")

    def test_both_builds_use_the_same_configuration(self):
        task_build.build_windows([])
        config = task_build.WINDOWS_CONFIG
        cmake_build = next(cmd for cmd in self.calls if cmd[:2] == ["cmake", "--build"])
        self.assertEqual(cmake_build[cmake_build.index("--config") + 1], config)
        msbuild = self.calls[-1]
        self.assertTrue(msbuild[1].endswith("Rhythm.vcxproj"), msbuild)
        self.assertIn(f"-p:Configuration={config}", msbuild)


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

    def test_windows_build_returns_non_zero_when_msbuild_fails(self):
        self._fail_on("Rhythm.vcxproj")
        self.assertEqual(task_build.build_windows([]), 1)

    def test_macos_build_returns_non_zero_when_the_core_fails(self):
        self._fail_on("cargo")
        self.assertEqual(task_build.build_macos([]), 1)

    def test_unknown_argument_is_a_usage_error(self):
        self.assertEqual(task_build.build_windows(["--nope"]), tasklib.USAGE_ERROR)
        self.assertEqual(task_build.build_macos(["--nope"]), tasklib.USAGE_ERROR)


if __name__ == "__main__":
    unittest.main()
