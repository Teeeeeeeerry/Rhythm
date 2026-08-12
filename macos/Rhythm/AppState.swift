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
    /// The core is resolving/connecting/prebuffering — real work, but nothing
    /// to show on the progress bar yet.
    @Published var isBuffering = false
    @Published var isResolvingURL = false
    @Published var urlError: String?
    /// What the resolver is doing right now — empty unless it's something the
    /// user should know about, like a first-run yt-dlp download.
    @Published var urlStatus = ""

    // Import feedback
    @Published var importAlertMessage: String?
    @Published var showImportAlert = false
    @Published var isImporting = false

    // Delete confirmation
    @Published var trackToDelete: Track?
    @Published var showDeleteConfirmation = false
    @Published var selectedTrackID: Int64?

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

        // #69: keep the play queue in sync so newly imported tracks are
        // reachable via "next" and deleted tracks are removed.
        if let q = queue, let current = currentTrack {
            q.replace(tracks: tracks)
            if current.id >= 0 { _ = q.jumpTo(current.id) }
        }
    }

    func importDirectory(_ url: URL) {
        guard let lib = library else { return }
        let count = lib.importDirectory(url.path)
        if count > 0 {
            refreshLibrary()
            importAlertMessage = L10n.importedTracks(count)
            showImportAlert = true
        } else if count == 0 {
            importAlertMessage = L10n.isChinese
                ? "该目录中未找到支持的音频文件"
                : "No supported audio files found in this directory."
            showImportAlert = true
        } else {
            importAlertMessage = L10n.isChinese
                ? "导入失败，请检查目录是否可访问"
                : "Import failed. Please check that the directory is accessible."
            showImportAlert = true
        }
    }

    func importFile(_ url: URL) {
        guard let lib = library else { return }
        let result = lib.importFile(url.path)
        if result > 0 {
            refreshLibrary()
            importAlertMessage = L10n.importedTracks(result)
            showImportAlert = true
        } else if result == 0 {
            importAlertMessage = L10n.isChinese
                ? "不支持的音频格式"
                : "Unsupported audio format."
            showImportAlert = true
        } else {
            importAlertMessage = L10n.isChinese
                ? "导入失败，文件可能已损坏或无法读取"
                : "Import failed. The file may be corrupted or unreadable."
            showImportAlert = true
        }
    }

    /// Dispatch a list of URLs — files go to `importFile`, directories to
    /// `importDirectory`.  Runs on a background queue so the UI stays
    /// responsive during metadata extraction and SQLite writes (#38).
    func importURLs(_ urls: [URL]) {
        guard !isImporting else { return }
        isImporting = true

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            var imported = 0
            var failed = 0
            for url in urls {
                guard let self else { return }
                let isDir = (try? url.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory ?? false
                if isDir {
                    let count = self.library?.importDirectory(url.path) ?? -1
                    if count > 0 { imported += count } else if count < 0 { failed += 1 }
                } else {
                    let r = self.library?.importFile(url.path) ?? -1
                    if r > 0 { imported += 1 } else if r < 0 { failed += 1 }
                }
            }
            DispatchQueue.main.async {
                guard let self else { return }
                self.isImporting = false
                if imported > 0 { self.refreshLibrary() }
                if imported > 0 && failed == 0 {
                    self.importAlertMessage = L10n.importedTracks(imported)
                } else if imported > 0 {
                    self.importAlertMessage = L10n.isChinese
                        ? "已导入 \(imported) 首，\(failed) 个失败"
                        : "Imported \(imported) tracks, \(failed) failed."
                } else if failed > 0 {
                    self.importAlertMessage = L10n.isChinese
                        ? "全部导入失败，请检查文件是否支持"
                        : "All imports failed. Check that the files are supported."
                } else {
                    self.importAlertMessage = L10n.isChinese
                        ? "未找到支持的音频文件"
                        : "No supported audio files found."
                }
                self.showImportAlert = true
            }
        }
    }

    /// Request deletion of a track — shows a confirmation dialog.
    func requestDeleteTrack(_ track: Track) {
        trackToDelete = track
        showDeleteConfirmation = true
    }

    /// Delete the currently selected track (triggered by Delete key).
    func deleteSelectedTrack() {
        guard let id = selectedTrackID,
              let track = tracks.first(where: { $0.id == id }) else { return }
        requestDeleteTrack(track)
    }

    /// Actually delete the track after user confirms.
    func confirmDeleteTrack() {
        guard let track = trackToDelete else { return }
        defer {
            trackToDelete = nil
        }

        // If the deleted track is currently playing, stop playback and
        // clear the queue so "next" doesn't try to play a dead track.
        if currentTrack?.id == track.id {
            player.stop()
            isPlaying = false
            currentTrack = nil
            queue = nil
        }

        _ = library?.removeTrack(track.id)
        refreshLibrary()
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
        player.stop() // #51: stop old playback before starting new
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

    /// Play a resolved URL track: persist it to the library so it survives
    /// restarts, then refresh the in-memory list from DB and rebuild the queue
    /// so "next" continues from the full library (#39, #66).
    private func playResolved(_ track: Track) {
        // Persist to database first — the returned track has the real id.
        let saved = library?.addTrack(track) ?? track
        refreshLibrary() // #66: reload from DB instead of manual insert for data consistency
        currentTrack = saved
        player.stop() // #51: stop old playback before starting new
        if let url = saved.sourceUrl {
            player.playURL(url)
        }
        isPlaying = true
        urlInput = ""
        if let q = RhythmQueue(tracks: tracks) {
            q.setMode(playMode)
            // Position the queue at the newly saved track by its real DB id.
            if saved.id >= 0 { _ = q.jumpTo(saved.id) }
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

    var canStop: Bool { isPlaying }

    /// Toggle between play and pause.
    func togglePlayPause() {
        if isPlaying {
            player.pause()
            isPlaying = false
            // Nothing polls while paused, so this would otherwise stay stuck on
            // whatever it was when the user hit pause.
            isBuffering = false
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
        player.stop() // #51: stop old playback before starting new
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
        player.stop() // #51: stop old playback before starting new
        switch prevTrack.sourceType {
        case "local":
            if let path = prevTrack.filePath { player.playFile(path) }
        default:
            if let url = prevTrack.sourceUrl { player.playURL(url) }
        }
        library?.recordPlay(prevTrack.id)
    }

    /// Stop playback entirely: stop the engine, reset transport state, clear
    /// the current track and queue.  The player bar reverts to its idle state
    /// and the tray / app menu can gate on `canStop`.
    func stop() {
        player.stop()
        isPlaying = false
        isBuffering = false
        currentTrack = nil
        queue = nil
        position = 0
        duration = 0
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
        isBuffering = state == 3
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

    /// Seek to a position in the current track (seconds).
    func seek(to seconds: Double) {
        player.seek(seconds)
        position = seconds
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
