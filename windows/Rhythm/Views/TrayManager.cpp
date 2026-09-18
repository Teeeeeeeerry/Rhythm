#include "pch.h"
#include "Views/TrayManager.h"
#include "AppState.h"
#include "ViewState.h"

#include <shellapi.h>

namespace winrt::Rhythm {

namespace {

constexpr UINT kTrayMessage = WM_APP + 1;
constexpr wchar_t kWindowClass[] = L"RhythmTrayWindow";
// Explorer broadcasts this after it restarts; the icon must be added again.
const UINT kTaskbarCreated = ::RegisterWindowMessageW(L"TaskbarCreated");

} // namespace

void TrayManager::Create(HWND mainWindow, rhythm::AppState* appState) {
    if (created_) return;

    appState_ = appState;
    mainWindow_ = mainWindow;

    WNDCLASSEXW wc{ sizeof(wc) };
    wc.lpfnWndProc = &TrayManager::MessageHandler;
    wc.hInstance = ::GetModuleHandleW(nullptr);
    wc.lpszClassName = kWindowClass;
    ::RegisterClassExW(&wc);
    // A hidden top-level window, not HWND_MESSAGE: the popup menu needs a
    // window that can take the foreground (or it never dismisses), and only
    // top-level windows receive the TaskbarCreated broadcast.
    messageWindow_ = ::CreateWindowExW(WS_EX_TOOLWINDOW, kWindowClass, L"", WS_POPUP,
                                       0, 0, 0, 0, nullptr, nullptr, wc.hInstance, nullptr);
    if (!messageWindow_) return;

    nid_ = {};
    nid_.cbSize = sizeof(NOTIFYICONDATAW);
    nid_.hWnd = messageWindow_;
    nid_.uID = 1;
    nid_.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid_.uCallbackMessage = kTrayMessage;
    wcscpy_s(nid_.szTip, L"Rhythm");
    nid_.hIcon = ::LoadIconW(nullptr, IDI_APPLICATION);

    ::Shell_NotifyIconW(NIM_ADD, &nid_);
    created_ = true;
}

void TrayManager::Remove() {
    if (!created_) return;
    ::Shell_NotifyIconW(NIM_DELETE, &nid_);
    ::DestroyWindow(messageWindow_);
    messageWindow_ = nullptr;
    created_ = false;
}

void TrayManager::ShowMainWindow() {
    ::ShowWindow(mainWindow_, SW_RESTORE);
    ::SetForegroundWindow(mainWindow_);
}

LRESULT TrayManager::MessageHandler(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    if (msg == kTrayMessage) {
        switch (LOWORD(lParam)) {
        case WM_RBUTTONUP: {
            POINT pt;
            ::GetCursorPos(&pt);

            // What the menu shows comes from the view state (#340); this only
            // builds the native menu from it.
            if (!appState_) break;
            auto model = rhythm::view::TrayMenuState(*appState_);
            HMENU menu = ::CreatePopupMenu();
            ::AppendMenuW(menu, MF_STRING | (model.playPauseEnabled ? 0 : MF_GRAYED), 1,
                          model.playPause.c_str());
            ::AppendMenuW(menu, MF_STRING, 2, model.showWindow.c_str());
            ::AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
            ::AppendMenuW(menu, MF_STRING, 3, model.quit.c_str());

            ::SetForegroundWindow(hwnd);
            ::TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, nullptr);
            // Documented companion of TrackPopupMenu for notification icons.
            ::PostMessageW(hwnd, WM_NULL, 0, 0);
            ::DestroyMenu(menu);
            break;
        }
        case WM_LBUTTONDBLCLK:
            ShowMainWindow();
            break;
        }
        return 0;
    }

    if (msg == kTaskbarCreated && created_) {
        ::Shell_NotifyIconW(NIM_ADD, &nid_);
        return 0;
    }

    if (msg == WM_COMMAND) {
        switch (LOWORD(wParam)) {
        case 1: {
            // #138: same entry as the player-bar button. Whether it may act
            // is the same model value that greyed the item out (#341).
            if (appState_ && rhythm::view::TrayMenuState(*appState_).playPauseEnabled) {
                appState_->TogglePlayPause();
            }
            break;
        }
        case 2:
            ShowMainWindow();
            break;
        case 3:
            TrayManager::Remove();
            winrt::Microsoft::UI::Xaml::Application::Current().Exit();
            break;
        }
        return 0;
    }

    return ::DefWindowProcW(hwnd, msg, wParam, lParam);
}

} // namespace winrt::Rhythm
