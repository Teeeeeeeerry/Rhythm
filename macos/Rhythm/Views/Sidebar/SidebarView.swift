import SwiftUI

struct SidebarView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        List(selection: $appState.selectedView) {
            ForEach(SidebarItem.allCases) { item in
                Label(item.rawValue, systemImage: item.icon)
                    .tag(item)
            }
        }
        .listStyle(.sidebar)
        .safeAreaInset(edge: .bottom) {
            VStack {
                Divider()
                HStack {
                    Text(appState.playlists.count > 0
                         ? "\(appState.playlists.count) 个播放列表"
                         : "")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    Spacer()
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 4)
            }
        }
    }
}
