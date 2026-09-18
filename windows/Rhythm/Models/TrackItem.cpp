#include "pch.h"
#include "Models/TrackItem.h"
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

TrackItem::TrackItem(rhythm::view::TrackRow row) : row_(std::move(row)) {}

hstring TrackItem::Title() const { return hstring{row_.title}; }

hstring TrackItem::Artist() const { return hstring{row_.artist}; }

hstring TrackItem::SourceTag() const { return hstring{row_.badge.tag}; }

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceForeground() const {
    return ToBrush(row_.badge.foreground);
}

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceBackground() const {
    return ToBrush(row_.badge.background);
}

hstring TrackItem::DurationText() const { return hstring{row_.durationText}; }

} // namespace winrt::Rhythm::Models::implementation
