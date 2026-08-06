import SwiftUI
import UniformTypeIdentifiers
import RhythmCore

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
            allowedContentTypes: [.plainText],
            allowsMultipleSelection: false
        ) { result in
            if case .success(let urls) = result, let url = urls.first {
                importM3U8(url)
            }
        }
        .alert(L10n.isChinese ? "导出失败" : "Export Failed", isPresented: $showExportError) {
            Button("OK", role: .cancel) { }
        } message: {
            Text(exportErrorMessage)
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
                .foregroundStyle(.rhythmTextSecondary)
            Text(L10n.isChinese ? "列表为空" : "Playlist is empty")
                .foregroundStyle(.rhythmTextSecondary)
            Text(L10n.isChinese
                 ? "从资料库右键添加歌曲"
                 : "Right-click a track in Library to add it here")
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
                exportErrorMessage = L10n.isChinese
                    ? "导出失败（错误码: \(result)），请重试。"
                    : "Export failed (code: \(result)). Please try again."
                showExportError = true
            }
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
