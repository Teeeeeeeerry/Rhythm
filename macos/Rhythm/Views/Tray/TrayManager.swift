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

        // Every item needs an explicit target. With `target == nil` AppKit
        // resolves the action against the responder chain, and TrayManager
        // isn't on it — so `autoenablesItems` greyed out the whole menu,
        // including Quit (#24).
        menu.addItem(item(L10n.trayPlayPause, #selector(togglePlay)))
        menu.addItem(item(L10n.trayStop, #selector(stopPlayback)))
        menu.addItem(.separator())
        menu.addItem(item(L10n.trayNext, #selector(nextTrack)))
        menu.addItem(item(L10n.trayPrev, #selector(previousTrack)))
        menu.addItem(.separator())
        menu.addItem(item(L10n.trayShow, #selector(showWindow)))
        menu.addItem(.separator())
        menu.addItem(item(L10n.trayQuit, #selector(quitApp), keyEquivalent: "q"))

        statusItem.menu = menu
    }

    /// Build a menu item wired to this object.
    ///
    /// No key equivalent by default: a status-bar menu's `keyEquivalent` only
    /// fires while the menu is open, so advertising one is misleading. The old
    /// `" "` on Play/Pause rendered as ⌘Space (items default to a `.command`
    /// modifier mask), which collides with Spotlight and didn't match the
    /// in-window bare Space binding in `RhythmApp.swift`.
    private func item(
        _ title: String,
        _ action: Selector,
        keyEquivalent: String = ""
    ) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: keyEquivalent)
        item.target = self
        return item
    }

    @objc private func togglePlay() {
        appState?.togglePlayPause()
    }

    @objc private func stopPlayback() {
        appState?.stop()
    }

    @objc private func nextTrack() {
        appState?.playNext()
    }

    @objc private func previousTrack() {
        appState?.playPrevious()
    }

    @objc private func showWindow() {
        NSApp.activate(ignoringOtherApps: true)
        mainWindow?.makeKeyAndOrderFront(nil)
    }

    @objc private func quitApp() {
        NSApp.terminate(nil)
    }

    /// `NSApp.windows` also holds the status item's own window and whatever
    /// auxiliary windows SwiftUI created, in no guaranteed order — pick the
    /// one that can actually become main.
    private var mainWindow: NSWindow? {
        NSApp.windows.first { $0.canBecomeMain }
    }
}

// MARK: - Menu Validation

extension TrayManager: NSMenuItemValidation {
    /// Drive each item from real playback state; with a target set but no
    /// validation the transport items would sit permanently enabled and do
    /// nothing when there's no queue.
    func validateMenuItem(_ menuItem: NSMenuItem) -> Bool {
        switch menuItem.action {
        case #selector(togglePlay):
            guard let appState else { return false }
            menuItem.title = appState.isPlaying ? L10n.trayPause : L10n.trayPlay
            return appState.canTogglePlayback
        case #selector(stopPlayback):
            return appState?.canStop ?? false
        case #selector(nextTrack):
            return appState?.canPlayNext ?? false
        case #selector(previousTrack):
            return appState?.canPlayPrevious ?? false
        case #selector(showWindow):
            return mainWindow != nil
        // Quit stays available unconditionally: with the app set to outlive
        // its last window, a disabled Quit leaves Force Quit as the only exit.
        default:
            return true
        }
    }
}
