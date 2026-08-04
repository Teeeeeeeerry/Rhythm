#include "pch.h"
#include "AppState.h"

#include <thread>

namespace rhythm {

AppState::AppState() {
    Player = std::make_unique<rhythm::Player>();
}

void AppState::OpenDatabase(const std::wstring& path) {
    Library = std::make_unique<rhythm::Library>(path);
    RefreshLibrary();
}

void AppState::RefreshLibrary() {
    if (!Library) return;
    Tracks = Library->AllTracks();
    Playlists = Library->AllPlaylists();
}

void AppState::ImportDirectory(const std::wstring& path) {
    if (!Library) return;
    Library->ImportDirectory(path);
    RefreshLibrary();
}

void AppState::DoSearch() {
    if (!Library) return;
    Tracks = SearchQuery.empty() ? Library->AllTracks() : Library->Search(SearchQuery);
}

void AppState::PlayTrack(const Track& track) {
    CurrentTrack = track;
    if (track.filePath) {
        Player->PlayFile(*track.filePath);
    } else if (track.sourceUrl) {
        Player->PlayURL(*track.sourceUrl);
    }
    IsPlaying = true;
    if (Library) Library->RecordPlay(track.id);
}

void AppState::TogglePlayPause() {
    if (IsPlaying) {
        Player->Pause();
        IsPlaying = false;
    } else {
        if (CurrentTrack) {
            PlayTrack(*CurrentTrack);
        } else if (!Tracks.empty()) {
            PlayTrack(Tracks[0]);
        }
    }
}

void AppState::SetVolume(double v) {
    Volume = v;
    Player->SetVolume(static_cast<float>(v));
}

void AppState::ResolveAndPlay(const std::wstring& url) {
    auto first = url.find_first_not_of(L" \t\r\n");
    if (first == std::wstring::npos) return;
    auto last = url.find_last_not_of(L" \t\r\n");
    auto trimmed = url.substr(first, last - first + 1);

    // Resolution may take a few seconds (yt-dlp); run it off the UI thread
    // and marshal the result back.
    auto dq = dispatcher_;
    std::thread([this, trimmed, dq] {
        auto track = rhythm::Resolver::ResolveURL(trimmed);
        if (dq) {
            dq.TryEnqueue([this, track] {
                Tracks.insert(Tracks.begin(), track);
                PlayTrack(track);
            });
        }
    }).detach();
}

} // namespace rhythm
