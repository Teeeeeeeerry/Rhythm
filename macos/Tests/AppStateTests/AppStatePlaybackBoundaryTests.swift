import XCTest
import Foundation
@testable import Rhythm

/// AS-27–39：AppState 播放编排边界情况（manifest: docs/testing/behavior/appstate-macos.md）。
final class AppStatePlaybackBoundaryTests: AppStatePlaybackTestCase {

    // MARK: - AS-27 togglePlayPause 无曲目可播

    func testTogglePlayPause_EmptyLibrary_NoOp() throws {
        appState.togglePlayPause()

        XCTAssertFalse(spy.hasAnyCall, "nothing to play — the player must not be touched")
        XCTAssertFalse(appState.isPlaying)
    }

    // MARK: - AS-28 playTrack 缺路径

    /// 缺 filePath/sourceUrl 时不进入播放状态：不置 currentTrack/isPlaying，
    /// 也不碰 player（无 stop 调用）——否则出现无声假播放（#78）。
    func testPlayTrack_MissingPath_DoesNotEnterPlaying() {
        let track = makeTrack(sourceType: "local", filePath: nil, sourceUrl: nil)
        appState.tracks = [track]

        appState.playTrack(track)

        XCTAssertFalse(appState.isPlaying, "no playable path — must not claim to be playing")
        XCTAssertNil(appState.currentTrack)
        XCTAssertFalse(spy.hasAnyCall)

        // The streamed branch of the same guard.
        let urlTrack = makeTrack(sourceType: "url", filePath: nil, sourceUrl: nil)
        appState.tracks = [urlTrack]
        appState.playTrack(urlTrack)

        XCTAssertFalse(appState.isPlaying)
        XCTAssertNil(appState.currentTrack)
        XCTAssertFalse(spy.hasAnyCall)
    }

    // MARK: - AS-28b playNext/playPrevious 跳过无路径曲目

    func testPlayNext_SkipsUnplayableTrack() {
        let a = addTrackToLibrary(makeLocalTrack(title: "A", path: "/tmp/a.mp3"))
        let b = makeTrack(sourceType: "local", filePath: nil, sourceUrl: nil)
        let c = addTrackToLibrary(makeLocalTrack(title: "C", path: "/tmp/c.mp3"))
        appState.tracks = [a, b, c]
        appState.playTrack(a)
        spy.reset()

        appState.playNext()

        XCTAssertEqual(appState.currentTrack, c, "must skip the unplayable B and land on C")
        XCTAssertTrue(appState.isPlaying)
        XCTAssertEqual(spy.calls, ["stop", "playFile:/tmp/c.mp3"])
    }

    func testPlayPrevious_SkipsUnplayableTrack() {
        let a = addTrackToLibrary(makeLocalTrack(title: "A", path: "/tmp/a.mp3"))
        let b = makeTrack(sourceType: "local", filePath: nil, sourceUrl: nil)
        let c = addTrackToLibrary(makeLocalTrack(title: "C", path: "/tmp/c.mp3"))
        appState.tracks = [a, b, c]
        appState.playTrack(c)
        spy.reset()

        appState.playPrevious()

        XCTAssertEqual(appState.currentTrack, a, "must skip the unplayable B and land on A")
        XCTAssertTrue(appState.isPlaying)
        XCTAssertEqual(spy.calls, ["stop", "playFile:/tmp/a.mp3"])
    }

    func testPlayNext_AllUnplayable_GivesUpWithoutTouchingState() {
        let a = addTrackToLibrary(makeLocalTrack(title: "A", path: "/tmp/a.mp3"))
        let b = makeTrack(sourceType: "local", filePath: nil, sourceUrl: nil)
        let c = makeTrack(sourceType: "local", filePath: nil, sourceUrl: nil)
        appState.tracks = [a, b, c]
        appState.playTrack(a)
        spy.reset()

        appState.playNext()

        XCTAssertEqual(appState.currentTrack, a, "current keeps playing when the rest is dead")
        XCTAssertTrue(appState.isPlaying)
        XCTAssertFalse(spy.hasAnyCall)
    }

    func testAutoAdvance_AllUnplayable_StopsClaimingPlayback() {
        let a = addTrackToLibrary(makeLocalTrack(title: "A", path: "/tmp/a.mp3"))
        let b = makeTrack(sourceType: "local", filePath: nil, sourceUrl: nil)
        let c = makeTrack(sourceType: "local", filePath: nil, sourceUrl: nil)
        appState.tracks = [a, b, c]
        appState.playTrack(a)
        spy.state = 5 // Finished
        spy.reset()

        appState.updatePlaybackProgress()

        XCTAssertFalse(
            appState.isPlaying,
            "an all-dead queue must not retry every tick with isPlaying stuck"
        )
    }

    // MARK: - AS-29 resolveAndImport 并发防重入

    func testResolveAndImport_Reentrancy_Ignored() throws {
        let blocker = BlockingResolver()
        appState.resolver = blocker.resolve

        appState.resolveAndImport("https://example.com/first")
        XCTAssertTrue(appState.isResolvingURL)
        appState.resolveAndImport("https://example.com/second") // must be ignored
        blocker.release()

        XCTAssertTrue(waitUntil { !appState.isResolvingURL })
        XCTAssertEqual(blocker.callCount, 1, "re-entrant call must not hit the resolver")
        XCTAssertEqual(appState.tracks.count, 1)
    }

