#include "pch.h"
#include "Views/LibraryView.xaml.h"
#if __has_include("Views/LibraryView.g.cpp")
#include "Views/LibraryView.g.cpp"
#endif
#include "Models/TrackItem.h"
#include "Views/SystemTheme.h"
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
    Populate();
}

void LibraryView::OnPivotChanged(IInspectable const&, SelectionChangedEventArgs const&) {
    Populate();
}

void LibraryView::OnTrackClick(IInspectable const&, ItemClickEventArgs const& args) {
    if (!appState_) return;
    auto item = args.ClickedItem().as<Rhythm::Models::TrackItem>();
    appState_->PlayTrack(get_self<Models::implementation::TrackItem>(item)->Model());
}

void LibraryView::Populate() {
    if (!appState_) return;
    // The sort rules live in the view state (#336); the pivot only picks one.
    auto sort = viewPivot().SelectedIndex() == 0 ? rhythm::view::LibrarySort::ArtistAlbum
                                                 : rhythm::view::LibrarySort::Alphabetical;
    const bool isDark = rhythm::shell::IsDarkTheme();  // #342: resolved once, by the shell
    ShowRows(rhythm::view::LibraryRows(*appState_, sort, isDark));
}

void LibraryView::ShowRows(std::vector<rhythm::view::TrackRow> const& rows) {
    ShowEmptyMessage(rows.empty());
    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (const auto& row : rows) {
        items.Append(winrt::make<Models::implementation::TrackItem>(row));  // #339
    }
    trackList().ItemsSource(items);
}

void LibraryView::ShowEmptyMessage(bool show) {
    emptyState().Visibility(show ? Visibility::Visible : Visibility::Collapsed);
    trackList().Visibility(show ? Visibility::Collapsed : Visibility::Visible);
}

} // namespace winrt::Rhythm::Views::implementation
