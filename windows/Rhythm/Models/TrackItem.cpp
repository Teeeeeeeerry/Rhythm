#include "pch.h"
#include "Models/TrackItem.h"
#if __has_include("Models/TrackItem.g.cpp")
#include "Models/TrackItem.g.cpp"
#endif

namespace winrt::Rhythm::Models::implementation {

TrackItem::TrackItem(rhythm::Track track) : track_(std::move(track)) {}

hstring TrackItem::Title() const { return hstring{track_.title}; }

hstring TrackItem::Artist() const { return hstring{track_.artist.value_or(L"")}; }

hstring TrackItem::SourceTag() const { return hstring{track_.SourceTag()}; }

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceForeground() const {
    return track_.SourceForegroundBrush();
}

winrt::Microsoft::UI::Xaml::Media::Brush TrackItem::SourceBackground() const {
    return track_.SourceBackgroundBrush();
}

hstring TrackItem::DurationText() const { return hstring{track_.DurationFormatted()}; }

} // namespace winrt::Rhythm::Models::implementation
