import Foundation
import RhythmCore

// MARK: - Library Wrapper

final class RhythmLibrary {
    private var ptr: OpaquePointer?

    /// The FFI handle, for calls that take a library parameter (e.g. the
    /// coordinator's `start`, which records plays in the database).
    var handle: OpaquePointer? { ptr }

    init?(path: String) {
        guard let ptr = rhythm_library_open(path) else { return nil }
        self.ptr = ptr
    }

    deinit {
        if let ptr { rhythm_library_close(ptr) }
    }

    /// Import every audio file under a directory, reporting the core's
    /// named outcome (#240). Nil only when the library handle is gone.
    func importDirectory(_ path: String) -> ImportOutcome? {
        guard let ptr, let json = rhythm_library_import_directory(ptr, path) else { return nil }
        defer { rhythm_free_string(json) }
        return GeneratedCodec.decodeImportOutcome(String(cString: json))
    }

    /// Import a single audio file, reporting the core's named outcome (#240).
    func importFile(_ path: String) -> ImportOutcome? {
        guard let ptr, let json = rhythm_library_import_single_file(ptr, path) else { return nil }
        defer { rhythm_free_string(json) }
        return GeneratedCodec.decodeImportOutcome(String(cString: json))
    }

    /// Import a mixed batch of directories and files. The "partial success"
    /// aggregation happens in the core (#240) — this layer never sums counts.
    func importPaths(_ paths: [String]) -> ImportOutcome? {
        guard let ptr,
              let payload = try? JSONSerialization.data(withJSONObject: paths),
              let json = rhythm_library_import_paths(ptr, String(decoding: payload, as: UTF8.self))
        else { return nil }
        defer { rhythm_free_string(json) }
        return GeneratedCodec.decodeImportOutcome(String(cString: json))
    }

    /// Parse an M3U8 playlist and import every entry — parsing, location
    /// mapping, title fallback and the success test all live in the core
    /// (#235). Returns nil when the playlist cannot be read.
    func importM3U8(_ path: String) -> M3u8ImportOutcome? {
        guard let ptr, let json = rhythm_import_m3u8_into_library(ptr, path) else { return nil }
        defer { rhythm_free_string(json) }
        return GeneratedCodec.decodeM3u8ImportOutcome(String(cString: json))
    }

    func allTracks() -> [Track] {
        guard let ptr, let json = rhythm_library_get_all_tracks(ptr) else { return [] }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json)) ?? []
    }

    func search(_ query: String) -> [Track] {
        guard let ptr, let json = rhythm_library_search(ptr, query) else { return [] }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json)) ?? []
    }

    func verifyFiles() -> [Int64] {
        guard let ptr, let json = rhythm_library_verify_files(ptr) else { return [] }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json)) ?? []
    }

    func allPlaylists() -> [Playlist] {
        guard let ptr, let json = rhythm_library_get_playlists(ptr) else { return [] }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json)) ?? []
    }

    func createPlaylist(name: String, description: String? = nil) -> Int64 {
        guard let ptr else { return -1 }
        return rhythm_library_create_playlist(ptr, name, description ?? "")
    }

    func addToPlaylist(playlistId: Int64, trackId: Int64) {
        guard let ptr else { return }
        _ = rhythm_library_playlist_add(ptr, playlistId, trackId)
    }

    func removeFromPlaylist(playlistId: Int64, trackId: Int64) {
        guard let ptr else { return }
        _ = rhythm_library_playlist_remove(ptr, playlistId, trackId)
    }

    func deletePlaylist(_ id: Int64) {
        guard let ptr else { return }
        _ = rhythm_library_delete_playlist(ptr, id)
    }

    /// Delete a track from the library. Returns true if a row was actually
    /// deleted, false when the id didn't match any track.
    /// Add a track to the library. Returns the saved track with its
    /// database id, or nil on failure. For URL tracks this is the
    /// persistence step that keeps them across restarts (#39).
    func addTrack(_ track: Track) -> Track? {
        guard let ptr, let json = rhythm_library_add_track(ptr, encodeJSON(track)) else { return nil }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
    }

    func removeTrack(_ id: Int64) -> Bool {
        guard let ptr else { return false }
        return rhythm_library_remove_track(ptr, id) == 0
    }

    func recordPlay(_ trackId: Int64) {
        guard let ptr else { return }
        _ = rhythm_library_record_play(ptr, trackId)
    }
}

