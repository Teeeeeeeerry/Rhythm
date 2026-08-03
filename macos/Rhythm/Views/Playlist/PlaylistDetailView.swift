import SwiftUI
import RhythmCore

struct PlaylistDetailView: View {
    let playlist: Playlist
    let onBack: () -> Void
    @EnvironmentObject var appState: AppState
    @State private var showImportSheet = false

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
                                Button(L10n.isChinese ? "从列表移除" : "Remove from Playlist") {
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
            allowedContentTypes: [.plainText, .audio],
            allowsMultipleSelection: false
        ) { result in
            if case .success(let urls) = result, let url = urls.first {
                importM3U8(url)
            }
        }
    }

    private var header: some View {
        HStack {
            Button(action: onBack) {
                Image(systemName: "chevron.left")
                Text(L10n.isChinese ? "返回" : "Back")
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
                .foregroundStyle(.secondary)
            Text(L10n.isChinese ? "列表为空" : "Playlist is empty")
                .foregroundStyle(.secondary)
            Text(L10n.isChinese
                 ? "从资料库右键添加歌曲"
                 : "Right-click a track in Library to add it here")
                .font(.caption)
                .foregroundStyle(.tertiary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func exportM3U8() {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.plainText]
        panel.nameFieldStringValue = "\(playlist.name).m3u8"
        if panel.runModal() == .OK, let url = panel.url {
            let data = try? JSONEncoder().encode(playlist.tracks)
            let json = data.flatMap { String(data: $0, encoding: .utf8) } ?? "[]"
            _ = rhythm_export_m3u8(url.path, json)
        }
    }

    private func importM3U8(_ url: URL) {
        guard let json = rhythm_import_m3u8(url.path) else { return }
        defer { rhythm_free_string(json) }
        let s = String(cString: json)
        if let entries: [[String?]] = decodeJSON(s) {
            for entry in entries {
                let title = entry.first.flatMap { $0 } ?? "Unknown"
                _ = entry.count > 1 ? entry[1] : nil
                _ = entry.count > 2 ? entry[2] : nil
                print("Imported: \(title)")
            }
            appState.refreshLibrary()
        }
    }
}
