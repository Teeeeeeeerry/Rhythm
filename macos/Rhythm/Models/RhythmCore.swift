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

    func recordPlay(_ trackId: Int64) {
        guard let ptr else { return }
        _ = rhythm_library_record_play(ptr, trackId)
    }
}

// MARK: - Player Wrapper

final class RhythmPlayer {
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
        let tracksJSON = (try? JSONEncoder().encode(tracks)).flatMap { String(data: $0, encoding: .utf8) } ?? "[]"
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
        let json = (try? JSONEncoder().encode(tracks)).flatMap { String(data: $0, encoding: .utf8) } ?? "[]"
        rhythm_queue_replace(ptr, json)
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

// MARK: - Helpers

func decodeJSON<T: Decodable>(_ string: String) -> T? {
    guard let data = string.data(using: .utf8) else { return nil }
    return try? JSONDecoder().decode(T.self, from: data)
}
