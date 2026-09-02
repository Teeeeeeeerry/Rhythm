// WA-01–28：Windows AppState 行为清单（manifest:
// docs/testing/behavior/windows-appstate.md）。
//
// 接缝（ticket #173）：AppState 的编排全部经 ICoordinator seam；测试注入
// SpyCoordinator（内置顺序队列模型，镜像协调器契约）——播放路径不再依赖音频
// 设备，原「无音频设备 SKIP」用例全部转为确定性断言。真正的编排规则在
// rust-core 的 coordinator_behavior.rs（CO-xx）测试。
//
// 这些测试在本机（macOS）不可运行——提交后在 Windows 上 `ctest` 验证。

#include "pch.h"
#include "AppState.h"

#include <algorithm>
#include <catch_amalgamated.hpp>
#include "TestHelpers.h"

using namespace rhythm;
using namespace rhythm_tests;

// ─── Test fixture ───────────────────────────────────────────────────

/// AppState with a SpyCoordinator injected and its event handler wired to
/// ApplyCoordinatorEvent (synchronous — no dispatcher in tests).
struct SpyApp {
    TempDir dir;
    AppState state;
    SpyCoordinator* spy;

    SpyApp() {
        spy = new SpyCoordinator();
        state.Coordinator.reset(spy);
        spy->SetEventHandler([this](const std::wstring& json) {
            state.ApplyCoordinatorEvent(json);
        });
    }
};

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

// ─── WA-05 PlayTrack 分派（#81 守卫在协调器）─────────────────────────

TEST_CASE("WA-05 PlayTrack dispatches through the coordinator") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());

    auto wav = writeWavAt(app.dir.path, L"play.wav", 3.0);
    auto saved = app.state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Play Me"));

    app.state.PlayTrack(saved);

    REQUIRE(app.state.CurrentTrack.has_value());
    REQUIRE(app.state.IsPlaying);
    REQUIRE(app.spy->startCalls.size() == 1);
    REQUIRE(app.spy->startCalls[0].track.id == saved.id);
    REQUIRE(app.spy->startCalls[0].queueTracks.size() == 1);
    // recordPlay 落库由 rust-core CO-05 覆盖（协调器内执行）。
}

TEST_CASE("WA-05 PlayTrack URL dispatch goes through the coordinator") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());

    auto urlTrack = makeUrlTrack(L"https://example.com/wa05.mp3", L"URL Track");
    app.state.PlayTrack(urlTrack);

    REQUIRE(app.state.CurrentTrack.has_value());
    REQUIRE(app.state.IsPlaying);
    REQUIRE(app.spy->startCalls.size() == 1);
    REQUIRE(app.spy->startCalls[0].track.sourceUrl == L"https://example.com/wa05.mp3");
}

/// #81（T7 修复）：缺 filePath/sourceUrl 时不进入播放状态——守卫在协调器
/// （no_playable_location 分类错误，rust-core CO-03/CO-04）。
TEST_CASE("WA-05 PlayTrack with no path must not enter playing") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());

    Track broken;
    broken.id = -1;
    broken.sourceType = L"local";
    broken.title = L"No Path";
    app.state.PlayTrack(broken);

    REQUIRE_FALSE(app.state.IsPlaying);
    REQUIRE_FALSE(app.state.CurrentTrack.has_value());
    REQUIRE(app.spy->startCalls.size() == 1);
    REQUIRE(app.spy->startCalls[0].track.title == L"No Path");
}

// ─── WA-06/07/08 TogglePlayPause（语义在协调器，ticket #171）─────────

TEST_CASE("WA-06 TogglePlayPause pauses while playing") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wav = writeWavAt(app.dir.path, L"pause.wav", 3.0);
    auto saved = app.state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Pause Me"));
    app.state.PlayTrack(saved);

    app.state.TogglePlayPause();

    REQUIRE_FALSE(app.state.IsPlaying);
    REQUIRE(app.spy->toggleCalls == 1);
    REQUIRE(app.spy->engineState == 2); // Paused
}

