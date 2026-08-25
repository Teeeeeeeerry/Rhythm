import XCTest
import Foundation
@testable import Rhythm

/// AS-01–26：AppState 播放编排主路径（manifest: docs/testing/behavior/appstate-macos.md）。
/// 接缝：SpyCoordinator（断言 UI → 协调器的调用）+ 可注入 resolver + 真 SQLite 库。
/// 编排规则本身（stop 先于 play、有界跳过、recordPlay 落库）在 rust-core 的
/// coordinator_behavior.rs 测试（CO-xx）；此处只断言 AppState 把状态渲染对。
final class AppStatePlaybackMainPathTests: AppStatePlaybackTestCase {

    // MARK: - AS-01 打开数据库

    func testOpenDatabase_LoadsLibrary() throws {
        // Seed the DB before opening it through AppState.
        let seed = RhythmLibrary(path: dbURL.path)
        let track = makeTrack(id: -1, title: "Seeded")
        _ = seed?.addTrack(track)

        appState.openDatabase(at: dbURL)

        XCTAssertNotNil(appState.library)
        XCTAssertEqual(appState.tracks.map(\.title), ["Seeded"])
        XCTAssertEqual(appState.playlists.count, 0)
    }

    // MARK: - AS-02 refreshLibrary 队列同步

