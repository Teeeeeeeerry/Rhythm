#pragma once

// View state (#317/#331): what the UI renders, decided by pure functions of
// AppState that return plain structs -- no UI framework type in or out. The
// tests assert these values; the XAML shell only copies them into controls.

#include "BehaviorPch.h"
#include "AppState.h"

namespace rhythm::view {

/// A UI-free icon name (#329/#335): the XAML shell maps it to its own symbol
/// set, so the view state carries no WinUI type.
enum class Icon { Play, Pause, List, Shuffle, RepeatOne, RepeatAll, Library, Playlists };

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

/// One entry of the always-expanded sidebar (#501): the page it selects, its
/// icon and label (both shown -- there is no icon-only state), and whether it
/// is the selected page, which the shell highlights in the brand accent like
/// the macOS sidebar.
struct SidebarEntry {
    SidebarItem item = SidebarItem::Library;
    Icon icon = Icon::Library;
    std::wstring label;
    bool selected = false;
};

/// The sidebar's entries, in macOS order: library, then playlists. Exactly
/// the entry of `AppState::SelectedView` is selected.
std::vector<SidebarEntry> SidebarState(const AppState& state);

/// The library's two orders (#336), declared with the state that holds the
/// chosen one (`AppState::LibraryOrder`, #502).
using LibrarySort = ::rhythm::LibrarySort;

/// The library list: one row per loaded library track, in `sort` order.
/// Stable -- rows that compare equal keep their library order -- and the
/// state is only read (#336). By artist/album it is the grouping of
/// `ArtistAlbumSections` read top to bottom (#503), by letter that of
/// `LetterSections` (#504), so the list and its sections never disagree.
/// The theme is resolved by the shell (#342).
std::vector<TrackRow> LibraryRows(const AppState& state, LibrarySort sort, bool isDarkTheme);

/// One album group of the artist/album view (#503): the album's name (the
/// unknown-album label when the tracks have none) and its tracks, by disc
/// then track number -- a missing number counts as 0 -- ties in library
/// order. The shell draws a cover placeholder beside each group.
struct AlbumGroup {
    std::wstring title;
    std::vector<TrackRow> rows;
};

/// One artist section of the artist/album view (#503): the artist's name
/// (the unknown-artist label when the tracks have none) and its albums, by
/// name.
struct ArtistSection {
    std::wstring title;
    std::vector<AlbumGroup> albums;
};

/// The artist/album view's structure, as macOS `groupByArtistAlbum` builds
/// it: artists by name, albums by name within each, the same album name
/// under two artists kept apart. The unknown labels come from the key table
/// and are grouped and sorted like any other name. The theme is resolved by
/// the shell (#342).
std::vector<ArtistSection> ArtistAlbumSections(const AppState& state, bool isDarkTheme);

/// One section of the by-letter view (#504): its heading and its tracks,
/// by title ignoring case (macOS `localizedCaseInsensitiveCompare`), ties in
/// library order.
struct LetterSection {
    std::wstring title;
    std::vector<TrackRow> rows;
};

/// The by-letter view's structure, as macOS `groupByFirstLetter` builds it:
/// a track's section is its title's first character upper-cased when that is
/// a letter -- any Unicode letter, CJK included -- and `#` otherwise (a
/// digit, punctuation, an empty title). Sections in heading order, so `#`
/// comes before the letters.
std::vector<LetterSection> LetterSections(const AppState& state, bool isDarkTheme);

/// The kinds of line the library list shows (#503).
enum class LibraryLineKind { Section, Album, Track };

/// One line of the library list (#503): a section heading or an album
/// heading (`title`), or a track (`row`). `inAlbum` marks a track drawn
/// inside an album group, which the shell lines up beside the group's cover
/// placeholder.
struct LibraryLine {
    LibraryLineKind kind = LibraryLineKind::Track;
    std::wstring title;
    TrackRow row;
    bool inAlbum = false;
};

/// What the library page lists, line by line, in the state's order (#503):
/// by artist/album the grouping of `ArtistAlbumSections` -- each section's
/// heading, then each album's heading followed by its tracks; by letter each
/// `LetterSections` heading followed by its tracks (#504). Empty for an
/// empty library. The page only turns each line into a list item.
std::vector<LibraryLine> LibraryLines(const AppState& state, bool isDarkTheme);

/// The library list in the order the state holds (#502): what the library
/// page renders, so the page never picks an order of its own.
std::vector<TrackRow> LibraryRows(const AppState& state, bool isDarkTheme);

/// One segment of the library's view switch (#502): the order it selects,
/// its label, and whether it is the order shown.
struct ViewSwitchSegment {
    LibrarySort sort = LibrarySort::ArtistAlbum;
    std::wstring label;
    bool selected = false;
};

/// The library page's single view switch, labelled as on macOS: by
/// artist/album, then by letter. Exactly the segment of
/// `AppState::LibraryOrder` is selected; a click sets that field to the
/// segment's `sort`.
std::vector<ViewSwitchSegment> LibraryViewSwitch(const AppState& state);

/// One row of the playlist list (#320). Carries the identifier a click hands
/// to `AppState::SelectPlaylist` -- never an address inside the state -- plus
/// the two values the row shows.
struct PlaylistRow {
    int64_t id = 0;
    std::wstring name;
    /// The playlist's track count, as shown.
    std::wstring trackCountText;
};

/// What the playlist list page renders (#320): one row per loaded playlist
/// that has an identifier, in list order, and the empty-state copy when there
/// are none.
struct PlaylistListPage {
    std::vector<PlaylistRow> rows;
    /// The key table's "no playlists yet" copy; empty when there are rows.
    std::wstring emptyMessage;
};

PlaylistListPage PlaylistListState(const AppState& state);

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
    /// The empty-state copy from the key table (#359): set exactly when no
    /// playlist is selected, so the page shows something determinate instead
    /// of an empty frame.
    std::wstring emptyMessage;
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
    /// Whether the progress bar can be dragged (#498): only with a current
    /// track whose duration is known -- an unknown duration has no positions
    /// to seek to.
    bool seekable = false;
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

/// The position, in seconds, a progress bar value (0-100) stands for (#498):
/// the value is clamped into range, and an unknown duration maps to zero.
/// The player bar hands it to `AppState::Seek` when a drag ends.
double SeekPosition(const AppState& state, double percent);

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