/// #82（T7 修复）：恢复经协调器续播（仅 Paused 可恢复，#111）。
TEST_CASE("WA-07 TogglePlayPause resume continues playback") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wav = writeWavAt(app.dir.path, L"resume.wav", 3.0);
    auto saved = app.state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Resume Me"));
    app.state.PlayTrack(saved);

    app.state.TogglePlayPause(); // pause
    REQUIRE_FALSE(app.state.IsPlaying);

    app.state.TogglePlayPause(); // resume
    REQUIRE(app.state.IsPlaying);
    REQUIRE(app.spy->engineState == 1); // Playing
}

TEST_CASE("WA-08 TogglePlayPause starts the first track when idle") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wav = writeWavAt(app.dir.path, L"first.wav", 3.0);
    auto saved = app.state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"First"));
    app.state.RefreshLibrary(); // feeds the coordinator's library mirror

    app.state.TogglePlayPause(); // idle, tracks non-empty → first playable

    REQUIRE(app.state.IsPlaying);
    REQUIRE(app.state.CurrentTrack.has_value());
    REQUIRE(app.state.CurrentTrack->id == saved.id);
    REQUIRE(app.spy->startCalls.size() == 1);
}

/// #111：非 Paused 状态下 resume 是 no-op，UI 不得进入播放状态。
TEST_CASE("WA-08b TogglePlayPause resume only when paused") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wav = writeWavAt(app.dir.path, L"resume-guard.wav", 3.0);
    auto saved = app.state.Library->AddTrack(makeLocalTrack(wav.wstring(), L"Resume Guard"));
    app.state.PlayTrack(saved);
    // Engine back to Stopped while CurrentTrack is still set — the exact
    // shape of the Error/Stopped divergence from #111.
    app.spy->engineState = 0;
    app.state.IsPlaying = false;

    app.state.TogglePlayPause();

    REQUIRE_FALSE(app.state.IsPlaying);
    REQUIRE(app.spy->engineState == 0); // resume must not have fired
}

TEST_CASE("WA-15 TogglePlayPause no-ops with nothing to play") {
    SpyApp app;

    app.state.TogglePlayPause();

    REQUIRE_FALSE(app.state.IsPlaying);
    REQUIRE_FALSE(app.state.CurrentTrack.has_value());
    REQUIRE(app.spy->toggleCalls == 1);
    REQUIRE(app.spy->startCalls.empty());
}

// ─── WA-09 SetVolume ────────────────────────────────────────────────

TEST_CASE("WA-09 SetVolume updates state and coordinator") {
    SpyApp app;

    app.state.SetVolume(0.42);

    REQUIRE(app.state.Volume == 0.42);
    REQUIRE(app.spy->lastVolume == Catch::Approx(0.42f));
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
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto controller = makeDispatcher();
    REQUIRE(controller);
    app.state.SetDispatcherQueue(controller.DispatcherQueue());

    app.state.ResolveAndPlay(L"  https://example.com/wa10-tone.mp3  "); // trims input
    // The callback clears IsResolvingUrl first, then persists/inserts/plays —
    // wait for the final observable effect instead of the flag.
    REQUIRE(waitFor([&] { return app.state.CurrentTrack.has_value(); }));

    REQUIRE(app.state.UrlError.empty());
    REQUIRE(app.state.Tracks.size() == 1);
    REQUIRE(app.state.CurrentTrack.has_value());
    // Saved with the database id (#39).
    REQUIRE(app.state.CurrentTrack->id > 0);
    REQUIRE(app.state.CurrentTrack->sourceUrl == L"https://example.com/wa10-tone.mp3");
    REQUIRE(app.state.IsPlaying);
    REQUIRE(app.spy->startCalls.size() == 1);
}

TEST_CASE("WA-11 ResolveAndPlay failure reports kind and message (#21)") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto controller = makeDispatcher();
    REQUIRE(controller);
    app.state.SetDispatcherQueue(controller.DispatcherQueue());

    std::wstring kind, message;
    app.state.OnUrlError = [&](const std::wstring& k, const std::wstring& m) {
        kind = k;
        message = m;
    };

    app.state.ResolveAndPlay(L"not a url");
    // The callback sets UrlError before OnUrlError; wait for the callback's
    // last write (kind/message) so the reads below don't race it.
    REQUIRE(waitFor([&] { return !kind.empty(); }));

    REQUIRE_FALSE(app.state.UrlError.empty());
    REQUIRE(kind == L"invalid_url");
    REQUIRE_FALSE(message.empty());
    REQUIRE(app.state.Tracks.empty()); // nothing queued for an unplayable URL
}

