import SwiftUI
#if SWIFT_PACKAGE
import RhythmTheme
#endif

struct SidebarView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        List {
            ForEach(SidebarItem.allCases) { item in
                Label(item.label, systemImage: item.icon)
                    .padding(.vertical, 3)
                    .padding(.horizontal, 8)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                    // 选中高亮完全品牌化：系统 selection 高亮无法自定义颜色，
                    // 因此手动管理选中态并绘制品牌强调色（#0D464D / #ABC8D4）
                    .background(
                        RoundedRectangle(cornerRadius: 5)
                            .fill(appState.selectedView == item
                                ? AnyShapeStyle(.rhythmAccent.opacity(0.15))
                                : AnyShapeStyle(.clear))
                    )
                    .foregroundStyle(
                        appState.selectedView == item
                            ? AnyShapeStyle(.rhythmAccent)
                            : AnyShapeStyle(.rhythmTextPrimary)
                    )
                    .listRowBackground(Color.clear)
                    .onTapGesture { appState.selectedView = item }
            }
        }
        .listStyle(.sidebar)
        .scrollContentBackground(.hidden)
        .background(.rhythmSurface)
    }
}
