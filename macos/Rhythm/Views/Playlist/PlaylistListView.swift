import SwiftUI

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
                                Text("\(pl.tracks.count) 首")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        .contentShape(Rectangle())
                        .onTapGesture { selectedPlaylist = pl }
                        .contextMenu {
                            Button("删除") {
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
                Text("新建播放列表")
                    .font(.headline)
                TextField("名称", text: $newPlaylistName)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 200)
                HStack {
                    Button("取消") { showNewPlaylist = false }
                    Button("创建") {
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
        }
    }

    var emptyState: some View {
        VStack(spacing: 12) {
            Image(systemName: "list.bullet")
                .font(.system(size: 40))
                .foregroundStyle(.secondary)
            Text("暂无播放列表")
                .font(.title3)
                .foregroundStyle(.secondary)
            Button("新建播放列表") { showNewPlaylist = true }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
