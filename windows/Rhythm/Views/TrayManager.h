#pragma once

#include "pch.h"

namespace rhythm {
class AppState;
}

namespace winrt::Rhythm {

/// Notification-area icon. Its messages go to a hidden window this class
/// owns (#428: the handler used to be defined but never attached to
/// any window, so the menu could not appear).
class TrayManager {
public:
    static void Create(HWND mainWindow, rhythm::AppState* appState);
    static void Remove();

private:
    static LRESULT CALLBACK MessageHandler(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
    static void ShowMainWindow();

    inline static NOTIFYICONDATAW nid_{};
    inline static bool created_ = false;
    inline static HWND mainWindow_ = nullptr;
    inline static HWND messageWindow_ = nullptr;
    inline static rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm
