import SwiftUI

@main
struct RhythmApp: App {
    @StateObject private var appState = AppState()
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .frame(minWidth: 800, minHeight: 500)
                .onAppear { appDelegate.appState = appState }
        }
        .windowResizability(.contentMinSize)
        .windowToolbarStyle(.unified)
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandGroup(replacing: .help) {}
            SidebarCommands()

            // Playback keyboard shortcuts
            CommandMenu(L10n.menuPlayback) {
                Button(L10n.menuPlayPause) { appState.togglePlayPause() }
                    .keyboardShortcut(.space, modifiers: [])
                Button(L10n.menuNext) { appState.playNext() }
                    .keyboardShortcut(.rightArrow, modifiers: [.command])
                Button(L10n.menuPrev) { appState.playPrevious() }
                    .keyboardShortcut(.leftArrow, modifiers: [.command])
                Divider()
                Button(L10n.menuToggleMode) { appState.cyclePlayMode() }
                    .keyboardShortcut("l", modifiers: [.command, .shift])
            }
        }
    }
}
