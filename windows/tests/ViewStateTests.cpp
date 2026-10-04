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
    // #503: no artist groups under the unknown-artist label and sorts by that
    // name, as on macOS (it used to sort as an empty name, first); ties keep
    // library order.
    REQUIRE(titlesOf(rows) ==
            std::vector<std::wstring>{L"known", L"loose-2", L"loose-1"});
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

// ─── VS-43/44 可拖动进度条（#498）──────────────────────────────────

TEST_CASE("VS-43 the progress bar is seekable only with a current track of known duration") {
    AppState state;
    state.Duration = 120.0;
    REQUIRE_FALSE(view::PlayerBarState(state).seekable);  // no current track

    Track track;
    track.title = L"Alpha";
    state.CurrentTrack = track;
    REQUIRE(view::PlayerBarState(state).seekable);

    state.Duration = 0.0;  // unknown duration: nothing to drag over
    REQUIRE_FALSE(view::PlayerBarState(state).seekable);
}

TEST_CASE("VS-44 a progress bar value maps to a position in the track") {
    AppState state;
    state.Duration = 2175.0;
    REQUIRE(view::SeekPosition(state, 50.0) == 1087.5);
    REQUIRE(view::SeekPosition(state, 0.0) == 0.0);
    REQUIRE(view::SeekPosition(state, 100.0) == 2175.0);
    // Out of range values are clamped, never a position outside the track.
    REQUIRE(view::SeekPosition(state, 140.0) == 2175.0);
    REQUIRE(view::SeekPosition(state, -10.0) == 0.0);
    // An unknown duration has no positions to map to.
    state.Duration = 0.0;
    REQUIRE(view::SeekPosition(state, 50.0) == 0.0);
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

// ─── VS-56 播放条是否显示艺人行（#512）──────────────────────────────

TEST_CASE("VS-56 the player bar shows the artist line only when there is an artist") {
    AppState state;
    REQUIRE_FALSE(view::PlayerBarState(state).showsArtist);

    auto track = makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha");
    state.CurrentTrack = track;
    REQUIRE_FALSE(view::PlayerBarState(state).showsArtist);

    track.artist = L"";
    state.CurrentTrack = track;
    REQUIRE_FALSE(view::PlayerBarState(state).showsArtist);

    track.artist = L"Artist A";
    state.CurrentTrack = track;
    REQUIRE(view::PlayerBarState(state).showsArtist);

    state.CurrentTrack.reset();
    REQUIRE_FALSE(view::PlayerBarState(state).showsArtist);
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

// ─── VS-30/31/32 托盘播放暂停可用性（#341）──────────────────────────

// The availability rules live in the core coordinator; SpyApp (TestHelpers.h)
// injects a SpyCoordinator that mirrors its contract.

TEST_CASE("VS-30 an empty library disables play/pause in the tray") {
    SpyApp s;
    REQUIRE_FALSE(view::TrayMenuState(s.state).playPauseEnabled);
}

TEST_CASE("VS-31 a current track enables play/pause in the tray") {
    SpyApp s;
    s.state.PlayTrack(makeLocalTrack(L"C:\\m\\a.mp3", L"a"));
    REQUIRE(s.state.CurrentTrack.has_value());
    REQUIRE(view::TrayMenuState(s.state).playPauseEnabled);
}

TEST_CASE("VS-32 tray availability agrees with the coordinator's query") {
    SpyApp s;
    auto agrees = [&] {
        return view::TrayMenuState(s.state).playPauseEnabled ==
               s.state.Coordinator->CanTogglePlayback();
    };
    REQUIRE(agrees());  // empty
    s.spy->SyncQueue({makeLocalTrack(L"C:\\m\\a.mp3", L"a")});
    REQUIRE(agrees());  // library, nothing current: the coordinator can idle-start
    REQUIRE(view::TrayMenuState(s.state).playPauseEnabled);
    s.spy->SyncQueue({});
    REQUIRE(agrees());
    REQUIRE_FALSE(view::TrayMenuState(s.state).playPauseEnabled);
}

// ─── VS-33/34/35 播放条链接解析状态（#317 收尾）─────────────────────

TEST_CASE("VS-33 no resolution in flight renders an empty URL status") {
    AppState state;
    ResolverStatus downloading;
    downloading.phase = L"downloading";
    downloading.received = 1024 * 1024;
    state.PollResolverStatus = [downloading] { return downloading; };
    REQUIRE(view::PlayerBarState(state).urlStatusText.empty());
}

TEST_CASE("VS-34 a resolution with nothing to report renders the resolving copy") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;
        state.IsResolvingUrl = true;
        for (const wchar_t* phase : {L"idle", L"ready"}) {
            ResolverStatus quiet;
            quiet.phase = phase;
            state.PollResolverStatus = [quiet] { return quiet; };
            REQUIRE(view::PlayerBarState(state).urlStatusText == L10n::Resolving());
        }
    }
}

