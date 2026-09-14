#!/usr/bin/env python3
"""Generate the platform L10n implementations from contracts/l10n-keys.json (#167 组).

The key table (zh/en + platform-diff fields) is the single source of truth;
the macOS `L10nKeys.swift` and the Windows `L10nKeys.h` are generated outputs.
Entries tagged with a "platform" field are only emitted for that platform.

Run: python3 scripts/gen-l10n.py
"""

import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCHEMA = os.path.join(ROOT, "contracts", "l10n-keys.json")
SWIFT_OUT = os.path.join(ROOT, "macos", "Rhythm", "Models", "L10nKeys.swift")
CPP_OUT = os.path.join(ROOT, "windows", "Rhythm", "Bridge", "L10nKeys.h")
# Windows named accessors (#371): the module's call surface, generated from the
# same key table so the checked surface and the called surface are one set.
ACCESSORS_OUT = os.path.join(ROOT, "windows", "Rhythm", "L10nAccessors.h")


def load() -> dict:
    with open(SCHEMA, encoding="utf-8") as f:
        return json.load(f)


def entries_for(table: dict, platform: str) -> dict:
    out = {}
    for key, entry in table["keys"].items():
        tag = entry.get("platform")
        if tag is not None and tag != platform:
            continue
        out[key] = {"zh": entry["zh"], "en": entry["en"]}
        if "accessor" in entry:
            out[key]["accessor"] = entry["accessor"]
    return out


def accessor_name(key: str, entry: dict) -> str:
    """Windows accessor name for a key (#371).

    Rule: PascalCase of the key. A template (text with `{placeholders}`) gets a
    `Template` suffix - the adapter layer fills it in a hand-written function
    that keeps the plain name (e.g. ImportedTracks(count)). The optional
    `accessor` field in the key table pins a legacy name so call sites stay
    untouched.
    """
    if entry.get("accessor"):
        return entry["accessor"]
    base = "".join(part[:1].upper() + part[1:] for part in key.split("_"))
    is_template = "{" in entry["zh"] or "{" in entry["en"]
    return base + "Template" if is_template else base


def swift_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")


def gen_swift(table: dict) -> str:
    entries = entries_for(table, "macos")
    lines = [
        "// 本文件由 scripts/gen-l10n.py 从 contracts/l10n-keys.json 生成（#167 组）。",
        "// 请勿手改——新增文案只改键表，再重新生成。",
        "",
        "import Foundation",
        "",
        "/// 键表生成的文案取值层（单一事实来源）。",
        "enum L10nKeys {",
        "    /// 语言解析：手动覆盖（AppLanguage）优先，否则跟随系统。",
        "    static var isChinese: Bool {",
        "        if let code = UserDefaults.standard.string(forKey: \"AppLanguage\") {",
        "            return Locale(identifier: code).identifier.hasPrefix(\"zh\")",
        "        }",
        "        return Locale.current.identifier.hasPrefix(\"zh\")",
        "    }",
        "",
        "    /// 键表（zh/en，生成自 contracts/l10n-keys.json）。",
        "    private static let table: [String: (zh: String, en: String)] = [",
    ]
    for key in sorted(entries):
        e = entries[key]
        lines.append(f'        "{key}": (zh: "{swift_escape(e["zh"])}", en: "{swift_escape(e["en"])}"),')
    lines += [
        "    ]",
        "",
        "    /// 取当前语言的文案；未知键回退键名（键表缺失会被校验脚本拦截）。",
        "    static func value(_ key: String) -> String {",
        "        guard let entry = table[key] else { return key }",
        "        return isChinese ? entry.zh : entry.en",
        "    }",
        "}",
        "",
    ]
    return "\n".join(lines)


def cpp_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")


def gen_cpp(table: dict) -> str:
    entries = entries_for(table, "windows")
    lines = [
        "// 本文件由 scripts/gen-l10n.py 从 contracts/l10n-keys.json 生成（#167 组）。",
        "// 请勿手改——新增文案只改键表，再重新生成。",
        "#pragma once",
        "",
        "namespace rhythm {",
        "",
        "// 键表生成的文案取值（单一事实来源）。Windows 的语言检测（系统 UI",
        "// 语言 + 注册表覆盖）留在 L10n.h 的 IsChinese()，本层只做键→文案映射。",
        "// 带 {占位符} 的模板由 L10n.h 的 Fill 填充。",
        "",
    ]
    for key in sorted(entries):
        e = entries[key]
        lines.append(f'inline const wchar_t* L10nKeys_zh_{key}() {{ return L"{cpp_escape(e["zh"])}"; }}')
        lines.append(f'inline const wchar_t* L10nKeys_en_{key}() {{ return L"{cpp_escape(e["en"])}"; }}')
    lines += [
        "",
        "} // namespace rhythm",
        "",
    ]
    return "\n".join(lines)


def gen_cpp_accessors(table: dict) -> str:
    """Named accessors for every windows key (#371); names must be unique."""
    entries = entries_for(table, "windows")
    names: dict[str, str] = {}
    for key in sorted(entries):
        name = accessor_name(key, entries[key])
        if name in names:
            raise ValueError(f"accessor name {name} used by both {names[name]} and {key}; "
                             f"pin one with an \"accessor\" field in the key table")
        names[name] = key
    lines = [
        "// 本文件由 scripts/gen-l10n.py 从 contracts/l10n-keys.json 生成（#371）。",
        "// 请勿手改——新增文案只改键表，再重新生成。",
        "//",
        "// Windows L10n 模块的具名访问器：每个 windows 键一个，调用面与检验面是同一份生成物。",
        "// 命名规则：键名转 PascalCase；带 {占位符} 的模板加 Template 后缀（由 L10n.h 的",
        "// 手写函数填充）；键表的 accessor 字段可固定历史名称。",
        "// 只由 L10n.h 在 Key() 定义之后包含。",
        "#pragma once",
        "",
        "#include <string>",
        "",
        "namespace rhythm {",
        "namespace L10n {",
        "",
    ]
    for name in sorted(names, key=lambda n: names[n]):
        lines.append(f'inline std::wstring {name}() {{ return Key("{names[name]}"); }}')
    lines += [
        "",
        "} // namespace L10n",
        "} // namespace rhythm",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    table = load()
    with open(SWIFT_OUT, "w", encoding="utf-8") as f:
        f.write(gen_swift(table))
    with open(CPP_OUT, "w", encoding="utf-8") as f:
        f.write(gen_cpp(table))
    with open(ACCESSORS_OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write(gen_cpp_accessors(table))
    print(f"wrote {SWIFT_OUT}")
    print(f"wrote {CPP_OUT}")
    print(f"wrote {ACCESSORS_OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