// MARK: - Coordinator Events

/// Events pushed by the coordinator (ticket #172): the UI subscribes instead
/// of polling the engine. Mirror of the core's `CoordinatorEvent` JSON.
enum CoordinatorEvent {
    /// The current track ended naturally. The coordinator auto-advances when
    /// the queue has a next track (a `trackChanged` event follows); when the
    /// queue is exhausted playback just ends.
    case finished
    /// Playback failed; `kind` is the #120 classification ("expired" /
    /// "cdn_rejected" / "other") when the failure was HTTP.
    case error(kind: String?, message: String)
    /// Playback progress (seconds).
    case progress(position: Double, duration: Double)
    /// Engine state transition (named: stopped/playing/paused/buffering/finished).
    case state(state: String)
    /// The current track changed (start / transport move / auto-advance).
    case trackChanged(track: Track)

    init?(json: String) {
        guard let payload: CoordinatorEventPayload = decodeJSON(json) else { return nil }
        switch payload.type {
        case "finished":
            self = .finished
        case "error":
            self = .error(kind: payload.kind, message: payload.message ?? "")
        case "progress":
            self = .progress(position: payload.position ?? 0, duration: payload.duration ?? 0)
        case "state":
            self = .state(state: payload.state ?? "")
        case "track_changed":
            guard let track = payload.track else { return nil }
            self = .trackChanged(track: track)
        default:
            return nil
        }
    }
}

/// Decoding shape of the core's event JSON (snake_case keys via decodeJSON).
private struct CoordinatorEventPayload: Codable {
    let type: String
    let kind: String?
    let message: String?
    let position: Double?
    let duration: Double?
    let state: String?
    let track: Track?
}

/// C trampoline: the core calls this on the playback thread; we hop to the
/// main queue and deliver to the coordinator's `onEvent` handler.
private let coordinatorEventCallback: @convention(c) (UnsafeMutableRawPointer?, UnsafeMutablePointer<CChar>?) -> Void = { context, json in
    guard let context, let json else { return }
    defer { rhythm_free_string(json) }
    let coordinator = Unmanaged<RhythmCoordinator>.fromOpaque(context).takeUnretainedValue()
    let text = String(cString: json)
    guard let event = CoordinatorEvent(json: text) else { return }
    DispatchQueue.main.async {
        coordinator.onEvent?(event)
    }
}

// MARK: - Coordinator Wrapper

/// Structured result of a coordinator call (mirror of the core's
/// `CoordinatorResult` JSON): success payload + classified error in one
/// return. `errorKind` is one of: no_playable_location, playback_failed,
/// invalid_input.
struct CoordinatorStartResult: Codable {
    let ok: Bool
    let errorKind: String?
    let errorMessage: String?
    let currentTrack: Track?
    /// Whether playback is active (engine Playing/Buffering) after the
    /// operation — what the UI should render for `isPlaying`.
    let playbackActive: Bool
}

/// The playback surface `AppState` orchestrates against (parent issue #165).
///
/// The coordinator owns the orchestration rules — stop old playback (#51),
/// dispatch by source type, record plays, queue build + positioning, bounded
/// skip of unplayable tracks (#78) — so the UI layer is a thin adapter that
/// renders the coordinator's state.
///
/// Test seam: tests inject a spy implementation to assert the exact calls
/// without touching the audio engine.
protocol CoordinatorProtocol {
    /// Event subscription (ticket #172): invoked on the main queue.
    var onEvent: ((CoordinatorEvent) -> Void)? { get set }
    /// Register the library handle for play recording (auto-advance).
    func setLibrary(_ library: RhythmLibrary?)

