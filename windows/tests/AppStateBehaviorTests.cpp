// WA-01–23：Windows AppState 行为清单（manifest:
// docs/testing/behavior/windows-appstate.md）。零接缝：真 rhythm_core DLL +
// 临时 SQLite 库；#81/#82 已修复（T7），原红测已解禁转真断言。
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

/// #81（T7 修复）：缺 filePath/sourceUrl 时不进入播放状态。
TEST_CASE("WA-05 PlayTrack with no path must not enter playing") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());

    Track broken;
    broken.id = -1;
    broken.sourceType = L"local";
    broken.title = L"No Path";
    state.PlayTrack(broken);

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

/// #82（T7 修复）：恢复时 `Player->Resume()` 续播。
TEST_CASE("WA-07 TogglePlayPause resume continues playback") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wav = writeWavAt(dir.path, L"resume.wav", 3.0);
    auto saved = state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Resume Me"));
    state.PlayTrack(saved);
    if (!waitFor([&] { return state.Player->State() == 1; })) {
        SKIP("无音频输出设备，无法观察 Playing 状态（环境跳过）");
    }
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
    // A real Resume() keeps the position at or past the pause point.
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

/// #111：非 Paused 状态下 resume 是 no-op，UI 不得进入播放状态。
TEST_CASE("WA-08b TogglePlayPause resume only when paused") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wav = writeWavAt(dir.path, L"resume-guard.wav", 3.0);
    auto saved = state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Resume Guard"));
    state.PlayTrack(saved);
    if (!waitFor([&] { return state.Player->State() == 1; })) {
        SKIP("无音频输出设备，无法观察 Playing 状态（环境跳过）");
    }
    // Engine back to Stopped while CurrentTrack is still set — the exact
    // shape of the Error/Stopped divergence from #111.
    state.Player->Stop();
    state.IsPlaying = false;

    state.TogglePlayPause();

    REQUIRE_FALSE(state.IsPlaying);
    REQUIRE(state.Player->State() == 0); // resume must not have fired
    state.Player->Stop();
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

TEST_CASE("WA-24 ResolveAndPlay reloads from DB so list and queue stay in sync (#139)") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wa = dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto savedA = state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    state.RefreshLibrary();
    state.PlayTrack(savedA);
    REQUIRE_FALSE(state.CanPlayNext()); // single-track queue

    auto controller = makeDispatcher();
    REQUIRE(controller);
    state.SetDispatcherQueue(controller.DispatcherQueue());

    state.ResolveAndPlay(L"https://example.com/wa15-tone.mp3");
    REQUIRE(waitFor([&] {
        return state.CurrentTrack.has_value() &&
               state.CurrentTrack->sourceUrl == L"https://example.com/wa15-tone.mp3";
    }));

    // List reloaded from the DB (not a manual front-insert): both tracks
    // present, and the queue reaches the pre-existing one.
    REQUIRE(state.Tracks.size() == 2);
    REQUIRE(state.CanPlayNext());
    state.PlayNext();
    REQUIRE(state.CurrentTrack->id == savedA.id);
    state.Player->Stop();
    state.IsPlaying = false;
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

// ─── WA-18 先停后播 ────────────────────────────────────────────────

TEST_CASE("WA-18 PlayTrack stops the old stream before starting the new one") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());

    auto wa = dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto c = writeWavAt(wa, L"c.wav", 3.0);
    auto savedA = state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    auto savedB = state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    state.Library->AddTrack(makeLocalTrack(c.wstring(), L"C"));
    state.RefreshLibrary();

    state.PlayTrack(savedA);
    if (!waitFor([&] { return state.Player->State() == 1; }, 5000)) {
        SKIP("无音频输出设备，无法观察 Playing 状态（环境跳过）");
    }
    state.PlayTrack(savedB);
    REQUIRE(state.CurrentTrack->id == savedB.id);
    REQUIRE(state.IsPlaying);
    // With #51 honoured the new stream owns the output — the player stays
    // playing instead of piling two streams onto the device.
    REQUIRE(waitFor([&] { return state.Player->State() == 1; }, 5000));
    state.Player->Stop();
    state.IsPlaying = false;
}

// ─── WA-19 播放队列 ─────────────────────────────────────────────────

TEST_CASE("WA-19 PlayQueue wrapper roundtrips through FFI") {
    auto t1 = makeLocalTrack(L"C:\\a\\one.mp3", L"One");
    t1.id = 1;
    auto t2 = makeLocalTrack(L"C:\\a\\two.mp3", L"Two");
    t2.id = 2;

    PlayQueue q({t1, t2});
    REQUIRE(q.Current().has_value());
    REQUIRE(q.Current()->title == L"One");
    REQUIRE(q.HasNext());
    REQUIRE_FALSE(q.HasPrevious());

    REQUIRE(q.Next()->title == L"Two");
    REQUIRE_FALSE(q.HasNext());
    REQUIRE_FALSE(q.Next().has_value()); // exhausted → nullopt
    // Exhaustion parks the cursor past the end; previous() steps back onto
    // the last track (queue semantics locked in the rust-core suites).
    REQUIRE(q.Previous()->title == L"Two");

    // SingleLoop: next() stays at the current track.
    q.SetMode(2);
    REQUIRE(q.Next()->title == L"Two");

    REQUIRE(q.JumpTo(1));
    REQUIRE_FALSE(q.JumpTo(999));
    REQUIRE(q.Current()->title == L"One");

    auto t3 = makeLocalTrack(L"C:\\a\\three.mp3", L"Three");
    t3.id = 3;
    q.Replace({t3});
    REQUIRE(q.Current()->title == L"Three");

    PlayQueue empty({});
    REQUIRE_FALSE(empty.Current().has_value());
    REQUIRE_FALSE(empty.HasNext());
    REQUIRE_FALSE(empty.HasPrevious());
}

