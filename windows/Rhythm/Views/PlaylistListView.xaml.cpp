#include "pch.h"
#include "PlaylistListView.xaml.h"
#include "L10n.h"

namespace winrt::Rhythm::Views::implementation {

PlaylistListView::PlaylistListView() {
    InitializeComponent();
    // #141: copy from the language layer.
    newPlaylistText().Text(rhythm::L10n::NewPlaylist());
    emptyMessage().Text(rhythm::L10n::PlaylistEmpty());
}

void PlaylistListView::OnNavigatedTo(Navigation::NavigationEventArgs const& args) {
    appState_ = winrt::unbox_value<rhythm::AppState*>(args.Parameter());
    Refresh();
}

void PlaylistListView::OnNewPlaylistClick(IInspectable const&, RoutedEventArgs const&) {
    if (!appState_ || !appState_->Library) return;

    // Simple name input dialog
    auto tb = TextBox();
    tb.PlaceholderText(rhythm::L10n::PlaylistNamePlaceholder());
    tb.Width(200);

    auto dialog = ContentDialog();
    dialog.Title(winrt::box_value(winrt::hstring{ rhythm::L10n::NewPlaylist() }));
    dialog.Content(tb);
    dialog.PrimaryButtonText(rhythm::L10n::Create());
    dialog.CloseButtonText(rhythm::L10n::Cancel());
    dialog.XamlRoot().XamlRoot();
    dialog.DefaultButton(ContentDialogButton::Primary);

    dialog.PrimaryButtonClick([&](auto const&, auto const&) {
        auto name = tb.Text();
        if (!name.empty()) {
            appState_->Library->CreatePlaylist(name.c_str());
            appState_->RefreshLibrary();
            Refresh();
        }
    });
}

void PlaylistListView::OnSelectionChanged(IInspectable const&, SelectionChangedEventArgs const&) {
    if (!appState_) return;
    auto idx = playlistList().SelectedIndex();
    if (idx >= 0 && idx < static_cast<int32_t>(appState_->Playlists.size())) {
        auto& pl = appState_->Playlists[idx];
        // Navigate to detail view
        auto frame = playlistList().XamlRoot().Content().try_as<Frame>();
        if (frame) {
            frame.Navigate(xaml_typename<Rhythm::Views::PlaylistDetailView>(),
                           box_value(&pl));
        }
    }
}

void PlaylistListView::Refresh() {
    if (!appState_) return;
    auto& playlists = appState_->Playlists;
    emptyMessage().Visibility(playlists.empty() ? Visibility::Visible : Visibility::Collapsed);
    playlistList().Visibility(playlists.empty() ? Visibility::Collapsed : Visibility::Visible);

    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (auto& pl : playlists) {
        items.Append(box_value(pl));
    }
    playlistList().ItemsSource(items);
}

} // namespace winrt::Rhythm::Views::implementation