    /// Start playback of `track` with `queueTracks` as the queue. The
    /// no-playable-location guard (#78) lives in the core: a track without a
    /// location returns a classified failure and nothing changes.
    @discardableResult
    func start(track: Track, queueTracks: [Track], mode: PlayMode, library: RhythmLibrary?) -> CoordinatorStartResult
    /// Advance to the next playable track (bounded skip of unplayable ones).
    @discardableResult
    func playNext(library: RhythmLibrary?) -> CoordinatorStartResult
    /// Move to the previous playable track (bounded skip of unplayable ones).
    @discardableResult
    func playPrevious(library: RhythmLibrary?) -> CoordinatorStartResult
    /// Toggle play/pause with the full transport semantics (pause while
    /// playing/buffering, resume only when paused, idle-start the first
    /// playable library track). The result's `playbackActive` tells the UI
    /// what to render.
    @discardableResult
    func togglePlayPause(library: RhythmLibrary?) -> CoordinatorStartResult
    /// Sync the queue after a library refresh (#69): replace + jump to the
    /// current track.
    func syncQueue(tracks: [Track])
    func pause()
    func resume()
    func stop()
    func setVolume(_ v: Float)
    func setPlayMode(_ mode: PlayMode)
    var volume: Float { get }
    var hasNext: Bool { get }
    var hasPrevious: Bool { get }
    var canTogglePlayback: Bool { get }
    var canStop: Bool { get }
    var currentTrack: Track? { get }
    var position: Double { get }
    var duration: Double { get }
    var state: Int32 { get }
    var errorMessage: String? { get }
    /// Classification of the last playback failure when it was HTTP:
    /// "expired" | "cdn_rejected" | "other"; nil otherwise (#120).
    var errorKind: String? { get }
    /// Seek to a position; returns false when the engine rejected the seek
    /// (e.g. out of range), so callers can roll back optimistic UI state (#147).
    @discardableResult
    func seek(_ seconds: Double) -> Bool
}

final class RhythmCoordinator: CoordinatorProtocol {
    private var ptr: OpaquePointer?
    /// Event subscription (ticket #172); invoked on the main queue.
    var onEvent: ((CoordinatorEvent) -> Void)?

    init() {
        ptr = rhythm_coordinator_create()
        installEventHandler()
    }

    deinit {
        if let ptr { rhythm_coordinator_destroy(ptr) }
    }

    /// Register the library handle for play recording (auto-advance).
    func setLibrary(_ library: RhythmLibrary?) {
        guard let ptr else { return }
        rhythm_coordinator_set_library(ptr, library?.handle)
    }

    /// Wire the C event callback to `onEvent`.
    private func installEventHandler() {
        guard let ptr else { return }
        let selfPtr = Unmanaged.passUnretained(self).toOpaque()
        rhythm_coordinator_set_event_callback(ptr, coordinatorEventCallback, selfPtr)
    }

    @discardableResult
    func start(track: Track, queueTracks: [Track], mode: PlayMode, library: RhythmLibrary?) -> CoordinatorStartResult {
        guard let ptr else {
            return CoordinatorStartResult(ok: false, errorKind: "invalid_input", errorMessage: "null coordinator handle", currentTrack: nil, playbackActive: false)
        }
        guard let json = rhythm_coordinator_start(
            ptr,
            library?.handle,
            encodeJSON(track),
            encodeJSON(queueTracks),
            mode.rawValue
        ) else {
            return CoordinatorStartResult(ok: false, errorKind: "internal", errorMessage: "Malformed coordinator response", currentTrack: nil, playbackActive: false)
        }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
            ?? CoordinatorStartResult(ok: false, errorKind: "internal", errorMessage: "Malformed coordinator response", currentTrack: nil, playbackActive: false)
    }

    @discardableResult
    func playNext(library: RhythmLibrary?) -> CoordinatorStartResult {
        guard let ptr, let json = rhythm_coordinator_next(ptr, library?.handle) else {
            return CoordinatorStartResult(ok: false, errorKind: "internal", errorMessage: "Malformed coordinator response", currentTrack: nil, playbackActive: false)
        }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
            ?? CoordinatorStartResult(ok: false, errorKind: "internal", errorMessage: "Malformed coordinator response", currentTrack: nil, playbackActive: false)
    }