TEST_CASE("VS-35 a first-use yt-dlp download renders its progress") {
    LanguageScope zh(L"zh");
    AppState state;
    state.IsResolvingUrl = true;
    ResolverStatus downloading;
    downloading.phase = L"downloading";
    downloading.received = 5 * 1024 * 1024;
    downloading.total = 36 * 1024 * 1024;
    // #350: the progress copy comes through the app state, not the view.
    state.PollResolverStatus = [downloading] { return downloading; };
    auto text = view::PlayerBarState(state).urlStatusText;
    REQUIRE(text == Resolver::StatusText(downloading));
    REQUIRE_FALSE(text.empty());
}

// ─── VS-36/37 导入导出反馈弹窗（#352）──────────────────────────────

TEST_CASE("VS-36 a pending export alert renders its own title and message") {
    AppState state;
    state.ShowExportAlert = true;
    state.ExportAlertTitle = L"T";
    state.ExportAlertMessage = L"M";
    auto alert = view::PendingAlert(state);
    REQUIRE(alert.has_value());
    REQUIRE(alert->title == L"T");
    REQUIRE(alert->message == L"M");
}

TEST_CASE("VS-36 a pending import alert renders under the import result title") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;
        state.ShowImportAlert = true;
        state.ImportAlertMessage = L10n::ImportedTracks(2);
        auto alert = view::PendingAlert(state);
        REQUIRE(alert.has_value());
        REQUIRE(alert->title == L10n::ImportResultTitle());
        REQUIRE(alert->message == L10n::ImportedTracks(2));
    }
}

TEST_CASE("VS-37 nothing pending renders no alert") {
    AppState state;
    state.ImportAlertMessage = L"stale";
    state.ExportAlertMessage = L"stale";
    REQUIRE_FALSE(view::PendingAlert(state).has_value());
}


// ─── VS-38 歌单详情按需取值（#358）─────────────────────────────────