TEST_CASE("WA-12 ResolveAndPlay ignores blank input") {
    SpyApp app;

    app.state.ResolveAndPlay(L"   \t\n ");

    REQUIRE_FALSE(app.state.IsResolvingUrl.load());
    REQUIRE(app.state.UrlError.empty());
}

TEST_CASE("WA-13 ResolveAndPlay ignores re-entrant calls") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto controller = makeDispatcher();
    REQUIRE(controller);
    app.state.SetDispatcherQueue(controller.DispatcherQueue());

    int errorCallbacks = 0;
    app.state.OnUrlError = [&](const std::wstring&, const std::wstring&) {
        ++errorCallbacks;
    };

    app.state.ResolveAndPlay(L"not a url");
    REQUIRE(app.state.IsResolvingUrl.load());
    // Re-entrancy is a same-thread check: the second call lands before the
    // (microsecond-fast) background failure can finish, so the window for
    // a spurious double-dispatch is the thread-start latency, not a race
    // in the product.
    app.state.ResolveAndPlay(L"also not a url"); // must be ignored

    REQUIRE(waitFor([&] { return errorCallbacks >= 1; }));
    ::Sleep(100); // grace period for a second (wrong) dispatch to land
    REQUIRE(errorCallbacks == 1); // only the first resolution ran
}

TEST_CASE("WA-14 ResolveAndPlay without dispatcher drops the result and resets") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    // No SetDispatcherQueue: the background thread just clears the flag.

    int errorCallbacks = 0;
    app.state.OnUrlError = [&](const std::wstring&, const std::wstring&) {
        ++errorCallbacks;
    };

    app.state.ResolveAndPlay(L"not a url");
    REQUIRE(waitFor([&] { return !app.state.IsResolvingUrl.load(); }));

    REQUIRE(app.state.UrlError.empty());
    REQUIRE(errorCallbacks == 0);
    REQUIRE(app.state.Tracks.empty());
}

TEST_CASE("WA-24 ResolveAndPlay reloads from DB so list and queue stay in sync (#139)") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wa = app.dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto savedA = app.state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    app.state.RefreshLibrary();
    app.state.PlayTrack(savedA);
    REQUIRE_FALSE(app.state.CanPlayNext()); // single-track queue

    auto controller = makeDispatcher();
    REQUIRE(controller);
    app.state.SetDispatcherQueue(controller.DispatcherQueue());

    app.state.ResolveAndPlay(L"https://example.com/wa15-tone.mp3");
    REQUIRE(waitFor([&] {
        return app.state.CurrentTrack.has_value() &&
               app.state.CurrentTrack->sourceUrl == L"https://example.com/wa15-tone.mp3";
    }));

    // List reloaded from the DB (not a manual front-insert): both tracks
    // present, and the queue reaches the pre-existing one.
    REQUIRE(app.state.Tracks.size() == 2);
    REQUIRE(app.state.CanPlayNext());
    app.state.PlayNext();
    REQUIRE(app.state.CurrentTrack->id == savedA.id);
}

// ─── WA-16 打开失败 ─────────────────────────────────────────────────

TEST_CASE("WA-16 Library open failure leaves methods as safe no-ops") {
    TempDir dir;
    AppState state;

    // A directory is not a valid database path — open fails inside Library.
    state.OpenDatabase(dir.path.wstring());

    REQUIRE(state.Tracks.empty());
    REQUIRE(state.Playlists.empty());
    REQUIRE_FALSE(state.Library->ImportDirectory(L"C:\\x").has_value());
    REQUIRE(state.Library->RemoveTrack(1) == false);

    auto original = makeLocalTrack(L"C:\\x\\t.mp3", L"T");
    auto saved = state.Library->AddTrack(original);
    REQUIRE(saved.id == original.id); // returned as-is on failure
    REQUIRE(state.Library->AllTracks().empty());
}

