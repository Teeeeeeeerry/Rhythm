#include "pch.h"
#include "LibraryView.xaml.h"
#include "L10n.h"

namespace winrt::Rhythm::Views::implementation {

LibraryView::LibraryView() {
    InitializeComponent();
    // #141: copy from the language layer.
    pivotArtistAlbum().Header(winrt::box_value(winrt::hstring{ rhythm::L10n::ByArtistAlbum() }));
    pivotByLetter().Header(winrt::box_value(winrt::hstring{ rhythm::L10n::ByLetter() }));
    // #225: the empty state reads the same two keys as macOS
    // (library_empty + import_hint) — one key per line of copy.
    emptyMessage().Text(rhythm::L10n::LibraryEmpty());
    emptyHint().Text(rhythm::L10n::ImportHint());
}

void LibraryView::OnNavigatedTo(Navigation::NavigationEventArgs const& args) {
    appState_ = winrt::unbox_value<rhythm::AppState*>(args.Parameter());
    PopulateArtistAlbum();
}

void LibraryView::OnPivotChanged(IInspectable const&, SelectionChangedEventArgs const&) {
    if (!appState_) return;
    if (viewPivot().SelectedIndex() == 0) {
        PopulateArtistAlbum();
    } else {
        PopulateAlphabetical();
    }
}

void LibraryView::OnTrackClick(IInspectable const&, ItemClickEventArgs const& args) {
    if (!appState_) return;
    auto track = winrt::unbox_value<rhythm::Track>(args.ClickedItem());
    appState_->PlayTrack(track);
}

void LibraryView::PopulateArtistAlbum() {
    if (!appState_) return;
    if (appState_->Tracks.empty()) { ShowEmptyMessage(true); return; }
    ShowEmptyMessage(false);

    // Sort a copy so the shared AppState::Tracks order is never mutated
    auto tracks = appState_->Tracks;

    // Sort by artist then album then track number
    std::sort(tracks.begin(), tracks.end(),
        [](const auto& a, const auto& b) {
            auto artA = a.artist.value_or(L"");
            auto artB = b.artist.value_or(L"");
            if (artA != artB) return artA < artB;
            auto albA = a.album.value_or(L"");
            auto albB = b.album.value_or(L"");
            if (albA != albB) return albA < albB;
            return a.trackNumber.value_or(0) < b.trackNumber.value_or(0);
        });

    // Build grouped collection
    auto items = winrt::single_threaded_observable_vector<winrt::Windows::Foundation::IInspectable>();
    for (auto& track : tracks) {
        items.Append(winrt::box_value(track));
    }
    trackList().ItemsSource(items);
}

void LibraryView::PopulateAlphabetical() {
    if (!appState_) return;
    if (appState_->Tracks.empty()) { ShowEmptyMessage(true); return; }
    ShowEmptyMessage(false);

    // Sort a copy so the shared AppState::Tracks order is never mutated
    auto tracks = appState_->Tracks;

    std::sort(tracks.begin(), tracks.end(),
        [](const auto& a, const auto& b) { return a.title < b.title; });

    auto items = winrt::single_threaded_observable_vector<winrt::Windows::Foundation::IInspectable>();
    for (auto& track : tracks) {
        items.Append(winrt::box_value(track));
    }
    trackList().ItemsSource(items);
}

void LibraryView::ShowEmptyMessage(bool show) {
    emptyState().Visibility(show ? Visibility::Visible : Visibility::Collapsed);
    trackList().Visibility(show ? Visibility::Collapsed : Visibility::Visible);
}

} // namespace winrt::Rhythm::Views::implementation
