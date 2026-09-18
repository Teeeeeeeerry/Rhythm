#include "pch.h"
#include "MainWindow.xaml.h"
#if __has_include("MainWindow.g.cpp")
#include "MainWindow.g.cpp"
#endif
#include "Views/LibraryView.xaml.h"
#include "Views/PlaylistListView.xaml.h"
#include "Views/PlaylistDetailView.xaml.h"
#include "Views/PlayerBarView.xaml.h"
#include "Views/TrayManager.h"
#include "Views/WindowHandle.h"
#include "L10n.h"

#include <filesystem>
#include <shlobj_core.h>

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::implementation {

namespace {

/// %LOCALAPPDATA%\Rhythm\library.db. An unpackaged app has no
/// ApplicationData container (ApplicationData::Current() throws, #428).
std::wstring LibraryDatabasePath() {
    PWSTR base = nullptr;
    winrt::check_hresult(::SHGetKnownFolderPath(FOLDERID_LocalAppData, 0, nullptr, &base));
    std::filesystem::path dir = std::filesystem::path(base) / L"Rhythm";
    ::CoTaskMemFree(base);
    std::filesystem::create_directories(dir);
    return (dir / L"library.db").wstring();
}

} // namespace

void MainWindow::InitializeComponent() {
    MainWindowT<MainWindow>::InitializeComponent();
    hwnd_ = rhythm::shell::WindowHandle(*this);
    AppWindow().Resize({960, 600});

    // #141: all static copy comes from the language layer (system UI
    // language, manual override in L10n::SetOverrideLanguage).
    navLibrary().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::LibraryTab() }));
    navPlaylists().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::PlaylistsTab() }));
    ToolTipService::SetToolTip(btnImport(), winrt::box_value(winrt::hstring{ rhythm::L10n::ImportFolderTooltip() }));
    ToolTipService::SetToolTip(btnImportFile(), winrt::box_value(winrt::hstring{ rhythm::L10n::ImportTooltip() }));
    searchBox().PlaceholderText(rhythm::L10n::SearchPlaceholder());
    comboArtistAlbum().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::ByArtistAlbum() }));
    comboByLetter().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::ByLetter() }));

    appState_.OpenDatabase(LibraryDatabasePath());

    // Wire the player bar to the shared state
    appState_.SetDispatcherQueue(DispatcherQueue());
    get_self<Views::implementation::PlayerBarView>(playerBar())->BindState(&appState_);

    // #172/#173: playback state, progress, auto-advance, and failure
    // reporting arrive as coordinator events — the old 500 ms polling timer
    // is gone. The player bar re-renders after every applied event.
    appState_.OnStateChanged = [this] {
        get_self<Views::implementation::PlayerBarView>(playerBar())->Update();
    };

    TrayManager::Create(hwnd_, &appState_);
    Closed([](auto&&, auto&&) { TrayManager::Remove(); });

    contentFrame().Navigated({ this, &MainWindow::OnFrameNavigated });
    navView().SelectedItem(navLibrary());
    ready_ = true;
    LoadLibraryView();
}

void MainWindow::OnFrameNavigated(IInspectable const&,
                                  Navigation::NavigationEventArgs const& args) {
    auto content = args.Content();
    if (auto page = content.try_as<Rhythm::Views::LibraryView>()) {
        get_self<Views::implementation::LibraryView>(page)->BindState(&appState_);
    } else if (auto page = content.try_as<Rhythm::Views::PlaylistListView>()) {
        get_self<Views::implementation::PlaylistListView>(page)->BindState(&appState_);
    } else if (auto page = content.try_as<Rhythm::Views::PlaylistDetailView>()) {
        get_self<Views::implementation::PlaylistDetailView>(page)->BindState(&appState_, hwnd_);
    }
}

void MainWindow::OnNavSelectionChanged(
    NavigationView const&,
    NavigationViewSelectionChangedEventArgs const& args) {
    if (!ready_) return;
    auto item = args.SelectedItem().try_as<NavigationViewItem>();
    if (!item) return;
    auto tag = winrt::unbox_value<hstring>(item.Tag());

    if (tag == L"Library") {
        appState_.SelectedView = rhythm::SidebarItem::Library;
        LoadLibraryView();
    } else if (tag == L"Playlists") {
        appState_.SelectedView = rhythm::SidebarItem::Playlists;
        LoadPlaylistListView();
    }
}

winrt::fire_and_forget MainWindow::OnImportClick(IInspectable const&, RoutedEventArgs const&) {
    auto lifetime = get_strong();
    winrt::Windows::Storage::Pickers::FolderPicker picker;
    picker.SuggestedStartLocation(
        winrt::Windows::Storage::Pickers::PickerLocationId::MusicLibrary);
    picker.FileTypeFilter().Append(L"*");
    rhythm::shell::ParentPicker(picker, hwnd_);

    // Awaited from the UI thread, so the continuation is back on it.
    auto folder = co_await picker.PickSingleFolderAsync();
    if (!folder) co_return;
    appState_.ImportDirectory(folder.Path().c_str());
    if (appState_.SelectedView == rhythm::SidebarItem::Library) LoadLibraryView();
}

/// #242/#243: Windows can import audio files, not only a folder, and more
/// than one at a time -- the capability macOS has always had. One file goes
/// through the single-file path, several through the core's batch import.
winrt::fire_and_forget MainWindow::OnImportFileClick(IInspectable const&, RoutedEventArgs const&) {
    auto lifetime = get_strong();
    winrt::Windows::Storage::Pickers::FileOpenPicker picker;
    picker.SuggestedStartLocation(
        winrt::Windows::Storage::Pickers::PickerLocationId::MusicLibrary);
    for (const auto& ext : rhythm::kAudioFileTypes) {
        picker.FileTypeFilter().Append(ext);
    }
    rhythm::shell::ParentPicker(picker, hwnd_);

    auto files = co_await picker.PickMultipleFilesAsync();
    if (!files || files.Size() == 0) co_return;
    if (files.Size() == 1) {
        appState_.ImportFile(files.GetAt(0).Path().c_str());
    } else {
        std::vector<std::wstring> paths;
        for (const auto& file : files) {
            paths.emplace_back(file.Path().c_str());
        }
        appState_.ImportPaths(paths);
    }
    if (appState_.SelectedView == rhythm::SidebarItem::Library) LoadLibraryView();
}

void MainWindow::OnSearchSubmitted(AutoSuggestBox const& sender,
                                   AutoSuggestBoxQuerySubmittedEventArgs const&) {
    appState_.SearchQuery = sender.Text().c_str();
    appState_.DoSearch();
    LoadLibraryView();
}

void MainWindow::OnSearchTextChanged(AutoSuggestBox const& sender,
                                     AutoSuggestBoxTextChangedEventArgs const&) {
    if (!ready_) return;
    if (sender.Text().empty()) {
        appState_.SearchQuery = L"";
        appState_.DoSearch();
        LoadLibraryView();
    }
}

void MainWindow::OnViewModeChanged(IInspectable const&, SelectionChangedEventArgs const&) {
    if (!ready_) return;
    LoadLibraryView();
}

void MainWindow::LoadLibraryView() {
    contentFrame().Navigate(winrt::xaml_typename<Rhythm::Views::LibraryView>());
}

void MainWindow::LoadPlaylistListView() {
    contentFrame().Navigate(winrt::xaml_typename<Rhythm::Views::PlaylistListView>());
}

} // namespace winrt::Rhythm::implementation
