#include "BehaviorPch.h"
#include "ViewState.h"
#include "L10n.h"

#include <tuple>

namespace rhythm::view {

namespace {

/// Seconds as m:ss, seconds zero-padded; anything not positive is 0:00.
std::wstring MinutesSeconds(double seconds) {
    const int whole = seconds > 0 ? static_cast<int>(seconds) : 0;
    return std::format(L"{}:{:02}", whole / 60, whole % 60);
}

/// The source badge palette (#184/#219/#249). The three marked regions are
/// written by scripts/gen-palette.py from testing/palette.json and compared
/// byte for byte by testing/l0/check-palette.py -- change the palette, not
/// these lines. Moved here from the Track model with the badge (#338).
struct SourcePalette {
    struct RGB {
        uint8_t r, g, b;
    };

    static std::optional<RGB> Lookup(std::wstring_view sourceType, bool isDarkTheme) {
        struct Entry {
            std::wstring_view name;
            RGB dark, light;
        };
        // BEGIN GENERATED SOURCE TABLE (#184) — 由 scripts/gen-palette.py 生成，勿手改
        static constexpr Entry kTable[] = {
            {L"bilibili", {0xC8, 0x8D, 0xA8}, {0x8C, 0x4D, 0x68}},
            {L"direct_url", {0x8C, 0xB8, 0x9A}, {0x4C, 0x78, 0x5A}},
            {L"local", {0x8A, 0xBC, 0xD0}, {0x3A, 0x7A, 0x8C}},
            {L"youtube", {0xD4, 0x95, 0x73}, {0x8B, 0x4A, 0x28}},
        };
        // END GENERATED SOURCE TABLE (#184)
        for (const auto& e : kTable) {
            if (e.name == sourceType) {
                return isDarkTheme ? e.dark : e.light;
            }
        }
        return std::nullopt;
    }

    // BEGIN GENERATED SOURCE FALLBACK (#219) — 由 scripts/gen-palette.py 生成，勿手改
    // 未知来源回退到正文色（rhythmTextPrimary），绝不返回系统 Gray（F4）
    static constexpr const wchar_t* kUnknownSourceDark = L"#ABC8D4";
    static constexpr const wchar_t* kUnknownSourceLight = L"#0D464D";
    // END GENERATED SOURCE FALLBACK (#219)

    // BEGIN GENERATED BADGE BACKGROUND (#249) — 由 scripts/gen-palette.py 生成，勿手改
    // 胶囊底 = 徽标前景色 @ 0.15（与 macOS `.background(color.opacity(0.15))` 同一声明）
    static constexpr uint8_t kSourceBadgeBackgroundAlpha = 38;
    // END GENERATED BADGE BACKGROUND (#249)
};

/// "#RRGGBB" (the generated fallback constants) -> opaque colour.
constexpr Color OpaqueFromHex(std::wstring_view hex) {
    auto nibble = [](wchar_t c) -> uint8_t {
        return static_cast<uint8_t>(c <= L'9' ? c - L'0' : (c | 0x20) - L'a' + 10);
    };
    auto byte = [&](size_t at) {
        return static_cast<uint8_t>(nibble(hex[at]) << 4 | nibble(hex[at + 1]));
    };
    return Color{0xFF, byte(1), byte(3), byte(5)};
}

/// The play mode control's icon, like macOS `PlayMode.icon` (#411).
Icon PlayModeIcon(PlayMode mode) {
    switch (mode) {
        case PlayMode::Shuffle:    return Icon::Shuffle;
        case PlayMode::SingleLoop: return Icon::RepeatOne;
        case PlayMode::ListLoop:   return Icon::RepeatAll;
        case PlayMode::Sequential:
        default:                   return Icon::List;
    }
}

} // namespace

SourceBadge SourceBadgeOf(std::wstring_view sourceType, bool isDarkTheme) {
    SourceBadge badge;
    badge.tag = L10n::SourceTag(std::wstring(sourceType));
    if (auto rgb = SourcePalette::Lookup(sourceType, isDarkTheme)) {
        badge.foreground = Color{0xFF, rgb->r, rgb->g, rgb->b};
    } else {
        badge.foreground = OpaqueFromHex(isDarkTheme ? SourcePalette::kUnknownSourceDark
                                                     : SourcePalette::kUnknownSourceLight);
    }
    badge.background = badge.foreground;
    badge.background.A = SourcePalette::kSourceBadgeBackgroundAlpha;
    return badge;
}

namespace {

/// One track as a list row -- the single place a row is built (#339).
TrackRow RowOf(const Track& track, bool isDarkTheme) {
    return TrackRow{track, track.title, track.artist.value_or(L""),
                    MinutesSeconds(track.duration),  // #337
                    SourceBadgeOf(track.sourceType, isDarkTheme)};
}

std::vector<TrackRow> RowsOf(const std::vector<Track>& tracks, bool isDarkTheme) {
    std::vector<TrackRow> rows;
    rows.reserve(tracks.size());
    for (const auto& track : tracks) {
        rows.push_back(RowOf(track, isDarkTheme));
    }
    return rows;
}

} // namespace

std::vector<TrackRow> LibraryRows(const AppState& state, LibrarySort sort, bool isDarkTheme) {
    auto rows = RowsOf(state.Tracks, isDarkTheme);

    switch (sort) {
        case LibrarySort::ArtistAlbum: {
            auto key = [](const Track& t) {
                return std::tuple(t.artist.value_or(L""), t.album.value_or(L""),
                                  t.trackNumber.value_or(0));
            };
            std::stable_sort(rows.begin(), rows.end(), [&](const TrackRow& a, const TrackRow& b) {
                return key(a.track) < key(b.track);
            });
            break;
        }
        case LibrarySort::Alphabetical:
            std::stable_sort(rows.begin(), rows.end(), [](const TrackRow& a, const TrackRow& b) {
                return a.track.title < b.track.title;
            });
            break;
    }
    return rows;
}

std::vector<TrackRow> PlaylistRows(const AppState& state, int64_t playlistId, bool isDarkTheme) {
    auto playlist = state.FindPlaylist(playlistId);
    return playlist ? RowsOf(playlist->tracks, isDarkTheme) : std::vector<TrackRow>{};
}

PlayerBar PlayerBarState(const AppState& state) {
    PlayerBar bar;
    if (state.CurrentTrack) {
        bar.title = state.CurrentTrack->title;
        bar.artist = state.CurrentTrack->artist.value_or(L"");
    } else {
        bar.title = L10n::NotPlaying();
    }
    if (state.Duration > 0) {
        bar.progressPercent = std::clamp(state.Position / state.Duration * 100.0, 0.0, 100.0);
    }
    bar.timeText = state.IsBuffering
        ? L10n::Buffering()
        : MinutesSeconds(state.Position) + L" / " + MinutesSeconds(state.Duration);
    bar.playIcon = state.IsPlaying || state.IsBuffering ? Icon::Pause : Icon::Play;
    bar.playModeIcon = PlayModeIcon(state.CurrentMode);
    bar.volumePercent = state.Volume * 100.0;
    if (state.IsResolvingUrl) {
        auto progress = state.ResolverStatusText();
        bar.urlStatusText = progress.empty() ? L10n::Resolving() : progress;
    }
    return bar;
}

TrayMenu TrayMenuState(const AppState& state) {
    // #141: tray copy follows the language layer like everything else.
    return TrayMenu{L10n::TrayPlayPause(), L10n::TrayShowWindow(), L10n::TrayQuit(),
                    state.CanTogglePlayback()};
}

} // namespace rhythm::view
