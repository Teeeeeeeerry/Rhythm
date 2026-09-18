// L1: Windows 来源徽标色单元测试——直接测真实的视图状态徽标（#122 解除桩；
// #338 起徽标从曲目模型搬进视图状态 ViewState）。
//
// 断言项（对应方案 L1 Windows 组）：
//   1. 4 个来源类型的 dark/light 双端色值与 palette.json sources 段一致；
//   2. 胶囊底 alpha == 38（≈ 15%，与 macOS opacity(0.15) 一致）；
//   3. 未知来源类型回退：颜色回退到正文色（非系统 Gray）。
//
// 构建与运行：python scripts/tasks.py test（配置名取 task_build.WINDOWS_CONFIG，
//       构建的 --config 与 ctest 的 -C 同源，#427）

#include <format>
#include <iostream>
#include <string>
#include <string_view>

#include "ViewState.h"

// ---- 期望值（palette.json sources 段；与生成的色表同步）----
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

static std::wstring hex(rhythm::view::Color c) {
    return std::format(L"#{:02X}{:02X}{:02X}", c.R, c.G, c.B);
}

static std::wstring foreground(std::wstring_view type, bool isDark) {
    return hex(rhythm::view::SourceBadgeOf(type, isDark).foreground);
}

int main() {
    // 1) 双端色值
    for (const auto& e : kExpects) {
        check(foreground(e.type, /*isDark=*/true) == e.dark,
              (std::wstring(L"dark 变体 ") + e.type.data()).c_str());
        check(foreground(e.type, /*isDark=*/false) == e.light,
              (std::wstring(L"light 变体 ") + e.type.data()).c_str());
    }

    // 2) 徽标胶囊底 alpha == 38（≈ 15%）
    check(rhythm::view::SourceBadgeOf(L"local", /*isDark=*/true).background.A == 38
              && rhythm::view::SourceBadgeOf(L"local", /*isDark=*/false).background.A == 38,
          L"胶囊底 alpha == 38");

    // 3) 未知类型回退：不得返回系统 Gray；回退到正文色
    //    （F4 在 macOS 侧同语义：SourceTagView 回退 teal 文字系）
    {
        auto fallback_dark = foreground(L"unknown_source", /*isDark=*/true);
        check(fallback_dark != L"#808080", L"未知类型 dark 回退非系统 Gray");
        check(fallback_dark == L"#ABC8D4", L"未知类型 dark 回退为正文色");
        auto fallback_light = foreground(L"unknown_source", /*isDark=*/false);
        check(fallback_light != L"#808080", L"未知类型 light 回退非系统 Gray");
        check(fallback_light == L"#0D464D", L"未知类型 light 回退为正文色");
    }

    if (g_failures == 0) {
        std::wcout << L"OK: source color tests passed\n";
        return 0;
    }
    std::wcerr << L"FAIL: " << g_failures << L" assertion(s)\n";
    return 1;
}
