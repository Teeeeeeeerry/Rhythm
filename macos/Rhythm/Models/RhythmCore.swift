import Foundation
import RhythmCore

// MARK: - Library Wrapper

final class RhythmLibrary {
    private var ptr: OpaquePointer?

    init?(path: String) {
        guard let ptr = rhythm_library_open(path) else { return nil }
        self.ptr = ptr
    }

    deinit {
        if let ptr { rhythm_library_close(ptr) }
    }

    func importDirectory(_ path: String) -> Int {
        guard let ptr else { return -1 }
        return Int(rhythm_library_import(ptr, path))
    }

    func importFile(_ path: String) -> Int {
        guard let ptr else { return -1 }
        return Int(rhythm_library_import_file(ptr, path))
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

// MARK: - Player Wrapper

/// The playback surface `AppState` orchestrates against.
///
/// Test seam: tests inject a spy implementation to assert the exact call
/// sequence (e.g. `stop()` before `playFile` — #51) without touching the
/// audio engine.
protocol RhythmPlayerProtocol {
    func playFile(_ path: String)
    func playURL(_ url: String)
    func pause()
    func resume()
    func stop()
    func setVolume(_ v: Float)
    func seek(_ seconds: Double)
    var position: Double { get }
    var duration: Double { get }
    var state: Int32 { get }
    var errorMessage: String? { get }
    /// Classification of the last playback failure when it was HTTP:
    /// "expired" | "cdn_rejected" | "other"; nil otherwise (#120).
    var errorKind: String? { get }
}

final class RhythmPlayer: RhythmPlayerProtocol {
    private var ptr: OpaquePointer?

    init() {
        ptr = rhythm_player_create()
    }

    deinit {
        if let ptr { rhythm_player_destroy(ptr) }
    }

    func playFile(_ path: String) {
        guard let ptr else { return }
        _ = rhythm_player_play_file(ptr, path)
    }

    func playURL(_ url: String) {
        guard let ptr else { return }
        _ = rhythm_player_play_url(ptr, url)
    }

    func pause() {
        guard let ptr else { return }
        rhythm_player_pause(ptr)
    }

    func resume() {
        guard let ptr else { return }
        rhythm_player_resume(ptr)
    }

    func stop() {
        guard let ptr else { return }
        rhythm_player_stop(ptr)
    }

    func setVolume(_ v: Float) {
        guard let ptr else { return }
        rhythm_player_set_volume(ptr, v)
    }

    var volume: Float {
        guard let ptr else { return 0 }
        return rhythm_player_get_volume(ptr)
    }

    var state: Int32 {
        guard let ptr else { return -1 }
        return rhythm_player_get_state(ptr)
    }

    /// Why playback failed, when `state` is 4 (Error).
    var errorMessage: String? {
        guard let ptr, let raw = rhythm_player_error(ptr) else { return nil }
        defer { rhythm_free_string(raw) }
        return String(cString: raw)
    }

    /// Classification of the last playback failure when it was HTTP (#120).
    var errorKind: String? {
        guard let ptr, let raw = rhythm_player_error_kind(ptr) else { return nil }
        defer { rhythm_free_string(raw) }
        return String(cString: raw)
    }

    func seek(_ seconds: Double) {
        guard let ptr else { return }
        _ = rhythm_player_seek(ptr, seconds)
    }

    var position: Double {
        guard let ptr else { return 0 }
        return rhythm_player_get_position(ptr)
    }

    var duration: Double {
        guard let ptr else { return 0 }
        return rhythm_player_get_duration(ptr)
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

enum PlayMode: Int32 {
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
        PlayMode(rawValue: (self.rawValue + 1) % 4) ?? .sequential
    }
}

final class RhythmQueue {
    private var ptr: OpaquePointer?

    init?(tracks: [Track]) {
        let tracksJSON = encodeJSON(tracks)
        guard let ptr = rhythm_queue_create(tracksJSON) else { return nil }
        self.ptr = ptr
    }

    deinit {
        if let ptr { rhythm_queue_destroy(ptr) }
    }

    func current() -> Track? {
        guard let ptr, let json = rhythm_queue_current(ptr) else { return nil }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
    }

    func next() -> Track? {
        guard let ptr, let json = rhythm_queue_next(ptr) else { return nil }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
    }

    func previous() -> Track? {
        guard let ptr, let json = rhythm_queue_previous(ptr) else { return nil }
        defer { rhythm_free_string(json) }
        return decodeJSON(String(cString: json))
    }

    func setMode(_ mode: PlayMode) {
        guard let ptr else { return }
        rhythm_queue_set_mode(ptr, mode.rawValue)
    }

    func jumpTo(_ trackId: Int64) -> Bool {
        guard let ptr else { return false }
        return rhythm_queue_jump_to(ptr, trackId) == 0
    }

    func replace(tracks: [Track]) {
        guard let ptr else { return }
        rhythm_queue_replace(ptr, encodeJSON(tracks))
    }

    var hasNext: Bool {
        guard let ptr else { return false }
        return rhythm_queue_has_next(ptr) != 0
    }

    var hasPrevious: Bool {
        guard let ptr else { return false }
        return rhythm_queue_has_previous(ptr) != 0
    }
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

/// Resolve a URL to a playable stream.
///
/// On failure the core records why (yt-dlp missing, timeout, private video…),
/// which is far more useful to show than a generic "resolution failed" (#21).
func resolveURL(_ url: String) -> Result<ResolvedInfo, ResolveError> {
    guard let json = rhythm_resolve_url(url) else {
        return .failure(lastResolveError() ?? .unknown)
    }
    defer { rhythm_free_string(json) }

    guard let resolved: ResolvedInfo = decodeJSON(String(cString: json)) else {
        return .failure(ResolveError(kind: "internal", message: "Malformed resolver response"))
    }
    return .success(resolved)
}

/// The core's most recent resolution failure, if any.
func lastResolveError() -> ResolveError? {
    guard let json = rhythm_last_error() else { return nil }
    defer { rhythm_free_string(json) }
    return decodeJSON(String(cString: json))
}

/// Resolver environment (yt-dlp path/version, PATH, log file) as raw JSON.
/// Useful when a user needs to attach it to a bug report.
func resolverDiagnostics() -> String {
    guard let json = rhythm_resolver_diagnostics() else { return "{}" }
    defer { rhythm_free_string(json) }
    return String(cString: json)
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
