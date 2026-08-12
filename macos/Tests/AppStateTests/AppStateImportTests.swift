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
final class AppStateImportTests: XCTestCase {
    var appState: AppState!
    var tempDir: URL!
    var dbPath: String!

    override func setUp() {
        super.setUp()
        appState = AppState()
        tempDir = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("RhythmTests-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(
            at: tempDir, withIntermediateDirectories: true)
        dbPath = tempDir.appendingPathComponent("test.db").path
        appState.library = RhythmLibrary(path: dbPath)
    }

    override func tearDown() {
        appState.player.stop()
        appState = nil
        if let dir = tempDir {
            try? FileManager.default.removeItem(at: dir)
        }
        super.tearDown()
    }

    // MARK: - Helpers

    func makeTrack(id: Int64 = -1) -> Track {
        Track(
            id: id,
            filePath: nil,
            sourceType: "direct_url",
            sourceUrl: "https://example.com/test.mp3",
            title: "Test Track",
            artist: "Test Artist",
            album: nil,
            albumArtist: nil,
            trackNumber: nil,
            discNumber: nil,
            genre: nil,
            year: nil,
            duration: 180.0,
            format: nil,
            bitrate: nil,
            sampleRate: nil,
            channels: nil,
            fileSize: nil,
            dateAdded: nil,
            lastPlayed: nil,
            playCount: 0,
            artworkPath: nil,
            isAvailable: true
        )
    }

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
}
