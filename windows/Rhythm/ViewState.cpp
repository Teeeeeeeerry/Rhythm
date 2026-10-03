#include "BehaviorPch.h"
#include "ViewState.h"
#include "L10n.h"

#include <iterator>
#include <map>

// Types and constants only: the functions are resolved at run time from the
// system's icu.dll (see SystemIcu), so nothing links against it (#504).
#include <icu.h>

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

/// The heading of every title that does not start with a letter (#504).
constexpr wchar_t kNonLetterHeading[] = L"#";

/// The system's ICU (#504), for what the Win32 character types cannot tell:
/// whether a code point outside the BMP is a letter, and its full upper-case
/// mapping. Resolved at run time: icu.dll ships from Windows 10 1903 while
/// the app runs from 1809 (WindowsTargetPlatformMinVersion), so a missing
/// library falls back to the Win32 calls instead of keeping the app from
/// loading.
struct SystemIcu {
    decltype(&u_hasBinaryProperty) hasBinaryProperty = nullptr;
    decltype(&u_strToUpper) strToUpper = nullptr;

    static const SystemIcu& Get() {
        static const SystemIcu icu = [] {
            SystemIcu loaded;
            if (HMODULE module = ::LoadLibraryExW(L"icu.dll", nullptr,
                                                  LOAD_LIBRARY_SEARCH_SYSTEM32)) {
                loaded.hasBinaryProperty = reinterpret_cast<decltype(&u_hasBinaryProperty)>(
                    ::GetProcAddress(module, "u_hasBinaryProperty"));
                loaded.strToUpper = reinterpret_cast<decltype(&u_strToUpper)>(
                    ::GetProcAddress(module, "u_strToUpper"));
            }
            if (!loaded.hasBinaryProperty || !loaded.strToUpper) return SystemIcu{};
            return loaded;
        }();
        return icu;
    }
};

/// The title in composed form (NFC): a letter written as base + combining
/// accent, as tags from macOS often are, becomes the one code point Swift
/// treats it as. Unchanged when it cannot be normalised.
std::wstring Composed(const std::wstring& title) {
    const int needed = ::NormalizeString(NormalizationC, title.c_str(),
                                         static_cast<int>(title.size()), nullptr, 0);
    if (needed <= 0) return title;
    std::wstring composed(static_cast<size_t>(needed), L' ');
    const int written = ::NormalizeString(NormalizationC, title.c_str(),
                                          static_cast<int>(title.size()), composed.data(), needed);
    if (written <= 0) return title;
    composed.resize(static_cast<size_t>(written));
    return composed;
}

/// The by-letter heading of a title (#504), as macOS `groupByFirstLetter`
/// derives it: the first character of the composed title, full upper-case
/// mapped, when it has the Unicode Alphabetic property -- what Swift's
/// `Character.isLetter` tests -- else "#".
std::wstring LetterOf(const std::wstring& rawTitle) {
    const std::wstring title = Composed(rawTitle);
    if (title.empty()) return kNonLetterHeading;
    const auto* text = reinterpret_cast<const UChar*>(title.c_str());
    int32_t end = 0;
    UChar32 first = 0;
    U16_NEXT(text, end, static_cast<int32_t>(title.size()), first);

    const auto& icu = SystemIcu::Get();
    if (icu.hasBinaryProperty) {
        if (!icu.hasBinaryProperty(first, UCHAR_ALPHABETIC)) return kNonLetterHeading;
        UChar upper[8] = {};  // one code point maps to at most three
        UErrorCode status = U_ZERO_ERROR;
        const int32_t written = icu.strToUpper(upper, 8, text, end, "", &status);
        if (U_FAILURE(status) || written <= 0 || written > 8) return title.substr(0, end);
        return std::wstring(reinterpret_cast<const wchar_t*>(upper), static_cast<size_t>(written));
    }

    // Without ICU: the Win32 character types, which see BMP letters only.
    WORD type = 0;
    if (end != 1 || !::GetStringTypeW(CT_CTYPE1, title.c_str(), 1, &type) || !(type & C1_ALPHA)) {
        return kNonLetterHeading;
    }
    wchar_t upper[4] = {};
    const int written = ::LCMapStringEx(LOCALE_NAME_INVARIANT, LCMAP_UPPERCASE, title.c_str(), 1,
                                        upper, 4, nullptr, nullptr, 0);
    return written > 0 ? std::wstring(upper, static_cast<size_t>(written)) : title.substr(0, 1);
}

