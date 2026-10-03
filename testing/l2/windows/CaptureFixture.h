#pragma once

// Fixture data of the L2 capture host (#495). Nothing here reads or writes
// the user's library: the main window opens an empty database in a scratch
// directory, and the library page and player bar are fed fixed tracks.

#include <filesystem>
#include <string>
#include <vector>

#include "AppState.h"

namespace rhythm::capture {

/// The scratch library the main window opens (see CaptureHost.cpp). Set once
/// before the first main window is created, removed when the host exits.
std::filesystem::path& FixtureLibraryPath();

/// A small library covering the row shapes the list renders: several
/// artists and albums, a track without artist or album, and every source.
inline std::vector<Track> FixtureTracks() {
    auto track = [](int64_t id, std::wstring title, std::optional<std::wstring> artist,
                    std::optional<std::wstring> album, std::wstring source, double duration) {
        Track t;
        t.id = id;
        t.title = std::move(title);
        t.artist = std::move(artist);
        t.album = std::move(album);
        t.sourceType = std::move(source);
        t.duration = duration;
        return t;
    };
    return {
        track(1, L"Blue in Green", L"Miles Davis", L"Kind of Blue", L"local", 337),
        track(2, L"So What", L"Miles Davis", L"Kind of Blue", L"local", 562),
        track(3, L"Naima", L"John Coltrane", L"Giant Steps", L"local", 261),
        track(4, L"Live Session 2024", L"Night Radio", std::nullopt, L"bilibili", 2175),
        track(5, L"Lo-fi Mix", L"Night Radio", std::nullopt, L"youtube", 3600),
        track(6, L"Field Recording", std::nullopt, std::nullopt, L"direct_url", 95),
    };
}

/// The track the "playing" player bar shows: a long online stream, the case
/// the player bar's time text and progress have to handle.
inline Track NowPlayingTrack() {
    for (const auto& track : FixtureTracks()) {
        if (track.sourceType == L"bilibili") return track;
    }
    return {};
}

} // namespace rhythm::capture
