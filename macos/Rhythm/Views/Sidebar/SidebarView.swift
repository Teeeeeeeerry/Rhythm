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
                    // 因此手动管理选中态并绘制品牌选中底色（配色 token rhythmSelection，#514）
                    .background(
                        RoundedRectangle(cornerRadius: 5)
                            .fill(appState.selectedView == item
                                ? AnyShapeStyle(.rhythmSelection)
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
