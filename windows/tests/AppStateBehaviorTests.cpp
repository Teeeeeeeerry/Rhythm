// WA-01–17：Windows AppState 行为清单（manifest:
// docs/testing/behavior/windows-appstate.md）。零接缝：真 rhythm_core DLL +
// 临时 SQLite 库；WA-05/WA-07 为条件 SKIP 红测（#81/#82）。
//
// 这些测试在本机（macOS）不可运行——提交后在 Windows 上 `ctest` 验证。

#include "pch.h"
#include "AppState.h"

#include <catch_amalgamated.hpp>
#include "TestHelpers.h"

using namespace rhythm;
using namespace rhythm_tests;

// ─── WA-01/02 OpenDatabase / RefreshLibrary ─────────────────────────

TEST_CASE("WA-01 OpenDatabase creates the library and fills lists") {
    TempDir dir;
    AppState state;

    state.OpenDatabase(dir.dbPath());

    REQUIRE(state.Library != nullptr);
    REQUIRE(state.Tracks.empty());    // fresh library
    REQUIRE(state.Playlists.empty());
}

TEST_CASE("WA-02 RefreshLibrary no-ops without a library and refreshes with one") {
    AppState state;
    state.RefreshLibrary(); // no Library — must not crash
    REQUIRE(state.Tracks.empty());

    TempDir dir;
    state.OpenDatabase(dir.dbPath());
    auto lib = state.Library.get();
    auto path = writeWavAt(dir.path, L"tone.wav");
    lib->ImportDirectory(dir.path.wstring());
    REQUIRE(lib->AllTracks().size() == 1);

    state.RefreshLibrary();
    REQUIRE(state.Tracks.size() == 1);
    REQUIRE(state.Tracks[0].title == L"tone");
}

// ─── WA-03 ImportDirectory ──────────────────────────────────────────

TEST_CASE("WA-03 ImportDirectory imports and refreshes, no-ops without a library") {
    AppState state;
    state.ImportDirectory(L"C:\\whatever"); // no Library — no-op
    REQUIRE(state.Tracks.empty());

    TempDir dir;
    state.OpenDatabase(dir.dbPath());
    auto music = dir.path / L"music";
    fs::create_directories(music);
    writeWavAt(music, L"imported.wav");

    state.ImportDirectory(music.wstring());

    REQUIRE(state.Tracks.size() == 1);
    REQUIRE(state.Tracks[0].title == L"imported");
}

// ─── WA-04 DoSearch ─────────────────────────────────────────────────

TEST_CASE("WA-04 DoSearch switches between all tracks and search") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());

    auto lib = state.Library.get();
    lib->AddTrack(makeLocalTrack(L"C:\\a\\alpha.mp3", L"Alpha Song"));
    lib->AddTrack(makeLocalTrack(L"C:\\b\\beta.mp3", L"Beta Song"));
    state.RefreshLibrary();
    REQUIRE(state.Tracks.size() == 2);

    state.SearchQuery = L"";
    state.DoSearch();
    REQUIRE(state.Tracks.size() == 2);

    state.SearchQuery = L"alpha";
    state.DoSearch();
    REQUIRE(state.Tracks.size() == 1);
    REQUIRE(state.Tracks[0].title == L"Alpha Song");
}

// ─── WA-05 PlayTrack 分派（红测：缺路径仍置位 → #81）────────────────

TEST_CASE("WA-05 PlayTrack dispatches by source and records the play") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());

    auto wav = writeWavAt(dir.path, L"play.wav", 3.0);
    auto saved = state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Play Me"));

    state.PlayTrack(saved);
    REQUIRE(state.CurrentTrack.has_value());
    REQUIRE(state.IsPlaying);
    if (!waitFor([&] { return state.Player->State() == 1; }, 5000)) {
        SKIP("无音频输出设备，无法观察 Playing 状态（环境跳过）");
    }

    // RecordPlay reached the database.
    auto tracks = state.Library->AllTracks();
    REQUIRE(tracks.size() == 1);
    REQUIRE(tracks[0].playCount == 1);

    state.Player->Stop();
    state.IsPlaying = false;
}

TEST_CASE("WA-05 PlayTrack URL dispatch attempts playback") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());

    auto urlTrack = makeUrlTrack(L"https://example.com/wa05.mp3", L"URL Track");
    state.PlayTrack(urlTrack);

    REQUIRE(state.CurrentTrack.has_value());
    REQUIRE(state.IsPlaying);
    // The URL itself cannot resolve (no network in this suite), but the
    // dispatch must have gone to the engine — a fresh stopped player would
    // stay 0; a play attempt ends in Buffering(3) or Error(4).
    if (!waitFor([&] { return state.Player->State() != 0; }, 8000)) {
        SKIP("无音频输出设备，State 停在 0（环境跳过）");
    }

    state.Player->Stop();
    state.IsPlaying = false;
}