    @discardableResult
    func playPrevious(library: RhythmLibrary?) -> CoordinatorStartResult {
        guard let ptr, let json = rhythm_coordinator_previous(ptr, library?.handle) else {
            return CoordinatorStartResult(ok: false, errorKind: "internal", errorMessage: "Malformed coordinator response", currentTrack: nil, playbackActive: false)
        }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
            ?? CoordinatorStartResult(ok: false, errorKind: "internal", errorMessage: "Malformed coordinator response", currentTrack: nil, playbackActive: false)
    }

    @discardableResult
    func togglePlayPause(library: RhythmLibrary?) -> CoordinatorStartResult {
        guard let ptr, let json = rhythm_coordinator_toggle_play_pause(ptr, library?.handle) else {
            return CoordinatorStartResult(ok: false, errorKind: "internal", errorMessage: "Malformed coordinator response", currentTrack: nil, playbackActive: false)
        }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
            ?? CoordinatorStartResult(ok: false, errorKind: "internal", errorMessage: "Malformed coordinator response", currentTrack: nil, playbackActive: false)
    }

    func syncQueue(tracks: [Track]) {
        guard let ptr else { return }
        rhythm_coordinator_sync_queue(ptr, encodeJSON(tracks))
    }

    func pause() {
        guard let ptr else { return }
        rhythm_coordinator_pause(ptr)
    }

    func resume() {
        guard let ptr else { return }
        rhythm_coordinator_resume(ptr)
    }

    func stop() {
        guard let ptr else { return }
        rhythm_coordinator_stop(ptr)
    }

    func setVolume(_ v: Float) {
        guard let ptr else { return }
        rhythm_coordinator_set_volume(ptr, v)
    }

    func setPlayMode(_ mode: PlayMode) {
        guard let ptr else { return }
        rhythm_coordinator_set_play_mode(ptr, mode.rawValue)
    }

    var volume: Float {
        guard let ptr else { return 0 }
        return rhythm_coordinator_get_volume(ptr)
    }

    var hasNext: Bool {
        guard let ptr else { return false }
        return rhythm_coordinator_has_next(ptr) != 0
    }

    var hasPrevious: Bool {
        guard let ptr else { return false }
        return rhythm_coordinator_has_previous(ptr) != 0
    }

    var canTogglePlayback: Bool {
        guard let ptr else { return false }
        return rhythm_coordinator_can_toggle_playback(ptr) != 0
    }

    var canStop: Bool {
        guard let ptr else { return false }
        return rhythm_coordinator_can_stop(ptr) != 0
    }

    var currentTrack: Track? {
        guard let ptr, let json = rhythm_coordinator_current_track(ptr) else { return nil }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
    }

    var position: Double {
        guard let ptr else { return 0 }
        return rhythm_coordinator_get_position(ptr)
    }

    var duration: Double {
        guard let ptr else { return 0 }
        return rhythm_coordinator_get_duration(ptr)
    }

    var state: Int32 {
        guard let ptr else { return -1 }
        return rhythm_coordinator_get_state(ptr)
    }

    var errorMessage: String? {
        guard let ptr, let raw = rhythm_coordinator_error(ptr) else { return nil }
        defer { rhythm_free_string(raw) }
        return String(cString: raw)
    }

    var errorKind: String? {
        guard let ptr, let raw = rhythm_coordinator_error_kind(ptr) else { return nil }
        defer { rhythm_free_string(raw) }
        return String(cString: raw)
    }

    func seek(_ seconds: Double) -> Bool {
        guard let ptr else { return false }
        return rhythm_coordinator_seek(ptr, seconds) == 0
    }
}

// MARK: - Playlist Model

struct Playlist: Identifiable, Codable {
    let id: Int64?
    let name: String
    let description: String?
    let dateCreated: String?
    let dateModified: String?
    let tracks: [Track]
}

// MARK: - Play Queue Wrapper