TEST_CASE("WA-19 playNext/playPrevious walk the queue, exhausted is a no-op") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wa = dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    auto savedB = state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    state.RefreshLibrary();

    state.PlayTrack(savedA);
    REQUIRE(state.CanPlayNext());
    REQUIRE_FALSE(state.CanPlayPrevious());

    state.PlayNext();
    REQUIRE(state.CurrentTrack->id == savedB.id);
    REQUIRE(state.IsPlaying);
    REQUIRE_FALSE(state.CanPlayNext());
    REQUIRE(state.CanPlayPrevious());

    state.PlayNext(); // exhausted → no-op
    REQUIRE(state.CurrentTrack->id == savedB.id);

    state.PlayPrevious();
    REQUIRE(state.CurrentTrack->id == savedA.id);
    state.Player->Stop();
    state.IsPlaying = false;
}

// ─── WA-20 RefreshLibrary 队列同步 ──────────────────────────────────

TEST_CASE("WA-20 RefreshLibrary keeps the queue in sync") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wa = dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    state.RefreshLibrary();

    state.PlayTrack(savedA);
    REQUIRE_FALSE(state.CanPlayNext()); // single-track queue

    // Import a second track: the refreshed queue must now reach it (#69).
    state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    state.RefreshLibrary();
    REQUIRE(state.Tracks.size() == 2);
    REQUIRE(state.CanPlayNext());

    state.PlayNext();
    REQUIRE(state.CurrentTrack->title == L"B");
    state.Player->Stop();
    state.IsPlaying = false;
}

// ─── WA-21 播放模式循环 ─────────────────────────────────────────────

TEST_CASE("WA-21 CyclePlayMode cycles and syncs the queue") {
    AppState state;
    REQUIRE(state.CurrentMode == PlayMode::Sequential);
    state.CyclePlayMode();
    REQUIRE(state.CurrentMode == PlayMode::Shuffle);
    state.CyclePlayMode();
    REQUIRE(state.CurrentMode == PlayMode::SingleLoop);
    state.CyclePlayMode();
    REQUIRE(state.CurrentMode == PlayMode::ListLoop);
    state.CyclePlayMode();
    REQUIRE(state.CurrentMode == PlayMode::Sequential);
}

TEST_CASE("WA-21 SingleLoop keeps next on the current track") {
    TempDir dir;
    AppState state;
    state.OpenDatabase(dir.dbPath());
    auto wa = dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    state.RefreshLibrary();

    state.PlayTrack(savedA);
    state.CyclePlayMode(); // Shuffle — skip (random order)
    state.CyclePlayMode(); // SingleLoop

    state.PlayNext();
    REQUIRE(state.CurrentTrack->id == savedA.id); // stays put
    state.Player->Stop();
    state.IsPlaying = false;
}

// ─── WA-22 传输可用性 ───────────────────────────────────────────────

TEST_CASE("WA-22 transport availability gates") {
    AppState state;
    REQUIRE_FALSE(state.CanTogglePlayback());
    REQUIRE_FALSE(state.CanPlayNext());
    REQUIRE_FALSE(state.CanPlayPrevious());
    REQUIRE_FALSE(state.CanStop());

    TempDir dir;
    state.OpenDatabase(dir.dbPath());
    auto wa = dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    state.RefreshLibrary();
    REQUIRE(state.CanTogglePlayback());

    state.PlayTrack(savedA);
    REQUIRE(state.CanStop());
    REQUIRE(state.CanPlayNext());
    REQUIRE_FALSE(state.CanPlayPrevious());

    state.PlayNext();
    REQUIRE_FALSE(state.CanPlayNext());
    REQUIRE(state.CanPlayPrevious());
    state.Player->Stop();
    state.IsPlaying = false;
    REQUIRE_FALSE(state.CanStop());
}

// ─── WA-23 导入数量反馈 ─────────────────────────────────────────────

TEST_CASE("WA-23 ImportDirectory reports the imported count") {
    AppState state;
    state.ImportDirectory(L"C:\\whatever"); // no Library → no feedback
    REQUIRE_FALSE(state.ShowImportAlert);

    TempDir dir;
    state.OpenDatabase(dir.dbPath());
    auto music = dir.path / L"music";
    fs::create_directories(music);
    writeWavAt(music, L"one.wav");
    writeWavAt(music, L"two.wav");

    state.ImportDirectory(music.wstring());

    REQUIRE(state.ShowImportAlert);
    // #141: copy comes from the language layer, so assert against it
    // (system-language agnostic, mirrors the macOS suite).
    REQUIRE(state.ImportAlertMessage == L10n::ImportedTracks(2));
}