// WA-17（解析失败各 kind 上报，P2）：invalid_url 已由 WA-11 覆盖；其余
// kind 需 stub yt-dlp 注入（Windows 无 shell stub 设施），顺延至后续票。

// ─── WA-18 先停后播（顺序在协调器内，rust-core CO-01）───────────────

TEST_CASE("WA-18 PlayTrack switches to the new track through the coordinator") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());

    auto wa = app.dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = app.state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    auto savedB = app.state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    app.state.RefreshLibrary();

    app.state.PlayTrack(savedA);
    app.state.PlayTrack(savedB);
    REQUIRE(app.state.CurrentTrack->id == savedB.id);
    REQUIRE(app.state.IsPlaying);
    // stop 先于 play 的顺序在协调器（rust-core CO-01）；此处断言协调器被正确调用。
    REQUIRE(app.spy->startCalls.size() == 2);
    REQUIRE(app.spy->startCalls[1].track.id == savedB.id);
}

// ─── WA-19 播放队列（真 FFI 队列包装往返）───────────────────────────


TEST_CASE("WA-19 playNext/playPrevious walk the queue, exhausted is a no-op") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wa = app.dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = app.state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    auto savedB = app.state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    app.state.RefreshLibrary();

    app.state.PlayTrack(savedA);
    REQUIRE(app.state.CanPlayNext());
    REQUIRE_FALSE(app.state.CanPlayPrevious());

    app.state.PlayNext();
    REQUIRE(app.state.CurrentTrack->id == savedB.id);
    REQUIRE(app.state.IsPlaying);
    REQUIRE(app.spy->nextCalls == 1);
    REQUIRE_FALSE(app.state.CanPlayNext());
    REQUIRE(app.state.CanPlayPrevious());

    app.state.PlayNext(); // exhausted → no-op
    REQUIRE(app.state.CurrentTrack->id == savedB.id);

    app.state.PlayPrevious();
    REQUIRE(app.state.CurrentTrack->id == savedA.id);
}

// ─── WA-20 RefreshLibrary 队列同步（在协调器内，#69）─────────────────

TEST_CASE("WA-20 RefreshLibrary keeps the queue in sync") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wa = app.dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = app.state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    app.state.RefreshLibrary();

    app.state.PlayTrack(savedA);
    REQUIRE_FALSE(app.state.CanPlayNext()); // single-track queue

    // Import a second track: the refreshed queue must now reach it (#69).
    app.state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    app.state.RefreshLibrary();
    REQUIRE(app.state.Tracks.size() == 2);
    REQUIRE(app.state.CanPlayNext());
    REQUIRE(app.spy->syncQueueCalls.size() >= 2);

    app.state.PlayNext();
    REQUIRE(app.state.CurrentTrack->title == L"B");
}

// ─── WA-21 播放模式循环 ─────────────────────────────────────────────

TEST_CASE("WA-21 CyclePlayMode cycles and syncs the queue") {
    SpyApp app;
    REQUIRE(app.state.CurrentMode == PlayMode::Sequential);
    app.state.CyclePlayMode();
    REQUIRE(app.state.CurrentMode == PlayMode::Shuffle);
    app.state.CyclePlayMode();
    REQUIRE(app.state.CurrentMode == PlayMode::SingleLoop);
    app.state.CyclePlayMode();
    REQUIRE(app.state.CurrentMode == PlayMode::ListLoop);
    app.state.CyclePlayMode();
    REQUIRE(app.state.CurrentMode == PlayMode::Sequential);
    REQUIRE(app.spy->setPlayModeCalls.size() == 4);
}

TEST_CASE("WA-21 SingleLoop keeps next on the current track") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wa = app.dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = app.state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    app.state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    app.state.RefreshLibrary();

    app.state.PlayTrack(savedA);
    app.state.CyclePlayMode(); // Shuffle
    app.state.CyclePlayMode(); // SingleLoop

    app.state.PlayNext();
    REQUIRE(app.state.CurrentTrack->id == savedA.id); // stays put
}

// ─── WA-22 传输可用性（来自协调器导出）───────────────────────────────

