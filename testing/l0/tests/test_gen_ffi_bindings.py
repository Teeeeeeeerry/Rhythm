#!/usr/bin/env python3
"""scripts/gen-ffi-bindings.py 的 C++ 生成物测试（零依赖，stdlib unittest）。

只断言外部行为：给定一份契约，生成哪些对象的编解码、每个字段产出什么形状。
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

# EVERY_TYPE 所在的契约：只需声明它引用的枚举。
EVERY_TYPE_SCHEMA = {"enums": {"source_type": ["local", "direct_url"]}}


class CppEncoderShapeTests(unittest.TestCase):
    """#360：编码器按字段声明类型分派，只有字符串走宽字符串转换。"""

    def setUp(self):
        encoder = gen.cpp_encode_object("Sample", EVERY_TYPE, "Sample", EVERY_TYPE_SCHEMA)
        self.lines = [line.strip() for line in encoder.splitlines()]

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
        encoder = gen.cpp_encode_object("Track", schema["track"], "Track", schema)
        for key, t in schema["track"].items():
            if t in ("i32?", "i64?", "f64?", "bool?"):
                self.assertNotIn(f'j["{key}"] = WideToUtf8(', encoder, key)


class CppEncodeDispatchTests(unittest.TestCase):
    """#413：必选与可选字段对同一类型产出同一编码表达式。"""

    # 必选字段 -> 同类型的可选字段（EVERY_TYPE 里的成对字段）。
    PAIRS = {"name": "maybeName", "count": "maybeCount", "size": "maybeSize",
             "ratio": "maybeRatio", "flag": "maybeFlag"}

    def test_optional_field_encodes_like_its_required_counterpart(self):
        lines = [line.strip() for line in
                 gen.cpp_encode_object("Sample", EVERY_TYPE, "Sample",
                                       EVERY_TYPE_SCHEMA).splitlines()]
        for required, maybe in self.PAIRS.items():
            required_rhs = next(l for l in lines if l.startswith(f'j["{required}"] = '))
            optional_rhs = next(l for l in lines if l.startswith(f"if (t.{maybe}) "))
            self.assertEqual(optional_rhs.split(" = ", 1)[1],
                             required_rhs.split(" = ", 1)[1].replace(f"t.{required}", f"*t.{maybe}"),
                             required)

    def test_unsupported_type_is_rejected_in_both_branches(self):
        for t in ("blob", "blob?"):
            with self.assertRaises(SystemExit):
                gen.cpp_encode_object("Sample", {"x": t}, "Sample", EVERY_TYPE_SCHEMA)


class CppIncludesTests(unittest.TestCase):
    """#361：生成物自带它使用的头文件包含，不依赖调用方的预编译头。"""

    def include_block(self, text: str) -> list[str]:
        return [line for line in text.splitlines() if line.startswith("#include")]

    def test_real_contract_output_declares_what_it_uses(self):
        cpp = gen.gen_cpp(gen.load_schema())
        includes = self.include_block(cpp)
        for header in ("<cstdint>", "<optional>", "<string>", "<nlohmann/json.hpp>"):
            self.assertIn(f"#include {header}", includes)

    def test_output_includes_the_headers_declaring_models_and_conversions(self):
        # #413：模型与 Utf8ToWide 在 RhythmCore.h，WideToUtf8 在 MessageSpec.h；
        # 生成物自己包含它们，单独包含 GeneratedCodec.h 即可编译。
        includes = self.include_block(gen.gen_cpp(gen.load_schema()))
        self.assertIn('#include "RhythmCore.h"', includes)
        self.assertIn('#include "MessageSpec.h"', includes)

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
        # 每个契约对象一个解码、一个编码。
        self.assertEqual(sum(1 for line in body if line.startswith("inline ")),
                         2 * len(gen.cpp_objects(schema)))


# 一份最小契约：一个枚举、一个被引用的对象、一个引用它的对象。
MINI_CONTRACT = {
    "version": 1,
    "doc": "mini",
    "enums": {"shade": ["light", "dark"]},
    "inner": {"n": "i32"},
    "outer": {
        "shade": "shade",
        "maybe_shade": "shade?",
        "inner": "inner?",
    },
}


class CppContractScopeTests(unittest.TestCase):
    """#362：生成范围取自契约——声明了的对象都有编解码，不再有另写的清单。"""

    def test_every_declared_object_gets_a_decoder_and_an_encoder(self):
        cpp = gen.gen_cpp(MINI_CONTRACT)
        for model in ("Inner", "Outer"):
            self.assertIn(f"inline {model} {model}FromJson(const json& j) {{", cpp)
            self.assertIn(f"inline json {model}ToJson(const {model}& t) {{", cpp)

    def test_every_real_contract_entry_is_generated(self):
        # #363：除版本、说明与枚举表外，契约里的每个条目都是生成的对象——
        # 「声明了却不生成」的条目不再存在。
        schema = gen.load_schema()
        functions = {line for line in gen.gen_cpp(schema).splitlines() if line.startswith("inline ")}
        for key in schema:
            if key in ("version", "doc", "enums"):
                continue
            model = gen.pascal(key)
            self.assertIn(f"inline {model} {model}FromJson(const json& j) {{", functions, key)
            self.assertIn(f"inline json {model}ToJson(const {model}& t) {{", functions, key)

    def test_the_object_visitor_lists_every_declared_object(self):
        # #365：往返用例经这张表遍历对象；契约新增的对象重新生成后自动在列。
        lines = [line.strip() for line in gen.gen_cpp(MINI_CONTRACT).splitlines()]
        self.assertIn('visit("inner", InnerFromJson, InnerToJson);', lines)
        self.assertIn('visit("outer", OuterFromJson, OuterToJson);', lines)
        self.assertLess(lines.index('visit("inner", InnerFromJson, InnerToJson);'),
                        lines.index('visit("outer", OuterFromJson, OuterToJson);'))

    def test_a_non_object_entry_is_rejected(self):
        with self.assertRaises(SystemExit):
            gen.gen_cpp({**MINI_CONTRACT, "stray": ["a", "b"]})


class CppReferenceTypeTests(unittest.TestCase):
    """#362：字段可引用契约里的对象与枚举。"""

    def setUp(self):
        self.lines = [line.strip() for line in gen.gen_cpp(MINI_CONTRACT).splitlines()]

    def test_object_field_goes_through_the_referenced_codec(self):
        self.assertIn('t.inner = InnerFromJson(j["inner"]);', self.lines)
        self.assertIn('if (t.inner) j["inner"] = InnerToJson(*t.inner);', self.lines)

    def test_enum_fields_travel_as_strings(self):
        self.assertIn('t.maybeShade = Utf8ToWide(j["maybe_shade"].get<std::string>());', self.lines)
        self.assertIn('j["shade"] = WideToUtf8(t.shade);', self.lines)
        self.assertIn('if (t.maybeShade) j["maybe_shade"] = WideToUtf8(*t.maybeShade);', self.lines)

    def test_a_missing_required_enum_decodes_to_its_first_value(self):
        self.assertIn('t.shade = Utf8ToWide(j.value("shade", std::string("light")));', self.lines)

    def test_an_object_must_be_declared_before_it_is_referenced(self):
        backwards = {"version": 1, "doc": "", "enums": {},
                     "outer": {"inner": "inner?"}, "inner": {"n": "i32"}}
        with self.assertRaises(SystemExit):
            gen.gen_cpp(backwards)


if __name__ == "__main__":
    unittest.main()
