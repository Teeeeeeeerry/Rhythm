import XCTest
import Foundation
@testable import Rhythm

/// Tests for the URL import flow (issue #71):
/// URL resolution should import tracks into the library WITHOUT
/// automatically starting playback — same behavior as local file import.
///
/// Seams under test:
/// 1. importResolved — persists to library, refreshes, shows alert, does NOT play
/// 2. playResolved (regression) — persists, refreshes, AND starts playback
final class AppStateImportTests: AppStatePlaybackTestCase {

    // MARK: - Seam 1: importResolved (does NOT play)

    /// importResolved persists the track to the library and refreshes the list.
    func testImportResolved_AddsTrackToLibrary() {
        let track = makeTrack()
        appState.importResolved(track)

        // The track list should now contain our track
        XCTAssertEqual(appState.tracks.count, 1, "importResolved should add the track to the library")
        XCTAssertEqual(appState.tracks.first?.title, "Test Track")
    }

    /// importResolved shows the import alert so the user knows it succeeded.
    func testImportResolved_ShowsImportAlert() {
        let track = makeTrack()
        appState.importResolved(track)

        XCTAssertTrue(appState.showImportAlert, "importResolved should show the import alert")
        XCTAssertNotNil(appState.importAlertMessage)
    }

    /// importResolved does NOT start playback.
    func testImportResolved_DoesNotStartPlayback() {
        let track = makeTrack()
        appState.importResolved(track)

        XCTAssertFalse(appState.isPlaying, "importResolved should NOT start playback")
        XCTAssertNil(appState.currentTrack, "importResolved should NOT set the current track")
    }

    /// importResolved does NOT create a play queue (no auto-advance after import).
    func testImportResolved_DoesNotCreateQueue() {
        let track = makeTrack()
        appState.importResolved(track)

        // With no currentTrack and no queue, canPlayNext should be false
        XCTAssertFalse(appState.canPlayNext, "importResolved should not create a play queue")
    }

    // MARK: - Seam 2: playResolved regression (imports AND plays)

    /// playResolved still persists and starts playback (backward-compatible).
    func testPlayResolved_StartsPlayback() {
        let track = makeTrack()
        appState.playResolved(track)

        XCTAssertTrue(appState.isPlaying, "playResolved should start playback")
        XCTAssertNotNil(appState.currentTrack, "playResolved should set the current track")
    }

    /// playResolved persists the track to the library.
    func testPlayResolved_AddsTrackToLibrary() {
        let track = makeTrack()
        appState.playResolved(track)

        XCTAssertEqual(appState.tracks.count, 1, "playResolved should add the track to the library")
    }

    // MARK: - Seam 3: resolveAndImport (only imports, never plays)

    /// Verifies resolveAndImport exists and starts resolution (sets isResolvingURL).
    /// The async resolution path is tested implicitly through importResolved above;
    /// this test guards the public entry point against accidental removal.
    func testResolveAndImport_StartsResolution() {
        appState.resolveAndImport("https://example.com/video")

        // Resolution should have started (synchronous flag set before async work)
        XCTAssertTrue(appState.isResolvingURL,
            "resolveAndImport should set isResolvingURL synchronously")
        // Playback must NOT have started (the method should not call playResolved)
        XCTAssertFalse(appState.isPlaying,
            "resolveAndImport should never start playback")
        XCTAssertNil(appState.currentTrack,
            "resolveAndImport should not set a current track")
    }

    // MARK: - Seam 4: importM3U8Entries (#136 regression)

    /// M3U8 entries are persisted to the library, not discarded — local
    /// paths become local tracks, http(s) locations become direct_url tracks.
    func testImportM3U8Entries_PersistsTracksWithMappedSources() {
        let entries: [[String?]] = [
            ["Local Song", "Local Artist", "/music/local.mp3"],
            ["Remote Song", nil, "https://example.com/remote.mp3"],
        ]
        let result = appState.importM3U8Entries(entries)

        XCTAssertEqual(result.imported, 2)
        XCTAssertEqual(result.failed, 0)
        XCTAssertEqual(appState.tracks.count, 2, "entries must be written to the database")
        let local = appState.tracks.first { $0.sourceType == "local" }
        let remote = appState.tracks.first { $0.sourceType == "direct_url" }
        XCTAssertEqual(local?.title, "Local Song")
        XCTAssertEqual(local?.artist, "Local Artist")
        XCTAssertEqual(local?.filePath, "/music/local.mp3")
        XCTAssertEqual(remote?.title, "Remote Song")
        XCTAssertEqual(remote?.sourceUrl, "https://example.com/remote.mp3")
        XCTAssertNil(remote?.filePath)
        XCTAssertTrue(appState.showImportAlert, "import should surface feedback")
    }

    /// Entries without a usable location are counted as failures and skipped.
    func testImportM3U8Entries_CountsInvalidEntriesAsFailed() {
        let entries: [[String?]] = [
            ["Good", nil, "/music/good.mp3"],
            ["No Location", nil, nil],
            ["Empty Location", nil, ""],
        ]
        let result = appState.importM3U8Entries(entries)

        XCTAssertEqual(result.imported, 1)
        XCTAssertEqual(result.failed, 2)
        XCTAssertEqual(appState.tracks.count, 1)
    }
}