    func testResolveAndImport_BlankInput_Ignored() throws {
        let blocker = BlockingResolver()
        appState.resolver = blocker.resolve

        appState.resolveAndImport("   \n ")

        XCTAssertFalse(appState.isResolvingURL)
        XCTAssertEqual(blocker.callCount, 0)
    }

    // MARK: - AS-30 resolver 状态轮询生命周期

    func testResolverStatusPolling_Lifecycle() throws {
        let blocker = BlockingResolver()
        appState.resolver = blocker.resolve

        appState.resolveAndImport("https://example.com/video")
        XCTAssertTrue(appState.isPollingResolverStatus, "polling starts with resolution")
        XCTAssertEqual(appState.urlStatus, "")

        blocker.release()
        XCTAssertTrue(waitUntil { !appState.isResolvingURL })
        XCTAssertFalse(appState.isPollingResolverStatus, "polling stops with resolution")
        XCTAssertEqual(appState.urlStatus, "")
    }

    // MARK: - AS-31 deleteSelectedTrack 无匹配

    func testDeleteSelectedTrack_NoMatch_NoOp() throws {
        appState.selectedTrackID = 999

        appState.deleteSelectedTrack()

        XCTAssertNil(appState.trackToDelete)
        XCTAssertFalse(appState.showDeleteConfirmation)
    }

    // MARK: - AS-32 importURLs 期间再次调用

    func testImportURLs_Reentrancy_Ignored() throws {
        let a = writeWAV(named: "a.wav")
        let b = writeWAV(named: "b.wav")

        appState.importURLs([a])
        XCTAssertTrue(appState.isImporting)
        appState.importURLs([b]) // ignored: first batch still in flight

        XCTAssertTrue(waitUntil { appState.showImportAlert })
        XCTAssertEqual(appState.tracks.count, 1, "only the first batch is imported")
        XCTAssertFalse(appState.isImporting)
    }

    // MARK: - AS-33 updatePlaybackProgress 非播放状态

    func testUpdatePlaybackProgress_NotPlaying_NoOp() throws {
        spy.position = 99
        spy.duration = 200

        appState.updatePlaybackProgress()

        XCTAssertEqual(appState.position, 0, "player state must not be read while paused")
        XCTAssertEqual(appState.duration, 0)
    }

    // MARK: - AS-34 playNext/playPrevious 无队列

    func testPlayNextAndPrevious_NoQueue_NoOp() throws {
        appState.playNext()
        appState.playPrevious()

        XCTAssertFalse(spy.hasAnyCall)
        XCTAssertNil(appState.currentTrack)
    }

    // MARK: - AS-35 refreshLibrary 无当前曲目

    func testRefreshLibrary_NoCurrentTrack_LeavesQueueAlone() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1)
        appState.playNext() // cursor at the end: canPlayNext == false
        XCTAssertFalse(appState.canPlayNext)

        appState.currentTrack = nil
        appState.refreshLibrary()

        XCTAssertEqual(appState.tracks.count, 2)
        // A replace would reset the cursor to the start and re-enable next;
        // untouched, the queue stays exhausted.
        XCTAssertFalse(appState.canPlayNext, "queue must not be touched without a current track")
    }

    // MARK: - AS-36 confirmDeleteTrack 删除非当前曲目

    func testConfirmDeleteTrack_NonCurrent_PlaybackUntouched() throws {
        let t1 = addTrackToLibrary(makeLocalTrack(title: "One", path: "/tmp/one.mp3"))
        let t2 = addTrackToLibrary(makeLocalTrack(title: "Two", path: "/tmp/two.mp3"))
        appState.playTrack(t1)
        spy.reset()

        appState.requestDeleteTrack(t2)
        appState.confirmDeleteTrack()

        XCTAssertFalse(spy.hasAnyCall, "deleting another track must not stop playback")
        XCTAssertTrue(appState.isPlaying)
        XCTAssertEqual(appState.currentTrack, t1)
        XCTAssertEqual(appState.tracks.map(\.id), [t1.id])
        XCTAssertTrue(appState.canTogglePlayback)
    }

    // MARK: - AS-37 playResolved 持久化失败

    func testPlayResolved_PersistFails_StillPlaysWithUnsavedID() throws {
        appState.library = nil
        let track = makeTrack(sourceUrl: "https://example.com/x.mp3")

        appState.playResolved(track)

        XCTAssertEqual(spy.calls, ["stop", "playURL:https://example.com/x.mp3"])
        XCTAssertTrue(appState.isPlaying)
        XCTAssertEqual(appState.currentTrack?.id, -1, "saved = track fallback (id -1)")
        XCTAssertEqual(appState.tracks.count, 0)
    }

    // MARK: - AS-38 importResolved 库未打开

    func testImportResolved_LibraryNil_StillShowsAlert() throws {
        appState.library = nil

        appState.importResolved(makeTrack())

        XCTAssertTrue(appState.showImportAlert)
        XCTAssertEqual(appState.tracks.count, 0, "refresh is a no-op without a library")
        XCTAssertFalse(appState.isPlaying)
        XCTAssertFalse(spy.hasAnyCall)
    }

    // MARK: - AS-39 seek 乐观更新

    func testSeek_UpdatesPositionOptimistically() throws {
        appState.seek(to: 75)

        XCTAssertEqual(spy.seekCalls, [75])
        XCTAssertEqual(appState.position, 75, "no round trip to the core (#73)")
    }
}
