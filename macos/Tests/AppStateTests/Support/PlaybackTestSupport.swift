import Foundation
import XCTest
@testable import Rhythm

// MARK: - SpyCoordinator

/// Records every call `AppState` makes against the coordinator protocol, so
/// tests can assert what the UI asked the core to do without touching the
/// audio engine.
///
/// The spy mirrors the coordinator's contract with a small sequential queue
/// model (skip unplayable tracks, jump back to the current id on sync, the
/// no-playable-location guard). The real rules live and are tested in
/// rust-core (`coordinator_behavior.rs`); this model exists so the AppState
/// tests stay behavioral instead of hand-wiring every answer.
final class SpyCoordinator: CoordinatorProtocol {
    /// Human-readable call log, mirroring the old SpyPlayer format so tests
    /// read the same way ("stop", "pause", "resume", "seek:30", …).
    private(set) var calls: [String] = []
    private(set) var startCalls: [(track: Track, queueTracks: [Track], mode: PlayMode)] = []
    private(set) var syncQueueCalls: [[Track]] = []
    private(set) var setPlayModeCalls: [PlayMode] = []
    private(set) var seekCalls: [Double] = []
    private(set) var stopCount = 0
    /// When false, `seek` reports rejection (engine-side out-of-range seek).
    var seekSucceeds = true

    // Engine mirror (what the UI polls between events).
    var position: Double = 0
    var duration: Double = 0
    var state: Int32 = 0
    var errorMessage: String?
    var errorKind: String?
    var volume: Float = 1

    // Mini queue model: sequential cursor over the last started queue.
    private var queueTracks: [Track] = []
    private var cursor = 0
    private var playMode: PlayMode = .sequential

    var hasAnyCall: Bool { !calls.isEmpty }

    func reset() {
        calls = []
        startCalls = []
        syncQueueCalls = []
        setPlayModeCalls = []
        seekCalls = []
        stopCount = 0
    }

    @discardableResult
    func start(track: Track, queueTracks: [Track], mode: PlayMode, library: RhythmLibrary?) -> CoordinatorStartResult {
        calls.append("start:\(track.title)")
        startCalls.append((track, queueTracks, mode))
        // Mirror of the core's no-playable-location guard (#78).
        guard playable(track) else {
            return CoordinatorStartResult(
                ok: false,
                errorKind: "no_playable_location",
                errorMessage: "track has no playable location",
                currentTrack: nil
            )
        }
        self.queueTracks = queueTracks
        self.playMode = mode
        cursor = queueTracks.firstIndex { $0.id == track.id } ?? 0
        self.currentTrack = track
        return CoordinatorStartResult(ok: true, errorKind: nil, errorMessage: nil, currentTrack: track)
    }

    @discardableResult
    func playNext(library: RhythmLibrary?) -> CoordinatorStartResult {
        calls.append("next")
        return advance(backwards: false)
    }

    @discardableResult
    func playPrevious(library: RhythmLibrary?) -> CoordinatorStartResult {
        calls.append("previous")
        return advance(backwards: true)
    }

    func syncQueue(tracks: [Track]) {
        calls.append("syncQueue")
        syncQueueCalls.append(tracks)
        // Mirror of the coordinator: replace, then jump back to the current
        // track by id (the coordinator's own current, not the UI's).
        queueTracks = tracks
        if let current = currentTrack, current.id >= 0,
           let pos = tracks.firstIndex(where: { $0.id == current.id }) {
            cursor = pos
        } else {
            cursor = 0
        }
    }

    func pause() {
        calls.append("pause")
        state = 2 // mirror the engine: Paused
    }

    func resume() {
        calls.append("resume")
        state = 1 // mirror the engine: Playing
    }

    func stop() {
        calls.append("stop")
        stopCount += 1
        currentTrack = nil
        queueTracks = []
        cursor = 0
    }

    func setVolume(_ v: Float) {
        calls.append("setVolume:\(v)")
    }

    func setPlayMode(_ mode: PlayMode) {
        calls.append("setPlayMode:\(mode)")
        setPlayModeCalls.append(mode)
        playMode = mode
    }

    func seek(_ seconds: Double) -> Bool {
        calls.append("seek:\(seconds)")
        seekCalls.append(seconds)
        return seekSucceeds
    }

    var currentTrack: Track?
    var hasNext: Bool {
        guard currentTrack != nil, !queueTracks.isEmpty else { return false }
        switch playMode {
        case .singleLoop: return true
        case .sequential: return cursor + 1 < queueTracks.count
        case .shuffle, .listLoop: return true
        }
    }

    var hasPrevious: Bool {
        guard currentTrack != nil, !queueTracks.isEmpty else { return false }
        return playMode == .sequential ? cursor > 0 : true
    }

    // ── Model helpers ─────────────────────────────────────────

    /// The player-reachable location check, mirroring the coordinator.
    private func playable(_ track: Track) -> Bool {
        if track.sourceType == "local" {
            return !(track.filePath ?? "").isEmpty
        }
        return !(track.sourceUrl ?? "").isEmpty
    }

    /// Walk the queue from the cursor (bounded by the queue length), skipping
    /// unplayable tracks (#78). Returns the unchanged current when nothing
    /// playable is found.
    private func advance(backwards: Bool) -> CoordinatorStartResult {
        guard let current = currentTrack, !queueTracks.isEmpty else {
            return CoordinatorStartResult(ok: true, errorKind: nil, errorMessage: nil, currentTrack: currentTrack)
        }
        if playMode == .singleLoop {
            // The queue repeats the current track.
            return CoordinatorStartResult(ok: true, errorKind: nil, errorMessage: nil, currentTrack: current)
        }
        let bound = queueTracks.count
        for _ in 0..<bound {
            let nextIndex = backwards ? cursor - 1 : cursor + 1
            guard nextIndex >= 0, nextIndex < queueTracks.count else { break }
            cursor = nextIndex
            let candidate = queueTracks[nextIndex]
            if playable(candidate) {
                currentTrack = candidate
                return CoordinatorStartResult(ok: true, errorKind: nil, errorMessage: nil, currentTrack: candidate)
            }
        }
        return CoordinatorStartResult(ok: true, errorKind: nil, errorMessage: nil, currentTrack: currentTrack)
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

/// Shared fixture: temp directory, temp SQLite library, and a SpyCoordinator
/// injected into `AppState`.
class AppStatePlaybackTestCase: XCTestCase {
    var appState: AppState!
    var spy: SpyCoordinator!
    var tempDir: URL!
    var dbURL: URL!

    override func setUp() {
        super.setUp()
        appState = AppState()
        spy = SpyCoordinator()
        appState.coordinator = spy
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
