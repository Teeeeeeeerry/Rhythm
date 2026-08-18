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

    // Progress update timer
    auto timer = winrt::Microsoft::UI::Xaml::DispatcherTimer();
    timer.Interval(std::chrono::milliseconds(500));
    timer.Tick([this](auto const&, auto const&) {
        if (appState_.IsPlaying) {
            appState_.Position = appState_.Player->Position();
            appState_.Duration = appState_.Player->Duration();

            auto state = appState_.Player->State();
            appState_.IsBuffering = state == 3;

            // #137: a track that ended naturally must advance the queue, or
            // stop claiming playback — mirror macOS updatePlaybackProgress.
            if (state == 5) {
                if (appState_.CanPlayNext()) {
                    auto before = appState_.CurrentTrack->id;
                    appState_.PlayNext();
                    if (appState_.CurrentTrack->id == before) {
                        // Next track is unplayable: stop claiming playback
                        // instead of retrying every tick (#78 semantics).
                        appState_.IsPlaying = false;
                    }
                } else {
                    appState_.IsPlaying = false;
                }
            } else if (state == 4) {
                // Otherwise a failed stream just sits at 0:00 with no explanation.
                appState_.IsPlaying = false;
                auto detail = appState_.Player->ErrorMessage();
                appState_.UrlError = detail;
                OutputDebugStringW((L"Playback failed: " + detail + L"\n").c_str());
                if (appState_.OnUrlError) {
                    // #120: classify HTTP failures so the dialog can tell a
                    // genuinely expired link from a CDN rejection.
                    auto kind = appState_.Player->ErrorKind();
                    std::wstring kindCode = L"playback_failed";
                    if (kind == L"expired") {
                        kindCode = L"playback_expired";
                    } else if (kind == L"cdn_rejected") {
                        kindCode = L"playback_cdn_rejected";
                    }
                    appState_.OnUrlError(kindCode, detail);
                }
            }
        }
        playerBar().Update();
    });
    timer.Start();
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
