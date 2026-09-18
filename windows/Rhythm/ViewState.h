#pragma once

// View state (#317/#331): what the UI renders, decided by pure functions of
// AppState that return plain structs -- no UI framework type in or out. The
// tests assert these values; the XAML shell only copies them into controls.

#include "BehaviorPch.h"
#include "AppState.h"

namespace rhythm::view {

/// One row of a track list. `track` is the row's action payload -- what a
/// click hands to `AppState::PlayTrack`, never rendered; the other fields are
/// exactly what the row shows, so the shell reads nothing off the model.
struct TrackRow {
    Track track;
    std::wstring title;
};

/// The library list: one row per loaded library track.
std::vector<TrackRow> LibraryRows(const AppState& state);

/// What the player bar shows.
struct PlayerBar {
    /// The current track's title and artist (empty when it has none); with
    /// no current track, the not-playing copy and an empty artist -- never
    /// the previous track's values (#334).
    std::wstring title;
    std::wstring artist;
    /// Progress bar value, 0-100. Zero while the duration is unknown -- no
    /// made-up progress -- and clamped when the position runs past it (#332).
    double progressPercent = 0.0;
    /// The buffering copy while buffering -- a link can take a while to
    /// start, and 0:00 / 0:00 reads as a dead player (#137) -- otherwise
    /// "position / duration" in m:ss (#333).
    std::wstring timeText;
};

PlayerBar PlayerBarState(const AppState& state);

} // namespace rhythm::view
