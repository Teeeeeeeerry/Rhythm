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
        auto outcome = rhythm::Resolver::ResolveURL(trimmed);
        if (!dq) return;

        dq.TryEnqueue([this, outcome] {
            if (!outcome.ok) {
                // Report the reason rather than queueing a track that cannot
                // play — the core distinguishes a missing yt-dlp from a
                // timeout, a private video, and so on (#21).
                UrlError = outcome.errorMessage;
                OutputDebugStringW(
                    (L"URL resolution failed [" + outcome.errorKind + L"]: " +
                     outcome.errorMessage + L"\n").c_str());
                if (OnUrlError) OnUrlError(outcome.errorKind, outcome.errorMessage);
                return;
            }
            UrlError.clear();
            Tracks.insert(Tracks.begin(), outcome.track);
            PlayTrack(outcome.track);
        });
    }).detach();
}

} // namespace rhythm
