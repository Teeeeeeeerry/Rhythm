#include "pch.h"
#include "Models/TrackItem.h"
#include "ViewState.h"
#if __has_include("Models/TrackItem.g.cpp")
#include "Models/TrackItem.g.cpp"
#endif

namespace winrt::Rhythm::Models::implementation {

namespace {

/// The view state's plain colour as a XAML brush (#328/#338: brushes are a
/// shell concern; the view state only knows bytes).
winrt::Microsoft::UI::Xaml::Media::SolidColorBrush ToBrush(rhythm::view::Color c) {
    return winrt::Microsoft::UI::Xaml::Media::SolidColorBrush(
        winrt::Windows::UI::Color{c.A, c.R, c.G, c.B});
}

} // namespace

TrackItem::TrackItem(rhythm::Track track, bool isDarkTheme)
    : track_(std::move(track)), isDarkTheme_(isDarkTheme) {}

hstring TrackItem::Title() const { return hstring{track_.title}; }

hstring TrackItem::Artist() const { return hstring{track_.artist.value_or(L"")}; }

// The badge comes from the view state (#338); rows bind to it directly from #339.
hstring TrackItem::SourceTag() const {
    return hstring{rhythm::view::SourceBadgeOf(track_.sourceType, isDarkTheme_).tag};
}

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceForeground() const {
    return ToBrush(rhythm::view::SourceBadgeOf(track_.sourceType, isDarkTheme_).foreground);
}

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceBackground() const {
    return ToBrush(rhythm::view::SourceBadgeOf(track_.sourceType, isDarkTheme_).background);
}

hstring TrackItem::DurationText() const { return hstring{rhythm::view::DurationText(track_)}; }

} // namespace winrt::Rhythm::Models::implementation
