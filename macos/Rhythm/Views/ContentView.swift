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
                    .help(L10n.importTooltip)
                }
                TextField(L10n.searchPlaceholder, text: $appState.searchQuery)
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
            appState.updatePlaybackProgress()
        }
        .alert(L10n.isChinese ? "导入结果" : "Import Result", isPresented: $appState.showImportAlert) {
            Button(L10n.ok, role: .cancel) {}
        } message: {
            Text(appState.importAlertMessage ?? "")
        }
    }

    private func importFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = true
        panel.allowedContentTypes = [.audio]
        panel.prompt = L10n.isChinese ? "导入" : "Import"
        if panel.runModal() == .OK {
            appState.importURLs(panel.urls)
        }
    }
}
