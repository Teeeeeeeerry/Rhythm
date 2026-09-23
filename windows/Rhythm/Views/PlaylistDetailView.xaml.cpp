#include "pch.h"
#include "Views/PlaylistDetailView.xaml.h"
#if __has_include("Views/PlaylistDetailView.g.cpp")
#include "Views/PlaylistDetailView.g.cpp"
#endif
#include "Models/TrackItem.h"
#include "Views/SystemTheme.h"
#include "Views/Win32Interop.h"
#include "L10n.h"
#include "ViewState.h"

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

void PlaylistDetailView::OnNavigatedTo(Navigation::NavigationEventArgs const&) {
    // #357: navigation carries no parameter -- which playlist this page shows
    // is the state's current playlist, read once the state is bound.
    Refresh();
}

void PlaylistDetailView::BindState(rhythm::AppState* state, HWND owner) {
    appState_ = state;
    owner_ = owner;
    Refresh();
}

void PlaylistDetailView::Refresh() {
    if (!appState_) return;
    // #358: which playlist this is comes from the state on every render --
    // the page holds neither a pointer nor an id of its own.
    const bool isDark = rhythm::shell::IsDarkTheme();  // #342: resolved once, by the shell
    auto detail = rhythm::view::PlaylistDetailOf(*appState_, isDark);
    if (!detail.hasPlaylist) return;

    playlistTitle().Text(detail.title);
    // The same rows as the library, from the view state (#339).
    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (auto& row : detail.rows) {
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
    if (!appState_ || !appState_->CurrentPlaylist) co_return;
    auto lifetime = get_strong();

    winrt::Windows::Storage::Pickers::FileSavePicker picker;
    picker.SuggestedFileName(appState_->CurrentPlaylist->name);
    auto extensions = winrt::single_threaded_vector<winrt::hstring>({ L".m3u8" });
    picker.FileTypeChoices().Insert(L"M3U8", extensions);
    rhythm::shell::ParentPicker(picker, owner_);

    try {
        auto file = co_await picker.PickSaveFileAsync();
        if (!file || !appState_) co_return;
        // #352/#358: the state exports the playlist it holds now (read again
        // after the await -- a refresh may have landed while the picker was
        // open) and picks the result or failure copy; this only shows it.
        auto current = appState_->CurrentPlaylist;
        if (!current || !current->id) co_return;
        appState_->ExportPlaylist(*current->id, file.Path().c_str());
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
