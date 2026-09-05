#!/usr/bin/env python3
"""配色生成器（scripts/gen-palette.py）自身的测试（零依赖，stdlib unittest）。

只断言外部行为：给定一份最小配色声明，产物里出现哪些关键片段。
不断言内部字符串拼接方式。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]


def _load_generator():
    spec = importlib.util.spec_from_file_location(
        "gen_palette", REPO_ROOT / "scripts" / "gen-palette.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


gen = _load_generator()

MINIMAL = {
    "sources": {
        "local": {"dark": "#8ABCD0", "light": "#3A7A8C"},
    },
}


def make_tree(root: Path) -> None:
    swift = root / gen.SWIFT_THEME
    swift.parent.mkdir(parents=True, exist_ok=True)
    swift.write_text(
        "extension ShapeStyle {\n"
        f"{gen.SWIFT_SOURCE_BEGIN}\n"
        "    // 旧内容\n"
        f"{gen.SWIFT_SOURCE_END}\n"
        "}\n", encoding="utf-8")
    cpp = root / gen.CPP_CORE
    cpp.parent.mkdir(parents=True, exist_ok=True)
    cpp.write_text(
        "struct Track {\n"
        f"{gen.CPP_SOURCE_BEGIN}\n"
        "        // 旧内容\n"
        f"{gen.CPP_SOURCE_END}\n"
        "};\n", encoding="utf-8")


class SwiftSourceColoursTest(unittest.TestCase):
    def setUp(self):
        self.text = "\n".join(gen.swift_source_lines(MINIMAL))

    def test_property_name_comes_from_the_source_type(self):
        self.assertIn("public static var rhythmSourceLocal: Color {", self.text)

    def test_both_appearances_are_emitted_as_byte_components(self):
        self.assertIn("NSColor(red: 0x8A / 255.0, green: 0xBC / 255.0, blue: 0xD0 / 255.0, alpha: 1.0)",
                      self.text)
        self.assertIn("NSColor(red: 0x3A / 255.0, green: 0x7A / 255.0, blue: 0x8C / 255.0, alpha: 1.0)",
                      self.text)

    def test_region_markers_wrap_the_output(self):
        self.assertTrue(self.text.startswith(gen.SWIFT_SOURCE_BEGIN))
        self.assertTrue(self.text.endswith(gen.SWIFT_SOURCE_END))


class CppSourceTableTest(unittest.TestCase):
    def setUp(self):
        self.text = "\n".join(gen.cpp_source_lines(MINIMAL))

    def test_table_entry_carries_both_appearances(self):
        self.assertIn('{L"local", {0x8A, 0xBC, 0xD0}, {0x3A, 0x7A, 0x8C}},', self.text)

    def test_region_markers_wrap_the_output(self):
        self.assertTrue(self.text.startswith(gen.CPP_SOURCE_BEGIN))
        self.assertTrue(self.text.endswith(gen.CPP_SOURCE_END))


class GenerateTest(unittest.TestCase):
    def test_only_the_marked_region_is_replaced(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_tree(root)
            out = gen.generate(MINIMAL, str(root))
            swift = out[gen.SWIFT_THEME]
            self.assertTrue(swift.startswith("extension ShapeStyle {\n"))
            self.assertTrue(swift.endswith("}\n"))
            self.assertNotIn("旧内容", swift)
            self.assertIn("rhythmSourceLocal", swift)

    def test_missing_marker_is_an_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            make_tree(root)
            path = root / gen.SWIFT_THEME
            path.write_text("no markers here\n", encoding="utf-8")
            with self.assertRaises(SystemExit):
                gen.generate(MINIMAL, str(root))

    def test_eight_digit_hex_declarations_contribute_only_their_rgb(self):
        self.assertEqual(gen.rgb("#4CABC8D4"), (0xAB, 0xC8, 0xD4))
        self.assertEqual(gen.rgb("#ABC8D4"), (0xAB, 0xC8, 0xD4))


if __name__ == "__main__":
    unittest.main()
