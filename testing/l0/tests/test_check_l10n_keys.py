#!/usr/bin/env python3
"""check-l10n-keys.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一棵文件树，校验返回零还是非零、失败时报出哪个键或哪个访问器。
夹具里的生成物由真实生成器产出，保证比对的是「键表 -> 生成物」这条链路本身。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPT = REPO_ROOT / "testing" / "l0" / "check-l10n-keys.py"

_spec = importlib.util.spec_from_file_location("gen_l10n", REPO_ROOT / "scripts" / "gen-l10n.py")
gen_l10n = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(gen_l10n)

TABLE = {
    "keys": {
        "library_tab": {"zh": "资料库", "en": "Library"},
        "tray_quit": {"zh": "退出 Rhythm", "en": "Quit Rhythm"},
    }
}

L10N_H_HEAD = """#pragma once
namespace rhythm {
namespace L10n {
inline bool IsChinese() { return true; }
inline const wchar_t* Key(const char* key) {
    static const Entry kTable[] = {
#define L10N_ENTRY(name) {#name, L10nKeys_zh_##name(), L10nKeys_en_##name()},
%s
#undef L10N_ENTRY
    };
    return L"";
}
"""

L10N_H_TAIL = """
} // namespace L10n
} // namespace rhythm
"""


def build_tree(root: Path, table: dict, accessors: str, caller: str = "") -> None:
    """写出最小可识别的仓库树：键表、双端生成物、L10n.h（访问器）与一个调用方。"""
    files = {
        "contracts/l10n-keys.json": json.dumps(table, ensure_ascii=False),
        "macos/Rhythm/Models/L10nKeys.swift": gen_l10n.gen_swift(table),
        "windows/Rhythm/Bridge/L10nKeys.h": gen_l10n.gen_cpp(table),
        "windows/Rhythm/L10n.h": L10N_H_HEAD % "\n".join(
            f"        L10N_ENTRY({k})" for k in sorted(gen_l10n.entries_for(table, "windows")))
            + accessors + L10N_H_TAIL,
        "windows/Rhythm/Views/Caller.cpp": caller,
    }
    for rel, text in files.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


COMPLETE_ACCESSORS = """
inline std::wstring LibraryTab() { return Key("library_tab"); }
inline std::wstring TrayQuit() { return Key("tray_quit"); }
"""


class CheckL10nAccessorTests(unittest.TestCase):
    def run_check(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root)],
            capture_output=True, text=True, encoding="utf-8",
        )

    def test_every_key_has_an_accessor_and_every_call_is_defined(self):
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), TABLE, COMPLETE_ACCESSORS,
                       caller="auto a = rhythm::L10n::TrayQuit();\n")
            result = self.run_check(Path(tmp))
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_called_accessor_without_definition_fails(self):
        # TrayQuit 被托盘与测试引用，却在 L10n.h 里不存在（#370 的现状）。
        accessors = '\ninline std::wstring LibraryTab() { return Key("library_tab"); }\n' \
                    'inline std::wstring QuitLabel() { return Key("tray_quit"); }\n'
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), TABLE, accessors,
                       caller="auto a = rhythm::L10n::TrayQuit();\n")
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("TrayQuit", result.stdout)
        self.assertIn("Caller.cpp", result.stdout)

    def test_new_key_without_accessor_fails(self):
        table = {"keys": {**TABLE["keys"],
                          "play_mode_tooltip": {"zh": "播放模式", "en": "Play Mode"}}}
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), table, COMPLETE_ACCESSORS)
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("play_mode_tooltip", result.stdout)

    def test_key_read_only_by_the_lookup_table_is_not_an_accessor(self):
        # Key() 的函数体列着全部键；只出现在那里的键仍算没有访问器。
        accessors = '\ninline std::wstring LibraryTab() { return Key("library_tab"); }\n'
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), TABLE, accessors)
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("tray_quit", result.stdout)


if __name__ == "__main__":
    unittest.main()
