import XCTest
import Foundation
@testable import Rhythm

/// AS-01–26：AppState 播放编排主路径（manifest: docs/testing/behavior/appstate-macos.md）。
/// 接缝：SpyPlayer（断言调用顺序）+ 可注入 resolver + 真 SQLite 库 / 真 RhythmQueue。
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
        // #51: stop must come strictly before the new play call.
        XCTAssertEqual(spy.calls, ["stop", "playFile:/tmp/local.mp3"])
        XCTAssertTrue(spy.playURLCalls.isEmpty)
        XCTAssertTrue(appState.isPlaying)
        XCTAssertFalse(appState.canPlayNext, "single-track queue has no next")
        // recordPlay persists: the library row's play counter advances.
        appState.refreshLibrary()
        XCTAssertEqual(appState.tracks.first?.playCount, 1, "recordPlay(id) must reach the DB")
    }

    // MARK: - AS-04 playTrack URL 曲目

    func testPlayTrack_URL_PlaysSourceURL() throws {
        let track = makeTrack(title: "URL", sourceUrl: "https://example.com/a.mp3")
        appState.tracks = [track]

        appState.playTrack(track)

        XCTAssertEqual(spy.calls, ["stop", "playURL:https://example.com/a.mp3"])
        XCTAssertTrue(spy.playFileCalls.isEmpty, "URL track must not touch playFile")
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

        XCTAssertEqual(spy.calls, ["pause"])
        XCTAssertFalse(appState.isPlaying)
        XCTAssertFalse(appState.isBuffering)
    }

    func testTogglePlayPause_ResumesWhenPaused() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        appState.togglePlayPause() // pause
        spy.reset()

        appState.togglePlayPause()

        XCTAssertEqual(spy.calls, ["resume"])
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

        XCTAssertEqual(spy.calls, ["pause"])
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
        let first = makeLocalTrack(title: "First", path: "/tmp/first.mp3")
        appState.tracks = [first]

        appState.togglePlayPause()

        XCTAssertEqual(appState.currentTrack, first)
        XCTAssertTrue(spy.calls.contains("playFile:/tmp/first.mp3"))
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
        XCTAssertEqual(spy.calls, ["stop", "playFile:/tmp/two.mp3"], "stop must precede dispatch")
        XCTAssertTrue(appState.isPlaying)
        appState.refreshLibrary()
        XCTAssertEqual(appState.tracks.first(where: { $0.id == t2.id })?.playCount, 1,
                       "recordPlay(next.id) must reach the DB")
    }

    func testPlayNext_NoNext_NoOp() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        appState.playTrack(t1)
        spy.reset()

        appState.playNext()

        XCTAssertEqual(appState.currentTrack, t1)
        XCTAssertFalse(spy.hasAnyCall)
    }

    // MARK: - AS-10 playPrevious

    func testPlayPrevious_GoesBack() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        let t2 = addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t2)
        spy.reset()

        appState.playPrevious()

        XCTAssertEqual(appState.currentTrack, t1)
        XCTAssertEqual(spy.calls, ["stop", "playFile:/tmp/one.mp3"])
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

    // MARK: - AS-12 进度同步

    func testUpdatePlaybackProgress_SyncsFromPlayer() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        spy.position = 42.5
        spy.duration = 100
        spy.state = 3 // Buffering

        appState.updatePlaybackProgress()

        XCTAssertEqual(appState.position, 42.5)
        XCTAssertEqual(appState.duration, 100)
        XCTAssertTrue(appState.isBuffering)

        spy.state = 2 // Playing
        appState.updatePlaybackProgress()
        XCTAssertFalse(appState.isBuffering)
    }

    // MARK: - AS-13 播完连播

    func testUpdatePlaybackProgress_Finished_AdvancesToNext() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        let t2 = addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1)
        spy.reset()
        spy.state = 5 // Finished

        appState.updatePlaybackProgress()

        XCTAssertEqual(appState.currentTrack, t2, "finished track must auto-advance")
        XCTAssertTrue(appState.isPlaying)
        XCTAssertEqual(spy.calls, ["stop", "playFile:/tmp/two.mp3"])
    }

    // MARK: - AS-14 播完终止

    func testUpdatePlaybackProgress_Finished_NoNext_Stops() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        appState.playTrack(t1)
        spy.state = 5 // Finished

        appState.updatePlaybackProgress()

        XCTAssertFalse(appState.isPlaying, "no next track — playback ends")
        XCTAssertEqual(appState.currentTrack, t1)
    }

    // MARK: - AS-15 播放失败

    func testUpdatePlaybackProgress_Error_SurfacesMessage() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        spy.state = 4 // Error
        spy.errorMessage = "core says boom"

        appState.updatePlaybackProgress()

        XCTAssertFalse(appState.isPlaying)
        XCTAssertNotNil(appState.urlError)
        XCTAssertTrue(appState.urlError!.contains("core says boom"),
                      "the core's detail must be visible (#23)")
    }

    // #120: an HTTP failure classified "expired" keeps the old "re-paste the
    // link" advice — that is the one case where re-pasting can help.
    func testUpdatePlaybackProgress_Error_ExpiredKind_KeepsRepasteAdvice() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        spy.state = 4 // Error
        spy.errorKind = "expired"
        spy.errorMessage = "GET …/videoplayback failed: HTTP 403"

        appState.updatePlaybackProgress()

        XCTAssertTrue(appState.urlError!.contains("重新粘贴"),
                      "expired links may still be re-pasted: \(appState.urlError!)")
    }

    // #120: a CDN rejecting a still-valid URL must NOT advise re-pasting —
    // the retry already re-resolved, and the network side is the problem.
    func testUpdatePlaybackProgress_Error_CdnRejectedKind_BlamesNetwork() throws {
        appState.tracks = [makeLocalTrack(path: "/tmp/p.mp3")]
        appState.playTrack(appState.tracks[0])
        spy.state = 4 // Error
        spy.errorKind = "cdn_rejected"
        spy.errorMessage = "GET …/videoplayback failed: HTTP 403"

        appState.updatePlaybackProgress()

        XCTAssertTrue(appState.urlError!.contains("网络"),
                      "cdn rejection is a network-side problem: \(appState.urlError!)")
        XCTAssertFalse(appState.urlError!.contains("重新粘贴"),
                       "re-pasting cannot fix a CDN rejection: \(appState.urlError!)")
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
        XCTAssertFalse(spy.hasAnyCall, "no player calls at all")
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
        XCTAssertFalse(spy.hasAnyCall)
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
        XCTAssertEqual(spy.calls, ["stop", "playURL:https://example.com/new.mp3"], "#51 stop first")
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
        // #79：importFile 对不支持格式走 Err（返回 -1），故此处落"全部导入失败"
        // 而非"未找到支持的音频文件"——锁定现状，返回值契约修复后本用例改断言。
        let txt = tempDir.appendingPathComponent("notes.txt")
        try! Data("not audio".utf8).write(to: txt)

        appState.importURLs([txt])

        XCTAssertTrue(waitUntil { appState.showImportAlert })
        XCTAssertEqual(appState.tracks.count, 0)
        let expected = L10n.isChinese
            ? "全部导入失败，请检查文件是否支持"
            : "All imports failed. Check that the files are supported."
        XCTAssertEqual(appState.importAlertMessage, expected)
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
        // #79：rust-core 对不支持格式走 Err 路径（ffi 返回 -1），"不支持的音频
        // 格式"（==0 分支）当前为不可达死代码——此处锁定现状：坏文件落失败文案。
        let txt = tempDir.appendingPathComponent("notes.txt")
        try! Data("not audio".utf8).write(to: txt)

        appState.importFile(txt)

        XCTAssertEqual(appState.tracks.count, 0)
        let expected = L10n.isChinese
            ? "导入失败，文件可能已损坏或无法读取"
            : "Import failed. The file may be corrupted or unreadable."
        XCTAssertEqual(appState.importAlertMessage, expected)
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