TEST_CASE("VS-38 the detail page renders the state's current playlist") {
    LanguageScope zh(L"zh");
    AppState state;
    Playlist other;
    other.id = 1;
    other.name = L"其他";
    other.tracks = {makeLocalTrack(L"C:\\m\\o.mp3", L"Other")};
    auto late = makeLocalTrack(L"C:\\m\\z.mp3", L"Zulu");
    late.duration = 65.0;
    Playlist mine;
    mine.id = 2;
    mine.name = L"晨跑";
    mine.tracks = {late, makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha")};
    state.Playlists = {other, mine};
    state.SelectPlaylist(2);

    auto detail = view::PlaylistDetailOf(state, false);

    REQUIRE(detail.hasPlaylist);
    REQUIRE(detail.title == L"晨跑");
    // The same rows as the library (#339), in playlist order.
    REQUIRE(titlesOf(detail.rows) == std::vector<std::wstring>{L"Zulu", L"Alpha"});
    REQUIRE(detail.rows.at(0).durationText == L"1:05");
    REQUIRE(detail.rows.at(0).badge.tag == L"本地");
    REQUIRE(hexOf(detail.rows.at(0).badge.foreground) ==
            hexOf(view::SourceBadgeOf(L"local", false).foreground));
    REQUIRE(detail.rows.at(0).track.title == L"Zulu");
}

TEST_CASE("VS-38 with nothing selected the detail page has no playlist") {
    AppState state;
    Playlist mine;
    mine.id = 2;
    mine.name = L"晨跑";
    state.Playlists = {mine};

    auto detail = view::PlaylistDetailOf(state, true);

    REQUIRE_FALSE(detail.hasPlaylist);
    REQUIRE(detail.title.empty());
    REQUIRE(detail.rows.empty());
}

TEST_CASE("VS-38 a refresh is visible on the next render") {
    TempDir dir;  // #455: before AppState, so the database closes before cleanup
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto id = state.CreatePlaylist(L"晨跑");
    state.SelectPlaylist(id);
    REQUIRE(view::PlaylistDetailOf(state, true).rows.empty());

    auto saved = state.Library->AddTrack(makeLocalTrack(L"C:\\m\\vs38.mp3", L"VS38"));
    state.Library->AddToPlaylist(id, saved.id);
    state.RefreshLibrary();  // what an M3U8 import triggers

    auto detail = view::PlaylistDetailOf(state, true);
    REQUIRE(detail.hasPlaylist);
    REQUIRE(titlesOf(detail.rows) == std::vector<std::wstring>{L"VS38"});
}

// ─── VS-39 未选中时的空态（#359）───────────────────────────────────

TEST_CASE("VS-39 with nothing selected the detail page shows the empty-state copy") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;
        Playlist mine;
        mine.id = 2;
        mine.name = L"晨跑";
        mine.tracks = {makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha")};
        state.Playlists = {mine};

        auto empty = view::PlaylistDetailOf(state, true);
        REQUIRE_FALSE(empty.hasPlaylist);
        REQUIRE(empty.emptyMessage == L10n::NoPlaylistSelected());  // from the key table
        REQUIRE_FALSE(empty.emptyMessage.empty());
        REQUIRE(empty.title.empty());
        REQUIRE(empty.rows.empty());

        // A selected playlist carries no empty-state copy.
        state.SelectPlaylist(2);
        auto selected = view::PlaylistDetailOf(state, true);
        REQUIRE(selected.hasPlaylist);
        REQUIRE(selected.emptyMessage.empty());
    }
}

TEST_CASE("VS-40 a selection cleared by a refresh falls back to the empty state") {
    TempDir dir;  // #455: before AppState, so the database closes before cleanup
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto id = state.CreatePlaylist(L"晨跑");
    state.SelectPlaylist(id);
    REQUIRE(view::PlaylistDetailOf(state, true).hasPlaylist);

    state.Library->DeletePlaylist(id);
    state.RefreshLibrary();

    auto detail = view::PlaylistDetailOf(state, true);
    REQUIRE_FALSE(detail.hasPlaylist);
    REQUIRE(detail.emptyMessage == L10n::NoPlaylistSelected());
    REQUIRE(detail.rows.empty());
}

// ─── VS-41 歌单列表行（#320）───────────────────────────────────────

TEST_CASE("VS-41 every loaded playlist renders as one row carrying its id") {
    AppState state;
    Playlist first;
    first.id = 7;
    first.name = L"晨跑";
    first.tracks = {makeLocalTrack(L"C:\\m\\a.mp3", L"Alpha"),
                    makeLocalTrack(L"C:\\m\\b.mp3", L"Beta")};
    Playlist second;
    second.id = 9;
    second.name = L"夜归";
    state.Playlists = {first, second};

    auto page = view::PlaylistListState(state);

    REQUIRE(page.rows.size() == 2);
    REQUIRE(page.rows.at(0).id == 7);
    REQUIRE(page.rows.at(0).name == L"晨跑");
    REQUIRE(page.rows.at(0).trackCountText == L"2");
    REQUIRE(page.rows.at(1).id == 9);
    REQUIRE(page.rows.at(1).trackCountText == L"0");
    REQUIRE(page.emptyMessage.empty());
}