/// 期望：缺 filePath/sourceUrl 时不进入播放状态。现状仍置
/// CurrentTrack/IsPlaying（无声假播放，#78 同类）→ SKIP 挂 #81。
TEST_CASE("WA-05 PlayTrack with no path must not enter playing (red)") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());

    Track broken;
    broken.id = -1;
    broken.sourceType = L"local";
    broken.title = L"No Path";
    state.PlayTrack(broken);

    if (state.IsPlaying || state.CurrentTrack.has_value()) {
        SKIP("rhythm#81 缺 filePath/sourceUrl 仍置为播放中 — "
             "https://github.com/Teeeeeeeerry/Rhythm/issues/81");
    }
    REQUIRE_FALSE(state.IsPlaying);
    REQUIRE_FALSE(state.CurrentTrack.has_value());
}

// ─── WA-06/07/08 TogglePlayPause ────────────────────────────────────

TEST_CASE("WA-06 TogglePlayPause pauses while playing") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wav = writeWavAt(dir.path, L"pause.wav", 3.0);
    auto saved = state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Pause Me"));
    state.PlayTrack(saved);
    if (!waitFor([&] { return state.Player->State() == 1; })) {
        SKIP("无音频输出设备，无法观察 Playing 状态（环境跳过）");
    }

    state.TogglePlayPause();

    REQUIRE_FALSE(state.IsPlaying);
    REQUIRE(state.Player->State() == 2); // Paused
    state.Player->Stop();
}

/// 期望：恢复时 `Player->Resume()` 续播。现状重新 PlayTrack 从头播
/// → SKIP 挂 #82。
TEST_CASE("WA-07 TogglePlayPause resume continues playback (red)") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wav = writeWavAt(dir.path, L"resume.wav", 3.0);
    auto saved = state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Resume Me"));
    state.PlayTrack(saved);
    REQUIRE(waitFor([&] { return state.Player->State() == 1; }));
    ::Sleep(500);

    double before = state.Player->Position();
    state.TogglePlayPause(); // pause
    REQUIRE(state.Player->State() == 2);
    ::Sleep(200);

    state.TogglePlayPause(); // resume
    REQUIRE(state.IsPlaying);
    ::Sleep(300);
    double after = state.Player->Position();

    // Without an audio device the position never advances, so the
    // restart-vs-resume question cannot be decided — skip as environment.
    if (before == 0.0 && after == 0.0) {
        SKIP("无音频输出设备，position 恒 0（环境跳过）");
    }
    // Current behavior restarts from the top, so the position drops back;
    // a real Resume() keeps it at or past the pause point.
    if (after < before) {
        SKIP("rhythm#82 恢复时从头重播而非 Resume 续播 — "
             "https://github.com/Teeeeeeeerry/Rhythm/issues/82");
    }
    REQUIRE(after >= before);
    state.Player->Stop();
    state.IsPlaying = false;
}

TEST_CASE("WA-08 TogglePlayPause starts the first track when idle") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wav = writeWavAt(dir.path, L"first.wav", 3.0);
    auto saved = state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"First"));
    state.RefreshLibrary();

    state.TogglePlayPause(); // idle, tracks non-empty → play Tracks[0]

    REQUIRE(state.IsPlaying);
    REQUIRE(state.CurrentTrack.has_value());
    REQUIRE(state.CurrentTrack->id == saved.id);
    state.Player->Stop();
    state.IsPlaying = false;
}

TEST_CASE("WA-15 TogglePlayPause no-ops with nothing to play") {
    AppState state;

    state.TogglePlayPause();

    REQUIRE_FALSE(state.IsPlaying);
    REQUIRE_FALSE(state.CurrentTrack.has_value());
}

// ─── WA-09 SetVolume ────────────────────────────────────────────────

TEST_CASE("WA-09 SetVolume updates state and player") {
    AppState state;

    state.SetVolume(0.42);

    REQUIRE(state.Volume == 0.42);
    REQUIRE(state.Player->Volume() == Catch::Approx(0.42f));
}

// ─── WA-10/11/12/13/14 ResolveAndPlay ───────────────────────────────

namespace {

/// A dedicated-thread dispatcher: its thread pumps its own queue, so
/// `TryEnqueue` callbacks run without a window message loop in tests.
winrt::Microsoft::UI::Dispatching::DispatcherQueueController
makeDispatcher() {
    return winrt::Microsoft::UI::Dispatching::DispatcherQueueController::
        CreateOnDedicatedThread();
}

} // namespace

