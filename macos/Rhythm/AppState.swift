import SwiftUI

final class AppState: ObservableObject {
    @Published var library: RhythmLibrary?
    @Published var player: RhythmPlayerProtocol = RhythmPlayer()
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
    /// Length of the queue backing `queue` — the skip loop in
    /// playNext/playPrevious is bounded by this (#78).
    private var queueTrackCount = 0
    private var resolverStatusTimer: Timer?

    /// Test seam: injectable resolver. Defaults to the global function that
    /// shells out to the Rust core (real yt-dlp chain).
    var resolver: (String) -> Result<ResolvedInfo, ResolveError> = { resolveURL($0) }

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
        openDatabase(at: dbURL)
    }

    /// Test seam: open a library at an arbitrary path (tests use a temp DB
    /// instead of touching the real application-support library).
    func openDatabase(at url: URL) {
        library = RhythmLibrary(path: url.path)
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
            importAlertMessage = L10n.importDirEmpty
            showImportAlert = true
        } else {
            importAlertMessage = L10n.importDirFailed
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
            importAlertMessage = L10n.importFileUnsupported
            showImportAlert = true
        } else {
            importAlertMessage = L10n.importFileFailed
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
                    self.importAlertMessage = L10n.importSomeFailed(imported, failed)
                } else if failed > 0 {
                    self.importAlertMessage = L10n.importAllFailed
                } else {
                    self.importAlertMessage = L10n.importNoneFound
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
        // #78: a track with no playable location must not enter the playing
        // state — the old code set currentTrack/isPlaying first and silently
        // skipped the player, so the UI claimed playback with nothing audible.
        guard let location = playableLocation(track) else { return }
        startPlayback(track, location)

        // Set up play queue
        if let q = RhythmQueue(tracks: queueTracks) {
            q.setMode(playMode)
            _ = q.jumpTo(track.id)
            queue = q
            queueTrackCount = queueTracks.count
        }
    }

    /// Stop the old engine, hand the location to the player, and flip the
    /// transport state — the one place the three play paths converge.
    private func startPlayback(_ track: Track, _ location: PlayableLocation) {
        currentTrack = track
        player.stop() // #51: stop old playback before starting new
        switch location {
        case .file(let path): player.playFile(path)
        case .url(let url): player.playURL(url)
        }
        isPlaying = true
        library?.recordPlay(track.id)
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

    /// Test seam: whether the status-polling timer is currently scheduled.
    /// Read-only mirror of the timer's lifecycle (AS-30).
    var isPollingResolverStatus: Bool {
        resolverStatusTimer != nil
    }

    /// Resolve a pasted URL (YouTube/Bilibili/direct audio) and import the
    /// track into the library without interrupting playback. Resolution may
    /// take a few seconds (yt-dlp), so it runs on a background queue; the
    /// result is applied on the main thread.
    func resolveAndImport(_ input: String) {
        let url = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !url.isEmpty, !isResolvingURL else { return }
        isResolvingURL = true
        urlError = nil
        startResolverStatusPolling()
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }
            let result = self.resolver(url)
            DispatchQueue.main.async {
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
                self.importResolved(track)
            }
        }
    }

    /// Import a resolved URL track into the library without starting
    /// playback — same behaviour as local file import (#71).
    func importResolved(_ track: Track) {
        // Persist to database first — the returned track has the real id.
        _ = library?.addTrack(track)
        refreshLibrary() // #66: reload from DB instead of manual insert for data consistency
        urlInput = ""
        importAlertMessage = L10n.importedTracks(1)
        showImportAlert = true
    }

    /// Persist M3U8 entries decoded by `playlist::import_m3u8` — the core
    /// only parses, the caller must add the tracks (#136). A location that
    /// looks like an http(s) URL is stored as a direct_url; anything else is
    /// a local file path. Returns the imported/failed counts.
    @discardableResult
    func importM3U8Entries(_ entries: [[String?]]) -> (imported: Int, failed: Int) {
        var imported = 0
        var failed = 0
        for entry in entries {
            let title = entry.first.flatMap { $0 } ?? "Unknown"
            let artist = entry.count > 1 ? entry[1] : nil
            guard let location = entry.count > 2 ? entry[2] : nil, !location.isEmpty else {
                failed += 1
                continue
            }
            let isURL = location.hasPrefix("http://") || location.hasPrefix("https://")
            let track = Track(
                id: -1,
                filePath: isURL ? nil : location,
                sourceType: isURL ? "direct_url" : "local",
                sourceUrl: isURL ? location : nil,
                title: title,
                artist: artist,
                album: nil,
                albumArtist: nil,
                trackNumber: nil,
                discNumber: nil,
                genre: nil,
                year: nil,
                duration: 0,
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
            if library?.addTrack(track) != nil {
                imported += 1
            } else {
                failed += 1
            }
        }
        refreshLibrary()
        if failed > 0 {
            importAlertMessage = L10n.importSomeFailed(imported, failed)
        } else if imported > 0 {
            importAlertMessage = L10n.importedTracks(imported)
        }
        if imported > 0 || failed > 0 {
            showImportAlert = true
        }
        return (imported, failed)
    }

    /// Play a resolved URL track: persist it to the library so it survives
    /// restarts, then refresh the in-memory list from DB and rebuild the queue
    /// so "next" continues from the full library (#39, #66).
    func playResolved(_ track: Track) {
        // Persist to database first — the returned track has the real id.
        let saved = library?.addTrack(track) ?? track
        refreshLibrary() // #66: reload from DB instead of manual insert for data consistency
        // #78: the same no-playable-location guard as playTrack — a resolved
        // track without a URL must not fake-play.
        guard let location = playableLocation(saved) else { return }
        startPlayback(saved, location)
        urlInput = ""
        if let q = RhythmQueue(tracks: tracks) {
            q.setMode(playMode)
            // Position the queue at the newly saved track by its real DB id.
            if saved.id >= 0 { _ = q.jumpTo(saved.id) }
            queue = q
            queueTrackCount = tracks.count
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
                // #111: resume only when the engine is actually paused. In
                // any other state (Error / Stopped / Buffering) resume is a
                // no-op, so claiming playback would desync the UI from the
                // engine.
                if player.state == 2 {
                    player.resume()
                    isPlaying = true
                }
            } else if let first = tracks.first(where: { playableLocation($0) != nil }) {
                // Skip over library tracks with no playable location rather
                // than dead-ending on tracks.first (#78).
                playTrack(first)
            }
        }
    }

    /// Play the next track in the queue.
    func playNext() {
        guard let q = queue else { return }
        // #78: skip past tracks with no playable location instead of
        // fake-playing them. Bounded by the queue length so an all-dead
        // queue cannot loop forever.
        for _ in 0..<queueTrackCount {
            guard let nextTrack = q.next() else { return }
            guard let location = playableLocation(nextTrack) else { continue }
            startPlayback(nextTrack, location)
            return
        }
        // Every remaining track is unplayable — give up without touching
        // state; the current track keeps playing.
    }

    /// Play the previous track in the queue.
    func playPrevious() {
        guard let q = queue else { return }
        // #78: same skip semantics as playNext, walking backwards.
        for _ in 0..<queueTrackCount {
            guard let prevTrack = q.previous() else { return }
            guard let location = playableLocation(prevTrack) else { continue }
            startPlayback(prevTrack, location)
            return
        }
    }

    /// The player-reachable location of a track: local tracks need a file
    /// path, streamed tracks a URL. Nil when nothing can be handed to the
    /// player (#78). Empty strings count as missing — they pass the old
    /// nil-only check and reach the player as a doomed play call.
    private enum PlayableLocation {
        case file(String)
        case url(String)
    }

    private func playableLocation(_ track: Track) -> PlayableLocation? {
        switch track.sourceType {
        case "local":
            guard let path = track.filePath, !path.isEmpty else { return nil }
            return .file(path)
        default:
            guard let url = track.sourceUrl, !url.isEmpty else { return nil }
            return .url(url)
        }
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
        queueTrackCount = 0
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
                let before = currentTrack?.id
                playNext()
                if currentTrack?.id == before {
                    // The next track was unplayable (or the queue is all
                    // dead): stop claiming playback instead of retrying
                    // every tick (#78).
                    isPlaying = false
                }
            } else {
                isPlaying = false
            }
        } else if state == 4 { // Error
            // Otherwise a failed stream just sits at 0:00 with no explanation.
            isPlaying = false
            let detail = player.errorMessage ?? ""
            let kind = player.errorKind
            NSLog("Playback failed: %@", detail)
            urlError = L10n.playbackFailed(kind: kind, detail: detail)
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