TEST_CASE("VS-41 the row's id is what selects that playlist") {
    AppState state;
    Playlist first;
    first.id = 7;
    first.name = L"晨跑";
    Playlist second;
    second.id = 9;
    second.name = L"夜归";
    state.Playlists = {first, second};

    state.SelectPlaylist(view::PlaylistListState(state).rows.at(1).id);

    REQUIRE(state.CurrentPlaylist.has_value());
    REQUIRE(state.CurrentPlaylist->name == L"夜归");
}

TEST_CASE("VS-42 with no playlists the list renders the empty-state copy") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;

        auto page = view::PlaylistListState(state);

        REQUIRE(page.rows.empty());
        REQUIRE(page.emptyMessage == L10n::PlaylistEmpty());
    }
}

TEST_CASE("VS-42 a playlist with no id renders no row") {
    AppState state;
    Playlist unsaved;  // never stored: nothing a click could select it by
    unsaved.name = L"未入库";
    state.Playlists = {unsaved};

    REQUIRE(view::PlaylistListState(state).rows.empty());
}

// ─── VS-45/46 常驻侧栏（#501）──────────────────────────────────────

TEST_CASE("VS-45 the sidebar renders library then playlists, each with icon and label") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;

        auto entries = view::SidebarState(state);

        REQUIRE(entries.size() == 2);
        REQUIRE(entries.at(0).item == SidebarItem::Library);
        REQUIRE(entries.at(0).icon == view::Icon::Library);
        REQUIRE(entries.at(0).label == L10n::LibraryTab());
        REQUIRE(entries.at(1).item == SidebarItem::Playlists);
        REQUIRE(entries.at(1).icon == view::Icon::Playlists);
        REQUIRE(entries.at(1).label == L10n::PlaylistsTab());
    }
}

TEST_CASE("VS-46 exactly the selected view's entry is highlighted") {
    AppState state;

    auto atStart = view::SidebarState(state);
    REQUIRE(atStart.at(0).selected);
    REQUIRE_FALSE(atStart.at(1).selected);

    state.SelectedView = SidebarItem::Playlists;
    auto onPlaylists = view::SidebarState(state);
    REQUIRE_FALSE(onPlaylists.at(0).selected);
    REQUIRE(onPlaylists.at(1).selected);
}

// ─── VS-47/48 资料库唯一的视图切换（#502）────────────────────────────

TEST_CASE("VS-47 the view switch has the two macOS segments, artist/album first") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;

        auto segments = view::LibraryViewSwitch(state);

        REQUIRE(segments.size() == 2);
        REQUIRE(segments.at(0).sort == view::LibrarySort::ArtistAlbum);
        REQUIRE(segments.at(0).label == L10n::ByArtistAlbum());
        REQUIRE(segments.at(1).sort == view::LibrarySort::Alphabetical);
        REQUIRE(segments.at(1).label == L10n::ByLetter());
    }
}

TEST_CASE("VS-48 switching the view selects its segment and reorders the list") {
    AppState state;
    state.Tracks = {sortTrack(L"Beta", L"Alpha", L"One", 1),
                    sortTrack(L"Alpha", L"Zeta", L"One", 1)};

    // At start: artist/album, the macOS default.
    REQUIRE(state.LibraryOrder == view::LibrarySort::ArtistAlbum);
    REQUIRE(view::LibraryViewSwitch(state).at(0).selected);
    REQUIRE_FALSE(view::LibraryViewSwitch(state).at(1).selected);
    REQUIRE(titlesOf(view::LibraryRows(state, true)) ==
            std::vector<std::wstring>{L"Beta", L"Alpha"});

    state.LibraryOrder = view::LibraryViewSwitch(state).at(1).sort;

    REQUIRE_FALSE(view::LibraryViewSwitch(state).at(0).selected);
    REQUIRE(view::LibraryViewSwitch(state).at(1).selected);
    REQUIRE(titlesOf(view::LibraryRows(state, true)) ==
            std::vector<std::wstring>{L"Alpha", L"Beta"});
}

// ─── VS-57 工具栏导入控件（#516）────────────────────────────────────

