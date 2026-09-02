#include "pch.h"
#include "MainWindow.xaml.h"
#include "Views/SidebarView.xaml.h"
#include "Views/LibraryView.xaml.h"
#include "Views/PlaylistListView.xaml.h"
#include "Views/PlayerBarView.xaml.h"
#include "Views/TrayManager.h"
#include "L10n.h"

namespace winrt::Rhythm::implementation {

MainWindow::MainWindow() {
    InitializeComponent();

    // #141: all static copy comes from the language layer (system UI
    // language, manual override in L10n::SetOverrideLanguage).
    navLibrary().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::LibraryTab() }));
    navPlaylists().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::PlaylistsTab() }));
    btnImport().ToolTip(winrt::box_value(winrt::hstring{ rhythm::L10n::ImportFolderTooltip() }));
    btnImportFile().ToolTip(winrt::box_value(winrt::hstring{ rhythm::L10n::ImportTooltip() }));
    searchBox().PlaceholderText(rhythm::L10n::SearchPlaceholder());
    comboArtistAlbum().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::ByArtistAlbum() }));
    comboByLetter().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::ByLetter() }));

    // Open database in AppData
    auto localFolder = winrt::Windows::Storage::ApplicationData::Current().LocalFolder();
    auto dbPath = localFolder.Path() + L"\\library.db";
    appState_.OpenDatabase(dbPath.c_str());

    // Wire the player bar to the shared state
    appState_.SetDispatcherQueue(DispatcherQueue());
    playerBar().BindState(&appState_);

    // Setup tray
    TrayManager::Create(*this, &appState_);

    // Default to library view
    LoadLibraryView();

    // #172/#173: playback state, progress, auto-advance, and failure
    // reporting arrive as coordinator events — the old 500 ms polling timer
    // is gone. The player bar re-renders after every applied event.
    appState_.OnStateChanged = [this] { playerBar().Update(); };
}

void MainWindow::OnNavSelectionChanged(
    NavigationView const& sender,
    NavigationViewSelectionChangedEventArgs const& args) {

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

void MainWindow::OnImportClick(IInspectable const&, RoutedEventArgs const&) {
    auto picker = winrt::Windows::Storage::Pickers::FolderPicker();
    picker.SuggestedStartLocation(
        winrt::Windows::Storage::Pickers::PickerLocationId::MusicLibrary);
    auto hwnd = GetWindowHandle();
    picker.as<winrt::Windows::Foundation::IInitializeWithWindow>()->Initialize(hwnd);

    picker.PickSingleFolderAsync().Completed([this](auto const& operation, auto) {
        if (auto folder = operation.GetResults()) {
            appState_.ImportDirectory(folder.Path().c_str());
        }
    });
}

/// #242: Windows can import a single audio file, not only a folder. The
/// picker offers files -- the capability macOS has always had.
void MainWindow::OnImportFileClick(IInspectable const&, RoutedEventArgs const&) {
    auto picker = winrt::Windows::Storage::Pickers::FileOpenPicker();
    picker.SuggestedStartLocation(
        winrt::Windows::Storage::Pickers::PickerLocationId::MusicLibrary);
    for (const auto& ext : rhythm::kAudioFileTypes) {
        picker.FileTypeFilter().Append(ext);
    }
    auto hwnd = GetWindowHandle();
    picker.as<winrt::Windows::Foundation::IInitializeWithWindow>()->Initialize(hwnd);

    picker.PickSingleFileAsync().Completed([this](auto const& operation, auto) {
        if (auto file = operation.GetResults()) {
            appState_.ImportFile(file.Path().c_str());
        }
    });
}

void MainWindow::OnSearchSubmitted(AutoSuggestBox const& sender,
                                   AutoSuggestBoxQuerySubmittedEventArgs const&) {
    appState_.SearchQuery = sender.Text().c_str();
    appState_.DoSearch();
}

void MainWindow::OnSearchTextChanged(AutoSuggestBox const& sender,
                                     AutoSuggestBoxTextChangedEventArgs const&) {
    auto text = sender.Text();
    if (text.empty()) {
        appState_.SearchQuery = L"";
        appState_.DoSearch();
    }
}

void MainWindow::OnViewModeChanged(IInspectable const&, SelectionChangedEventArgs const&) {
    LoadLibraryView();
}

void MainWindow::LoadLibraryView() {
    contentFrame().Navigate(
        winrt::xaml_typename<Rhythm::Views::LibraryView>(),
        box_value(appState_));
}

void MainWindow::LoadPlaylistListView() {
    contentFrame().Navigate(
        winrt::xaml_typename<Rhythm::Views::PlaylistListView>(),
        box_value(appState_));
}

} // namespace winrt::Rhythm::implementation

// Entry point
int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int) {
    winrt::init_apartment();
    winrt::Microsoft::UI::Xaml::Application::Start(
        [](auto&&) { winrt::make<Rhythm::implementation::App>(); }
    );
    return 0;
}
