#pragma once

#include "pch.h"

namespace rhythm {

// Track and Playlist models are defined in Bridge/RhythmCore.h
// This file provides additional view-model helpers.

struct TrackViewModel {
    rhythm::Track track;

    winrt::hstring Title() const { return track.title; }
    winrt::hstring Artist() const { return track.artist ? winrt::hstring(*track.artist) : L""; }
    winrt::hstring Album() const { return track.album ? winrt::hstring(*track.album) : L""; }
    winrt::hstring DurationText() const { return track.DurationFormatted(); }
    winrt::hstring SourceTag() const { return track.SourceTag(); }
    winrt::hstring SourceColor() const { return track.SourceColor(); }
    int32_t TrackCount() const { return static_cast<int32_t>(track.playCount); }
};

} // namespace rhythm
