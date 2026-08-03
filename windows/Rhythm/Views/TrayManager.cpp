#include "pch.h"
#include "TrayManager.h"

namespace winrt::Rhythm {

void TrayManager::Create(Window const& window) {
    if (created_) return;

    auto hwnd = window.GetWindowHandle();

    nid_ = {};
    nid_.cbSize = sizeof(NOTIFYICONDATAW);
    nid_.hWnd = hwnd;
    nid_.uID = 1;
    nid_.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid_.uCallbackMessage = WM_APP + 1;
    wcscpy_s(nid_.szTip, L"Rhythm");

    LoadIconMetric(nullptr, IDI_APPLICATION, LIM_SMALL, &nid_.hIcon);

    Shell_NotifyIconW(NIM_ADD, &nid_);
    created_ = true;
}

void TrayManager::Remove() {
    if (!created_) return;
    Shell_NotifyIconW(NIM_DELETE, &nid_);
    created_ = false;
}

LRESULT TrayManager::MessageHandler(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    if (msg == WM_APP + 1) {
        switch (lParam) {
        case WM_RBUTTONUP: {
            POINT pt;
            GetCursorPos(&pt);

            HMENU menu = CreatePopupMenu();
            AppendMenuW(menu, MF_STRING, 1, L"播放 / 暂停");
            AppendMenuW(menu, MF_STRING, 2, L"显示主窗口");
            AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
            AppendMenuW(menu, MF_STRING, 3, L"退出 Rhythm");

            SetForegroundWindow(hwnd);
            TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, nullptr);
            DestroyMenu(menu);
            break;
        }
        case WM_LBUTTONDBLCLK:
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
            break;
        }
        return 0;
    }

    if (msg == WM_COMMAND) {
        switch (LOWORD(wParam)) {
        case 1: break; // Play/Pause — handled by app
        case 2:
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
            break;
        case 3:
            TrayManager::Remove();
            PostQuitMessage(0);
            break;
        }
        return 0;
    }

    return DefWindowProc(hwnd, msg, wParam, lParam);
}

} // namespace winrt::Rhythm