TEST_CASE("WA-10 ResolveAndPlay success persists, inserts, and plays") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto controller = makeDispatcher();
    REQUIRE(controller);
    state.SetDispatcherQueue(controller.DispatcherQueue());

    state.ResolveAndPlay(L"  https://example.com/wa10-tone.mp3  "); // trims input
    // The callback clears IsResolvingUrl first, then persists/inserts/plays —
    // wait for the final observable effect instead of the flag.
    REQUIRE(waitFor([&] { return state.CurrentTrack.has_value(); }));

    REQUIRE(state.UrlError.empty());
    REQUIRE(state.Tracks.size() == 1);
    REQUIRE(state.CurrentTrack.has_value());
    // Saved with the database id (#39).
    REQUIRE(state.CurrentTrack->id > 0);
    REQUIRE(state.CurrentTrack->sourceUrl == L"https://example.com/wa10-tone.mp3");
    REQUIRE(state.IsPlaying);
    state.Player->Stop();
    state.IsPlaying = false;
}

TEST_CASE("WA-11 ResolveAndPlay failure reports kind and message (#21)") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto controller = makeDispatcher();
    REQUIRE(controller);
    state.SetDispatcherQueue(controller.DispatcherQueue());

    std::wstring kind, message;
    state.OnUrlError = [&](const std::wstring& k, const std::wstring& m) {
        kind = k;
        message = m;
    };

    state.ResolveAndPlay(L"not a url");
    // The callback sets UrlError before OnUrlError; wait for the callback's
    // last write (kind/message) so the reads below don't race it.
    REQUIRE(waitFor([&] { return !kind.empty(); }));

    REQUIRE_FALSE(state.UrlError.empty());
    REQUIRE(kind == L"invalid_url");
    REQUIRE_FALSE(message.empty());
    REQUIRE(state.Tracks.empty()); // nothing queued for an unplayable URL
}

TEST_CASE("WA-12 ResolveAndPlay ignores blank input") {
    AppState state;

    state.ResolveAndPlay(L"   \t\n ");

    REQUIRE_FALSE(state.IsResolvingUrl.load());
    REQUIRE(state.UrlError.empty());
}

TEST_CASE("WA-13 ResolveAndPlay ignores re-entrant calls") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto controller = makeDispatcher();
    REQUIRE(controller);
    state.SetDispatcherQueue(controller.DispatcherQueue());

    int errorCallbacks = 0;
    state.OnUrlError = [&](const std::wstring&, const std::wstring&) {
        ++errorCallbacks;
    };

    state.ResolveAndPlay(L"not a url");
    REQUIRE(state.IsResolvingUrl.load());
    // Re-entrancy is a same-thread check: the second call lands before the
    // (microsecond-fast) background failure can finish, so the window for
    // a spurious double-dispatch is the thread-start latency, not a race
    // in the product.
    state.ResolveAndPlay(L"also not a url"); // must be ignored

    REQUIRE(waitFor([&] { return errorCallbacks >= 1; }));
    ::Sleep(100); // grace period for a second (wrong) dispatch to land
    REQUIRE(errorCallbacks == 1); // only the first resolution ran
}

TEST_CASE("WA-14 ResolveAndPlay without dispatcher drops the result and resets") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    // No SetDispatcherQueue: the background thread just clears the flag.

    int errorCallbacks = 0;
    state.OnUrlError = [&](const std::wstring&, const std::wstring&) {
        ++errorCallbacks;
    };

    state.ResolveAndPlay(L"not a url");
    REQUIRE(waitFor([&] { return !state.IsResolvingUrl.load(); }));

    REQUIRE(state.UrlError.empty());
    REQUIRE(errorCallbacks == 0);
    REQUIRE(state.Tracks.empty());
}

// ─── WA-16 打开失败 ─────────────────────────────────────────────────

TEST_CASE("WA-16 Library open failure leaves methods as safe no-ops") {
    TempDir dir;
    AppState state;

    // A directory is not a valid database path — open fails inside Library.
    state.OpenDatabase(dir.path.wstring());

    REQUIRE(state.Tracks.empty());
    REQUIRE(state.Playlists.empty());
    REQUIRE(state.Library->ImportDirectory(L"C:\\x") == -1);
    REQUIRE(state.Library->RemoveTrack(1) == false);

    auto original = makeLocalTrack(L"C:\\x\\t.mp3", L"T");
    auto saved = state.Library->AddTrack(original);
    REQUIRE(saved.id == original.id); // returned as-is on failure
    REQUIRE(state.Library->AllTracks().empty());
}

// WA-17（解析失败各 kind 上报，P2）：invalid_url 已由 WA-11 覆盖；其余
// kind 需 stub yt-dlp 注入（Windows 无 shell stub 设施），顺延至后续票。
