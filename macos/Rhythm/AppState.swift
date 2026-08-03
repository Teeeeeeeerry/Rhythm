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
    @Published var playMode: PlayMode = .sequential

    private var queue: RhythmQueue?

    var dbURL: URL {
        let appSupport = FileManager.default.urls(
            for: .applicationSupportDirectory, in: .userDomainMask
        ).first!
        let dir = appSupport.appendingPathComponent("Rhythm")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("library.db")
    }

    var artworkCacheURL: URL {
        let appSupport = FileManager.default.urls(
            for: .applicationSupportDirectory, in: .userDomainMask
        ).first!
        let dir = appSupport.appendingPathComponent("Rhythm/Artwork")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
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

    /// Play a track and set up the queue from the current track list.
    func playTrack(_ track: Track) {
        playTrack(track, queueTracks: tracks)
    }

    /// Play a track with a specific queue (e.g., from a playlist).
    func playTrack(_ track: Track, queueTracks: [Track]) {
        currentTrack = track
        switch track.sourceType {
        case "local":
            if let path = track.filePath { player.playFile(path) }
        default:
            if let url = track.sourceUrl { player.playURL(url) }
        }
        isPlaying = true
        library?.recordPlay(track.id)

        // Set up play queue
        if let q = RhythmQueue(tracks: queueTracks) {
            q.setMode(playMode)
            _ = q.jumpTo(track.id)
            queue = q
        }
    }

    /// Toggle between play and pause.
    func togglePlayPause() {
        if isPlaying {
            player.pause()
            isPlaying = false
        } else {
            if let track = currentTrack {
                player.resume()
                isPlaying = true
            } else if let first = tracks.first {
                playTrack(first)
            }
        }
    }

    /// Play the next track in the queue.
    func playNext() {
        guard let q = queue, let nextTrack = q.next() else { return }
        currentTrack = nextTrack
        isPlaying = true
        switch nextTrack.sourceType {
        case "local":
            if let path = nextTrack.filePath { player.playFile(path) }
        default:
            if let url = nextTrack.sourceUrl { player.playURL(url) }
        }
        library?.recordPlay(nextTrack.id)
    }

    /// Play the previous track in the queue.
    func playPrevious() {
        guard let q = queue, let prevTrack = q.previous() else { return }
        currentTrack = prevTrack
        isPlaying = true
        switch prevTrack.sourceType {
        case "local":
            if let path = prevTrack.filePath { player.playFile(path) }
        default:
            if let url = prevTrack.sourceUrl { player.playURL(url) }
        }
        library?.recordPlay(prevTrack.id)
    }

    /// Cycle to the next play mode.
    func cyclePlayMode() {
        playMode = playMode.next()
        queue?.setMode(playMode)
    }

    /// Called by the progress timer to check for track-end auto-advance.
    func updatePlaybackProgress() {
        guard isPlaying else { return }
        position = player.position
        duration = player.duration

        let state = player.state
        if state == 5 { // Finished
            if queue?.hasNext == true {
                playNext()
            } else {
                isPlaying = false
            }
        }
    }
}

enum SidebarItem: String, CaseIterable, Identifiable {
    case library
    case playlists

    var id: String { rawValue }

    var label: String {
        switch self {
        case .library: L10n.libraryTab
        case .playlists: L10n.playlistsTab
        }
    }

    var icon: String {
        switch self {
        case .library: "music.note.list"
        case .playlists: "list.bullet"
        }
    }
}