TEST_CASE("VS-57 the import controls show on the library page only, a progress indicator while importing") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;

        auto controls = view::ImportControlsState(state);
        REQUIRE(controls.showsButtons);
        REQUIRE_FALSE(controls.showsProgress);

        state.IsImporting = true;
        controls = view::ImportControlsState(state);
        REQUIRE_FALSE(controls.showsButtons);
        REQUIRE(controls.showsProgress);
        REQUIRE(controls.progressLabel == L10n::Importing());

        state.SelectedView = SidebarItem::Playlists;
        controls = view::ImportControlsState(state);
        REQUIRE_FALSE(controls.showsButtons);
        REQUIRE_FALSE(controls.showsProgress);

        state.IsImporting = false;
        controls = view::ImportControlsState(state);
        REQUIRE_FALSE(controls.showsButtons);
        REQUIRE_FALSE(controls.showsProgress);
    }
}

// ─── VS-49/50/51 按艺人/专辑分组（#503）──────────────────────────────

namespace {

std::vector<std::wstring> sectionTitles(const std::vector<view::ArtistSection>& sections) {
    std::vector<std::wstring> titles;
    for (const auto& s : sections) titles.push_back(s.title);
    return titles;
}

std::vector<std::wstring> albumTitles(const view::ArtistSection& section) {
    std::vector<std::wstring> titles;
    for (const auto& a : section.albums) titles.push_back(a.title);
    return titles;
}

} // namespace

TEST_CASE("VS-49 artists form sections by name, albums group within them by name") {
    AppState state;
    state.Tracks = {
        sortTrack(L"b-two", L"Beta", L"Two", 1),
        sortTrack(L"a-one", L"Alpha", L"One", 1),
        sortTrack(L"b-one", L"Beta", L"One", 1),
        sortTrack(L"a-two", L"Alpha", L"Two", 1),
    };

    auto sections = view::ArtistAlbumSections(state, true);

    REQUIRE(sectionTitles(sections) == std::vector<std::wstring>{L"Alpha", L"Beta"});
    REQUIRE(albumTitles(sections.at(0)) == std::vector<std::wstring>{L"One", L"Two"});
    REQUIRE(albumTitles(sections.at(1)) == std::vector<std::wstring>{L"One", L"Two"});
    REQUIRE(titlesOf(sections.at(1).albums.at(0).rows) == std::vector<std::wstring>{L"b-one"});
}

TEST_CASE("VS-49 tracks in an album follow disc then track number, ties in library order") {
    AppState state;
    auto disc = [](Track t, int32_t number) {
        t.discNumber = number;
        return t;
    };
    state.Tracks = {
        disc(sortTrack(L"d2-t1", L"A", L"X", 1), 2),
        disc(sortTrack(L"d1-t2", L"A", L"X", 2), 1),
        sortTrack(L"tie-first", L"A", L"X", std::nullopt),
        disc(sortTrack(L"d1-t1", L"A", L"X", 1), 1),
        sortTrack(L"tie-second", L"A", L"X", std::nullopt),
    };

    auto rows = view::ArtistAlbumSections(state, true).at(0).albums.at(0).rows;

    // A missing disc or track number counts as 0, as on macOS.
    REQUIRE(titlesOf(rows) == std::vector<std::wstring>{
                                  L"tie-first", L"tie-second", L"d1-t1", L"d1-t2", L"d2-t1"});
}

TEST_CASE("VS-50 a missing artist or album groups under the unknown label") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;
        state.Tracks = {
            sortTrack(L"no-album", L"Alpha", std::nullopt, 1),
            sortTrack(L"nothing", std::nullopt, std::nullopt, std::nullopt),
            sortTrack(L"no-artist", std::nullopt, L"Live", std::nullopt),
            sortTrack(L"known", L"Alpha", L"One", 1),
        };

        auto sections = view::ArtistAlbumSections(state, true);

        // The unknown groups are ordinary groups named by the label, sorted
        // by it like any other name (macOS groupByArtistAlbum).
        REQUIRE(sections.size() == 2);
        REQUIRE(sections.at(0).title == L"Alpha");
        REQUIRE(albumTitles(sections.at(0)) ==
                std::vector<std::wstring>{L"One", L10n::UnknownAlbum()});
        REQUIRE(sections.at(1).title == L10n::UnknownArtist());
        auto expected = std::vector<std::wstring>{L"Live", L10n::UnknownAlbum()};
        std::sort(expected.begin(), expected.end());
        REQUIRE(albumTitles(sections.at(1)) == expected);
    }
}

