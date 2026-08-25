#!/usr/bin/env python3
"""Generate the dual-platform codec bindings from contracts/ffi-contract.json (#180).

Single contract declaration -> Swift and C++ codec bindings, so field lists
and enum values never need to be synced by hand between the platforms.

Outputs:
  macos/Rhythm/Models/GeneratedCodec.swift
  windows/Rhythm/Bridge/GeneratedCodec.h

Run: python3 scripts/gen-ffi-bindings.py
"""

import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCHEMA = os.path.join(ROOT, "contracts", "ffi-contract.json")
SWIFT_OUT = os.path.join(ROOT, "macos", "Rhythm", "Models", "GeneratedCodec.swift")
CPP_OUT = os.path.join(ROOT, "windows", "Rhythm", "Bridge", "GeneratedCodec.h")


def camel(key: str) -> str:
    """snake_case key -> camelCase (Swift/C++ property names)."""
    parts = key.split("_")
    return parts[0] + "".join(p.title() for p in parts[1:])


def load_schema() -> dict:
    with open(SCHEMA, encoding="utf-8") as f:
        return json.load(f)


def fields_of(schema: dict, name: str) -> dict:
    return schema[name]


# ─── Swift codec ─────────────────────────────────────────────────────

SWIFT_HEADER = """// 本文件由 scripts/gen-ffi-bindings.py 从 contracts/ffi-contract.json 生成（#180）。
// 请勿手改——改契约后重新生成。

import Foundation

/// 契约驱动的编解码（#180）：与 Codable+convertFromSnakeCase 路径等价，
/// 由契约测试锁定两者产物一致。
enum GeneratedCodec {
"""

SWIFT_FOOTER = """
}
"""


def swift_type(t: str) -> str:
    base, _, opt = t.partition("?")
    if base in ("i64", "i32"):
        base = "Int"
    elif base == "f64":
        base = "Double"
    elif base == "bool":
        base = "Bool"
    elif base == "string":
        base = "String"
    elif base == "map":
        return "[String: String]"
    return base + ("?" if opt else "")


def swift_decode_object(name: str, fields: dict, model: str) -> str:
    lines = [f"    /// Decode a {name} from the core's snake_case JSON."]
    lines.append(f"    static func decode{model}(_ json: String) -> {model}? {{")
    lines.append("        guard let data = json.data(using: .utf8),")
    lines.append('              let obj = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { return nil }')
    args = []
    for key, t in fields.items():
        prop = camel(key)
        if t == "i64":
            args.append(f"{prop}: (obj[\"{key}\"] as? NSNumber)?.int64Value ?? 0")
        elif t == "i32":
            args.append(f"{prop}: (obj[\"{key}\"] as? NSNumber)?.intValue ?? 0")
        elif t == "f64":
            args.append(f"{prop}: (obj[\"{key}\"] as? NSNumber)?.doubleValue ?? 0")
        elif t == "bool":
            args.append(f"{prop}: (obj[\"{key}\"] as? NSNumber)?.boolValue ?? false")
        elif t == "string":
            args.append(f"{prop}: (obj[\"{key}\"] as? String) ?? \"\"")
        elif t == "string?":
            args.append(f"{prop}: obj[\"{key}\"] as? String")
        elif t == "i32?":
            args.append(f"{prop}: (obj[\"{key}\"] as? NSNumber)?.intValue")
        elif t == "i64?":
            args.append(f"{prop}: (obj[\"{key}\"] as? NSNumber)?.int64Value")
        elif t == "f64?":
            args.append(f"{prop}: (obj[\"{key}\"] as? NSNumber)?.doubleValue")
        elif t == "bool?":
            args.append(f"{prop}: (obj[\"{key}\"] as? NSNumber)?.boolValue")
        elif t == "source_type":
            args.append(f"{prop}: (obj[\"{key}\"] as? String) ?? \"direct_url\"")
        else:
            raise SystemExit(f"unsupported swift type {t} for {key}")
    lines.append(f"        return {model}(")
    for i, a in enumerate(args):
        lines.append("            " + a + ("," if i < len(args) - 1 else ""))
    lines.append("        )")
    lines.append("    }")
    return "\n".join(lines)


