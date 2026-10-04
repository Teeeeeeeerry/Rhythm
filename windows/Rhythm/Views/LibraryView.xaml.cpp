#include "pch.h"
#include "Views/LibraryView.xaml.h"
#if __has_include("Views/LibraryView.g.cpp")
#include "Views/LibraryView.g.cpp"
#endif
#include "Models/TrackItem.h"
#include "Models/LibraryItems.h"
#include "Views/SystemTheme.h"
#include "L10n.h"

#include <winrt/Microsoft.UI.Xaml.Automation.h>

using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using winrt::Windows::Foundation::IInspectable;

namespace winrt::Rhythm::Views::implementation {

namespace {

/// A track inside an album group starts where the group's title does: past
/// the 48 px cover placeholder and its 8 px gap (#503).
constexpr double kAlbumTrackIndent = 56.0;

} // namespace

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
    const RadioButton buttons[] = {segmentArtistAlbum(), segmentByLetter()};
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
    auto item = args.ClickedItem().try_as<Rhythm::Models::TrackItem>();
    if (!item) return;  // a heading
    appState_->PlayTrack(get_self<Models::implementation::TrackItem>(item)->Model());
}

/// Headings are part of the list but not rows (#503): they take no pointer
/// and no keyboard focus. Not disabled -- a disabled container dims its
/// heading. Containers are recycled across kinds, so every assignment sets
/// both ways.
void LibraryView::OnContainerContentChanging(ListViewBase const&,
                                             ContainerContentChangingEventArgs const& args) {
    const bool isTrack = static_cast<bool>(args.Item().try_as<Rhythm::Models::TrackItem>());
    args.ItemContainer().IsHitTestVisible(isTrack);
    args.ItemContainer().IsTabStop(isTrack);
}

/// Whatever still selects a heading (#503), the previous selection stays.
void LibraryView::OnSelectionChanged(IInspectable const&, SelectionChangedEventArgs const& args) {
    auto selected = trackList().SelectedItem();
    if (!selected || selected.try_as<Rhythm::Models::TrackItem>()) return;
    auto removed = args.RemovedItems();
    trackList().SelectedItem(removed.Size() > 0 ? removed.GetAt(0) : nullptr);
}

void LibraryView::Populate() {
    if (!appState_) return;
    RenderViewSwitch();
    // The sort rules live in the view state (#336), the chosen order in the
    // app state (#502), so a page rebuilt by a refresh keeps it.
    const bool isDark = rhythm::shell::IsDarkTheme(RequestedTheme());  // #342/#495: a theme pinned on the view wins, else the system
    // #503: what the lines are, and in what order, comes whole from the view
    // state; this only turns each line into its list item.
    using Kind = rhythm::view::LibraryLineKind;
    auto items = winrt::single_threaded_observable_vector<IInspectable>();
    for (const auto& line : rhythm::view::LibraryLines(*appState_, isDark)) {
        switch (line.kind) {
            case Kind::Section:
                items.Append(winrt::make<Models::implementation::LibraryHeaderItem>(line.title));
                break;
            case Kind::Album:
                items.Append(winrt::make<Models::implementation::AlbumGroupItem>(line.title));
                break;
            case Kind::Track:
                items.Append(winrt::make<Models::implementation::TrackItem>(
                    line.row, line.inAlbum ? kAlbumTrackIndent : 0.0));  // #339
                break;
        }
    }
    ShowEmptyMessage(items.Size() == 0);
    trackList().ItemsSource(items);
}

void LibraryView::RenderViewSwitch() {
    struct Slot {
        RadioButton button;
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
        // #516: checked mirrors the view state, so UI Automation reads the
        // selected segment through the radio button's selection item.
        slot.button.IsChecked(segment.selected);
        slot.back.Style(style(segment.selected ? L"SegmentBackSelectedStyle" : L"SegmentBackStyle"));
        slot.label.Style(style(segment.selected ? L"SegmentLabelSelectedStyle" : L"SegmentLabelStyle"));
    }
}

void LibraryView::ShowEmptyMessage(bool show) {
    emptyState().Visibility(show ? Visibility::Visible : Visibility::Collapsed);
    trackList().Visibility(show ? Visibility::Collapsed : Visibility::Visible);
}

} // namespace winrt::Rhythm::Views::implementation