TEST_CASE("VS-50 the same album name under two artists stays two groups") {
    AppState state;
    state.Tracks = {sortTrack(L"a", L"Alpha", L"Greatest Hits", 1),
                    sortTrack(L"b", L"Beta", L"Greatest Hits", 1)};

    auto sections = view::ArtistAlbumSections(state, true);

    REQUIRE(sections.size() == 2);
    REQUIRE(titlesOf(sections.at(0).albums.at(0).rows) == std::vector<std::wstring>{L"a"});
    REQUIRE(titlesOf(sections.at(1).albums.at(0).rows) == std::vector<std::wstring>{L"b"});
}

TEST_CASE("VS-51 the flat artist/album order is the grouping read top to bottom") {
    AppState state;
    state.Tracks = {
        sortTrack(L"loose", std::nullopt, std::nullopt, std::nullopt),
        sortTrack(L"b1", L"Beta", L"One", 1),
        sortTrack(L"a1", L"Alpha", L"One", 1),
    };

    std::vector<std::wstring> grouped;
    for (const auto& section : view::ArtistAlbumSections(state, true)) {
        for (const auto& album : section.albums) {
            for (const auto& row : album.rows) grouped.push_back(row.title);
        }
    }

    REQUIRE(titlesOf(view::LibraryRows(state, view::LibrarySort::ArtistAlbum, true)) == grouped);
}

TEST_CASE("VS-50 the unknown-artist section sorts by its label in each language") {
    for (const wchar_t* language : {L"zh", L"en"}) {
        LanguageScope scope(language);
        AppState state;
        state.Tracks = {sortTrack(L"z", L"Zeta", L"One", 1),
                        sortTrack(L"loose", std::nullopt, std::nullopt, std::nullopt)};

        auto expected = std::vector<std::wstring>{L"Zeta", L10n::UnknownArtist()};
        std::sort(expected.begin(), expected.end());
        REQUIRE(sectionTitles(view::ArtistAlbumSections(state, true)) == expected);
    }
}

// ─── VS-52 资料库列表逐行（#503）─────────────────────────────────────

namespace {

std::vector<std::pair<view::LibraryLineKind, std::wstring>> lineShape(
    const std::vector<view::LibraryLine>& lines) {
    std::vector<std::pair<view::LibraryLineKind, std::wstring>> shape;
    for (const auto& line : lines) {
        shape.emplace_back(line.kind, line.kind == view::LibraryLineKind::Track
                                          ? line.row.title : line.title);
    }
    return shape;
}

} // namespace

TEST_CASE("VS-52 by artist/album the list reads section, album, then its tracks") {
    using K = view::LibraryLineKind;
    AppState state;
    state.Tracks = {sortTrack(L"b1", L"Beta", L"One", 1),
                    sortTrack(L"a2", L"Alpha", L"One", 2),
                    sortTrack(L"a1", L"Alpha", L"One", 1)};

    auto lines = view::LibraryLines(state, true);

    REQUIRE(lineShape(lines) == std::vector<std::pair<K, std::wstring>>{
                                    {K::Section, L"Alpha"}, {K::Album, L"One"},
                                    {K::Track, L"a1"}, {K::Track, L"a2"},
                                    {K::Section, L"Beta"}, {K::Album, L"One"},
                                    {K::Track, L"b1"}});
    for (const auto& line : lines) {
        if (line.kind == K::Track) REQUIRE(line.inAlbum);
    }
}