def swift_encode_object(name: str, fields: dict, model: str) -> str:
    lines = [f"    /// Encode a {name} with snake_case keys (mirror of the core's JSON)."]
    lines.append(f"    static func encode{model}(_ value: {model}) -> String {{")
    lines.append("        var obj: [String: Any] = [:]")
    for key, t in fields.items():
        prop = camel(key)
        if t.endswith("?"):
            lines.append(f"        if let v = value.{prop} {{ obj[\"{key}\"] = v }}")
        elif t == "map":
            lines.append(f"        obj[\"{key}\"] = value.{prop}")
        else:
            lines.append(f"        obj[\"{key}\"] = value.{prop}")
    lines.append("        guard let data = try? JSONSerialization.data(withJSONObject: obj) else { return \"{}\" }")
    lines.append('        return String(data: data, encoding: .utf8) ?? "{}"')
    lines.append("    }")
    return "\n".join(lines)


def gen_swift(schema: dict) -> str:
    out = [SWIFT_HEADER]
    for model, fields in (("Track", schema["track"]), ("M3u8Entry", schema["m3u8_entry"])):
        out.append(swift_decode_object(model, fields, model))
        out.append("")
        out.append(swift_encode_object(model, fields, model))
        out.append("")
    # List decode for M3u8Entry (the import path consumes the whole list).
    list_decoder = (
        "    /// Decode a list of M3u8Entry objects (the M3U8 import path).\n"
        "    static func decodeM3u8Entries(_ json: String) -> [M3u8Entry]? {\n"
        "        guard let data = json.data(using: .utf8),\n"
        '              let objects = (try? JSONSerialization.jsonObject(with: data)) as? [[String: Any]] else { return nil }\n'
        "        return objects.compactMap { obj in\n"
        "            M3u8Entry(\n"
        '                title: (obj["title"] as? String) ?? "",\n'
        '                artist: obj["artist"] as? String,\n'
        '                location: (obj["location"] as? String) ?? ""\n'
        "            )\n"
        "        }\n"
        "    }\n"
    )
    out.append(list_decoder)
    out.append(SWIFT_FOOTER)
    return "\n".join(out)


# ─── C++ codec ──────────────────────────────────────────────────────

CPP_HEADER = """// 本文件由 scripts/gen-ffi-bindings.py 从 contracts/ffi-contract.json 生成（#180）。
// 请勿手改——改契约后重新生成。
#pragma once

#include <nlohmann/json.hpp>

namespace rhythm::generated {

// Utf8ToWide / WideToUtf8 由 RhythmCore.cpp 提供（见 RhythmCore.h）。
using nlohmann::json;

"""

CPP_FOOTER = """
} // namespace rhythm::generated
"""


def cpp_type(t: str) -> str:
    if t == "i64":
        return "int64_t"
    if t == "i32":
        return "int32_t"
    if t == "f64":
        return "double"
    if t == "bool":
        return "bool"
    if t == "string":
        return "std::wstring"
    if t == "map":
        return "std::map<std::wstring, std::wstring>"
    raise SystemExit(f"unsupported cpp type {t}")


