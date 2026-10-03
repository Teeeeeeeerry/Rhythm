#pragma once

// What the main window needs from the process hosting it (#495). The app
// (ShellHost.cpp next to this file) opens the user's library and installs the
// tray icon; the L2 capture host links its own definitions, so rendering the
// main window never touches the user's library or the notification area.

#include <string>

namespace rhythm {
class AppState;
}

namespace rhythm::shell {

/// Path of the library database the main window opens.
std::wstring LibraryDatabasePath();

/// Called once the main window is set up, and when it closes.
void AttachTray(HWND mainWindow, rhythm::AppState* state);
void DetachTray();

} // namespace rhythm::shell
