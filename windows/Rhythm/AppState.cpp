#include "pch.h"
#include "AppState.h"

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

} // namespace rhythm