/// Playback mode — the FFI contract values (0-3) are locked by
/// SW-03b/CO tests; the canonical declaration lives in rust-core
/// `queue::PlayMode` (#179).
enum PlayMode: Int32, CaseIterable {
    case sequential = 0
    case shuffle = 1
    case singleLoop = 2
    case listLoop = 3

    var label: String {
        switch self {
        case .sequential: L10n.modeSequential
        case .shuffle: L10n.modeShuffle
        case .singleLoop: L10n.modeSingleLoop
        case .listLoop: L10n.modeListLoop
        }
    }

    var icon: String {
        switch self {
        case .sequential: "arrow.right"
        case .shuffle: "shuffle"
        case .singleLoop: "repeat.1"
        case .listLoop: "repeat"
        }
    }

    func next() -> PlayMode {
        PlayMode(rawValue: (self.rawValue + 1) % Int32(Self.allCases.count)) ?? .sequential
    }
}

// MARK: - M3U8 Entry

/// One imported M3U8 entry — named fields across the seam (title, artist,
/// location), so callers never index by position (#177).
struct M3u8Entry: Codable {
    let title: String
    let artist: String?
    let location: String
}

/// Named outcome of an M3U8 import (#234): how many entries the core stored
/// and how many it could not. The counts are the whole contract — the UI
/// never re-derives "did this entry make it" (#217).
struct M3u8ImportOutcome: Codable, Equatable {
    let imported: Int
    let failed: Int
}

/// Named outcome of a library import (#237): how many tracks were stored,
/// how many were skipped because the format is unsupported, and how many
/// failed to read. The three counts stay separate — folding "unsupported"
/// into "failed" loses the only detail the user can act on.
struct ImportOutcome: Codable, Equatable {
    let imported: Int
    let unsupported: Int
    let failed: Int
}

// MARK: - Resolver Types

struct ResolvedInfo: Codable {
    let title: String
    let artist: String?
    let streamUrl: String?
    let duration: Double
    let sourceType: String?
    let thumbnailUrl: String?
}

/// A resolution failure reported by the Rust core.
///
/// `kind` is one of: invalid_url, yt_dlp_missing, timeout, network,
/// unavailable, no_audio_stream, yt_dlp_outdated, internal. `message` is the
/// English detail, including install commands and yt-dlp's own output.
struct ResolveError: Codable, Error {
    let kind: String
    let message: String

    /// Fallback used when the core returns no error payload at all.
    static let unknown = ResolveError(kind: "internal", message: L10n.urlResolveFailed)
}

// MARK: - Helpers

/// Decode a JSON string produced by the Rust core. Rust serializes with
/// snake_case keys, so the decoder converts them to the Swift models'
/// camelCase property names (e.g. `source_type` → `sourceType`).
func decodeJSON<T: Decodable>(_ string: String) -> T? {
    guard let data = string.data(using: .utf8) else { return nil }
    let decoder = JSONDecoder()
    decoder.keyDecodingStrategy = .convertFromSnakeCase
    return try? decoder.decode(T.self, from: data)
}

/// Encode a Swift value into JSON expected by the Rust core (snake_case keys).
func encodeJSON<T: Encodable>(_ value: T) -> String {
    let encoder = JSONEncoder()
    encoder.keyEncodingStrategy = .convertToSnakeCase
    return (try? encoder.encode(value)).flatMap { String(data: $0, encoding: .utf8) } ?? "[]"
}

// MARK: - URL Resolver

/// Decoding shape of the core's structured resolve result (#176): success
/// payload + classified error in a single return — the old
/// "null, then query the global error slot" two-step protocol is gone.
private struct ResolveResultPayload: Codable {
    let ok: Bool
    let resolved: ResolvedInfo?
    let errorKind: String?
    let errorMessage: String?
}

