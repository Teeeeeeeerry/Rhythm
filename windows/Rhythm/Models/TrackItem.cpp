#include "pch.h"
#include "Models/TrackItem.h"
#if __has_include("Models/TrackItem.g.cpp")
#include "Models/TrackItem.g.cpp"
#endif

namespace winrt::Rhythm::Models::implementation {

namespace {

/// "#RRGGBB" (the bridge's SourceColor format) -> opaque colour.
winrt::Windows::UI::Color ParseHexColor(std::wstring const& hex) {
    auto channel = [&](size_t at) {
        return static_cast<uint8_t>(std::stoi(hex.substr(at, 2), nullptr, 16));
    };
    return winrt::Windows::UI::Color{0xFF, channel(1), channel(3), channel(5)};
}

} // namespace

TrackItem::TrackItem(rhythm::Track track) : track_(std::move(track)) {}

hstring TrackItem::Title() const { return hstring{track_.title}; }

hstring TrackItem::Artist() const { return hstring{track_.artist.value_or(L"")}; }

hstring TrackItem::SourceTag() const { return hstring{track_.SourceTag()}; }

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceForeground() const {
    return winrt::Microsoft::UI::Xaml::Media::SolidColorBrush(ParseHexColor(track_.SourceColor()));
}

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceBackground() const {
    return track_.SourceBackgroundBrush();
}

hstring TrackItem::DurationText() const { return hstring{track_.DurationFormatted()}; }

} // namespace winrt::Rhythm::Models::implementation
