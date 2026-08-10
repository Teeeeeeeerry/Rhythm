import SwiftUI
#if SWIFT_PACKAGE
import RhythmTheme
#endif

struct PlaylistListView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedPlaylist: Playlist?
    @State private var showNewPlaylist = false
    @State private var newPlaylistName = ""

    var body: some View {
        VStack(spacing: 0) {
            if appState.playlists.isEmpty {
                emptyState
            } else if let pl = selectedPlaylist {
                PlaylistDetailView(playlist: pl) {
                    selectedPlaylist = nil
                }
            } else {
                List {
                    ForEach(appState.playlists) { pl in
                        HStack {
                            Image(systemName: "list.bullet")
                            VStack(alignment: .leading, spacing: 2) {
                                Text(pl.name)
                                    .font(.body)
                                Text(L10n.isChinese
                                     ? "\(pl.tracks.count) 首"
                                     : "\(pl.tracks.count) tracks")
                                    .font(.caption)
                                    .foregroundStyle(.rhythmTextSecondary)
                            }
                        }
                        .contentShape(Rectangle())
                        .onTapGesture { selectedPlaylist = pl }
                        .contextMenu {
                            Button(L10n.isChinese ? "删除" : "Delete") {
                                if let id = pl.id { appState.library?.deletePlaylist(id) }
                                appState.refreshLibrary()
                            }
                        }
                    }
                }
                .listStyle(.inset)
            }
        }
        .toolbar {
            Button(action: { showNewPlaylist = true }) {
                Image(systemName: "plus")
            }
        }
        .sheet(isPresented: $showNewPlaylist) {
            VStack(spacing: 16) {
                Text(L10n.newPlaylist)
                    .font(.headline)
                TextField(L10n.playlistName, text: $newPlaylistName)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 200)
                HStack {
                    Button(L10n.cancel) { showNewPlaylist = false }
                    Button(L10n.create) {
                        if !newPlaylistName.isEmpty {
                            _ = appState.library?.createPlaylist(name: newPlaylistName)
                            appState.refreshLibrary()
                            newPlaylistName = ""
                            showNewPlaylist = false
                        }
                    }
                    .keyboardShortcut(.defaultAction)
                }
            }
            .padding()
            .frame(width: 280, height: 140)
            .background(.rhythmSurface)
        }
    }

    var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "list.bullet")
                .font(.system(size: 40))
                .foregroundStyle(.rhythmTextSecondary)
            Text(L10n.isChinese ? "暂无播放列表" : "No playlists yet")
                .font(.title3)
                .foregroundStyle(.rhythmTextSecondary)
            Button(L10n.newPlaylist) { showNewPlaylist = true }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
