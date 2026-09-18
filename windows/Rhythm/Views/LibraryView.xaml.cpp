#include "pch.h"
#include "Views/LibraryView.xaml.h"
#if __has_include("Views/LibraryView.g.cpp")
#include "Views/LibraryView.g.cpp"
#endif
#include "Models/TrackItem.h"
#include "L10n.h"

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::Views::implementation {

void LibraryView::InitializeComponent() {
    LibraryViewT<LibraryView>::InitializeComponent();
    // #141: copy from the language layer.
    pivotArtistAlbum().Header(winrt::box_value(winrt::hstring{ rhythm::L10n::ByArtistAlbum() }));
    pivotByLetter().Header(winrt::box_value(winrt::hstring{ rhythm::L10n::ByLetter() }));
    // #225: the empty state reads the same two keys as macOS
    // (library_empty + import_hint) — one key per line of copy.
    emptyMessage().Text(rhythm::L10n::LibraryEmpty());
    emptyHint().Text(rhythm::L10n::ImportHint());
}

void LibraryView::BindState(rhythm::AppState* state) {
    appState_ = state;
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
    auto item = args.ClickedItem().as<Rhythm::Models::TrackItem>();
    appState_->PlayTrack(get_self<Models::implementation::TrackItem>(item)->Model());
}

void LibraryView::PopulateArtistAlbum() {
    if (!appState_) return;
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
    ShowTracks(tracks);
}

void LibraryView::PopulateAlphabetical() {
    if (!appState_) return;
    // Sort a copy so the shared AppState::Tracks order is never mutated
    auto tracks = appState_->Tracks;
    std::sort(tracks.begin(), tracks.end(),
        [](const auto& a, const auto& b) { return a.title < b.title; });
    ShowTracks(tracks);
}

void LibraryView::ShowTracks(std::vector<rhythm::Track> const& tracks) {
    ShowEmptyMessage(tracks.empty());
    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (const auto& track : tracks) {
        items.Append(winrt::make<Models::implementation::TrackItem>(track));
    }
    trackList().ItemsSource(items);
}

void LibraryView::ShowEmptyMessage(bool show) {
    emptyState().Visibility(show ? Visibility::Visible : Visibility::Collapsed);
    trackList().Visibility(show ? Visibility::Collapsed : Visibility::Visible);
}

} // namespace winrt::Rhythm::Views::implementation
