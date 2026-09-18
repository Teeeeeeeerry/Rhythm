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

// ─── VS-05/06/07 播放条时间文案（#333）──────────────────────────────

TEST_CASE("VS-05 buffering renders the buffering copy instead of a clock") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;
        state.IsBuffering = true;
        state.Position = 12.0;
        state.Duration = 200.0;
        REQUIRE(view::PlayerBarState(state).timeText == L10n::Buffering());
    }
}

TEST_CASE("VS-06 not buffering renders position / duration in m:ss") {
    AppState state;
    state.Position = 65.0;
    state.Duration = 200.4;
    REQUIRE(view::PlayerBarState(state).timeText == L"1:05 / 3:20");
    state.Position = 5.9;
    REQUIRE(view::PlayerBarState(state).timeText == L"0:05 / 3:20");
}

TEST_CASE("VS-07 nothing loaded renders 0:00 / 0:00") {
    AppState state;
    REQUIRE(view::PlayerBarState(state).timeText == L"0:00 / 0:00");
    // A position that is not positive reads as 0:00, never "-0:-3".
    state.Position = -3.0;
    state.Duration = 10.0;
    REQUIRE(view::PlayerBarState(state).timeText == L"0:00 / 0:10");
}

// ─── VS-08/09/10 播放条标题与艺人（#334）────────────────────────────

TEST_CASE("VS-08 the current track's title and artist are rendered") {
    AppState state;
    auto track = makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha");
    track.artist = L"Artist A";
    state.CurrentTrack = track;

    auto bar = view::PlayerBarState(state);
    REQUIRE(bar.title == L"Alpha");
    REQUIRE(bar.artist == L"Artist A");
}

TEST_CASE("VS-09 a current track without an artist renders an empty artist") {
    AppState state;
    state.CurrentTrack = makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha");
    REQUIRE(view::PlayerBarState(state).artist.empty());
}

TEST_CASE("VS-10 no current track renders the not-playing copy, never the last track") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;
        auto track = makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha");
        track.artist = L"Artist A";
        state.CurrentTrack = track;
        REQUIRE(view::PlayerBarState(state).title == L"Alpha");

        state.CurrentTrack.reset();
        auto bar = view::PlayerBarState(state);
        REQUIRE(bar.title == L10n::NotPlaying());
        REQUIRE(bar.artist.empty());
    }
}
