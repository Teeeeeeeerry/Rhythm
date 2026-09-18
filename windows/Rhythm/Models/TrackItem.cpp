#include "pch.h"
#include "Models/TrackItem.h"
#if __has_include("Models/TrackItem.g.cpp")
#include "Models/TrackItem.g.cpp"
#endif

namespace winrt::Rhythm::Models::implementation {

namespace {

/// The model's plain colour as a XAML brush (#328: brushes are a shell
/// concern; the model only knows bytes). Theme resolved at render time.
winrt::Microsoft::UI::Xaml::Media::SolidColorBrush ToBrush(rhythm::Color c) {
    return winrt::Microsoft::UI::Xaml::Media::SolidColorBrush(
        winrt::Windows::UI::Color{c.A, c.R, c.G, c.B});
}

} // namespace

TrackItem::TrackItem(rhythm::Track track) : track_(std::move(track)) {}

hstring TrackItem::Title() const { return hstring{track_.title}; }

hstring TrackItem::Artist() const { return hstring{track_.artist.value_or(L"")}; }

hstring TrackItem::SourceTag() const { return hstring{track_.SourceTag()}; }

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceForeground() const {
    return ToBrush(track_.SourceForegroundColor(rhythm::IsDarkTheme()));
}

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceBackground() const {
    return ToBrush(track_.SourceBackgroundColor(rhythm::IsDarkTheme()));
}

hstring TrackItem::DurationText() const { return hstring{track_.DurationFormatted()}; }

} // namespace winrt::Rhythm::Models::implementation