/// Heading order by code point, as Swift compares strings (#504). UTF-16
/// code units disagree with it in one place -- a surrogate (a letter outside
/// the BMP) is smaller than U+E000..U+FFFF -- so those two ranges swap ranks.
struct CodePointLess {
    static uint32_t Rank(wchar_t unit) {
        if (unit < 0xD800) return unit;
        return unit >= 0xE000 ? unit - 0x800u : unit + 0x2000u;
    }
    bool operator()(const std::wstring& a, const std::wstring& b) const {
        return std::lexicographical_compare(a.begin(), a.end(), b.begin(), b.end(),
                                            [](wchar_t x, wchar_t y) { return Rank(x) < Rank(y); });
    }
};

/// Title order within a letter section: ignoring case, in the user's locale
/// (macOS `localizedCaseInsensitiveCompare` uses the current locale too),
/// punctuation counted as it is there -- string sort, not word sort.
bool TitleBefore(const std::wstring& a, const std::wstring& b) {
    return ::CompareStringEx(LOCALE_NAME_USER_DEFAULT, LINGUISTIC_IGNORECASE | SORT_STRINGSORT,
                             a.c_str(), static_cast<int>(a.size()), b.c_str(),
                             static_cast<int>(b.size()), nullptr, nullptr, 0) == CSTR_LESS_THAN;
}

} // namespace

std::vector<LetterSection> LetterSections(const AppState& state, bool isDarkTheme) {
    // #504: sectioned as macOS groupByFirstLetter does.
    std::map<std::wstring, std::vector<TrackRow>, CodePointLess> letters;
    for (auto& row : RowsOf(state.Tracks, isDarkTheme)) {
        letters[LetterOf(row.track.title)].push_back(std::move(row));
    }
    std::vector<LetterSection> sections;
    for (auto& [letter, rows] : letters) {
        std::stable_sort(rows.begin(), rows.end(), [](const TrackRow& a, const TrackRow& b) {
            return TitleBefore(a.track.title, b.track.title);
        });
        sections.push_back(LetterSection{letter, std::move(rows)});
    }
    return sections;
}

std::vector<ArtistSection> ArtistAlbumSections(const AppState& state, bool isDarkTheme) {
    // #503: grouped as macOS groupByArtistAlbum does -- by display name, so a
    // missing artist or album is a group named by its label and sorts by it.
    const std::wstring unknownArtist = L10n::UnknownArtist();
    const std::wstring unknownAlbum = L10n::UnknownAlbum();
    std::map<std::wstring, std::map<std::wstring, std::vector<TrackRow>>> artists;
    for (auto& row : RowsOf(state.Tracks, isDarkTheme)) {
        const auto artist = row.track.artist.value_or(unknownArtist);
        const auto album = row.track.album.value_or(unknownAlbum);
        artists[artist][album].push_back(std::move(row));
    }

    auto position = [](const TrackRow& row) {
        return std::pair(row.track.discNumber.value_or(0), row.track.trackNumber.value_or(0));
    };
    std::vector<ArtistSection> sections;
    for (auto& [artist, albums] : artists) {
        ArtistSection section{artist, {}};
        for (auto& [album, rows] : albums) {
            std::stable_sort(rows.begin(), rows.end(), [&](const TrackRow& a, const TrackRow& b) {
                return position(a) < position(b);
            });
            section.albums.push_back(AlbumGroup{album, std::move(rows)});
        }
        sections.push_back(std::move(section));
    }
    return sections;
}

std::vector<TrackRow> LibraryRows(const AppState& state, LibrarySort sort, bool isDarkTheme) {
    switch (sort) {
        case LibrarySort::ArtistAlbum: {
            // #503: one artist/album rule -- the grouping, read top to bottom.
            std::vector<TrackRow> rows;
            for (auto& section : ArtistAlbumSections(state, isDarkTheme)) {
                for (auto& album : section.albums) {
                    std::move(album.rows.begin(), album.rows.end(), std::back_inserter(rows));
                }
            }
            return rows;
        }
        case LibrarySort::Alphabetical: {
            // #504: one by-letter rule -- the sections, read top to bottom.
            std::vector<TrackRow> rows;
            for (auto& section : LetterSections(state, isDarkTheme)) {
                std::move(section.rows.begin(), section.rows.end(), std::back_inserter(rows));
            }
            return rows;
        }
    }
    return {};
}

