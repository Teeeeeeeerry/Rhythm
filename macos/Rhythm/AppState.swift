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
    @Published var urlInput = ""
    @Published var isResolvingURL = false
    @Published var urlError: String?
    /// What the resolver is doing right now — empty unless it's something the
    /// user should know about, like a first-run yt-dlp download.
    @Published var urlStatus = ""

    private var queue: RhythmQueue?
    private var resolverStatusTimer: Timer?

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

    /// Watch the core's provisioning status while a resolution runs. The
    /// first URL a fresh install plays downloads yt-dlp (~40 MB), and that
    /// should read as progress rather than as a stall.
    private func startResolverStatusPolling() {
        urlStatus = ""
        resolverStatusTimer?.invalidate()
        resolverStatusTimer = Timer.scheduledTimer(withTimeInterval: 0.3, repeats: true) {
            [weak self] _ in
            guard let self else { return }
            guard let status = resolverStatus(), !status.isQuiet else {
                self.urlStatus = ""
                return
            }
            self.urlStatus = L10n.resolverStatusText(
                phase: status.phase,
                received: status.received,
                total: status.total
            )
        }
    }

    private func stopResolverStatusPolling() {
        resolverStatusTimer?.invalidate()
        resolverStatusTimer = nil
        urlStatus = ""
    }

    /// Resolve a pasted URL (YouTube/Bilibili/direct audio) and start
    /// playing it. Resolution may take a few seconds (yt-dlp), so it runs on
    /// a background queue; the result is applied on the main thread.
    func resolveAndPlay(_ input: String) {
        let url = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !url.isEmpty, !isResolvingURL else { return }
        isResolvingURL = true
        urlError = nil
        startResolverStatusPolling()
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let result = resolveURL(url)
            DispatchQueue.main.async {
                guard let self else { return }
                self.isResolvingURL = false
                self.stopResolverStatusPolling()
                let resolved: ResolvedInfo
                switch result {
                case .success(let info):
                    resolved = info
                case .failure(let error):
                    // Show what actually went wrong instead of a generic
                    // failure — the core distinguishes a missing yt-dlp from
                    // a timeout, a private video, and so on (#21).
                    NSLog("URL resolution failed [%@]: %@", error.kind, error.message)
                    self.urlError = L10n.urlResolveError(kind: error.kind, detail: error.message)
                    return
                }
                let track = Track(
                    id: -1,
                    filePath: nil,
                    sourceType: resolved.sourceType ?? "direct_url",
                    // Keep the page URL, not the resolved CDN link: the core
                    // re-resolves (from cache) at playback time, and those
                    // CDN links carry a deadline that expires.
                    sourceUrl: url,
                    title: resolved.title.isEmpty ? url : resolved.title,
                    artist: resolved.artist,
                    album: nil,
                    albumArtist: nil,
                    trackNumber: nil,
                    discNumber: nil,
                    genre: nil,
                    year: nil,
                    duration: resolved.duration,
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
                self.playResolved(track)
            }
        }
    }

    /// Play a resolved URL track: prepend it to the library view list and
    /// rebuild the queue so "next" continues from the previously played list.
    private func playResolved(_ track: Track) {
        tracks.insert(track, at: 0)
        currentTrack = track
        if let url = track.sourceUrl {
            player.playURL(url)
        }
        isPlaying = true
        urlInput = ""
        if let q = RhythmQueue(tracks: tracks) {
            q.setMode(playMode)
            queue = q
        }
    }

    // MARK: - Transport Availability
    //
    // The tray menu validates against these (#24). They mirror exactly what
    // the transport methods below will actually do, so a menu item is never
    // enabled for an action that would silently no-op.

    /// `togglePlayPause` needs either something playing or something to start.
    var canTogglePlayback: Bool {
        currentTrack != nil || !tracks.isEmpty
    }

    var canPlayNext: Bool {
        queue?.hasNext ?? false
    }

    var canPlayPrevious: Bool {
        queue?.hasPrevious ?? false
    }

    /// Toggle between play and pause.
    func togglePlayPause() {
        if isPlaying {
            player.pause()
            isPlaying = false
        } else {
            if currentTrack != nil {
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
        } else if state == 4 { // Error
            // Otherwise a failed stream just sits at 0:00 with no explanation.
            isPlaying = false
            let detail = player.errorMessage ?? ""
            NSLog("Playback failed: %@", detail)
            urlError = L10n.playbackFailed(detail: detail)
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
