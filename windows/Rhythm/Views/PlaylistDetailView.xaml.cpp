#include "pch.h"
#include "PlaylistDetailView.xaml.h"

namespace winrt::Rhythm::Views::implementation {

PlaylistDetailView::PlaylistDetailView() {
    InitializeComponent();
}

void PlaylistDetailView::OnNavigatedTo(Navigation::NavigationEventArgs const& args) {
    if (!args.Parameter()) return;
    playlist_ = winrt::unbox_value<rhythm::Playlist*>(args.Parameter());
    if (!playlist_) return;

    playlistTitle().Text(playlist_->name);

    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (auto& track : playlist_->tracks) {
        items.Append(box_value(track));
    }
    trackList().ItemsSource(items);
}

void PlaylistDetailView::OnBackClick(IInspectable const&, RoutedEventArgs const&) {
    if (auto frame = this->Parent().try_as<Frame>()) {
        frame.GoBack();
    }
}

void PlaylistDetailView::OnImportClick(IInspectable const&, RoutedEventArgs const&) {
    auto picker = WinRT::Windows::Storage::Pickers::FileOpenPicker();
    picker.FileTypeFilter().Append(L".m3u8");
    picker.FileTypeFilter().Append(L".m3u");

    auto hwnd = winrt::Microsoft::UI::GetWindowFromElement(*this).GetWindowHandle();
    picker.as<IInitializeWithWindow>()->Initialize(hwnd);

    picker.PickSingleFileAsync().Completed([this](auto const& op, auto) {
        if (auto file = op.GetResults()) {
            rhythm_import_m3u8(winrt::to_string(file.Path()).c_str());
            if (appState_) appState_->RefreshLibrary();
        }
    });
}

void PlaylistDetailView::OnExportClick(IInspectable const&, RoutedEventArgs const&) {
    if (!playlist_) return;

    auto picker = WinRT::Windows::Storage::Pickers::FileSavePicker();
    picker.SuggestedFileName(playlist_->name);
    picker.FileTypeChoices().Insert(L"M3U8", { L".m3u8" });

    auto hwnd = winrt::Microsoft::UI::GetWindowFromElement(*this).GetWindowHandle();
    picker.as<IInitializeWithWindow>()->Initialize(hwnd);

    picker.PickSaveFileAsync().Completed([this](auto const& op, auto) {
        if (auto file = op.GetResults()) {
            std::string json = "[]"; // simplified
            rhythm_export_m3u8(winrt::to_string(file.Path()).c_str(), json.c_str());
        }
    });
}

void PlaylistDetailView::OnTrackClick(IInspectable const&, ItemClickEventArgs const& args) {
    if (!appState_) return;
    auto track = winrt::unbox_value<rhythm::Track>(args.ClickedItem());
    appState_->PlayTrack(track);
}

} // namespace winrt::Rhythm::Views::implementation
