#include "pch.h"
#include "Views/PlaylistDetailView.xaml.h"
#if __has_include("Views/PlaylistDetailView.g.cpp")
#include "Views/PlaylistDetailView.g.cpp"
#endif
#include "Models/TrackItem.h"
#include "Views/SystemTheme.h"
#include "Views/Win32Interop.h"
#include "L10n.h"

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::Views::implementation {

void PlaylistDetailView::InitializeComponent() {
    PlaylistDetailViewT<PlaylistDetailView>::InitializeComponent();
    // #141: copy from the language layer.
    btnImport().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::ImportM3U8() }));
    btnExport().Content(winrt::box_value(winrt::hstring{ rhythm::L10n::ExportM3U8() }));
}

void PlaylistDetailView::OnNavigatedTo(Navigation::NavigationEventArgs const& args) {
    if (auto id = args.Parameter().try_as<int64_t>()) {
        playlistId_ = *id;
    }
    Refresh();
}

void PlaylistDetailView::BindState(rhythm::AppState* state, HWND owner) {
    appState_ = state;
    owner_ = owner;
    Refresh();
}

rhythm::Playlist const* PlaylistDetailView::CurrentPlaylist() const {
    if (!appState_ || !playlistId_) return nullptr;
    return appState_->FindPlaylist(*playlistId_);  // #339: one lookup, shared with the rows
}

void PlaylistDetailView::Refresh() {
    auto playlist = CurrentPlaylist();
    if (!playlist) return;

    playlistTitle().Text(playlist->name);
    // The same rows as the library, from the view state (#339).
    const bool isDark = rhythm::shell::IsDarkTheme();  // #342: resolved once, by the shell
    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (auto& row : rhythm::view::PlaylistRows(*appState_, *playlistId_, isDark)) {
        items.Append(winrt::make<Models::implementation::TrackItem>(std::move(row)));
    }
    trackList().ItemsSource(items);
}

void PlaylistDetailView::OnBackClick(IInspectable const&, RoutedEventArgs const&) {
    if (Frame().CanGoBack()) Frame().GoBack();
}

winrt::fire_and_forget PlaylistDetailView::OnImportClick(IInspectable const&, RoutedEventArgs const&) {
    auto lifetime = get_strong();
    winrt::Windows::Storage::Pickers::FileOpenPicker picker;
    picker.FileTypeFilter().Append(L".m3u8");
    picker.FileTypeFilter().Append(L".m3u");
    rhythm::shell::ParentPicker(picker, owner_);

    // #173: entries are persisted and counted, with the same import alert
    // as macOS.
    try {
        auto file = co_await picker.PickSingleFileAsync();
        if (file && appState_) {
            appState_->DismissAlerts();  // #352: only this import's feedback
            appState_->ImportM3U8(file.Path().c_str());
            Refresh();
            co_await ShowPendingAlert();
        }
    } catch (winrt::hresult_error const& e) {
        // An exception leaving a fire_and_forget coroutine ends the process.
        OutputDebugStringW((L"M3U8 import failed: " + e.message() + L"\n").c_str());
    }
}

winrt::fire_and_forget PlaylistDetailView::OnExportClick(IInspectable const&, RoutedEventArgs const&) {
    auto playlist = CurrentPlaylist();
    if (!playlist) co_return;
    auto lifetime = get_strong();

    winrt::Windows::Storage::Pickers::FileSavePicker picker;
    picker.SuggestedFileName(playlist->name);
    auto extensions = winrt::single_threaded_vector<winrt::hstring>({ L".m3u8" });
    picker.FileTypeChoices().Insert(L"M3U8", extensions);
    rhythm::shell::ParentPicker(picker, owner_);

    try {
        auto file = co_await picker.PickSaveFileAsync();
        if (!file || !appState_ || !playlistId_) co_return;
        // #352: the state exports the playlist it holds now (re-read after
        // the await) and picks the result or failure copy; this only shows it.
        appState_->DismissAlerts();
        appState_->ExportPlaylist(*playlistId_, file.Path().c_str());
        co_await ShowPendingAlert();
    } catch (winrt::hresult_error const& e) {
        OutputDebugStringW((L"M3U8 export failed: " + e.message() + L"\n").c_str());
    }
}

winrt::Windows::Foundation::IAsyncAction PlaylistDetailView::ShowPendingAlert() {
    if (!appState_) co_return;
    auto alert = rhythm::view::PendingAlert(*appState_);
    if (!alert) co_return;
    appState_->DismissAlerts();

    ContentDialog dialog;
    dialog.XamlRoot(XamlRoot());
    dialog.Title(winrt::box_value(winrt::hstring{ alert->title }));
    dialog.Content(winrt::box_value(winrt::hstring{ alert->message }));
    dialog.CloseButtonText(rhythm::L10n::Ok());
    co_await dialog.ShowAsync();
}

void PlaylistDetailView::OnTrackClick(IInspectable const&, ItemClickEventArgs const& args) {
    if (!appState_) return;
    auto item = args.ClickedItem().as<Rhythm::Models::TrackItem>();
    appState_->PlayTrack(get_self<Models::implementation::TrackItem>(item)->Model());
}

} // namespace winrt::Rhythm::Views::implementation
