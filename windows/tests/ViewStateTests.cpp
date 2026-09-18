// VS：视图状态模块行为清单（manifest: docs/testing/behavior/windows-viewstate.md，#317）。
// 「界面该渲染什么」是纯函数：输入 AppState，输出普通结构体，不经 XAML 即可断言（#331）。

#include "BehaviorPch.h"
#include "ViewState.h"

#include <catch_amalgamated.hpp>
#include "TestHelpers.h"

#include <algorithm>

using namespace rhythm;
using namespace rhythm_tests;

// ─── VS-01 资料库行（#331）──────────────────────────────────────────

TEST_CASE("VS-01 an empty library renders no rows") {
    AppState state;
    REQUIRE(view::LibraryRows(state).empty());
}

TEST_CASE("VS-01 every library track renders as one row carrying the track") {
    AppState state;
    state.Tracks = {makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha"),
                    makeLocalTrack(L"C:\\m\\b.mp3", L"Beta")};

    auto rows = view::LibraryRows(state);

    REQUIRE(rows.size() == 2);
    for (const auto& row : rows) {
        REQUIRE(row.title == row.track.title);
    }
    auto hasTitle = [&](const wchar_t* title) {
        return std::any_of(rows.begin(), rows.end(),
                           [&](const view::TrackRow& r) { return r.title == title; });
    };
    REQUIRE(hasTitle(L"Alpha"));
    REQUIRE(hasTitle(L"Beta"));
}
