#pragma once

#include "pch.h"

namespace rhythm {
class AppState;
}

namespace winrt::Rhythm {

class TrayManager {
public:
    static void Create(winrt::Microsoft::UI::Xaml::Window const& window,
                       rhythm::AppState* appState);
    static void Remove();

private:
    static LRESULT CALLBACK MessageHandler(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);

    inline static NOTIFYICONDATAW nid_{};
    inline static bool created_ = false;
    inline static rhythm::AppState* appState_ = nullptr;
};

} // namespace winrt::Rhythm
