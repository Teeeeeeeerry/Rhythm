import SwiftUI

final class AppState: ObservableObject {
    @Published var library: RhythmLibrary?
    /// Test seam: injectable coordinator. Defaults to the real FFI-backed one
    /// (owns the engine, queue, current track, and play mode — parent issue
    /// #165).
    var coordinator: CoordinatorProtocol = RhythmCoordinator()
    @Published var selectedView: SidebarItem = .library

    /// Subscribe to coordinator events (ticket #172): progress, state,
    /// finished, and playback-failure events replace the progress polling.
    /// Events arrive on the main queue.
    init() {
        coordinator.onEvent = { [weak self] event in
            self?.handleCoordinatorEvent(event)
        }
    }
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
        coordinator.setLibrary(library)
        refreshLibrary()
    }

    func refreshLibrary() {
        guard let lib = library else { return }
        tracks = lib.allTracks()
        playlists = lib.allPlaylists()

        // #69: keep the play queue in sync so newly imported tracks are
        // reachable via "next" and deleted tracks are removed. The sync now
        // happens inside the coordinator (#170).
        coordinator.syncQueue(tracks: tracks)
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
            coordinator.stop()
            isPlaying = false
            currentTrack = nil
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
    ///
    /// The #78 guard now lives in the coordinator: a track with no playable
    /// location comes back as a classified failure and nothing changes —
    /// the UI never claims playback with nothing audible.
    func playTrack(_ track: Track, queueTracks: [Track]) {
        let outcome = coordinator.start(track: track, queueTracks: queueTracks, mode: playMode, library: library)
        guard outcome.ok else { return }
        currentTrack = track
        isPlaying = true
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

    /// Import an M3U8 playlist through the core entry point (#235).
    ///
    /// The core parses the file and stores every entry — location type,
    /// title fallback and the success test all live there (#217). This layer
    /// only picks the alert text, reloads the list from the database (#66)
    /// and returns the counts.
    @discardableResult
    func importM3U8(_ url: URL) -> M3u8ImportOutcome? {
        guard let outcome = library?.importM3U8(url.path) else { return nil }
        refreshLibrary()
        if outcome.failed > 0 {
            importAlertMessage = L10n.importSomeFailed(outcome.imported, outcome.failed)
        } else if outcome.imported > 0 {
            importAlertMessage = L10n.importedTracks(outcome.imported)
        }
        if outcome.imported > 0 || outcome.failed > 0 {
            showImportAlert = true
        }
        return outcome
    }

    /// Play a resolved URL track: persist it to the library so it survives
    /// restarts, then refresh the in-memory list from DB and rebuild the queue
    /// so "next" continues from the full library (#39, #66).
    func playResolved(_ track: Track) {
        // Persist to database first — the returned track has the real id.
        let saved = library?.addTrack(track) ?? track
        refreshLibrary() // #66: reload from DB instead of manual insert for data consistency
        // #78: the no-playable-location guard applies here too — the
        // coordinator rejects a resolved track without a URL.
        let outcome = coordinator.start(track: saved, queueTracks: tracks, mode: playMode, library: library)
        guard outcome.ok else { return }
        currentTrack = saved
        isPlaying = true
        urlInput = ""
    }

    // MARK: - Transport Availability
    //
    // The tray menu validates against these (#24). They mirror exactly what
    // the transport methods below will actually do, so a menu item is never
    // enabled for an action that would silently no-op. The queue-side checks
    // come from the coordinator (#170).

    /// Transport availability comes from the coordinator (ticket #171) —
    /// the UI renders it, it does not compute it.
    var canTogglePlayback: Bool {
        coordinator.canTogglePlayback
    }

    var canPlayNext: Bool {
        coordinator.hasNext
    }

    var canPlayPrevious: Bool {
        coordinator.hasPrevious
    }

    var canStop: Bool {
        coordinator.canStop
    }

    /// Toggle between play and pause. The full semantics live in the
    /// coordinator (ticket #171): pause while playing/buffering (#111),
    /// resume only when the engine is actually Paused (#111), idle-start the
    /// first playable library track (#78).
    func togglePlayPause() {
        let outcome = coordinator.togglePlayPause(library: library)
        guard outcome.ok else { return }
        if let track = outcome.currentTrack {
            currentTrack = track
        }
        isPlaying = outcome.playbackActive
        if !isPlaying {
            // Nothing polls while paused, so this would otherwise stay stuck
            // on whatever it was when the user hit pause.
            isBuffering = false
        }
    }

    /// Play the next track in the queue. The bounded skip of unplayable
    /// tracks happens inside the coordinator (#78, #170).
    func playNext() {
        applyTransportOutcome(coordinator.playNext(library: library))
    }

    /// Play the previous track in the queue (bounded skip — #78).
    func playPrevious() {
        applyTransportOutcome(coordinator.playPrevious(library: library))
    }

    /// Apply a transport result: on success the coordinator reports the new
    /// current track; when nothing moved (no queue / exhausted / all
    /// unplayable) the current track keeps playing untouched.
    private func applyTransportOutcome(_ outcome: CoordinatorStartResult) {
        guard outcome.ok else { return }
        if let track = outcome.currentTrack {
            currentTrack = track
            isPlaying = true
            isBuffering = false
        }
    }

    /// Stop playback entirely: stop the engine, reset transport state, clear
    /// the current track and queue.  The player bar reverts to its idle state
    /// and the tray / app menu can gate on `canStop`.
    func stop() {
        coordinator.stop()
        isPlaying = false
        isBuffering = false
        currentTrack = nil
        position = 0
        duration = 0
    }

    /// Cycle to the next play mode.
    func cyclePlayMode() {
        playMode = playMode.next()
        coordinator.setPlayMode(playMode)
    }

    /// Handle a coordinator event (ticket #172). Replaces the old progress
    /// polling: position/duration/state come from events, auto-advance is
    /// core-driven (a `trackChanged` follows `finished` when the queue has a
    /// next track), and playback failures surface with their #120
    /// classification.
    /// Internal for tests: the event handler is the seam the
    /// AppState tests drive.
    func handleCoordinatorEvent(_ event: CoordinatorEvent) {
        switch event {
        case .progress(let position, let duration):
            self.position = position
            self.duration = duration
        case .state(let state):
            isBuffering = state == "buffering"
            isPlaying = state == "playing" || state == "buffering"
        case .finished:
            // The coordinator already auto-advanced if possible (a
            // trackChanged event follows); when the queue is exhausted,
            // stop claiming playback.
            isPlaying = false
            isBuffering = false
        case .error(let kind, let message):
            isPlaying = false
            isBuffering = false
            NSLog("Playback failed: %@", message)
            urlError = L10n.playbackFailed(kind: kind, detail: message)
        case .trackChanged(let track):
            currentTrack = track
            isPlaying = true
            isBuffering = false
        }
    }

    /// Seek to a position in the current track (seconds).
    /// Position updates only when the engine accepts the seek; a rejected
    /// (e.g. out-of-range) seek keeps the previous position (#147).
    func seek(to seconds: Double) {
        if coordinator.seek(seconds) {
            position = seconds
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