TEST_CASE("WA-22 transport availability gates") {
    SpyApp app;
    REQUIRE_FALSE(app.state.CanTogglePlayback());
    REQUIRE_FALSE(app.state.CanPlayNext());
    REQUIRE_FALSE(app.state.CanPlayPrevious());
    REQUIRE_FALSE(app.state.CanStop());

    app.state.OpenDatabase(app.dir.dbPath());
    auto wa = app.dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = app.state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    app.state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    app.state.RefreshLibrary();
    REQUIRE(app.state.CanTogglePlayback());

    app.state.PlayTrack(savedA);
    REQUIRE(app.state.CanStop());
    REQUIRE(app.state.CanPlayNext());
    REQUIRE_FALSE(app.state.CanPlayPrevious());

    app.state.PlayNext();
    REQUIRE_FALSE(app.state.CanPlayNext());
    REQUIRE(app.state.CanPlayPrevious());
    app.state.Coordinator->Stop();
    app.state.IsPlaying = false;
    REQUIRE_FALSE(app.state.CanStop());
}

// ─── WA-23 导入数量反馈 ─────────────────────────────────────────────

TEST_CASE("WA-23 ImportDirectory reports the imported count") {
    SpyApp app;
    app.state.ImportDirectory(L"C:\\whatever"); // no Library → no feedback
    REQUIRE_FALSE(app.state.ShowImportAlert);

    app.state.OpenDatabase(app.dir.dbPath());
    auto music = app.dir.path / L"music";
    fs::create_directories(music);
    writeWavAt(music, L"one.wav");
    writeWavAt(music, L"two.wav");

    app.state.ImportDirectory(music.wstring());

    REQUIRE(app.state.ShowImportAlert);
    // #141: copy comes from the language layer, so assert against it
    // (system-language agnostic, mirrors the macOS suite).
    REQUIRE(app.state.ImportAlertMessage == L10n::ImportedTracks(2));

    // #241: the three arms come from the named counts, not a magic integer.
    auto empty = app.dir.path / L"empty";
    fs::create_directories(empty);
    app.state.ImportDirectory(empty.wstring());
    REQUIRE(app.state.ImportAlertMessage == L10n::ImportNoFiles());

    app.state.ImportDirectory((app.dir.path / L"missing").wstring());
    REQUIRE(app.state.ImportAlertMessage == L10n::ImportFailed());
}

// ─── WA-25 事件驱动（ticket #172/#173，替代定时器轮询）──────────────

TEST_CASE("WA-25 progress and state events drive the UI") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());

    app.spy->FireEvent(L"{\"type\":\"progress\",\"position\":42.5,\"duration\":100.0}");
    REQUIRE(app.state.Position == 42.5);
    REQUIRE(app.state.Duration == 100.0);

    app.spy->FireEvent(L"{\"type\":\"state\",\"state\":\"buffering\"}");
    REQUIRE(app.state.IsBuffering);
    REQUIRE(app.state.IsPlaying);

    app.spy->FireEvent(L"{\"type\":\"state\",\"state\":\"paused\"}");
    REQUIRE_FALSE(app.state.IsPlaying);
    REQUIRE_FALSE(app.state.IsBuffering);
}

TEST_CASE("WA-25 finished auto-advance renders via track_changed") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());
    auto wa = app.dir.path / L"wa";
    fs::create_directories(wa);
    auto a = writeWavAt(wa, L"a.wav", 3.0);
    auto b = writeWavAt(wa, L"b.wav", 3.0);
    auto savedA = app.state.Library->AddTrack(makeLocalTrack(a.wstring(), L"A"));
    auto savedB = app.state.Library->AddTrack(makeLocalTrack(b.wstring(), L"B"));
    app.state.RefreshLibrary();
    app.state.PlayTrack(savedA);

    // The core auto-advances on Finished (CO-25/CO-26); the UI renders the
    // Finished event, then the TrackChanged event that follows.
    app.spy->FireEvent(L"{\"type\":\"finished\"}");
    REQUIRE_FALSE(app.state.IsPlaying);
    REQUIRE(app.state.CurrentTrack->id == savedA.id);

    std::string trackJson = R"({"id":)" + std::to_string(savedB.id) +
        R"(,"file_path":")" + WideToUtf8ForTest(savedB.filePath ? *savedB.filePath : L"") +
        R"(","source_type":"local","title":")" + WideToUtf8ForTest(savedB.title) + R"("})";
    app.spy->FireEvent(L"{\"type\":\"track_changed\",\"track\":" +
                       std::wstring(trackJson.begin(), trackJson.end()) + L"}");

    REQUIRE(app.state.CurrentTrack->id == savedB.id);
    REQUIRE(app.state.IsPlaying);
}

