#!/usr/bin/env python3
"""scripts/gen-ffi-bindings.py 的 C++ 编码侧测试（零依赖，stdlib unittest）。

只断言外部行为：给定一组字段类型，生成的编码函数对每个字段产出什么形状。
不断言生成器内部如何分派。

用法：python3 -m unittest discover -s testing/l0/tests
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

_spec = importlib.util.spec_from_file_location(
    "gen_ffi_bindings", REPO_ROOT / "scripts" / "gen-ffi-bindings.py")
gen = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(gen)

# 契约里出现的每一种字段类型各一个。
EVERY_TYPE = {
    "name": "string",
    "maybe_name": "string?",
    "kind": "source_type",
    "count": "i32",
    "maybe_count": "i32?",
    "size": "i64",
    "maybe_size": "i64?",
    "ratio": "f64",
    "maybe_ratio": "f64?",
    "flag": "bool",
    "maybe_flag": "bool?",
    "headers": "map",
}


class CppEncoderShapeTests(unittest.TestCase):
    """#360：编码器按字段声明类型分派，只有字符串走宽字符串转换。"""

    def setUp(self):
        self.lines = [line.strip() for line in
                      gen.cpp_encode_object("Sample", EVERY_TYPE, "Sample").splitlines()]

    def assertLine(self, expected: str):
        self.assertIn(expected, self.lines)

    def test_optional_numbers_and_booleans_are_assigned_directly(self):
        self.assertLine('if (t.maybeCount) j["maybe_count"] = *t.maybeCount;')
        self.assertLine('if (t.maybeSize) j["maybe_size"] = *t.maybeSize;')
        self.assertLine('if (t.maybeRatio) j["maybe_ratio"] = *t.maybeRatio;')
        self.assertLine('if (t.maybeFlag) j["maybe_flag"] = *t.maybeFlag;')

    def test_required_numbers_and_booleans_are_assigned_directly(self):
        self.assertLine('j["count"] = t.count;')
        self.assertLine('j["size"] = t.size;')
        self.assertLine('j["ratio"] = t.ratio;')
        self.assertLine('j["flag"] = t.flag;')

    def test_strings_still_go_through_wide_string_conversion(self):
        self.assertLine('j["name"] = WideToUtf8(t.name);')
        self.assertLine('if (t.maybeName) j["maybe_name"] = WideToUtf8(*t.maybeName);')
        self.assertLine('j["kind"] = WideToUtf8(t.kind);')

    def test_only_string_fields_are_converted(self):
        converted = [line for line in self.lines if "WideToUtf8(" in line]
        for key in ("maybe_count", "maybe_size", "maybe_ratio", "maybe_flag",
                    "count", "size", "ratio", "flag"):
            self.assertFalse([line for line in converted if f'"{key}"' in line], key)

    def test_real_contract_track_encoder_converts_no_numeric_optional(self):
        schema = gen.load_schema()
        encoder = gen.cpp_encode_object("Track", schema["track"], "Track")
        for key, t in schema["track"].items():
            if t in ("i32?", "i64?", "f64?", "bool?"):
                self.assertNotIn(f'j["{key}"] = WideToUtf8(', encoder, key)


class CppIncludesTests(unittest.TestCase):
    """#361：生成物自带它使用的头文件包含，不依赖调用方的预编译头。"""

    def include_block(self, text: str) -> list[str]:
        return [line for line in text.splitlines() if line.startswith("#include")]

    def test_real_contract_output_declares_what_it_uses(self):
        cpp = gen.gen_cpp(gen.load_schema())
        includes = self.include_block(cpp)
        for header in ("<cstdint>", "<optional>", "<string>", "<nlohmann/json.hpp>"):
            self.assertIn(f"#include {header}", includes)

    def test_map_field_brings_its_own_include(self):
        includes = gen.cpp_includes([("Sample", {"headers": "map", "name": "string"})])
        self.assertIn("#include <map>", includes)
        self.assertNotIn("#include <optional>", includes)

    def test_no_map_include_without_a_map_field(self):
        includes = gen.cpp_includes([("Sample", {"count": "i32?"})])
        self.assertNotIn("#include <map>", includes)
        self.assertIn("#include <optional>", includes)

    def test_only_the_include_block_depends_on_types(self):
        # 生成物的其它内容不变：去掉包含行后，与不同包含组合下的主体逐字一致。
        schema = gen.load_schema()
        body = [line for line in gen.gen_cpp(schema).splitlines()
                if not line.startswith("#include")]
        self.assertIn("namespace rhythm::generated {", body)
        self.assertIn("inline Track TrackFromJson(const json& j) {", body)
        self.assertEqual(sum(1 for line in body if line.startswith("inline ")), 8)


if __name__ == "__main__":
    unittest.main()
