import SwiftUI

struct LibraryView: View {
    @EnvironmentObject var appState: AppState
    @State private var viewMode: LibraryViewMode = .artistAlbum

    enum LibraryViewMode: String, CaseIterable {
        case artistAlbum = "按艺人/专辑"
        case alphabetical = "按首字母"
    }

    var body: some View {
        VStack(spacing: 0) {
            Picker("视图", selection: $viewMode) {
                ForEach(LibraryViewMode.allCases, id: \.self) { mode in
                    Text(mode.rawValue).tag(mode)
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
                .foregroundStyle(.secondary)
            Text("资料库为空")
                .font(.title3)
                .foregroundStyle(.secondary)
            Text("点击工具栏 + 按钮导入音乐文件夹")
                .font(.caption)
                .foregroundStyle(.tertiary)
            Button("导入文件夹") {
                importFolder()
            }
            .padding(.top, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func importFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.prompt = "导入"
        if panel.runModal() == .OK, let url = panel.url {
            appState.importDirectory(url)
        }
    }
}
