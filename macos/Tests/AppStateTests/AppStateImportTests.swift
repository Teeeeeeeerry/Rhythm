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

    // MARK: - Seam 4: importM3U8 result rendering (#235)

    /// The core entry point stores the entries; this layer only renders the
    /// counts and reloads the list from the database. The storage rules
    /// themselves are asserted in rust-core (PL-17 to PL-24).
    func testImportM3U8_RendersCountsAndReloadsLibrary() {
        let playlist = writePlaylist([
            ("180,Local Artist - Local Song", "/music/local.mp3"),
            ("0,Remote Artist - Remote Song", "https://example.com/remote.mp3"),
        ])

        let outcome = appState.importM3U8(playlist)

        XCTAssertEqual(outcome, M3u8ImportOutcome(imported: 2, failed: 0))
        XCTAssertEqual(appState.tracks.count, 2, "the list must be reloaded from the database")
        XCTAssertTrue(appState.showImportAlert, "import must surface feedback")
        XCTAssertEqual(appState.importAlertMessage, L10n.importedTracks(2))
    }

    /// An unreadable playlist reports nothing — no alert, no list change.
    func testImportM3U8_UnreadableFileShowsNoAlert() {
        let missing = FileManager.default.temporaryDirectory
            .appendingPathComponent("missing-\(UUID().uuidString).m3u8")

        let outcome = appState.importM3U8(missing)

        XCTAssertNil(outcome)
        XCTAssertFalse(appState.showImportAlert)
        XCTAssertTrue(appState.tracks.isEmpty)
    }

    /// Write a playlist file whose entries are `(extinf, location)` pairs.
    private func writePlaylist(_ entries: [(String, String)]) -> URL {
        var body = "#EXTM3U\n"
        for (extinf, location) in entries {
            body += "#EXTINF:\(extinf)\n\(location)\n"
        }
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("playlist-\(UUID().uuidString).m3u8")
        try? body.write(to: url, atomically: true, encoding: .utf8)
        return url
    }
}
