import SwiftUI
import AppKit

/// Media key codes from IOKit/hidsystem/ev_keymap.h
private let NX_KEYTYPE_PLAY: Int32 = 16
private let NX_KEYTYPE_NEXT: Int32 = 17
private let NX_KEYTYPE_PREVIOUS: Int32 = 18
private let NX_KEYTYPE_FAST: Int32 = 19
private let NX_KEYTYPE_REWIND: Int32 = 20

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var trayManager: TrayManager?
    private var mediaKeyMonitor: Any?
    weak var appState: AppState? {
        didSet {
            if appState != nil && trayManager == nil {
                trayManager = TrayManager(appState: appState)
            }
        }
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        setupMediaKeyMonitor()
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

    // MARK: - Media Key Monitoring

    private func setupMediaKeyMonitor() {
        mediaKeyMonitor = NSEvent.addLocalMonitorForEvents(
            matching: .systemDefined
        ) { [weak self] event in
            self?.handleMediaKey(event)
            return event
        }
    }

    private func handleMediaKey(_ event: NSEvent) {
        guard event.type == .systemDefined,
              event.subtype.rawValue == 8 else { return }

        let keyCode = (event.data1 & 0xFFFF_0000) >> 16
        let keyFlags = event.data1 & 0x0000_FFFF
        let keyState = (keyFlags & 0xFF00) >> 8
        let keyRepeat = keyFlags & 0x00FF
        let isPressed = keyState == 0xA

        guard isPressed && keyRepeat == 0 else { return }

        switch Int32(keyCode) {
        case NX_KEYTYPE_PLAY:
            appState?.togglePlayPause()
        case NX_KEYTYPE_NEXT, NX_KEYTYPE_FAST:
            appState?.playNext()
        case NX_KEYTYPE_PREVIOUS, NX_KEYTYPE_REWIND:
            appState?.playPrevious()
        default:
            break
        }
    }
}
