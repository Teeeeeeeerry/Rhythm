// L1: Windows 来源徽标色单元测试——直接测真实 RhythmCore.h（#122 解除桩）。
//
// 断言项（对应方案 L1 Windows 组）：
//   1. 4 个来源类型的 dark/light 双端色值与 palette.json sources 段一致；
//   2. SourceBackgroundBrush alpha == 38（≈ 15%，与 macOS opacity(0.15) 一致）；
//   3. 未知来源类型回退：颜色回退到主题次要文字色（非系统 Gray）。
//
// 构建：见同目录 CMakeLists.txt（cmake -S . -B build && cmake --build build
//       && ctest --test-dir build --output-on-failure）

#include <iostream>
#include <string>
#include <string_view>

#include "Rhythm/Bridge/RhythmCore.h"

// ---- 期望值（palette.json sources 段；与 RhythmCore.h 同步）----
struct Expect {
    std::wstring_view type;
    std::wstring_view dark;
    std::wstring_view light;
};

static constexpr Expect kExpects[] = {
    {L"local",       L"#8ABCD0", L"#3A7A8C"},
    {L"youtube",     L"#D49573", L"#8B4A28"},
    {L"bilibili",    L"#C88DA8", L"#8C4D68"},
    {L"direct_url",  L"#8CB89A", L"#4C785A"},
};

static int g_failures = 0;

static void check(bool ok, const wchar_t* label) {
    if (!ok) {
        std::wcerr << L"FAIL: " << label << L"\n";
        ++g_failures;
    }
}

int main() {
    // Brush 构造需要已初始化的 WinRT apartment（同 windows/tests 的 main）。
    winrt::init_apartment();

    rhythm::Track track;  // 真实模型（SourceColor / SourceBackgroundBrush 均内联于头文件）

    // 1) 双端色值
    for (const auto& e : kExpects) {
        check(track.SourceColor(e.type, /*isDark=*/true) == e.dark,
              (std::wstring(L"dark 变体 ") + e.type.data()).c_str());
        check(track.SourceColor(e.type, /*isDark=*/false) == e.light,
              (std::wstring(L"light 变体 ") + e.type.data()).c_str());
    }

    // 2) 徽标背景 alpha == 38（≈ 15%）
    check(track.SourceBackgroundBrush().Color().A == 38, L"SourceBackgroundBrush alpha == 38");

    // 3) 未知类型回退：不得返回系统 Gray；回退到主题次要文字色
    //    （F4 在 macOS 侧同语义：SourceTagView 回退 .rhythmTextTertiary）
    {
        auto fallback_dark = track.SourceColor(L"unknown_source", /*isDark=*/true);
        check(fallback_dark != L"Gray", L"未知类型 dark 回退非系统 Gray");
        check(fallback_dark == L"#ABC8D4" || fallback_dark == L"#B2ABC8D4"
                  || fallback_dark == L"#8CABC8D4",
              L"未知类型 dark 回退为 teal 文字系（secondary/tertiary）");
        auto fallback_light = track.SourceColor(L"unknown_source", /*isDark=*/false);
        check(fallback_light != L"Gray", L"未知类型 light 回退非系统 Gray");
    }

    if (g_failures == 0) {
        std::wcout << L"OK: source color tests passed\n";
        return 0;
    }
    std::wcerr << L"FAIL: " << g_failures << L" assertion(s)\n";
    return 1;
}
