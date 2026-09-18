#pragma once

// View state (#317/#331): what the UI renders, decided by pure functions of
// AppState that return plain structs -- no UI framework type in or out. The
// tests assert these values; the XAML shell only copies them into controls.

#include "BehaviorPch.h"
#include "AppState.h"

namespace rhythm::view {

/// A UI-free icon name (#329/#335): the XAML shell maps it to its own symbol
/// set, so the view state carries no WinUI type.
enum class Icon { Play, Pause, List, Shuffle, RepeatOne, RepeatAll };

/// A colour as plain bytes (#328/#338). Field names and order mirror
/// `Windows::UI::Color` on purpose, so the shell converts field by field when
/// it builds a brush.
struct Color {
    uint8_t A, R, G, B;
};

/// A source badge's three render values (#338): the tag, its colour, and the
/// capsule behind it -- the colour at the palette's declared badge opacity,
/// like macOS `.background(color.opacity(...))`. Colours come from the
/// generated source palette (testing/palette.json); an unknown source falls
/// back to the body text colour, never system grey (F4).
struct SourceBadge {
    std::wstring tag;
    Color foreground;
    Color background;
};

SourceBadge SourceBadgeOf(std::wstring_view sourceType, bool isDarkTheme);

/// One row of a track list. `track` is the row's action payload -- what a
/// click hands to `AppState::PlayTrack`, never rendered; the other fields are
/// exactly what the row shows. The list views bind to them from #339; until
/// then the shell's row model still derives its text from the track.
struct TrackRow {
    Track track;
    std::wstring title;
    /// m:ss, seconds zero-padded; past an hour the minutes keep counting
    /// (62:05), as on macOS (#337).
    std::wstring durationText;
    /// The track's source badge in the rendered theme (#338).
    SourceBadge badge;
};

/// A track's duration as a row shows it (#337). Until the list rows are
/// rebound to TrackRow (#339), the shell's row model reads it from here.
std::wstring DurationText(const Track& track);

/// The library's two orders (#336).
enum class LibrarySort {
    /// By artist, then album, then track number. A missing artist or album
    /// sorts as an empty name, a missing track number as 0.
    ArtistAlbum,
    /// By title, ascending.
    Alphabetical,
};

/// The library list: one row per loaded library track, in `sort` order.
/// Stable -- rows that compare equal keep their library order -- and the
/// state is only read (#336). The theme is resolved by the shell (#342).
std::vector<TrackRow> LibraryRows(const AppState& state, LibrarySort sort, bool isDarkTheme);

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
    /// Pause while playing -- buffering counts as playing -- play otherwise
    /// (#335).
    Icon playIcon = Icon::Play;
    /// The play mode control's icon (#411, moved here from AppState by #335).
    Icon playModeIcon = Icon::List;
    /// The volume slider's value, 0-100 (#335).
    double volumePercent = 0.0;
};

PlayerBar PlayerBarState(const AppState& state);

} // namespace rhythm::view
