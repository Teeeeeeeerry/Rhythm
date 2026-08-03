import AppKit

final class TrayManager: NSObject {
    private var statusItem: NSStatusItem!
    weak var appState: AppState?

    init(appState: AppState?) {
        self.appState = appState
        super.init()
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = statusItem.button {
            button.image = NSImage(
                systemSymbolName: "music.note",
                accessibilityDescription: "Rhythm"
            )
        }
        setupMenu()
    }

    private func setupMenu() {
        let menu = NSMenu()

        menu.addItem(NSMenuItem(
            title: L10n.trayPlayPause,
            action: #selector(togglePlay),
            keyEquivalent: " "
        ))
        menu.addItem(NSMenuItem(
            title: L10n.trayNext,
            action: #selector(nextTrack),
            keyEquivalent: ""
        ))
        menu.addItem(NSMenuItem(
            title: L10n.trayPrev,
            action: #selector(previousTrack),
            keyEquivalent: ""
        ))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(
            title: L10n.trayShow,
            action: #selector(showWindow),
            keyEquivalent: ""
        ))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(
            title: L10n.trayQuit,
            action: #selector(quitApp),
            keyEquivalent: "q"
        ))

        statusItem.menu = menu
    }

    @objc private func togglePlay() {
        appState?.togglePlayPause()
    }

    @objc private func nextTrack() {
        appState?.playNext()
    }

    @objc private func previousTrack() {
        appState?.playPrevious()
    }

    @objc private func showWindow() {
        NSApp.activate(ignoringOtherApps: true)
        NSApp.windows.first?.makeKeyAndOrderFront(nil)
    }

    @objc private func quitApp() {
        NSApp.terminate(nil)
    }
}