TEST_CASE("VS-52 the list follows the state's order and is empty for an empty library") {
    AppState state;
    REQUIRE(view::LibraryLines(state, true).empty());

    state.Tracks = {sortTrack(L"b1", L"Beta", L"One", 1)};
    state.LibraryOrder = view::LibrarySort::Alphabetical;
    auto lines = view::LibraryLines(state, true);
    REQUIRE_FALSE(lines.empty());
    REQUIRE(lines.back().kind == view::LibraryLineKind::Track);
    REQUIRE(lines.back().row.title == L"b1");
    REQUIRE_FALSE(lines.back().inAlbum);
}

// ─── VS-53/54/55 按首字母分节（#504）─────────────────────────────────

namespace {

std::vector<std::wstring> letterTitles(const std::vector<view::LetterSection>& sections) {
    std::vector<std::wstring> titles;
    for (const auto& s : sections) titles.push_back(s.title);
    return titles;
}

} // namespace

TEST_CASE("VS-53 tracks section by their title's first letter, upper-cased, in order") {
    AppState state;
    state.Tracks = {sortTrack(L"beta", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Alpha", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"apple", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Bravo", L"X", std::nullopt, std::nullopt)};

    auto sections = view::LetterSections(state, true);

    REQUIRE(letterTitles(sections) == std::vector<std::wstring>{L"A", L"B"});
    REQUIRE(sections.at(0).rows.size() == 2);
    REQUIRE(sections.at(1).rows.size() == 2);
}

TEST_CASE("VS-53 a title not starting with a letter goes under # ahead of the letters") {
    AppState state;
    state.Tracks = {sortTrack(L"Zulu", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"1999", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"(intro)", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"", L"X", std::nullopt, std::nullopt)};

    auto sections = view::LetterSections(state, true);

    REQUIRE(letterTitles(sections) == std::vector<std::wstring>{L"#", L"Z"});
    REQUIRE(sections.at(0).rows.size() == 3);
}

