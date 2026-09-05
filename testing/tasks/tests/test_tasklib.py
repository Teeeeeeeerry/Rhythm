#!/usr/bin/env python3
"""编排层共享实现（scripts/tasklib.py）的行为测试。

覆盖两条会真正付代价的职责：失败计数与退出码聚合（#144 的缺陷形状），
以及子进程调用的错误传播（批处理版「失败还报成功」的形状）。
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))

import tasklib  # noqa: E402


class RepoPathsTest(unittest.TestCase):
    def test_repo_root_finds_workspace_manifest(self):
        self.assertEqual(tasklib.repo_root(), ROOT)

    def test_repo_root_raises_outside_repo(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(RuntimeError):
                tasklib.repo_root(Path(tmp))

    def test_log_path_keeps_existing_naming(self):
        # 迁移不改变 CI 收集产物的路径：testing/logs/<名字>.log
        self.assertEqual(
            tasklib.log_path("l1-macos-swift-test"),
            ROOT / "testing" / "logs" / "l1-macos-swift-test.log",
        )


class FailureTallyTest(unittest.TestCase):
    def test_all_green_exits_zero(self):
        f = tasklib.Failures()
        f.record("a", True)
        f.record_code("b", 0)
        self.assertEqual(f.count, 0)
        self.assertEqual(f.exit_code(), 0)

    def test_any_red_exits_non_zero(self):
        f = tasklib.Failures()
        f.record("a", True)
        f.record_code("b", 3)
        self.assertEqual(f.count, 1)
        self.assertEqual(f.exit_code(), 1)

    def test_multiple_reds_are_counted(self):
        f = tasklib.Failures()
        for step in ("a", "b", "c"):
            f.record(step, False)
        self.assertEqual(f.count, 3)
        self.assertEqual(f.failed, ["a", "b", "c"])

    def test_tolerance_requires_explicit_opt_in(self):
        f = tasklib.Failures()
        f.record("a", False)
        self.assertEqual(f.exit_code(), 1, "默认严格：不显式豁免就必须非零")
        self.assertEqual(f.exit_code(allow_expected=True), 0)

    def test_summary_lists_failed_steps(self):
        f = tasklib.Failures()
        f.record("L0 parity", False)
        self.assertIn("L0 parity", f.summary())


class SubprocessTest(unittest.TestCase):
    def test_success_returns_zero(self):
        self.assertEqual(tasklib.run([sys.executable, "-c", "pass"], echo=False), 0)

    def test_failure_returns_the_child_exit_code(self):
        code = tasklib.run([sys.executable, "-c", "raise SystemExit(7)"], echo=False)
        self.assertEqual(code, 7)

    def test_missing_executable_becomes_exit_code_not_traceback(self):
        code = tasklib.run(["rhythm-no-such-tool-xyz"], echo=False)
        self.assertEqual(code, 127)

    def test_run_checked_propagates_failure(self):
        with self.assertRaises(tasklib.StepFailed) as ctx:
            tasklib.run_checked([sys.executable, "-c", "raise SystemExit(4)"], echo=False)
        self.assertEqual(ctx.exception.code, 4)

    def test_run_checked_is_silent_on_success(self):
        tasklib.run_checked([sys.executable, "-c", "pass"], echo=False)

    def test_output_is_copied_to_the_log_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "step.log"
            tasklib.run(
                [sys.executable, "-c", "print('hello from step')"],
                log=log, echo=False,
            )
            self.assertIn("hello from step", log.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
