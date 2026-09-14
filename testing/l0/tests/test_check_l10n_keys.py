#!/usr/bin/env python3
"""check-l10n-keys.py 自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一棵文件树，校验返回零还是非零、失败时报出哪个键或哪个访问器。
夹具里的生成物（含 #371 的访问器）由真实生成器产出，保证比对的是
「键表 -> 生成物」这条链路本身。

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

L10N_H = """#pragma once
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
} // namespace L10n
} // namespace rhythm
#include "L10nAccessors.h"
"""


def build_tree(root: Path, table: dict, *, accessors: str | None = None,
               swift: str | None = None, caller: str = "") -> None:
    """写出最小可识别的仓库树。accessors / swift 缺省为生成器对 table 的产出。"""
    windows_keys = sorted(gen_l10n.entries_for(table, "windows"))
    files = {
        "contracts/l10n-keys.json": json.dumps(table, ensure_ascii=False),
        "macos/Rhythm/Models/L10nKeys.swift": gen_l10n.gen_swift(table) if swift is None else swift,
        "windows/Rhythm/Bridge/L10nKeys.h": gen_l10n.gen_cpp(table),
        "windows/Rhythm/L10nAccessors.h":
            gen_l10n.gen_cpp_accessors(table) if accessors is None else accessors,
        "windows/Rhythm/L10n.h": L10N_H % "\n".join(
            f"        L10N_ENTRY({k})" for k in windows_keys),
        "windows/Rhythm/Views/Caller.cpp": caller,
    }
    for rel, text in files.items():
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


class CheckL10nAccessorTests(unittest.TestCase):
    def run_check(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root)],
            capture_output=True, text=True, encoding="utf-8",
        )

    def test_generated_accessors_cover_the_key_table_and_every_call(self):
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), TABLE, caller="auto a = rhythm::L10n::TrayQuit();\n")
            result = self.run_check(Path(tmp))
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_called_accessor_without_definition_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), TABLE, caller="auto a = rhythm::L10n::QuitNow();\n")
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("QuitNow", result.stdout)
        self.assertIn("Caller.cpp", result.stdout)

    def test_new_key_without_regenerating_accessors_fails(self):
        # 键表加了一条，访问器却还是旧的生成物：漂移与无访问器都报出。
        table = {"keys": {**TABLE["keys"],
                          "play_mode_tooltip": {"zh": "播放模式", "en": "Play Mode"}}}
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), table, accessors=gen_l10n.gen_cpp_accessors(TABLE))
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("play_mode_tooltip", result.stdout)
        self.assertIn("L10nAccessors.h", result.stdout)

    def test_key_read_only_by_the_lookup_table_is_not_an_accessor(self):
        # Key() 的函数体列着全部键；手删掉一个访问器后，只出现在那里的键仍算没有访问器。
        generated = gen_l10n.gen_cpp_accessors(TABLE)
        edited = "\n".join(line for line in generated.splitlines()
                           if 'Key("tray_quit")' not in line) + "\n"
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), TABLE, accessors=edited)
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("tray_quit", result.stdout)


PLATFORM_TABLE = {
    "keys": {
        **TABLE["keys"],
        "yt_dlp_install_command": {"zh": "brew install yt-dlp", "en": "brew install yt-dlp",
                                   "platform": "macos"},
        "yt_dlp_install_command_windows": {"zh": "winget install yt-dlp",
                                           "en": "winget install yt-dlp", "platform": "windows"},
    }
}


class PlatformSplitTests(unittest.TestCase):
    """平台差异键只进对应平台生成物，校验器对两端分别计算（#372）。"""

    def run_check(self, root: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root)],
            capture_output=True, text=True, encoding="utf-8",
        )

    def test_generated_outputs_keep_platform_keys_apart(self):
        accessors = gen_l10n.gen_cpp_accessors(PLATFORM_TABLE)
        values = gen_l10n.gen_cpp(PLATFORM_TABLE)
        swift = gen_l10n.gen_swift(PLATFORM_TABLE)
        self.assertIn('Key("yt_dlp_install_command_windows")', accessors)
        self.assertNotIn('Key("yt_dlp_install_command")', accessors)
        self.assertNotIn("L10nKeys_zh_yt_dlp_install_command()", values)
        self.assertIn('"yt_dlp_install_command":', swift)
        self.assertNotIn('"yt_dlp_install_command_windows":', swift)

    def test_split_tree_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), PLATFORM_TABLE)
            result = self.run_check(Path(tmp))
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_macos_key_leaking_into_windows_accessors_fails(self):
        leaked = gen_l10n.gen_cpp_accessors(PLATFORM_TABLE).replace(
            "} // namespace L10n",
            'inline std::wstring YtDlpInstallCommand() { return Key("yt_dlp_install_command"); }\n'
            "} // namespace L10n")
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), PLATFORM_TABLE, accessors=leaked)
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("macOS 专用键", result.stdout)
        self.assertIn("yt_dlp_install_command", result.stdout)

    def test_windows_key_leaking_into_swift_table_fails(self):
        leaked = gen_l10n.gen_swift(PLATFORM_TABLE).replace(
            "    ]\n",
            '        "yt_dlp_install_command_windows": (zh: "winget", en: "winget"),\n    ]\n', 1)
        with tempfile.TemporaryDirectory() as tmp:
            build_tree(Path(tmp), PLATFORM_TABLE, swift=leaked)
            result = self.run_check(Path(tmp))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Windows 专用键", result.stdout)
        self.assertIn("yt_dlp_install_command_windows", result.stdout)


class AccessorNamingTests(unittest.TestCase):
    """生成器的访问器命名规则（#371）。"""

    def test_pascal_case_template_suffix_and_pinned_name(self):
        entries = gen_l10n.entries_for({"keys": {
            "tray_quit": {"zh": "退出", "en": "Quit"},
            "imported_tracks": {"zh": "已导入 {count} 首", "en": "Imported {count}"},
            "url_play": {"zh": "播放链接", "en": "Play URL", "accessor": "PlayUrl"},
        }}, "windows")
        self.assertEqual(gen_l10n.accessor_name("tray_quit", entries["tray_quit"]), "TrayQuit")
        self.assertEqual(gen_l10n.accessor_name("imported_tracks", entries["imported_tracks"]),
                         "ImportedTracksTemplate")
        self.assertEqual(gen_l10n.accessor_name("url_play", entries["url_play"]), "PlayUrl")

    def test_duplicate_accessor_names_are_rejected(self):
        table = {"keys": {
            "no_playlists": {"zh": "a", "en": "a", "accessor": "PlaylistEmpty"},
            "playlist_empty": {"zh": "b", "en": "b"},
        }}
        with self.assertRaises(ValueError):
            gen_l10n.gen_cpp_accessors(table)


if __name__ == "__main__":
    unittest.main()
