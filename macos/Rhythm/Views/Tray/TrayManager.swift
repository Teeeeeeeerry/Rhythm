import AppKit

final class TrayManager: NSObject {
    private var statusItem: NSStatusItem!

    override init() {
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
            title: "播放 / 暂停",
            action: #selector(togglePlay),
            keyEquivalent: " "
        ))
        menu.addItem(NSMenuItem(
            title: "下一首",
            action: #selector(nextTrack),
            keyEquivalent: ""
        ))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(
            title: "显示主窗口",
            action: #selector(showWindow),
            keyEquivalent: ""
        ))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(
            title: "退出 Rhythm",
            action: #selector(quitApp),
            keyEquivalent: "q"
        ))

        statusItem.menu = menu
    }

    @objc private func togglePlay() {}
    @objc private func nextTrack() {}

    @objc private func showWindow() {
        NSApp.activate(ignoringOtherApps: true)
        NSApp.windows.first?.makeKeyAndOrderFront(nil)
    }

    @objc private func quitApp() {
        NSApp.terminate(nil)
    }
}