def cpp_decode_object(name: str, fields: dict, model: str) -> str:
    lines = [f"/// Decode a {name} from the core's snake_case JSON (contract #{name})."]
    lines.append(f"inline {model} {name}FromJson(const json& j) {{")
    lines.append(f"    {model} t;")
    for key, t in fields.items():
        prop = camel(key)
        if t.endswith("?"):
            base = t[:-1]
            lines.append(f"    if (j.contains(\"{key}\") && !j[\"{key}\"].is_null()) {{")
            if base == "string":
                lines.append(f"        t.{prop} = Utf8ToWide(j[\"{key}\"].get<std::string>());")
            elif base in ("i32", "i64"):
                lines.append(f"        t.{prop} = j[\"{key}\"].get<{cpp_type(base)}>();")
            elif base == "f64":
                lines.append(f"        t.{prop} = j[\"{key}\"].get<double>();")
            else:
                raise SystemExit(f"unsupported cpp optional type {t} for {key}")
            lines.append("    }")
        elif t in ("string", "source_type"):
            default = "local" if t == "source_type" else ""
            lines.append(f"    t.{prop} = Utf8ToWide(j.value(\"{key}\", std::string(\"{default}\")));")
        elif t == "i64":
            lines.append(f"    t.{prop} = j.value(\"{key}\", (int64_t)0);")
        elif t == "i32":
            lines.append(f"    t.{prop} = j.value(\"{key}\", (int32_t)0);")
        elif t == "f64":
            lines.append(f"    t.{prop} = j.value(\"{key}\", 0.0);")
        elif t == "bool":
            lines.append(f"    t.{prop} = j.value(\"{key}\", false);")
        elif t == "map":
            lines.append(f"    if (j.contains(\"{key}\") && !j[\"{key}\"].is_null()) {{")
            lines.append(f"        for (const auto& [k, v] : j[\"{key}\"].items()) {{")
            lines.append(f"            t.{prop}[Utf8ToWide(k)] = Utf8ToWide(v.get<std::string>());")
            lines.append("        }")
            lines.append("    }")
        else:
            raise SystemExit(f"unsupported cpp type {t} for {key}")
    lines.append("    return t;")
    lines.append("}")
    return "\n".join(lines)


def cpp_encode_object(name: str, fields: dict, model: str) -> str:
    lines = [f"/// Encode a {name} with snake_case keys (contract #{name})."]
    lines.append(f"inline json {name}ToJson(const {model}& t) {{")
    lines.append("    json j;")
    for key, t in fields.items():
        prop = camel(key)
        if t.endswith("?"):
            lines.append(f"    if (t.{prop}) j[\"{key}\"] = WideToUtf8(*t.{prop});")
        elif t in ("string", "source_type"):
            lines.append(f"    j[\"{key}\"] = WideToUtf8(t.{prop});")
        elif t == "i64":
            lines.append(f"    j[\"{key}\"] = t.{prop};")
        elif t == "i32":
            lines.append(f"    j[\"{key}\"] = t.{prop};")
        elif t == "f64":
            lines.append(f"    j[\"{key}\"] = t.{prop};")
        elif t == "bool":
            lines.append(f"    j[\"{key}\"] = t.{prop};")
        elif t == "map":
            lines.append(f"    for (const auto& [k, v] : t.{prop}) {{")
            lines.append(f"        j[\"{key}\"][WideToUtf8(k)] = WideToUtf8(v);")
            lines.append("    }")
        else:
            raise SystemExit(f"unsupported cpp type {t} for {key}")
    lines.append("    return j;")
    lines.append("}")
    return "\n".join(lines)


def gen_cpp(schema: dict) -> str:
    out = [CPP_HEADER]
    for name, fields in (("Track", schema["track"]), ("M3u8Entry", schema["m3u8_entry"])):
        out.append(cpp_decode_object(name, fields, name))
        out.append("")
        out.append(cpp_encode_object(name, fields, name))
        out.append("")
    out.append(CPP_FOOTER)
    return "\n".join(out)


def main() -> int:
    schema = load_schema()
    swift = gen_swift(schema)
    cpp = gen_cpp(schema)
    with open(SWIFT_OUT, "w", encoding="utf-8") as f:
        f.write(swift)
    with open(CPP_OUT, "w", encoding="utf-8") as f:
        f.write(cpp)
    print(f"wrote {SWIFT_OUT}")
    print(f"wrote {CPP_OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