    func testRefreshLibrary_SyncsQueue() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        let t2 = addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1) // queue: [One, Two], cursor at One
        XCTAssertTrue(appState.canPlayNext)

        // Deleting "Two" and refreshing must remove it from the queue (#69/#72).
        _ = appState.library!.removeTrack(t2.id)
        appState.refreshLibrary()

        // "Two" is gone from the queue: no next any more, current still One.
        XCTAssertFalse(appState.canPlayNext, "queue should follow the library")
        XCTAssertEqual(appState.currentTrack?.id, t1.id, "jumpTo(current.id) keeps the position")
    }

    // MARK: - AS-03 playTrack 本地曲目

    func testPlayTrack_Local_StopBeforePlay() throws {
        let track = addTrackToLibrary(makeLocalTrack(title: "Local", path: "/tmp/local.mp3"))

        appState.playTrack(track)

        XCTAssertEqual(appState.currentTrack, track)
        // The start path went through the coordinator with the right payload.
        XCTAssertEqual(spy.startCalls.count, 1)
        XCTAssertEqual(spy.startCalls[0].track.id, track.id)
        XCTAssertEqual(spy.startCalls[0].queueTracks.map(\.id), [track.id])
        XCTAssertEqual(spy.startCalls[0].mode, .sequential)
        // #51 stop-before-play and recordPlay 落库由 rust-core CO-01/CO-05 覆盖。
        XCTAssertTrue(appState.isPlaying)
        XCTAssertFalse(appState.canPlayNext, "single-track queue has no next")
    }

    // MARK: - AS-04 playTrack URL 曲目

    func testPlayTrack_URL_PlaysSourceURL() throws {
        let track = makeTrack(title: "URL", sourceUrl: "https://example.com/a.mp3")
        appState.tracks = [track]

        appState.playTrack(track)

        XCTAssertEqual(spy.startCalls.count, 1)
        XCTAssertEqual(spy.startCalls[0].track.sourceUrl, "https://example.com/a.mp3")
        // URL 分派（playURL 而非 playFile）由 rust-core CO-02 覆盖。
        XCTAssertTrue(appState.isPlaying)
    }

    // MARK: - AS-05 playTrack 自定义队列

    func testPlayTrack_CustomQueue_FromQueueTracks() throws {
        _ = addTrackToLibrary(makeLocalTrack(title: "A", path: "/tmp/a.mp3"))
        let b = addTrackToLibrary(makeLocalTrack(title: "B", path: "/tmp/b.mp3"))
        let c = addTrackToLibrary(makeLocalTrack(title: "C", path: "/tmp/c.mp3"))
        // tracks now holds all three; the default queue would have a behind b.

        appState.playTrack(b, queueTracks: [b, c])

        // Queue built from queueTracks ([b, c], positioned at b): c ahead, and
        // NOTHING behind — had the default tracks list been used, a would
        // make canPlayPrevious true.
        XCTAssertTrue(appState.canPlayNext)
        XCTAssertFalse(appState.canPlayPrevious, "queue must come from queueTracks, not tracks")
    }

    // MARK: - AS-06/07 togglePlayPause 暂停与恢复

    func testTogglePlayPause_PausesWhilePlaying() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        spy.reset()

        appState.togglePlayPause()

        XCTAssertTrue(spy.calls.contains("pause"), "toggle dispatches pause while playing")
        XCTAssertFalse(appState.isPlaying)
        XCTAssertFalse(appState.isBuffering)
    }
    func testTogglePlayPause_ResumesWhenPaused() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        appState.togglePlayPause() // pause
        spy.reset()

        appState.togglePlayPause()

        XCTAssertTrue(spy.calls.contains("resume"), "toggle resumes when the engine is Paused")
        XCTAssertTrue(appState.isPlaying)
    }

    // #111: pause during Buffering must still dispatch pause — the engine
    // honors it now, so the UI may claim stopped.
    func testTogglePlayPause_PausesWhileBuffering() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        spy.reset()
        spy.state = 3 // Buffering

        appState.togglePlayPause()

        XCTAssertTrue(spy.calls.contains("pause"), "toggle dispatches pause while playing")
        XCTAssertFalse(appState.isPlaying)
        XCTAssertFalse(appState.isBuffering)
    }

    // #111: resume in a non-Paused state is a no-op — the UI must not claim
    // playback when the engine cannot resume.
    func testTogglePlayPause_ResumeOnlyWhenPaused() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        spy.reset()
        spy.state = 4 // Error — resume is a no-op

        appState.togglePlayPause()

        XCTAssertFalse(appState.isPlaying, "no-op resume must not claim playback")
        XCTAssertFalse(spy.calls.contains("resume"), "no resume dispatch outside Paused")
    }

    // MARK: - AS-08 togglePlayPause 空闲启动

    func testTogglePlayPause_StartsFirstTrackWhenIdle() throws {
        // The idle-start candidate selection lives in the coordinator, fed by
        // the library mirror (syncQueue) — so seed via the library, not a
        // direct tracks assignment.
        let first = addTrackToLibrary(makeLocalTrack(title: "First", path: "/tmp/first.mp3"))

        appState.togglePlayPause()

        XCTAssertEqual(appState.currentTrack, first)
        XCTAssertEqual(spy.startCalls.count, 1)
        XCTAssertEqual(spy.startCalls[0].track.id, first.id)
        XCTAssertTrue(appState.isPlaying)
    }

    // MARK: - AS-09 playNext

    func testPlayNext_AdvancesQueue() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        let t2 = addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1)
        spy.reset()

        appState.playNext()

        XCTAssertEqual(appState.currentTrack, t2)
        // 有界跳过与 stop 先于分派由 rust-core CO-09/CO-10 覆盖。
        XCTAssertEqual(spy.calls, ["next"])
        XCTAssertTrue(appState.isPlaying)
    }

    func testPlayNext_NoNext_NoOp() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        appState.playTrack(t1)
        spy.reset()

        appState.playNext()

        XCTAssertEqual(appState.currentTrack, t1)
        XCTAssertEqual(spy.calls, ["next"], "next 派发但无下一首时 current 不变")
    }

    // MARK: - AS-10 playPrevious

    func testPlayPrevious_GoesBack() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        let t2 = addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t2)
        spy.reset()

        appState.playPrevious()

        XCTAssertEqual(appState.currentTrack, t1)
        XCTAssertEqual(spy.calls, ["previous"])
        XCTAssertTrue(appState.isPlaying)
    }

    // MARK: - AS-11 stop

    func testStop_ResetsEverything() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        appState.playTrack(t1)
        appState.position = 30
        appState.duration = 180
        spy.reset()

        appState.stop()

        XCTAssertEqual(spy.calls, ["stop"])
        XCTAssertFalse(appState.isPlaying)
        XCTAssertFalse(appState.isBuffering)
        XCTAssertNil(appState.currentTrack)
        XCTAssertFalse(appState.canPlayNext, "queue must be cleared")
        XCTAssertFalse(appState.canPlayPrevious)
        XCTAssertEqual(appState.position, 0)
        XCTAssertEqual(appState.duration, 0)
    }

    // MARK: - AS-12/13/14/15 事件驱动（ticket #172，替代轮询）

    func testProgressEvent_UpdatesPositionAndDuration() {
        spy.onEvent?(.progress(position: 42.5, duration: 100))

        XCTAssertEqual(appState.position, 42.5)
        XCTAssertEqual(appState.duration, 100)
    }

    func testStateEvent_UpdatesBufferingAndPlaying() {
        spy.onEvent?(.state(state: "buffering"))
        XCTAssertTrue(appState.isBuffering)
        XCTAssertTrue(appState.isPlaying, "buffering still counts as playing")

        spy.onEvent?(.state(state: "playing"))
        XCTAssertFalse(appState.isBuffering)

        spy.onEvent?(.state(state: "paused"))
        XCTAssertFalse(appState.isPlaying, "paused stops claiming playback")
    }

    func testFinishedEvent_AutoAdvanceFollowedByTrackChanged() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        let t2 = addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1)

        // The core auto-advances on Finished (CO-26); the UI renders the
        // Finished event, then the TrackChanged event that follows.
        spy.onEvent?(.finished)
        XCTAssertFalse(appState.isPlaying, "finished stops claiming playback")
        XCTAssertEqual(appState.currentTrack, t1)

        spy.onEvent?(.trackChanged(track: t2))
        XCTAssertEqual(appState.currentTrack, t2)
        XCTAssertTrue(appState.isPlaying)
    }

    func testFinishedEvent_NoNext_Stops() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        appState.playTrack(t1)

        spy.onEvent?(.finished)

        XCTAssertFalse(appState.isPlaying, "no next track — playback ends")
        XCTAssertEqual(appState.currentTrack, t1)
    }

    func testErrorEvent_SurfacesMessage() throws {
        spy.onEvent?(.error(kind: nil, message: "core says boom"))

        XCTAssertFalse(appState.isPlaying)
        XCTAssertNotNil(appState.urlError)
        XCTAssertTrue(appState.urlError!.contains("core says boom"),
                      "the core's detail must be visible (#23)")
    }

    // #120: an HTTP failure classified "expired" keeps the old "re-paste the
    // link" advice — that is the one case where re-pasting can help.
    // #135: locale must be pinned — the assertions below are Chinese, and on
    // an English machine the un-pinned test used to pass for the wrong reason
    // (the early English return ignored `kind` entirely).
    func testUpdatePlaybackProgress_Error_ExpiredKind_KeepsRepasteAdvice() throws {        UserDefaults.standard.set("zh", forKey: "AppLanguage")
        defer { UserDefaults.standard.removeObject(forKey: "AppLanguage") }

        spy.onEvent?(.error(kind: "expired", message: "GET …/videoplayback failed: HTTP 403"))

        XCTAssertTrue(appState.urlError!.contains("重新粘贴"),
                      "expired links may still be re-pasted: \(appState.urlError!)")
    }

    // #120: a CDN rejecting a still-valid URL must NOT advise re-pasting —
    // the retry already re-resolved, and the network side is the problem.
    func testUpdatePlaybackProgress_Error_CdnRejectedKind_BlamesNetwork() throws {
        UserDefaults.standard.set("zh", forKey: "AppLanguage")
        defer { UserDefaults.standard.removeObject(forKey: "AppLanguage") }

        spy.onEvent?(.error(kind: "cdn_rejected", message: "GET …/videoplayback failed: HTTP 403"))

        XCTAssertTrue(appState.urlError!.contains("网络"),
                      "cdn rejection is a network-side problem: \(appState.urlError!)")
        XCTAssertFalse(appState.urlError!.contains("重新粘贴"),
                       "re-pasting cannot fix a CDN rejection: \(appState.urlError!)")
    }

    // #135: the English branch must carry the same classification — this
    // failed before the fix (early return dropped `kind`).
    func testPlaybackFailed_English_ClassifiesKind() {
        UserDefaults.standard.set("en", forKey: "AppLanguage")
        defer { UserDefaults.standard.removeObject(forKey: "AppLanguage") }
        XCTAssertTrue(L10n.playbackFailed(kind: "expired", detail: "d").contains("past"),
                      "English expired copy must advise re-pasting")
        XCTAssertTrue(L10n.playbackFailed(kind: "cdn_rejected", detail: "d").contains("network"),
                      "English cdn_rejected copy must blame the network")
        XCTAssertFalse(L10n.playbackFailed(kind: "cdn_rejected", detail: "d").contains("past"),
                       "English cdn_rejected copy must not advise re-pasting")
    }

    // MARK: - AS-16 seek

    func testSeek_ForwardsAndUpdatesOptimistically() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        spy.reset()

        appState.seek(to: 30)

        XCTAssertEqual(spy.seekCalls, [30])
        XCTAssertEqual(appState.position, 30, "position updates immediately (#73)")
    }
    // MARK: - AS-17 cyclePlayMode

    func testCyclePlayMode_CyclesAndSyncsQueue() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1)
        XCTAssertEqual(appState.playMode, .sequential)

        appState.cyclePlayMode()
        XCTAssertEqual(appState.playMode, .shuffle)

        // Shuffle → singleLoop: next() must repeat the current track, which is
        // only possible if setMode reached the real queue.
        appState.cyclePlayMode()
        XCTAssertEqual(appState.playMode, .singleLoop)
        spy.reset()
        appState.playNext()

        XCTAssertEqual(appState.currentTrack, t1, "singleLoop repeats the current track")
    }

    // MARK: - AS-18 传输可用性

    func testTransportAvailability_MirrorsActions() throws {
        XCTAssertFalse(appState.canTogglePlayback)

        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1)

        XCTAssertTrue(appState.canTogglePlayback)
        XCTAssertTrue(appState.canPlayNext)
        XCTAssertFalse(appState.canPlayPrevious)
        XCTAssertTrue(appState.canStop)

        appState.playNext()
        XCTAssertFalse(appState.canPlayNext, "at the end of the queue")
        XCTAssertTrue(appState.canPlayPrevious)
        let before = appState.currentTrack
        appState.playNext() // gated action must no-op when unavailable
        XCTAssertEqual(appState.currentTrack, before)

        appState.stop()
        XCTAssertFalse(appState.canStop)
        XCTAssertFalse(appState.canPlayNext, "queue cleared on stop")
    }

    // MARK: - AS-19 resolveAndImport 成功

    func testResolveAndImport_Success_ImportsWithoutPlaying() throws {
        appState.resolver = { _ in .success(ResolvedInfo(
            title: "Resolved Title",
            artist: "Someone",
            streamUrl: "https://cdn.example.com/expiring.mp3",
            duration: 90,
            sourceType: "youtube",
            thumbnailUrl: nil
        )) }

        appState.resolveAndImport("  https://page.example.com/watch?v=1  ")

        XCTAssertTrue(waitUntil { appState.showImportAlert }, "resolution should finish")
        XCTAssertEqual(appState.tracks.count, 1)
        XCTAssertEqual(appState.tracks[0].title, "Resolved Title")
        // The page URL is stored, not the expiring CDN link (#74).
        XCTAssertEqual(appState.tracks[0].sourceUrl, "https://page.example.com/watch?v=1")
        XCTAssertFalse(appState.isPlaying, "import must not auto-play (#74)")
        XCTAssertNil(appState.currentTrack)
        XCTAssertTrue(spy.startCalls.isEmpty, "no playback started at all")
        XCTAssertFalse(appState.isResolvingURL)
        XCTAssertEqual(appState.urlInput, "")
    }

    // MARK: - AS-20 resolveAndImport 失败

    func testResolveAndImport_Failure_SurfacesError() throws {
        appState.resolver = { _ in .failure(ResolveError(kind: "timeout", message: "timed out")) }

        appState.resolveAndImport("https://example.com/video")

        XCTAssertTrue(waitUntil { appState.urlError != nil }, "failure should surface")
        XCTAssertTrue(!appState.urlError!.isEmpty)
        XCTAssertFalse(appState.showImportAlert, "no import alert on failure")
        XCTAssertEqual(appState.tracks.count, 0)
        XCTAssertFalse(appState.isResolvingURL)
    }

    // MARK: - AS-21 importResolved

    func testImportResolved_PersistsRefreshesShowsAlert() throws {
        appState.urlInput = "https://example.com/video"

        appState.importResolved(makeTrack())

        XCTAssertEqual(appState.tracks.count, 1, "persisted to the library (#71)")
        XCTAssertEqual(appState.urlInput, "")
        XCTAssertTrue(appState.showImportAlert)
        XCTAssertNotNil(appState.importAlertMessage)
        XCTAssertFalse(appState.isPlaying)
        XCTAssertTrue(spy.startCalls.isEmpty)
    }

    // MARK: - AS-22 playResolved

    func testPlayResolved_PersistsThenPlaysWithRealID() throws {
        // Pre-existing library track, so the rebuilt queue has two entries and
        // the jumpTo(real id) can be observed through the availability plane.
        addTrackToLibrary(makeTrack(title: "Older", sourceUrl: "https://example.com/old.mp3"))
        let fresh = makeTrack(title: "Fresh", sourceUrl: "https://example.com/new.mp3")

        appState.playResolved(fresh)

        XCTAssertTrue(appState.isPlaying)
        XCTAssertNotNil(appState.currentTrack)
        XCTAssertGreaterThanOrEqual(appState.currentTrack!.id, 0, "saved with the real DB id (#39)")
        XCTAssertEqual(appState.currentTrack!.title, "Fresh")
        // 持久化后经协调器起播（queue = 刷新后的 tracks）。
        XCTAssertEqual(spy.startCalls.count, 1)
        XCTAssertEqual(spy.startCalls[0].track.id, appState.currentTrack!.id)
        // Queue rebuilt from the library (#39) and positioned exactly at the
        // saved track, wherever it sits in the DB order.
        XCTAssertEqual(appState.tracks.count, 2)
        let pos = appState.tracks.firstIndex { $0.id == appState.currentTrack!.id } ?? 0
        XCTAssertEqual(appState.canPlayPrevious, pos > 0, "jumpTo(real id) positioned the queue")
        XCTAssertEqual(appState.canPlayNext, pos < appState.tracks.count - 1)
        XCTAssertEqual(appState.urlInput, "")
    }

    // MARK: - AS-23 importURLs 批量导入

    func testImportURLs_AllSuccess() throws {
        let a = writeWAV(named: "a.wav")
        let b = writeWAV(named: "b.wav")

        appState.importURLs([a, b])
        XCTAssertTrue(appState.isImporting, "runs in background with the flag set (#38)")

        XCTAssertTrue(waitUntil { appState.showImportAlert })
        XCTAssertEqual(appState.tracks.count, 2)
        XCTAssertEqual(appState.importAlertMessage, L10n.importedTracks(2))
        XCTAssertFalse(appState.isImporting)
    }

    func testImportURLs_PartialFailure() throws {
        let ok = writeWAV(named: "ok.wav")
        let missing = tempDir.appendingPathComponent("missing.mp3")

        appState.importURLs([ok, missing])

        XCTAssertTrue(waitUntil { appState.showImportAlert })
        XCTAssertEqual(appState.tracks.count, 1)
        let expected = L10n.isChinese
            ? "已导入 1 首，1 个失败"
            : "Imported 1 tracks, 1 failed."
        XCTAssertEqual(appState.importAlertMessage, expected)
    }

    func testImportURLs_AllFailed() throws {
        let missing1 = tempDir.appendingPathComponent("m1.mp3")
        let missing2 = tempDir.appendingPathComponent("m2.mp3")

        appState.importURLs([missing1, missing2])

        XCTAssertTrue(waitUntil { appState.showImportAlert })
        XCTAssertEqual(appState.tracks.count, 0)
        let expected = L10n.isChinese
            ? "全部导入失败，请检查文件是否支持"
            : "All imports failed. Check that the files are supported."
        XCTAssertEqual(appState.importAlertMessage, expected)
    }

    func testImportURLs_NoSupportedFiles() throws {
        // #109/#142：importFile 对不支持格式返回 0（UnsupportedFormat），importURLs
        // 既不计数导入也不计数失败，落"未找到支持的音频文件"分支。中/英分支
        // 各自固定 locale 确定性断言（旧写法依赖机器语言，单机只测到一个分支）。
        let txt = tempDir.appendingPathComponent("notes.txt")
        try! Data("not audio".utf8).write(to: txt)

        XCTAssertEqual(importURLsAlert(locale: "en", [txt]), "No supported audio files found.")
        XCTAssertEqual(importURLsAlert(locale: "zh", [txt]), "未找到支持的音频文件")
        XCTAssertEqual(appState.tracks.count, 0)
    }

    func testImportURLs_DirectoryDispatchesToImportDirectory() throws {
        _ = writeWAV(named: "in.wav")

        appState.importURLs([tempDir])

        XCTAssertTrue(waitUntil { appState.showImportAlert })
        XCTAssertEqual(appState.tracks.count, 1, "directory URLs go through importDirectory")
        XCTAssertEqual(appState.importAlertMessage, L10n.importedTracks(1))
    }

    // MARK: - AS-24 importDirectory / importFile

    func testImportFile_Success() throws {
        let wav = writeWAV()

        appState.importFile(wav)

        XCTAssertEqual(appState.tracks.count, 1)
        XCTAssertEqual(appState.importAlertMessage, L10n.importedTracks(1))
        XCTAssertTrue(appState.showImportAlert)
    }

    func testImportFile_UnsupportedFormat() throws {
        // #109/#142：rust-core 对不支持格式返回 0（UnsupportedFormat），命中
        // "不支持的音频格式"分支（==0 三态之一）。中/英分支各自固定 locale
        // 确定性断言（旧写法依赖机器语言，单机只测到一个分支）。
        let txt = tempDir.appendingPathComponent("notes.txt")
        try! Data("not audio".utf8).write(to: txt)

        XCTAssertEqual(importFileAlert(locale: "en", txt), "Unsupported audio format.")
        XCTAssertEqual(importFileAlert(locale: "zh", txt), "不支持的音频格式")
        XCTAssertEqual(appState.tracks.count, 0)
    }

    // MARK: - #142 中/英分支确定性覆盖（固定 locale 后执行并取弹窗文案）

    private func importFileAlert(locale: String, _ url: URL) -> String? {
        UserDefaults.standard.set(locale, forKey: "AppLanguage")
        defer { UserDefaults.standard.removeObject(forKey: "AppLanguage") }
        appState.importFile(url)
        return appState.importAlertMessage
    }

    private func importURLsAlert(locale: String, _ urls: [URL]) -> String? {
        UserDefaults.standard.set(locale, forKey: "AppLanguage")
        defer { UserDefaults.standard.removeObject(forKey: "AppLanguage") }
        appState.showImportAlert = false // 上一轮弹窗仍为 true，直接 wait 会读到旧文案
        appState.importURLs(urls)
        guard waitUntil({ appState.showImportAlert }) else { return nil }
        return appState.importAlertMessage
    }

    func testImportFile_Failure() throws {
        appState.importFile(tempDir.appendingPathComponent("missing.mp3"))

        XCTAssertEqual(appState.tracks.count, 0)
        let expected = L10n.isChinese
            ? "导入失败，文件可能已损坏或无法读取"
            : "Import failed. The file may be corrupted or unreadable."
        XCTAssertEqual(appState.importAlertMessage, expected)
    }

    func testImportDirectory_Success() throws {
        _ = writeWAV(named: "in.wav")

        appState.importDirectory(tempDir)

        XCTAssertEqual(appState.tracks.count, 1)
        XCTAssertEqual(appState.importAlertMessage, L10n.importedTracks(1))
    }

    func testImportDirectory_Empty() throws {
        let empty = tempDir.appendingPathComponent("empty", isDirectory: true)
        try! FileManager.default.createDirectory(at: empty, withIntermediateDirectories: true)

        appState.importDirectory(empty)

        XCTAssertEqual(appState.tracks.count, 0)
        XCTAssertNotNil(appState.importAlertMessage)
    }

    // MARK: - AS-25 confirmDeleteTrack 删除当前播放曲目

    func testConfirmDeleteTrack_StopsPlayback() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        let t2 = addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1)
        spy.reset()

        appState.requestDeleteTrack(t1)
        appState.confirmDeleteTrack()

        XCTAssertGreaterThanOrEqual(spy.stopCount, 1, "playback must stop (#33)")
        XCTAssertFalse(appState.isPlaying)
        XCTAssertNil(appState.currentTrack)
        XCTAssertFalse(appState.canPlayNext, "queue cleared so next can't hit a dead track")
        XCTAssertEqual(appState.tracks.map(\.id), [t2.id],
                       "only the deleted track disappears")
        XCTAssertNil(appState.trackToDelete)
    }

    // MARK: - AS-26 search

    func testSearch_EmptyQuery_ReturnsAll() throws {
        addTrackToLibrary(makeLocalTrack(title: "Alpha Song", path: "/tmp/alpha.mp3"))
        addTrackToLibrary(makeLocalTrack(title: "Beta Song", path: "/tmp/beta.mp3"))
        XCTAssertEqual(appState.tracks.count, 2)

        appState.search("")

        XCTAssertEqual(appState.tracks.count, 2)
    }

    func testSearch_Query_Filters() throws {
        addTrackToLibrary(makeLocalTrack(title: "Alpha Song", path: "/tmp/alpha.mp3"))
        addTrackToLibrary(makeLocalTrack(title: "Beta Song", path: "/tmp/beta.mp3"))

        appState.search("Alpha")

        XCTAssertEqual(appState.tracks.map(\.title), ["Alpha Song"])
    }
}
