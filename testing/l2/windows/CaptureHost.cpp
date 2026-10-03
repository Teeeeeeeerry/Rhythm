#include "pch.h"
#include "ShellHost.h"
#include "CaptureFixture.h"

// The capture host's answers to what the main window asks of its process
// (#495): an empty scratch library instead of %LOCALAPPDATA%, and no tray
// icon -- capturing must leave no trace in the user's session.

namespace rhythm::capture {

std::filesystem::path& FixtureLibraryPath() {
    static std::filesystem::path path;
    return path;
}

} // namespace rhythm::capture

namespace rhythm::shell {

std::wstring LibraryDatabasePath() {
    return rhythm::capture::FixtureLibraryPath().wstring();
}

void AttachTray(HWND, rhythm::AppState*) {}

void DetachTray() {}

} // namespace rhythm::shell
