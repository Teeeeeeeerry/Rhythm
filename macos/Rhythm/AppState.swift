import SwiftUI

final class AppState: ObservableObject {
    @Published var library: RhythmLibrary?
    @Published var player = RhythmPlayer()
    @Published var selectedView: SidebarItem = .library
    @Published var searchQuery = ""
    @Published var tracks: [Track] = []
    @Published var playlists: [Playlist] = []
    @Published var currentTrack: Track?
    @Published var isPlaying = false
    @Published var volume: Double = 1.0
    @Published var position: Double = 0
    @Published var duration: Double = 0

    var dbURL: URL {
        let appSupport = FileManager.default.urls(
            for: .applicationSupportDirectory, in: .userDomainMask
        ).first!
        let dir = appSupport.appendingPathComponent("Rhythm")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("library.db")
    }

    func openDatabase() {
        library = RhythmLibrary(path: dbURL.path)
        refreshLibrary()
    }

    func refreshLibrary() {
        guard let lib = library else { return }
        tracks = lib.allTracks()
        playlists = lib.allPlaylists()
    }

    func importDirectory(_ url: URL) {
        guard let lib = library else { return }
        let count = lib.importDirectory(url.path)
        if count > 0 { refreshLibrary() }
    }

    func search(_ query: String) {
        guard let lib = library else { return }
        tracks = query.isEmpty ? lib.allTracks() : lib.search(query)
    }

    func playTrack(_ track: Track) {
        currentTrack = track
        switch track.sourceType {
        case "local":
            if let path = track.filePath { player.playFile(path) }
        default:
            if let url = track.sourceUrl { player.playURL(url) }
        }
        isPlaying = true
        library?.recordPlay(track.id)
    }
}

enum SidebarItem: String, CaseIterable, Identifiable {
    case library = "资料库"
    case playlists = "播放列表"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .library: "music.note.list"
        case .playlists: "list.bullet"
        }
    }
}
