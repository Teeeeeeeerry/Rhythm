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

/// One row of a track list -- the same row for the library and a playlist's
/// detail (#339). `track` is the row's action payload -- what a click hands
/// to `AppState::PlayTrack`, never rendered; the other fields are exactly
/// what the row shows, and the shell's row model only copies them.
struct TrackRow {
    Track track;
    std::wstring title;
    /// Empty when the track has none.
    std::wstring artist;
    /// m:ss, seconds zero-padded; past an hour the minutes keep counting
    /// (62:05), as on macOS (#337).
    std::wstring durationText;
    /// The track's source badge in the rendered theme (#338).
    SourceBadge badge;
};

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

/// A playlist's detail list: one row per track, in playlist order (#339).
/// No rows when the state holds no playlist with that id.
std::vector<TrackRow> PlaylistRows(const AppState& state, int64_t playlistId, bool isDarkTheme);

/// What the playlist detail page renders (#358). Read from the state's
/// current playlist on every render, so the page holds nothing of its own --
/// not a pointer, not an id.
struct PlaylistDetail {
    /// True when a playlist is selected; false leaves the other fields empty.
    bool hasPlaylist = false;
    /// The current playlist's name.
    std::wstring title;
    /// Its tracks, the same rows as the library, in playlist order.
    std::vector<TrackRow> rows;
};

PlaylistDetail PlaylistDetailOf(const AppState& state, bool isDarkTheme);

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
    /// The line under the URL box: empty unless a link is being resolved;
    /// then the provisioning progress (a first-use yt-dlp download reads as
    /// progress, not a stall), or the plain "resolving" copy when the
    /// resolver has nothing to report (#317).
    std::wstring urlStatusText;
};

/// The provisioning copy comes from `AppState::ResolverStatusText()` (#350),
/// so the player bar never queries the resolver; tests drive it through the
/// app state's `PollResolverStatus`.
PlayerBar PlayerBarState(const AppState& state);

/// The notification-area menu (#340): its item labels, in the language the
/// language layer currently resolves, and whether play/pause is available.
/// The tray builds the native menu from this and gates the play/pause command
/// on the same value (#341).
struct TrayMenu {
    std::wstring playPause;
    std::wstring showWindow;
    std::wstring quit;
    /// Whether "play / pause" can do anything -- the coordinator's own
    /// availability query (#138/#341): off for an empty library with nothing
    /// current, so a dead click never claims playback.
    bool playPauseEnabled = false;
};

TrayMenu TrayMenuState(const AppState& state);

/// A feedback dialog's two texts (#352).
struct Alert {
    std::wstring title;
    std::wstring message;
};

/// The feedback a finished import or export asks for (#352): the export
/// alert with its own title, else the import alert under the import result
/// title; nothing when neither is pending. The shell shows it, then calls
/// `AppState::DismissAlerts`.
std::optional<Alert> PendingAlert(const AppState& state);

} // namespace rhythm::view
