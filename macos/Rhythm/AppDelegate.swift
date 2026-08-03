import SwiftUI
import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var trayManager: TrayManager?

    func applicationDidFinishLaunching(_ notification: Notification) {
        trayManager = TrayManager()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag {
            NSApp.windows.first?.makeKeyAndOrderFront(nil)
        }
        return true
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }
}