/// Resolve a URL to a playable stream.
///
/// The core returns one structured result: on failure it carries why
/// (yt-dlp missing, timeout, private video…), which is far more useful to
/// show than a generic "resolution failed" (#21, #176).
func resolveURL(_ url: String) -> Result<ResolvedInfo, ResolveError> {
    guard let json = rhythm_resolve_url(url) else {
        return .failure(.unknown)
    }
    defer { rhythm_free_string(json) }

    guard let payload: ResolveResultPayload = decodeJSON(String(cString: json)) else {
        return .failure(ResolveError(kind: "internal", message: "Malformed resolver response"))
    }
    if payload.ok, let resolved = payload.resolved {
        return .success(resolved)
    }
    return .failure(ResolveError(
        kind: payload.errorKind ?? "internal",
        message: payload.errorMessage ?? "Failed to resolve the URL."
    ))
}

/// Resolver environment (yt-dlp path/version, PATH, log file) as raw JSON.
/// Useful when a user needs to attach it to a bug report.
func resolverDiagnostics() -> String {
    guard let json = rhythm_resolver_diagnostics() else { return "{}" }
    defer { rhythm_free_string(json) }
    return String(cString: json)
}

// ─── 核心消息规格（#227/#228）──────────────────────────────────────

/// 核心产出的一段文案：键表条目（可带占位符参数）或原样输出的字面量。
enum MessageSegment: Decodable {
    case key(String, [String: String])
    case literal(String)

    private enum CodingKeys: String, CodingKey {
        case segment, key, params, text
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        if try c.decode(String.self, forKey: .segment) == "key" {
            self = .key(
                try c.decode(String.self, forKey: .key),
                try c.decodeIfPresent([String: String].self, forKey: .params) ?? [:]
            )
        } else {
            self = .literal(try c.decode(String.self, forKey: .text))
        }
    }
}

/// 一条消息规格：按顺序拼接的段。段为空表示不显示任何文案。
struct MessageSpec: Decodable {
    let segments: [MessageSegment]
}

/// 解码消息规格（键名都是单词，不走 snake_case 转换，参数名原样保留）。
private func decodeMessageSpec(_ json: String) -> MessageSpec? {
    guard let data = json.data(using: .utf8) else { return nil }
    return try? JSONDecoder().decode(MessageSpec.self, from: data)
}

/// 播放失败的消息规格。选哪个键、中英拼成什么形状由核心决定（#227）；
/// `language` 是本层解析出的语言标识，语言解析仍是平台特异的。
func playbackFailureSpec(kind: String?, detail: String, language: String) -> MessageSpec? {
    guard let json = rhythm_message_playback_failure(kind ?? "", detail, language) else {
        return nil
    }
    defer { rhythm_free_string(json) }
    return decodeMessageSpec(String(cString: json))
}

/// 解析失败的消息规格。平台差异（yt-dlp 安装命令）由核心按构建目标
/// 选键（#229）；本层只解析出语言标识。
func resolveFailureSpec(kind: String, detail: String, language: String) -> MessageSpec? {
    guard let json = rhythm_message_resolve_failure(kind, detail, language) else { return nil }
    defer { rhythm_free_string(json) }
    return decodeMessageSpec(String(cString: json))
}

/// 解析器供给状态的消息规格。阶段分派、字节到 MB 的换算与「已收 / 总量」
/// 的格式化都在核心（#231）；静默阶段返回空段列表。
func resolverStatusSpec(phase: String, received: Int64?, total: Int64?) -> MessageSpec? {
    guard let json = rhythm_message_resolver_status(phase, received ?? 0, total ?? 0) else {
        return nil
    }
    defer { rhythm_free_string(json) }
    return decodeMessageSpec(String(cString: json))
}

/// Progress of yt-dlp provisioning, polled while a resolution is running so a
/// first-run download doesn't look like a hang.
struct ResolverStatus: Decodable {
    let phase: String
    let received: Int64?
    let total: Int64?
    let message: String?

    /// Nothing worth telling the user about.
    var isQuiet: Bool {
        phase == "idle" || phase == "ready"
    }
}

func resolverStatus() -> ResolverStatus? {
    guard let json = rhythm_resolver_status() else { return nil }
    defer { rhythm_free_string(json) }
    return decodeJSON(String(cString: json))
}
