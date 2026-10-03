#include "pch.h"
#include "Views/LibraryView.xaml.h"
#if __has_include("Views/LibraryView.g.cpp")
#include "Views/LibraryView.g.cpp"
#endif
#include "Models/TrackItem.h"
#include "Views/SystemTheme.h"
#include "L10n.h"

#include <winrt/Microsoft.UI.Xaml.Automation.h>

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::Views::implementation {

void LibraryView::InitializeComponent() {
    LibraryViewT<LibraryView>::InitializeComponent();
    // #225: the empty state reads the same two keys as macOS
    // (library_empty + import_hint) — one key per line of copy.
    emptyMessage().Text(rhythm::L10n::LibraryEmpty());
    emptyHint().Text(rhythm::L10n::ImportHint());
}

void LibraryView::BindState(rhythm::AppState* state) {
    appState_ = state;
    Populate();
}

void LibraryView::OnSegmentClick(IInspectable const& sender, RoutedEventArgs const&) {
    if (!appState_) return;
    // The clicked segment's order is whatever the view state put there.
    const Button buttons[] = {segmentArtistAlbum(), segmentByLetter()};
    const auto segments = rhythm::view::LibraryViewSwitch(*appState_);
    for (size_t i = 0; i < segments.size() && i < std::size(buttons); ++i) {
        if (sender != buttons[i]) continue;
        appState_->LibraryOrder = segments[i].sort;
        Populate();
        return;
    }
}

void LibraryView::OnTrackClick(IInspectable const&, ItemClickEventArgs const& args) {
    if (!appState_) return;
    auto item = args.ClickedItem().as<Rhythm::Models::TrackItem>();
    appState_->PlayTrack(get_self<Models::implementation::TrackItem>(item)->Model());
}

void LibraryView::Populate() {
    if (!appState_) return;
    RenderViewSwitch();
    // The sort rules live in the view state (#336), the chosen order in the
    // app state (#502), so a page rebuilt by a refresh keeps it.
    const bool isDark = rhythm::shell::IsDarkTheme(RequestedTheme());  // #342/#495: a theme pinned on the view wins, else the system
    ShowRows(rhythm::view::LibraryRows(*appState_, isDark));
}

void LibraryView::RenderViewSwitch() {
    struct Slot {
        Button button;
        Border back;
        TextBlock label;
    };
    const Slot slots[] = {
        {segmentArtistAlbum(), segmentArtistAlbumBack(), segmentArtistAlbumLabel()},
        {segmentByLetter(), segmentByLetterBack(), segmentByLetterLabel()},
    };
    auto style = [this](const wchar_t* key) {
        return Resources().Lookup(winrt::box_value(key)).as<winrt::Microsoft::UI::Xaml::Style>();
    };
    const auto segments = rhythm::view::LibraryViewSwitch(*appState_);
    for (size_t i = 0; i < segments.size() && i < std::size(slots); ++i) {
        const auto& segment = segments[i];
        const auto& slot = slots[i];
        slot.label.Text(segment.label);
        // The button's content is a layout, so screen readers need the name.
        Automation::AutomationProperties::SetName(slot.button, segment.label);
        slot.back.Style(style(segment.selected ? L"SegmentBackSelectedStyle" : L"SegmentBackStyle"));
        slot.label.Style(style(segment.selected ? L"SegmentLabelSelectedStyle" : L"SegmentLabelStyle"));
    }
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
