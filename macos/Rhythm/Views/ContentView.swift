import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        NavigationSplitView {
            SidebarView()
                .navigationSplitViewColumnWidth(min: 180, ideal: 220, max: 300)
        } detail: {
            VStack(spacing: 0) {
                switch appState.selectedView {
                case .library:
                    LibraryView()
                case .playlists:
                    PlaylistListView()
                }
                PlayerBarView()
            }
        }
        .onAppear { appState.openDatabase() }
        .toolbar {
            ToolbarItemGroup {
                if appState.selectedView == .library {
                    Button(action: importFolder) {
                        Image(systemName: "plus")
                    }
                    .help("导入文件夹")
                }
                TextField("搜索...", text: $appState.searchQuery)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 160)
                    .onSubmit { appState.search(appState.searchQuery) }
                    .onChange(of: appState.searchQuery) { _, q in
                        if q.isEmpty { appState.search(q) }
                    }
            }
        }
        .onReceive(
            Timer.publish(every: 0.5, on: .main, in: .common).autoconnect()
        ) { _ in
            if appState.isPlaying {
                appState.position = appState.player.position
                appState.duration = appState.player.duration
            }
        }
    }

    private func importFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "导入"
        if panel.runModal() == .OK, let url = panel.url {
            appState.importDirectory(url)
        }
    }
}
