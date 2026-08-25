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
        .onDeleteCommand { appState.deleteSelectedTrack() }
        .toolbar {
            ToolbarItemGroup {
                if appState.selectedView == .library {
                    if appState.isImporting {
                        ProgressView()
                            .controlSize(.small)
                            .help(L10n.importing)
                    } else {
                        Button(action: importFolder) {
                            Image(systemName: "plus")
                        }
                        .help(L10n.importTooltip)
                    }
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
        .alert(L10n.importResultTitle, isPresented: $appState.showImportAlert) {
            Button(L10n.ok, role: .cancel) {}
        } message: {
            Text(appState.importAlertMessage ?? "")
        }
        .alert(L10n.deleteConfirmTitle, isPresented: $appState.showDeleteConfirmation) {
            Button(L10n.cancel, role: .cancel) {
                appState.trackToDelete = nil
            }
            Button(L10n.deleteButton, role: .destructive) {
                appState.confirmDeleteTrack()
            }
        } message: {
            Text(appState.trackToDelete.map { L10n.deleteConfirmMessage($0.title) } ?? "")
        }
    }

    private func importFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = true
        panel.allowsMultipleSelection = true
        panel.allowedContentTypes = [.audio]
        panel.prompt = L10n.importButton
        if panel.runModal() == .OK {
            appState.importURLs(panel.urls)
        }
    }
}
