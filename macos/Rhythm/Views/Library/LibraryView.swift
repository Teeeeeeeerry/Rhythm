import SwiftUI
#if SWIFT_PACKAGE
import RhythmTheme
#endif

struct LibraryView: View {
    @EnvironmentObject var appState: AppState
    @State private var viewMode: LibraryViewMode = .artistAlbum

    enum LibraryViewMode: String, CaseIterable {
        case artistAlbum
        case alphabetical

        var label: String {
            switch self {
            case .artistAlbum: L10n.byArtistAlbum
            case .alphabetical: L10n.byLetter
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            Picker(L10n.view, selection: $viewMode) {
                ForEach(LibraryViewMode.allCases, id: \.self) { mode in
                    Text(mode.label).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)

            Divider()

            if appState.tracks.isEmpty {
                emptyLibrary
            } else {
                switch viewMode {
                case .artistAlbum:
                    ArtistAlbumView()
                case .alphabetical:
                    AlphabeticalView()
                }
            }
        }
    }

    var emptyLibrary: some View {
        VStack(spacing: 16) {
            Image(systemName: "music.note.list")
                .font(.system(size: 48))
                .foregroundStyle(.rhythmTextSecondary)
            Text(L10n.libraryEmpty)
                .font(.title3)
                .foregroundStyle(.rhythmTextSecondary)
            Text(L10n.importHint)
                .font(.caption)
                .foregroundStyle(.rhythmTextTertiary)
            if appState.isImporting {
                ProgressView()
                    .controlSize(.small)
            } else {
                Button(L10n.importTooltip) {
                    importFolder()
                }
                .padding(.top, 4)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
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
