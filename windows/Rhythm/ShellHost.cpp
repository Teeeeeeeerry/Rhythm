#include "pch.h"
#include "ShellHost.h"
#include "Views/TrayManager.h"

#include <filesystem>
#include <shlobj_core.h>

namespace rhythm::shell {

/// %LOCALAPPDATA%\Rhythm\library.db. An unpackaged app has no
/// ApplicationData container (ApplicationData::Current() throws, #428).
std::wstring LibraryDatabasePath() {
    PWSTR base = nullptr;
    HRESULT hr = ::SHGetKnownFolderPath(FOLDERID_LocalAppData, 0, nullptr, &base);
    std::filesystem::path root = SUCCEEDED(hr) ? base : L"";
    ::CoTaskMemFree(base);  // freed on failure too (API contract)
    winrt::check_hresult(hr);
    std::filesystem::path dir = root / L"Rhythm";
    std::filesystem::create_directories(dir);
    return (dir / L"library.db").wstring();
}

void AttachTray(HWND mainWindow, rhythm::AppState* state) {
    winrt::Rhythm::TrayManager::Create(mainWindow, state);
}

void DetachTray() {
    winrt::Rhythm::TrayManager::Remove();
}

} // namespace rhythm::shell
