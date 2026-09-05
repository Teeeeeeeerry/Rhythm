#!/usr/bin/env python3
"""任务入口（scripts/tasks.py）的行为测试。

范围收窄到唯一曾经付过代价的那条：退出码聚合（#144 的缺陷形状——全量入口
即使测试全红仍以零退出）。构建任务、日志落点、子进程调用不在此立清单。
"""

from __future__ import annotations

import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))

import task_test  # noqa: E402
import tasklib  # noqa: E402
import tasks  # noqa: E402


def green(name: str, static_analysis: bool = True) -> "tasks.Step":
    return tasks.Step(name, lambda: 0, static_analysis)


def red(name: str, code: int = 1, static_analysis: bool = True) -> "tasks.Step":
    return tasks.Step(name, lambda: code, static_analysis)


class TaskListingTest(unittest.TestCase):
    def test_entry_runs_and_lists_every_task(self):
        out = io.StringIO()
        with redirect_stdout(out):
            code = tasks.main([])
        self.assertEqual(code, 0)
        listing = out.getvalue()
        for task in tasks.TASKS:
            self.assertIn(task.name, listing)

    def test_unknown_task_is_a_usage_error(self):
        out = io.StringIO()
        with redirect_stdout(out):
            code = tasks.main(["no-such-task"])
        self.assertEqual(code, tasks.USAGE_ERROR)


class ExitCodeAggregationTest(unittest.TestCase):
    """任一步失败则整体非零退出。"""

    def test_all_green_exits_zero(self):
        out = io.StringIO()
        with redirect_stdout(out):
            code = tasks.run_steps([green("a"), green("b")])
        self.assertEqual(code, 0)

    def test_one_red_among_green_exits_non_zero(self):
        out = io.StringIO()
        with redirect_stdout(out):
            code = tasks.run_steps([green("a"), red("b"), green("c")])
        self.assertEqual(code, 1)

    def test_every_step_runs_even_after_a_failure(self):
        ran: list[str] = []

        def track(name: str, rc: int) -> "tasks.Step":
            return tasks.Step(name, lambda: (ran.append(name), rc)[1])

        out = io.StringIO()
        with redirect_stdout(out):
            tasks.run_steps([track("a", 1), track("b", 0), track("c", 1)])
        self.assertEqual(ran, ["a", "b", "c"])


class StaticAnalysisSwitchTest(unittest.TestCase):
    """只跑静态分析的开关生效，步骤集合正确。"""

    def setUp(self):
        self.steps = [
            green("L0 parity"),
            green("L0 contrast"),
            green("L1 swift test", static_analysis=False),
            green("L1 asan", static_analysis=False),
        ]

    def test_default_runs_every_step(self):
        picked = [s.name for s in tasks.select_steps(self.steps, l0_only=False)]
        self.assertEqual(len(picked), 4)

    def test_l0_only_keeps_static_analysis_steps(self):
        picked = [s.name for s in tasks.select_steps(self.steps, l0_only=True)]
        self.assertEqual(picked, ["L0 parity", "L0 contrast"])

    def test_l0_only_still_reports_failures(self):
        steps = [red("L0 parity"), green("L1 swift test", static_analysis=False)]
        out = io.StringIO()
        with redirect_stdout(out):
            code = tasks.run_steps(steps, l0_only=True)
        self.assertEqual(code, 1)

    def test_flag_parsing_sets_the_switch(self):
        self.assertTrue(tasks.parse_test_flags(["--l0-only"], env={}).l0_only)
        self.assertFalse(tasks.parse_test_flags([], env={}).l0_only)


class MacosStepTableTest(unittest.TestCase):
    """迁移后的全量测试步骤集合（#262）：L1 段必须被 --l0-only 排除。"""

    def setUp(self):
        out = io.StringIO()
        with redirect_stdout(out):
            self.steps = task_test.macos_steps(tasklib.repo_root())

    def test_every_l0_check_script_is_a_step(self):
        scripts = sorted((tasklib.repo_root() / "testing" / "l0").glob("check-*.py"))
        listed = [s.name for s in self.steps]
        for script in scripts:
            self.assertTrue(any(script.name in name for name in listed), script.name)

    def test_l0_only_drops_the_swift_test_steps(self):
        picked = [s.name for s in tasks.select_steps(self.steps, l0_only=True)]
        self.assertFalse([n for n in picked if "swift test" in n or "ASan" in n])
        self.assertTrue([n for n in picked if "零 emoji" in n])


class ExpectedFailureWaiverTest(unittest.TestCase):
    """显式豁免开关（命令行与环境变量两种形式）生效时才容错，默认不容错。"""

    def test_default_does_not_tolerate(self):
        flags = tasks.parse_test_flags([], env={})
        self.assertFalse(flags.allow_expected)
        out = io.StringIO()
        with redirect_stdout(out):
            code = tasks.run_steps([red("a")], allow_expected=flags.allow_expected)
        self.assertEqual(code, 1)

    def test_command_line_form_tolerates(self):
        flags = tasks.parse_test_flags(["--allow-expected-failures"], env={})
        self.assertTrue(flags.allow_expected)
        out = io.StringIO()
        with redirect_stdout(out):
            code = tasks.run_steps([red("a")], allow_expected=flags.allow_expected)
        self.assertEqual(code, 0)

    def test_environment_variable_form_tolerates(self):
        flags = tasks.parse_test_flags([], env={"ALLOW_EXPECTED_FAILURES": "1"})
        self.assertTrue(flags.allow_expected)
        out = io.StringIO()
        with redirect_stdout(out):
            code = tasks.run_steps([red("a")], allow_expected=flags.allow_expected)
        self.assertEqual(code, 0)

    def test_environment_variable_off_does_not_tolerate(self):
        self.assertFalse(
            tasks.parse_test_flags([], env={"ALLOW_EXPECTED_FAILURES": "0"}).allow_expected
        )

    def test_waiver_does_not_hide_the_failure_from_the_summary(self):
        out = io.StringIO()
        with redirect_stdout(out):
            tasks.run_steps([red("L0 parity")], allow_expected=True)
        self.assertIn("L0 parity", out.getvalue())

    def test_unknown_flag_is_a_usage_error(self):
        with self.assertRaises(SystemExit):
            tasks.parse_test_flags(["--nope"], env={})


if __name__ == "__main__":
    unittest.main()
