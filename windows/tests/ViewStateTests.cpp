// VS：视图状态模块行为清单（manifest: docs/testing/behavior/windows-viewstate.md，#317）。
// 「界面该渲染什么」是纯函数：输入 AppState，输出普通结构体，不经 XAML 即可断言（#331）。

#include "BehaviorPch.h"
#include "ViewState.h"

#include <catch_amalgamated.hpp>
#include "TestHelpers.h"

#include <algorithm>
#include <cmath>

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

// ─── VS-02 播放条进度百分比（#332）──────────────────────────────────

TEST_CASE("VS-02 progress is position over duration when the duration is known") {
    AppState state;
    state.Position = 30.0;
    state.Duration = 120.0;
    REQUIRE(view::PlayerBarState(state).progressPercent == 25.0);
}

TEST_CASE("VS-03 an unknown (zero) duration renders zero progress, never a division") {
    AppState state;
    state.Position = 5.0;
    state.Duration = 0.0;
    auto percent = view::PlayerBarState(state).progressPercent;
    REQUIRE(std::isfinite(percent));
    REQUIRE(percent == 0.0);
}

TEST_CASE("VS-03 progress drops to zero when the duration becomes unknown") {
    // The view used to skip the update on a zero duration, leaving the
    // previous track's progress on screen.
    AppState state;
    state.Position = 60.0;
    state.Duration = 120.0;
    REQUIRE(view::PlayerBarState(state).progressPercent == 50.0);
    state.Duration = 0.0;
    REQUIRE(view::PlayerBarState(state).progressPercent == 0.0);
}

TEST_CASE("VS-04 a position outside the duration is clamped into range") {
    AppState state;
    state.Duration = 100.0;
    state.Position = 130.0;
    REQUIRE(view::PlayerBarState(state).progressPercent == 100.0);
    state.Position = -3.0;
    REQUIRE(view::PlayerBarState(state).progressPercent == 0.0);
}
