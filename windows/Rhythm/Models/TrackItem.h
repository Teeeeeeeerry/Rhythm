#pragma once

#include "Models/TrackItem.g.h"
#include "Bridge/RhythmCore.h"
#include "ViewState.h"

namespace winrt::Rhythm::Models::implementation {

/// A track-list row for x:Bind (#428): wraps a view-state row and only copies
/// its fields into bindings -- what to render is decided by the view state
/// (#339). The library and a playlist's detail share it.
struct TrackItem : TrackItemT<TrackItem> {
    /// `indent` is the row's left inset in pixels: the shell sets it for a
    /// track drawn inside an album group (#503).
    explicit TrackItem(rhythm::view::TrackRow row, double indent = 0.0);

    hstring Title() const;
    hstring Artist() const;
    bool HasArtist() const;
    double Indent() const;
    hstring SourceTag() const;
    winrt::Microsoft::UI::Xaml::Media::Brush SourceForeground() const;
    winrt::Microsoft::UI::Xaml::Media::Brush SourceBackground() const;
    hstring DurationText() const;

    /// The row's track (not projected): what the click handlers play.
    rhythm::Track const& Model() const { return row_.track; }

private:
    rhythm::view::TrackRow row_;
    double indent_ = 0.0;
};

} // namespace winrt::Rhythm::Models::implementation
