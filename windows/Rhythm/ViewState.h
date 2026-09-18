#pragma once

// View state (#317/#331): what the UI renders, decided by pure functions of
// AppState that return plain structs -- no UI framework type in or out. The
// tests assert these values; the XAML shell only copies them into controls.

#include "BehaviorPch.h"
#include "AppState.h"

namespace rhythm::view {

/// One row of a track list.
struct TrackRow {
    /// The track the row plays when clicked.
    Track track;
    std::wstring title;
};

/// The library list: one row per loaded library track.
std::vector<TrackRow> LibraryRows(const AppState& state);

} // namespace rhythm::view