TEST_CASE("WA-25 playback failure event surfaces classified copy (#120)") {
    LocaleOverride zh(L"zh");
    SpyApp app;

    std::wstring kind, message;
    app.state.OnUrlError = [&](const std::wstring& k, const std::wstring& m) {
        kind = k;
        message = m;
    };

    app.spy->FireEvent(
        L"{\"type\":\"error\",\"kind\":\"cdn_rejected\",\"message\":\"GET x failed: HTTP 403\"}");

    REQUIRE_FALSE(app.state.IsPlaying);
    REQUIRE_FALSE(app.state.UrlError.empty());
    REQUIRE(kind == L"playback_cdn_rejected");
    REQUIRE(message == L"GET x failed: HTTP 403");
}

// ─── WA-29 单文件导入（#242，Windows 首次具备该能力）──────────────

TEST_CASE("WA-29 ImportFile imports one audio file and reports the outcome") {
    SpyApp app;
    app.state.ImportFile(L"C:\\whatever\\x.wav"); // no Library -> no feedback
    REQUIRE_FALSE(app.state.ShowImportAlert);

    app.state.OpenDatabase(app.dir.dbPath());
    auto wav = writeWavAt(app.dir.path, L"single.wav");

    app.state.ImportFile(wav.wstring());

    REQUIRE(app.state.ShowImportAlert);
    REQUIRE(app.state.ImportAlertMessage == L10n::ImportedTracks(1));
    // Reloaded from the database, not stitched together by hand.
    REQUIRE(app.state.Tracks.size() == 1);
    REQUIRE(app.state.Tracks[0].title == L"single");
}

TEST_CASE("WA-29 ImportFile tells unsupported apart from unreadable") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());

    auto txt = app.dir.path / L"notes.txt";
    {
        std::ofstream out(txt);
        out << "not audio";
    }

    app.state.ImportFile(txt.wstring());
    REQUIRE(app.state.ImportAlertMessage == L10n::ImportFileUnsupported());

    app.state.ImportFile((app.dir.path / L"missing.wav").wstring());
    REQUIRE(app.state.ImportAlertMessage == L10n::ImportFileFailed());

    REQUIRE(app.state.Tracks.empty());
}

// ─── WA-26 M3U8 导入结果渲染（#236，入库策略在核心）────────────────

/// The core parses and stores; this layer only renders the counts and
/// reloads the list. The storage rules are asserted in rust-core
/// (playlist_m3u8_behavior.rs PL-17 to PL-24). Same playlist shape as the
/// macOS test, so both platforms must report the same counts and text.
TEST_CASE("WA-26 ImportM3U8 renders the core's counts and reloads the list") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());

    auto playlist = app.dir.path / L"list.m3u8";
    {
        std::ofstream out(playlist);
        out << "#EXTM3U\n";
        out << "#EXTINF:180,Local Artist - Local Song\n";
        out << "/music/local.mp3\n";
        out << "#EXTINF:0,Remote Artist - Remote Song\n";
        out << "https://example.com/remote.mp3\n";
    }

    app.state.ImportM3U8(playlist.wstring());

    REQUIRE(app.state.Tracks.size() == 2);
    REQUIRE(app.state.ShowImportAlert);
    REQUIRE(app.state.ImportAlertMessage == L10n::ImportedTracks(2));
}

TEST_CASE("WA-26 ImportM3U8 on an unreadable playlist shows no alert") {
    SpyApp app;
    app.state.OpenDatabase(app.dir.dbPath());

    app.state.ImportM3U8((app.dir.path / L"missing.m3u8").wstring());

    REQUIRE_FALSE(app.state.ShowImportAlert);
    REQUIRE(app.state.Tracks.empty());
}
