import Foundation
import XCTest
@testable import Rhythm

// MARK: - SpyPlayer

/// Records every call `AppState` makes against the player protocol, so tests
/// can assert the exact sequence (e.g. `stop` before `playFile` — #51)
/// without touching the audio engine.
final class SpyPlayer: RhythmPlayerProtocol {
    private(set) var calls: [String] = []
    private(set) var playFileCalls: [String] = []
    private(set) var playURLCalls: [String] = []
    private(set) var seekCalls: [Double] = []
    private(set) var stopCount = 0

    var position: Double = 0
    var duration: Double = 0
    var state: Int32 = 0
    var errorMessage: String?

    var hasAnyCall: Bool { !calls.isEmpty }

    func reset() {
        calls = []
        playFileCalls = []
        playURLCalls = []
        seekCalls = []
        stopCount = 0
    }

    func playFile(_ path: String) {
        calls.append("playFile:\(path)")
        playFileCalls.append(path)
    }

    func playURL(_ url: String) {
        calls.append("playURL:\(url)")
        playURLCalls.append(url)
    }

    func pause() {
        calls.append("pause")
    }

    func resume() {
        calls.append("resume")
    }

    func stop() {
        calls.append("stop")
        stopCount += 1
    }

    func setVolume(_ v: Float) {
        calls.append("setVolume:\(v)")
    }

    func seek(_ seconds: Double) {
        calls.append("seek:\(seconds)")
        seekCalls.append(seconds)
    }
}

// MARK: - Track fixtures

func makeTrack(
    id: Int64 = -1,
    title: String = "Test Track",
    sourceType: String = "direct_url",
    filePath: String? = nil,
    sourceUrl: String? = "https://example.com/test.mp3",
    duration: Double = 180.0
) -> Track {
    Track(
        id: id,
        filePath: filePath,
        sourceType: sourceType,
        sourceUrl: sourceUrl,
        title: title,
        artist: "Test Artist",
        album: nil,
        albumArtist: nil,
        trackNumber: nil,
        discNumber: nil,
        genre: nil,
        year: nil,
        duration: duration,
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

func makeLocalTrack(
    id: Int64 = -1,
    title: String = "Local Track",
    path: String
) -> Track {
    makeTrack(id: id, title: title, sourceType: "local", filePath: path, sourceUrl: nil)
}

// MARK: - WAV fixture

/// A minimal valid stereo 16-bit WAV file (44100 Hz, 1 s of a 440 Hz sine
/// wave) — the same fixture shape the rust-core tests use. `importFile` /
/// `importDirectory` run a real metadata scan, so the file must decode.
func makeWAVData(seconds: Double = 1.0) -> Data {
    let sampleRate: UInt32 = 44100
    let channels: UInt16 = 2
    let bits: UInt16 = 16
    let frames = Int(sampleRate * UInt32(seconds))
    let dataLen = frames * Int(channels) * 2

    var wav = Data()
    wav.append(contentsOf: "RIFF".utf8)
    wav.append(contentsOf: withUnsafeBytes(of: UInt32(36 + dataLen).littleEndian) { Data($0) })
    wav.append(contentsOf: "WAVE".utf8)
    wav.append(contentsOf: "fmt ".utf8)
    wav.append(contentsOf: withUnsafeBytes(of: UInt32(16).littleEndian) { Data($0) })
    wav.append(contentsOf: withUnsafeBytes(of: UInt16(1).littleEndian) { Data($0) }) // PCM
    wav.append(contentsOf: withUnsafeBytes(of: channels.littleEndian) { Data($0) })
    wav.append(contentsOf: withUnsafeBytes(of: sampleRate.littleEndian) { Data($0) })
    wav.append(contentsOf: withUnsafeBytes(of: (sampleRate * UInt32(channels) * 2).littleEndian) { Data($0) })
    wav.append(contentsOf: withUnsafeBytes(of: (channels * bits / 8).littleEndian) { Data($0) })
    wav.append(contentsOf: withUnsafeBytes(of: bits.littleEndian) { Data($0) })
    wav.append(contentsOf: "data".utf8)
    wav.append(contentsOf: withUnsafeBytes(of: UInt32(dataLen).littleEndian) { Data($0) })
    for i in 0..<frames {
        let v = sin(440.0 * 2.0 * Double.pi * Double(i) / Double(sampleRate))
        let sample = Int16(v * 32767.0)
        wav.append(contentsOf: withUnsafeBytes(of: sample.littleEndian) { Data($0) })
        wav.append(contentsOf: withUnsafeBytes(of: Int16(0).littleEndian) { Data($0) })
    }
    return wav
}

// MARK: - Async helpers

/// Poll the main run loop until `condition` holds (or the timeout expires).
/// `resolveAndImport` / `importURLs` finish their work on the main queue, and
/// spinning the run loop lets those blocks execute without a completion
/// callback on the production API.
@discardableResult
func waitUntil(timeout: TimeInterval = 5, _ condition: () -> Bool) -> Bool {
    let deadline = Date().addingTimeInterval(timeout)
    while !condition() && Date() < deadline {
        RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.02))
    }
    return condition()
}

/// A resolver stub that blocks on a semaphore until `release()` is called —
/// for asserting re-entrancy guards and polling lifecycle while a resolution
/// is in flight.
final class BlockingResolver {
    private let semaphore = DispatchSemaphore(value: 0)
    private(set) var callCount = 0
    var result: Result<ResolvedInfo, ResolveError> = .success(
        ResolvedInfo(
            title: "Resolved",
            artist: nil,
            streamUrl: "https://cdn.example.com/x.mp3",
            duration: 60,
            sourceType: "direct_url",
            thumbnailUrl: nil
        )
    )

    func resolve(_ url: String) -> Result<ResolvedInfo, ResolveError> {
        callCount += 1
        semaphore.wait()
        return result
    }

    func release() {
        semaphore.signal()
    }

    /// If a test fails before `release()`, the blocked resolver thread would
    /// otherwise wait forever; signal on deinit so the process can exit.
    deinit {
        semaphore.signal()
    }
}

// MARK: - Test base

/// Shared fixture: temp directory, temp SQLite library, and a SpyPlayer
/// injected into `AppState`.
class AppStatePlaybackTestCase: XCTestCase {
    var appState: AppState!
    var spy: SpyPlayer!
    var tempDir: URL!
    var dbURL: URL!

    override func setUp() {
        super.setUp()
        appState = AppState()
        spy = SpyPlayer()
        appState.player = spy
        tempDir = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("RhythmTests-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(
            at: tempDir, withIntermediateDirectories: true)
        dbURL = tempDir.appendingPathComponent("test.db")
        appState.library = RhythmLibrary(path: dbURL.path)
    }

    override func tearDown() {
        appState = nil
        if let dir = tempDir {
            try? FileManager.default.removeItem(at: dir)
        }
        super.tearDown()
    }

    /// Persist a track and return the saved copy (real database id).
    @discardableResult
    func addTrackToLibrary(_ track: Track) -> Track {
        let saved = appState.library!.addTrack(track)!
        appState.refreshLibrary()
        return saved
    }

    /// Write a WAV fixture into the temp directory.
    func writeWAV(named name: String = "tone.wav") -> URL {
        let url = tempDir.appendingPathComponent(name)
        try! makeWAVData().write(to: url)
        return url
    }
}
