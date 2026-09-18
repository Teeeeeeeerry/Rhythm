#pragma once

#include "Models/TrackItem.g.h"
#include "Bridge/RhythmCore.h"

namespace winrt::Rhythm::Models::implementation {

/// A library row for x:Bind (#428): wraps the behaviour-library Track. The
/// theme is resolved by the view at render time and passed in (#342).
struct TrackItem : TrackItemT<TrackItem> {
    TrackItem(rhythm::Track track, bool isDarkTheme);

    hstring Title() const;
    hstring Artist() const;
    hstring SourceTag() const;
    winrt::Microsoft::UI::Xaml::Media::Brush SourceForeground() const;
    winrt::Microsoft::UI::Xaml::Media::Brush SourceBackground() const;
    hstring DurationText() const;

    /// The wrapped model (not projected): what the click handlers play.
    rhythm::Track const& Model() const { return track_; }

private:
    rhythm::Track track_;
    bool isDarkTheme_;
};

} // namespace winrt::Rhythm::Models::implementation
