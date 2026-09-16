#pragma once

#include "pch.h"
#include "Bridge/RhythmCore.h"

namespace rhythm {

// Track and Playlist models are defined in Bridge/RhythmCore.h (included above, #408).
// This file provides additional view-model helpers.

struct TrackViewModel {
    rhythm::Track track;

    winrt::hstring Title() const { return winrt::hstring{track.title}; }
    winrt::hstring Artist() const { return track.artist ? winrt::hstring(*track.artist) : L""; }
    winrt::hstring Album() const { return track.album ? winrt::hstring(*track.album) : L""; }
    winrt::hstring DurationText() const { return winrt::hstring{track.DurationFormatted()}; }
    winrt::hstring SourceTag() const { return winrt::hstring{track.SourceTag()}; }
    winrt::hstring SourceColor() const { return winrt::hstring{track.SourceColor()}; }
    int32_t TrackCount() const { return static_cast<int32_t>(track.playCount); }
};

} // namespace rhythm
