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
    REQUIRE(view::LibraryRows(state, view::LibrarySort::ArtistAlbum, true).empty());
}

TEST_CASE("VS-01 every library track renders as one row carrying the track") {
    AppState state;
    state.Tracks = {makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha"),
                    makeLocalTrack(L"C:\\m\\b.mp3", L"Beta")};

    auto rows = view::LibraryRows(state, view::LibrarySort::ArtistAlbum, true);

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

// ─── VS-14/15/16/17 资料库排序（#336）───────────────────────────────

namespace {

Track sortTrack(const wchar_t* title, std::optional<std::wstring> artist,
                std::optional<std::wstring> album, std::optional<int32_t> number) {
    auto t = makeLocalTrack(std::wstring(L"C:\\m\\") + title + L".mp3", title);
    t.artist = std::move(artist);
    t.album = std::move(album);
    t.trackNumber = number;
    return t;
}

std::vector<std::wstring> titlesOf(const std::vector<view::TrackRow>& rows) {
    std::vector<std::wstring> titles;
    for (const auto& row : rows) titles.push_back(row.title);
    return titles;
}

} // namespace

TEST_CASE("VS-14 by artist/album sorts by artist, then album, then track number") {
    AppState state;
    state.Tracks = {
        sortTrack(L"b2", L"Beta", L"Two", 1),
        sortTrack(L"a1-2", L"Alpha", L"One", 2),
        sortTrack(L"b1", L"Beta", L"One", 7),
        sortTrack(L"a1-1", L"Alpha", L"One", 1),
        sortTrack(L"a2", L"Alpha", L"Two", 1),
    };
    auto rows = view::LibraryRows(state, view::LibrarySort::ArtistAlbum, true);
    REQUIRE(titlesOf(rows) ==
            std::vector<std::wstring>{L"a1-1", L"a1-2", L"a2", L"b1", L"b2"});
}

TEST_CASE("VS-15 by letter sorts by title ascending") {
    AppState state;
    state.Tracks = {sortTrack(L"Gamma", L"Z", std::nullopt, std::nullopt),
                    sortTrack(L"Alpha", L"Y", std::nullopt, std::nullopt),
                    sortTrack(L"Beta", L"X", std::nullopt, std::nullopt)};
    auto rows = view::LibraryRows(state, view::LibrarySort::Alphabetical, true);
    REQUIRE(titlesOf(rows) == std::vector<std::wstring>{L"Alpha", L"Beta", L"Gamma"});
}

TEST_CASE("VS-16 tracks without an artist are kept, together and in library order") {
    AppState state;
    state.Tracks = {
        sortTrack(L"known", L"Alpha", L"One", 1),
        sortTrack(L"loose-2", std::nullopt, std::nullopt, std::nullopt),
        sortTrack(L"loose-1", std::nullopt, std::nullopt, std::nullopt),
    };
    auto rows = view::LibraryRows(state, view::LibrarySort::ArtistAlbum, true);
    // No artist sorts as an empty name: first, and ties keep library order.
    REQUIRE(titlesOf(rows) ==
            std::vector<std::wstring>{L"loose-2", L"loose-1", L"known"});
}

TEST_CASE("VS-17 sorting leaves the state's track order untouched") {
    AppState state;
    state.Tracks = {sortTrack(L"Gamma", L"Z", std::nullopt, std::nullopt),
                    sortTrack(L"Alpha", L"Y", std::nullopt, std::nullopt)};
    view::LibraryRows(state, view::LibrarySort::Alphabetical, true);
    view::LibraryRows(state, view::LibrarySort::ArtistAlbum, true);
    REQUIRE(state.Tracks[0].title == L"Gamma");
    REQUIRE(state.Tracks[1].title == L"Alpha");
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

// ─── VS-11/12/13 播放条音量与播放图标（#335）────────────────────────

TEST_CASE("VS-11 the volume slider shows the current volume") {
    AppState state;
    state.Volume = 0.35;
    REQUIRE(view::PlayerBarState(state).volumePercent == Catch::Approx(35.0));
    state.Volume = 1.0;
    REQUIRE(view::PlayerBarState(state).volumePercent == Catch::Approx(100.0));
}

TEST_CASE("VS-12 the play button shows pause while playing and play otherwise") {
    AppState state;
    state.IsPlaying = true;
    REQUIRE(view::PlayerBarState(state).playIcon == view::Icon::Pause);
    state.IsPlaying = false;
    REQUIRE(view::PlayerBarState(state).playIcon == view::Icon::Play);
}

TEST_CASE("VS-13 buffering counts as playing for the play button") {
    AppState state;
    state.IsPlaying = false;
    state.IsBuffering = true;
    REQUIRE(view::PlayerBarState(state).playIcon == view::Icon::Pause);
}

// ─── VS-18/19/20 曲目行时长文案（#337，原 WB-01）────────────────────

namespace {

std::wstring durationTextFor(double seconds) {
    AppState state;
    auto track = makeLocalTrack(L"C:\\m\\a.mp3", L"a");
    track.duration = seconds;
    state.Tracks = {track};
    return view::LibraryRows(state, view::LibrarySort::Alphabetical, true).at(0).durationText;
}

} // namespace

TEST_CASE("VS-18 a row's duration renders as m:ss with the seconds zero-padded") {
    REQUIRE(durationTextFor(65.0) == L"1:05");
    REQUIRE(durationTextFor(5.0) == L"0:05");
    REQUIRE(durationTextFor(125.9) == L"2:05");
}

TEST_CASE("VS-19 a zero duration renders 0:00") {
    REQUIRE(durationTextFor(0.0) == L"0:00");
}

TEST_CASE("VS-20 over an hour the minutes keep counting (macOS parity)") {
    // Track.swift durationFormatted is "%d:%02d" of whole minutes too.
    REQUIRE(durationTextFor(3725.0) == L"62:05");
    REQUIRE(durationTextFor(36000.0) == L"600:00");
}

// ─── VS-21..25 来源徽标三元素（#338，原 WB-02/03/04/20）──────────────

namespace {

std::wstring hexOf(view::Color c) {
    return std::format(L"#{:02X}{:02X}{:02X}", c.R, c.G, c.B);
}

} // namespace

TEST_CASE("VS-21 the source tag follows the source type in both languages") {
    auto tag = [](const wchar_t* sourceType) {
        return view::SourceBadgeOf(sourceType, true).tag;
    };
    {
        LanguageScope zh(L"zh");
        REQUIRE(tag(L"local") == L"本地");
        REQUIRE(tag(L"youtube") == L"YT");
        REQUIRE(tag(L"bilibili") == L"B站");
        REQUIRE(tag(L"direct_url") == L"链接");
        REQUIRE(tag(L"something_else") == L"");
    }
    {
        LanguageScope en(L"en");
        REQUIRE(tag(L"local") == L"Local");
        REQUIRE(tag(L"youtube") == L"YT");
        REQUIRE(tag(L"bilibili") == L"Bili");
        REQUIRE(tag(L"direct_url") == L"Link");
        REQUIRE(tag(L"something_else") == L"");
    }
}

TEST_CASE("VS-22 the badge foreground follows source type and theme (#121)") {
    struct Expect {
        const wchar_t* type;
        const wchar_t* dark;
        const wchar_t* light;
    };
    // palette.json sources
    const Expect expects[] = {
        {L"local", L"#8ABCD0", L"#3A7A8C"},
        {L"youtube", L"#D49573", L"#8B4A28"},
        {L"bilibili", L"#C88DA8", L"#8C4D68"},
        {L"direct_url", L"#8CB89A", L"#4C785A"},
    };
    for (const auto& e : expects) {
        auto dark = view::SourceBadgeOf(e.type, true).foreground;
        auto light = view::SourceBadgeOf(e.type, false).foreground;
        REQUIRE(hexOf(dark) == e.dark);
        REQUIRE(hexOf(light) == e.light);
        REQUIRE(dark.A == 0xFF);
        REQUIRE(light.A == 0xFF);
    }
}

TEST_CASE("VS-23 an unknown source falls back to the text colour, never system grey (F4)") {
    auto dark = view::SourceBadgeOf(L"nope", true).foreground;
    auto light = view::SourceBadgeOf(L"nope", false).foreground;
    REQUIRE(hexOf(dark) == L"#ABC8D4");
    REQUIRE(hexOf(light) == L"#0D464D");
    REQUIRE(hexOf(dark) != L"#808080");
    REQUIRE(hexOf(light) != L"#808080");
}

TEST_CASE("VS-24 the capsule is the foreground at the declared opacity") {
    // palette.json sourceBadge.backgroundOpacity 0.15 -> alpha 38, the macOS
    // `.background(color.opacity(0.15))` treatment -- unknown sources included.
    for (const wchar_t* type : {L"local", L"youtube", L"bilibili", L"direct_url", L"nope"}) {
        for (bool isDark : {true, false}) {
            auto badge = view::SourceBadgeOf(type, isDark);
            REQUIRE(badge.background.A == 38);
            REQUIRE(hexOf(badge.background) == hexOf(badge.foreground));
        }
    }
}

TEST_CASE("VS-25 library rows carry their track's badge in the given theme") {
    LanguageScope zh(L"zh");
    AppState state;
    state.Tracks = {makeLocalTrack(L"C:\\m\\a.mp3", L"a")};
    for (bool isDark : {true, false}) {
        auto row = view::LibraryRows(state, view::LibrarySort::Alphabetical, isDark).at(0);
        auto expected = view::SourceBadgeOf(L"local", isDark);
        REQUIRE(row.badge.tag == L"本地");
        REQUIRE(hexOf(row.badge.foreground) == hexOf(expected.foreground));
        REQUIRE(row.badge.background.A == expected.background.A);
    }
}

// ─── VS-26/27/28 列表行：艺人与歌单详情（#339）──────────────────────

TEST_CASE("VS-26 a row carries its track's artist, empty when it has none") {
    AppState state;
    auto withArtist = makeLocalTrack(L"C:\\m\\a.mp3", L"a");
    withArtist.artist = L"Artist A";
    state.Tracks = {withArtist, makeLocalTrack(L"C:\\m\\b.mp3", L"b")};
    auto rows = view::LibraryRows(state, view::LibrarySort::Alphabetical, true);
    REQUIRE(rows.at(0).artist == L"Artist A");
    REQUIRE(rows.at(1).artist.empty());
}

TEST_CASE("VS-27 playlist rows are the same rows, in playlist order") {
    LanguageScope zh(L"zh");
    AppState state;
    auto late = makeLocalTrack(L"C:\\m\\z.mp3", L"Zulu");
    late.duration = 65.0;
    Playlist other;
    other.id = 1;
    other.tracks = {makeLocalTrack(L"C:\\m\\o.mp3", L"Other")};
    Playlist mine;
    mine.id = 2;
    mine.tracks = {late, makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha")};
    state.Playlists = {other, mine};

    auto rows = view::PlaylistRows(state, 2, false);

    REQUIRE(titlesOf(rows) == std::vector<std::wstring>{L"Zulu", L"Alpha"});
    REQUIRE(rows.at(0).durationText == L"1:05");
    REQUIRE(rows.at(0).badge.tag == L"本地");
    REQUIRE(hexOf(rows.at(0).badge.foreground) ==
            hexOf(view::SourceBadgeOf(L"local", false).foreground));
    REQUIRE(rows.at(0).track.title == L"Zulu");
}

TEST_CASE("VS-28 an unknown playlist renders no rows") {
    AppState state;
    Playlist mine;
    mine.id = 2;
    mine.tracks = {makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha")};
    state.Playlists = {mine};
    REQUIRE(view::PlaylistRows(state, 99, true).empty());
}

// ─── VS-29 托盘菜单文案（#340）──────────────────────────────────────

TEST_CASE("VS-29 the tray menu carries its three labels in the current language") {
    AppState state;
    {
        LanguageScope zh(L"zh");
        auto menu = view::TrayMenuState(state);
        REQUIRE(menu.playPause == L"播放 / 暂停");
        REQUIRE(menu.showWindow == L"显示主窗口");
        REQUIRE(menu.quit == L"退出 Rhythm");
    }
    {
        LanguageScope en(L"en");
        auto menu = view::TrayMenuState(state);
        REQUIRE(menu.playPause == L"Play / Pause");
        REQUIRE(menu.showWindow == L"Show Window");
        REQUIRE(menu.quit == L"Quit Rhythm");
    }
}