std::vector<TrackRow> LibraryRows(const AppState& state, bool isDarkTheme) {
    return LibraryRows(state, state.LibraryOrder, isDarkTheme);
}

std::vector<LibraryLine> LibraryLines(const AppState& state, bool isDarkTheme) {
    std::vector<LibraryLine> lines;
    if (state.LibraryOrder == LibrarySort::ArtistAlbum) {
        for (auto& section : ArtistAlbumSections(state, isDarkTheme)) {
            lines.push_back(LibraryLine{LibraryLineKind::Section, section.title, {}, false});
            for (auto& album : section.albums) {
                lines.push_back(LibraryLine{LibraryLineKind::Album, album.title, {}, false});
                for (auto& row : album.rows) {
                    lines.push_back(LibraryLine{LibraryLineKind::Track, {}, std::move(row), true});
                }
            }
        }
        return lines;
    }
    for (auto& section : LetterSections(state, isDarkTheme)) {
        lines.push_back(LibraryLine{LibraryLineKind::Section, section.title, {}, false});
        for (auto& row : section.rows) {
            lines.push_back(LibraryLine{LibraryLineKind::Track, {}, std::move(row), false});
        }
    }
    return lines;
}

std::vector<ViewSwitchSegment> LibraryViewSwitch(const AppState& state) {
    // #502: one switch, labelled like the macOS segmented picker.
    auto segment = [&](LibrarySort sort, std::wstring label) {
        return ViewSwitchSegment{sort, std::move(label), state.LibraryOrder == sort};
    };
    return {segment(LibrarySort::ArtistAlbum, L10n::ByArtistAlbum()),
            segment(LibrarySort::Alphabetical, L10n::ByLetter())};
}

PlaylistListPage PlaylistListState(const AppState& state) {
    // #320: the list renders from an assertable return value; a click carries
    // the row's identifier, so the list and the detail correspond by identity
    // rather than by position in the state's storage.
    PlaylistListPage page;
    for (const auto& playlist : state.Playlists) {
        if (!playlist.id) continue;  // not stored: nothing to select it by
        PlaylistRow row;
        row.id = *playlist.id;
        row.name = playlist.name;
        row.trackCountText = std::to_wstring(playlist.tracks.size());
        page.rows.push_back(std::move(row));
    }
    if (page.rows.empty()) page.emptyMessage = L10n::PlaylistEmpty();
    return page;
}

PlaylistDetail PlaylistDetailOf(const AppState& state, bool isDarkTheme) {
    // #358: the values come from the state's current playlist, taken fresh on
    // every render -- the page keeps nothing between renders.
    PlaylistDetail detail;
    if (!state.CurrentPlaylist) {
        detail.emptyMessage = L10n::NoPlaylistSelected();  // #359
        return detail;
    }
    detail.hasPlaylist = true;
    detail.title = state.CurrentPlaylist->name;
    detail.rows = RowsOf(state.CurrentPlaylist->tracks, isDarkTheme);
    return detail;
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
    bar.seekable = state.CurrentTrack.has_value() && state.Duration > 0;
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

double SeekPosition(const AppState& state, double percent) {
    if (state.Duration <= 0) return 0.0;
    return std::clamp(percent, 0.0, 100.0) / 100.0 * state.Duration;
}

std::vector<SidebarEntry> SidebarState(const AppState& state) {
    // #501: labels come from the same keys the old navigation items used.
    auto entry = [&](SidebarItem item, Icon icon, std::wstring label) {
        return SidebarEntry{item, icon, std::move(label), state.SelectedView == item};
    };
    return {entry(SidebarItem::Library, Icon::Library, L10n::LibraryTab()),
            entry(SidebarItem::Playlists, Icon::Playlists, L10n::PlaylistsTab())};
}

TrayMenu TrayMenuState(const AppState& state) {
    // #141: tray copy follows the language layer like everything else.
    return TrayMenu{L10n::TrayPlayPause(), L10n::TrayShowWindow(), L10n::TrayQuit(),
                    state.CanTogglePlayback()};
}

std::optional<Alert> PendingAlert(const AppState& state) {
    if (state.ShowExportAlert) return Alert{ state.ExportAlertTitle, state.ExportAlertMessage };
    if (state.ShowImportAlert) return Alert{ L10n::ImportResultTitle(), state.ImportAlertMessage };
    return std::nullopt;
}

} // namespace rhythm::view