TEST_CASE("VS-53 a title starting with a CJK character is its own section, after Latin") {
    AppState state;
    state.Tracks = {sortTrack(L"明天见", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Alpha", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"明月", L"X", std::nullopt, std::nullopt)};

    auto sections = view::LetterSections(state, true);

    // macOS counts any Unicode letter (Character.isLetter), CJK included.
    REQUIRE(letterTitles(sections) == std::vector<std::wstring>{L"A", L"明"});
    REQUIRE(sections.at(1).rows.size() == 2);
}

TEST_CASE("VS-54 within a section titles sort ignoring case, ties in library order") {
    AppState state;
    state.Tracks = {sortTrack(L"banana", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Bee", L"first", std::nullopt, std::nullopt),
                    sortTrack(L"Apple", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"bee", L"second", std::nullopt, std::nullopt),
                    sortTrack(L"Blue", L"X", std::nullopt, std::nullopt)};

    auto sections = view::LetterSections(state, true);

    REQUIRE(titlesOf(sections.at(1).rows) ==
            std::vector<std::wstring>{L"banana", L"Bee", L"bee", L"Blue"});
    REQUIRE(sections.at(1).rows.at(1).artist == L"first");
}

TEST_CASE("VS-55 the flat letter order is the sections read top to bottom") {
    AppState state;
    state.Tracks = {sortTrack(L"beta", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"9 Lives", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Alpha", L"X", std::nullopt, std::nullopt)};

    std::vector<std::wstring> sectioned;
    for (const auto& section : view::LetterSections(state, true)) {
        for (const auto& row : section.rows) sectioned.push_back(row.title);
    }

    REQUIRE(titlesOf(view::LibraryRows(state, view::LibrarySort::Alphabetical, true)) == sectioned);
}

TEST_CASE("VS-55 by letter the list reads each letter heading, then its tracks") {
    using K = view::LibraryLineKind;
    AppState state;
    state.LibraryOrder = view::LibrarySort::Alphabetical;
    state.Tracks = {sortTrack(L"beta", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Alpha", L"X", std::nullopt, std::nullopt)};

    auto lines = view::LibraryLines(state, true);

    REQUIRE(lineShape(lines) == std::vector<std::pair<K, std::wstring>>{
                                    {K::Section, L"A"}, {K::Track, L"Alpha"},
                                    {K::Section, L"B"}, {K::Track, L"beta"}});
    for (const auto& line : lines) REQUIRE_FALSE(line.inAlbum);
}

// ─── VS-58 按首字母的字母索引条（#520）──────────────────────────────

TEST_CASE("VS-58 by letter the index lists the section headings, each pointing at its heading line") {
    using K = view::LibraryLineKind;
    AppState state;
    state.LibraryOrder = view::LibrarySort::Alphabetical;
    state.Tracks = {sortTrack(L"beta", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"9 Lives", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Alpha", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"bravo", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"晴天", L"X", std::nullopt, std::nullopt)};

    auto index = view::LetterIndex(state);
    auto sections = view::LetterSections(state, true);
    auto lines = view::LibraryLines(state, true);

    REQUIRE(index.size() == sections.size());
    for (size_t i = 0; i < index.size(); ++i) {
        REQUIRE(index[i].title == sections[i].title);
        REQUIRE(index[i].line < lines.size());
        REQUIRE(lines[index[i].line].kind == K::Section);
        REQUIRE(lines[index[i].line].title == index[i].title);
    }
    REQUIRE(index.front().title == L"#");
}

TEST_CASE("VS-58 by artist/album, or with no tracks, there is no index") {
    AppState state;
    state.Tracks = {sortTrack(L"Alpha", L"X", std::nullopt, std::nullopt)};
    REQUIRE(state.LibraryOrder == view::LibrarySort::ArtistAlbum);
    REQUIRE(view::LetterIndex(state).empty());

    state.LibraryOrder = view::LibrarySort::Alphabetical;
    REQUIRE(view::LetterIndex(state).size() == 1);

    state.Tracks.clear();
    REQUIRE(view::LetterIndex(state).empty());
}

TEST_CASE("VS-53 a letter outside the BMP is a section, an emoji goes under #") {
    AppState state;
    // U+1D477 MATHEMATICAL BOLD ITALIC CAPITAL P is Alphabetic, as Swift's
    // Character.isLetter counts it; an emoji is not.
    state.Tracks = {sortTrack(L"𝑷laylist", L"X", std::nullopt, std::nullopt),
                    // U+1F3B5, escaped: the repository carries no emoji.
                    sortTrack(L"\U0001F3B5 Song", L"X", std::nullopt, std::nullopt)};

    auto sections = view::LetterSections(state, true);

    REQUIRE(letterTitles(sections) == std::vector<std::wstring>{L"#", L"𝑷"});
}

TEST_CASE("VS-53 a decomposed accent heads the same section as a precomposed one") {
    AppState state;
    // E + U+0301 (as tags written on macOS often are) and U+00C9: one
    // character to Swift, so one section headed by the accented letter.
    state.Tracks = {sortTrack(L"Éclair", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Été", L"X", std::nullopt, std::nullopt)};

    auto sections = view::LetterSections(state, true);

    REQUIRE(letterTitles(sections) == std::vector<std::wstring>{L"É"});
    REQUIRE(sections.at(0).rows.size() == 2);
}

TEST_CASE("VS-53 sections order by code point, as Swift compares strings") {
    AppState state;
    // U+FF21 FULLWIDTH A comes before U+1D477 by code point, though its
    // UTF-16 unit is larger than the surrogate that starts U+1D477.
    state.Tracks = {sortTrack(L"𝑷laylist", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"Ａlpha", L"X", std::nullopt, std::nullopt)};

    REQUIRE(letterTitles(view::LetterSections(state, true)) ==
            std::vector<std::wstring>{L"Ａ", L"𝑷"});
}

TEST_CASE("VS-54 punctuation inside a title counts when ordering a section") {
    AppState state;
    state.Tracks = {sortTrack(L"ab", L"X", std::nullopt, std::nullopt),
                    sortTrack(L"a-z", L"X", std::nullopt, std::nullopt)};

    // A hyphen sorts before letters (string sort), not ignored (word sort).
    REQUIRE(titlesOf(view::LetterSections(state, true).at(0).rows) ==
            std::vector<std::wstring>{L"a-z", L"ab"});
}
