#!/usr/bin/env python3
"""任务入口（scripts/tasks.py）的行为测试。

范围收窄到唯一曾经付过代价的那条：退出码聚合（#144 的缺陷形状——全量入口
即使测试全红仍以零退出）。构建任务、日志落点、子进程调用不在此立清单。
"""

from __future__ import annotations

import io
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
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


class SharedStaticAnalysisPrefixTest(unittest.TestCase):
    """新增一个校验脚本，两个平台自动纳入（#344）。"""

    def test_new_check_script_joins_both_platforms(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "testing" / "l0").mkdir(parents=True)
            (root / "testing" / "l0" / "check-brand-new.py").write_text("", encoding="utf-8")
            out = io.StringIO()
            with redirect_stdout(out):
                macos = [s.name for s in task_test.macos_steps(root)]
            windows = [s.name for s in task_test.windows_steps(root)]
        for platform, names in (("macos", macos), ("windows", windows)):
            self.assertTrue([n for n in names if "check-brand-new.py" in n], platform)


class EmptyStepSetTest(unittest.TestCase):
    """零步集合不得判为通过（#343）：一步没跑是最危险的绿。"""

    def run_quietly(self, steps, **kwargs):
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = tasks.run_steps(steps, **kwargs)
        return code, out.getvalue() + err.getvalue()

    def test_empty_step_list_exits_non_zero_and_says_nothing_ran(self):
        code, output = self.run_quietly([])
        self.assertNotEqual(code, 0)
        self.assertIn("没有步骤被执行", output)
        self.assertNotIn("全部通过", output)

    def test_filter_that_selects_nothing_exits_non_zero(self):
        steps = [green("L1 swift test", static_analysis=False)]
        code, output = self.run_quietly(steps, l0_only=True)
        self.assertNotEqual(code, 0)
        self.assertIn("没有步骤被执行", output)

    def test_waiver_does_not_rescue_an_empty_set(self):
        # 显式豁免只作用于「有失败步骤」的情形，不把零步变成通过。
        code, _ = self.run_quietly([], allow_expected=True)
        self.assertNotEqual(code, 0)

    def test_non_empty_semantics_are_unchanged(self):
        self.assertEqual(self.run_quietly([green("a")])[0], 0)
        self.assertEqual(self.run_quietly([red("a")])[0], 1)
        self.assertEqual(self.run_quietly([red("a")], allow_expected=True)[0], 0)

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

    def test_step_order_is_unchanged(self):
        # 共享前缀提取后 macOS 的集合与顺序不变（#344）：先 L0 脚本（按名排序），紧接零 emoji。
        scripts = sorted((tasklib.repo_root() / "testing" / "l0").glob("check-*.py"))
        names = [s.name for s in self.steps]
        for i, script in enumerate(scripts):
            self.assertIn(script.name, names[i])
        self.assertIn("零 emoji", names[len(scripts)])
        self.assertIn("L0 校验脚本自测", names[len(scripts) + 1])
        self.assertIn("编排层自测", names[len(scripts) + 2])
        self.assertIn("拷贝 L1", names[len(scripts) + 3])

    def test_l0_only_drops_the_swift_test_steps(self):
        picked = [s.name for s in tasks.select_steps(self.steps, l0_only=True)]
        self.assertFalse([n for n in picked if "swift test" in n or "ASan" in n])
        self.assertTrue([n for n in picked if "零 emoji" in n])


class WindowsStepTableTest(unittest.TestCase):
    """Windows 测试步骤表（#264）：L1 齐备，冒烟段是可选开关；
    未实现的 L2 不得占着步骤（#387）。"""

    def setUp(self):
        self.root = tasklib.repo_root()

    def test_every_l0_check_script_is_a_step(self):
        # 共享静态分析前缀（#344）：Windows 与 macOS 跑同一组校验脚本。
        scripts = sorted((self.root / "testing" / "l0").glob("check-*.py"))
        listed = [s.name for s in task_test.windows_steps(self.root)]
        for script in scripts:
            self.assertTrue(any(script.name in name for name in listed), script.name)

    def test_emoji_and_self_tests_run_on_windows(self):
        # 零 emoji 与两组自测并入共享前缀，Windows 上也执行（#345）。
        names = " | ".join(s.name for s in task_test.windows_steps(self.root))
        for segment in ("零 emoji", "L0 校验脚本自测", "编排层自测"):
            self.assertIn(segment, names)

    def test_full_windows_entry_runs_at_least_twelve_steps(self):
        self.assertGreaterEqual(len(task_test.windows_steps(self.root)), 12)

    def test_l0_only_runs_the_static_analysis_steps(self):
        picked = tasks.select_steps(task_test.windows_steps(self.root), l0_only=True)
        scripts = list((self.root / "testing" / "l0").glob("check-*.py"))
        self.assertGreaterEqual(len(picked), len(scripts))

    def test_l1_segments_are_present(self):
        names = " | ".join(s.name for s in task_test.windows_steps(self.root))
        for segment in ("L1 颜色测试", "L1b 应用工程测试"):
            self.assertIn(segment, names)

    def test_unimplemented_l2_is_not_a_step(self):
        # 截屏宿主没有工程文件、golden 目录不存在：这类步骤注定失败（#387）。
        names = [s.name for s in task_test.windows_steps(self.root, smoke=True)]
        self.assertFalse([n for n in names if n.startswith("L2")], names)

    def test_configure_step_names_the_missing_project_file(self):
        # 源目录里没有工程文件时，失败信息指出缺的是哪个文件，而不是 cmake 的笼统报错。
        with tempfile.TemporaryDirectory() as tmp:
            steps = task_test.windows_steps(Path(tmp))
            configure = [s for s in steps if "cmake 配置" in s.name]
            self.assertTrue(configure)
            for step in configure:
                out = io.StringIO()
                with redirect_stdout(out):
                    code = step.action()
                self.assertEqual(code, 1, step.name)
                self.assertIn("CMakeLists.txt", out.getvalue(), step.name)

    def test_smoke_segment_is_opt_in(self):
        without = task_test.windows_steps(self.root, smoke=False)
        with_smoke = task_test.windows_steps(self.root, smoke=True)
        self.assertEqual(len(with_smoke), len(without) + 1)
        self.assertFalse([s for s in without if "冒烟" in s.name])
        self.assertTrue([s for s in with_smoke if "冒烟" in s.name])

    def test_smoke_flag_is_rejected_where_there_is_no_smoke_segment(self):
        self.assertTrue(tasks.parse_test_flags(["--smoke"], env={}, allow_smoke=True).smoke)
        with self.assertRaises(SystemExit):
            tasks.parse_test_flags(["--smoke"], env={}, allow_smoke=False)


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
