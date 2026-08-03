import SwiftUI

struct SidebarView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        List(selection: $appState.selectedView) {
            ForEach(SidebarItem.allCases) { item in
                Label(item.label, systemImage: item.icon)
                    .tag(item)
            }
        }
        .listStyle(.sidebar)
    }
}
