import SwiftUI
import UniformTypeIdentifiers
import RhythmCore
#if SWIFT_PACKAGE
import RhythmTheme
#endif

struct PlaylistDetailView: View {
    let playlist: Playlist
    let onBack: () -> Void
    @EnvironmentObject var appState: AppState
    @State private var showImportSheet = false
    @State private var showExportError = false
    @State private var exportErrorMessage = ""

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()

            if playlist.tracks.isEmpty {
                emptyList
            } else {
                List {
                    ForEach(playlist.tracks) { track in
                        TrackRowView(track: track)
                            .opacity(track.isAvailable ? 1 : 0.35)
                            .contextMenu {
                                Button(L10n.removeFromPlaylist) {
                                    if let pid = playlist.id {
                                        appState.library?.removeFromPlaylist(playlistId: pid, trackId: track.id)
                                        appState.refreshLibrary()
                                    }
                                }
                            }
                    }
                }
                .listStyle(.inset)
            }
        }
        .fileImporter(
            isPresented: $showImportSheet,
            allowedContentTypes: [.plainText],
            allowsMultipleSelection: false
        ) { result in
            if case .success(let urls) = result, let url = urls.first {
                importM3U8(url)
            }
        }
        .alert(L10n.exportFailedTitle, isPresented: $showExportError) {
            Button(L10n.ok, role: .cancel) { }
        } message: {
            Text(exportErrorMessage)
        }
    }

    private var header: some View {
        HStack {
            Button(action: onBack) {
                Image(systemName: "chevron.left")
                Text(L10n.back)
            }
            .buttonStyle(.plain)
            Spacer()
            Text(playlist.name).font(.headline)
            Spacer()
            HStack(spacing: 8) {
                Button(L10n.importM3U8) { showImportSheet = true }
                Button(L10n.exportM3U8) { exportM3U8() }
            }
            .controlSize(.small)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var emptyList: some View {
        VStack {
            Image(systemName: "music.note")
                .font(.system(size: 32))
                .foregroundStyle(.rhythmTextSecondary)
            Text(L10n.playlistEmpty)
                .foregroundStyle(.rhythmTextSecondary)
            Text(L10n.playlistEmptyHint)
                .font(.caption)
                .foregroundStyle(.rhythmTextTertiary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func exportM3U8() {
        let panel = NSSavePanel()
        if let m3u8Type = UTType(filenameExtension: "m3u8") {
            panel.allowedContentTypes = [m3u8Type]
        } else {
            panel.allowedContentTypes = [.plainText]
        }
        panel.nameFieldStringValue = "\(playlist.name).m3u8"
        if panel.runModal() == .OK, let url = panel.url {
            let json = encodeJSON(playlist.tracks)
            let result = rhythm_export_m3u8(url.path, json)
            if result != 0 {
                exportErrorMessage = L10n.exportFailed(Int(result))
                showExportError = true
            }
        }
    }

    private func importM3U8(_ url: URL) {
        guard let json = rhythm_import_m3u8(url.path) else { return }
        defer { rhythm_free_string(json) }
        let s = String(cString: json)
        // #136: the core only parses the file — persist every entry here,
        // otherwise the import is a silent no-op.
        if let entries: [M3u8Entry] = decodeJSON(s) {
            appState.importM3U8Entries(entries)
        }
    }
}
