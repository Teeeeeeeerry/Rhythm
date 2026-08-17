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

    // WA-20 (#69): keep the play queue in sync so newly imported tracks are
    // reachable via "next" and deleted tracks are removed.
    if (Queue && CurrentTrack) {
        Queue->Replace(Tracks);
        if (CurrentTrack->id >= 0) Queue->JumpTo(CurrentTrack->id);
    }
}

void AppState::ImportDirectory(const std::wstring& path) {
    if (!Library) return;
    int32_t count = Library->ImportDirectory(path);
    // WA-23: mirror the macOS import alert, including its zero/failure arms.
    if (count > 0) {
        RefreshLibrary();
        ImportAlertMessage = std::format(L"已导入 {} 首歌曲", count);
        ShowImportAlert = true;
    } else if (count == 0) {
        ImportAlertMessage = L"该目录中未找到支持的音频文件";
        ShowImportAlert = true;
    } else {
        ImportAlertMessage = L"导入失败，请检查目录是否可访问";
        ShowImportAlert = true;
    }
}

void AppState::DoSearch() {
    if (!Library) return;
    Tracks = SearchQuery.empty() ? Library->AllTracks() : Library->Search(SearchQuery);
}

void AppState::PlayTrack(const Track& track) {
    // #81: without a playable path there is nothing to play — don't enter
    // the playing state (silent fake playback).
    if (!track.filePath && !track.sourceUrl) return;

    CurrentTrack = track;
    StartTrack(track);

    // WA-19: rebuild the play queue from the current track list.
    auto q = std::make_unique<PlayQueue>(Tracks);
    q->SetMode(static_cast<int32_t>(CurrentMode));
    if (track.id >= 0) q->JumpTo(track.id);
    Queue = std::move(q);
}

/// Stop → playFile/playURL → IsPlaying → RecordPlay (#51: stop old playback
/// before starting new). The caller guards the #81 no-path case.
void AppState::StartTrack(const Track& track) {
    Player->Stop();
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
        // #111: Pause() also responds in Buffering, so the engine cannot
        // start Playing and push audio after the UI shows paused.
        Player->Pause();
        IsPlaying = false;
    } else {
        if (CurrentTrack) {
            // #111: resume in place instead of restarting from the top (#82),
            // and only when the engine is actually Paused — in any other state
            // (Error/Stopped/Buffering) Resume is a no-op and claiming playback
            // would desync the UI from the engine.
            if (Player->State() == 2) {
                Player->Resume();
                IsPlaying = true;
            }
        } else if (!Tracks.empty()) {
            PlayTrack(Tracks[0]);
        }
    }
}

void AppState::SetVolume(double v) {
    Volume = v;
    Player->SetVolume(static_cast<float>(v));
}

// ─── Transport availability (WA-22) ────────────────────────────────

bool AppState::CanTogglePlayback() const {
    return CurrentTrack.has_value() || !Tracks.empty();
}

bool AppState::CanPlayNext() const {
    return Queue ? Queue->HasNext() : false;
}

bool AppState::CanPlayPrevious() const {
    return Queue ? Queue->HasPrevious() : false;
}

bool AppState::CanStop() const {
    return IsPlaying;
}

// ─── Queue transport (WA-19) ───────────────────────────────────────

void AppState::PlayNext() {
    if (!Queue) return;
    auto next = Queue->Next();
    if (!next) return;
    if (!next->filePath && !next->sourceUrl) return; // #81 guard
    CurrentTrack = *next;
    StartTrack(*next);
}

void AppState::PlayPrevious() {
    if (!Queue) return;
    auto previous = Queue->Previous();
    if (!previous) return;
    if (!previous->filePath && !previous->sourceUrl) return; // #81 guard
    CurrentTrack = *previous;
    StartTrack(*previous);
}

// ─── Play mode (WA-21) ─────────────────────────────────────────────

void AppState::CyclePlayMode() {
    CurrentMode = static_cast<PlayMode>((static_cast<int32_t>(CurrentMode) + 1) % 4);
    if (Queue) Queue->SetMode(static_cast<int32_t>(CurrentMode));
}

void AppState::ResolveAndPlay(const std::wstring& url) {
    auto first = url.find_first_not_of(L" \t\r\n");
    if (first == std::wstring::npos) return;
    auto last = url.find_last_not_of(L" \t\r\n");
    auto trimmed = url.substr(first, last - first + 1);

    // Resolution may take a few seconds (yt-dlp, plus a one-off download on
    // first use); run it off the UI thread and marshal the result back.
    if (IsResolvingUrl) return;
    IsResolvingUrl = true;

    auto dq = dispatcher_;
    std::thread([this, trimmed, dq] {
        auto outcome = rhythm::Resolver::ResolveURL(trimmed);
        if (!dq) {
            IsResolvingUrl = false;
            return;
        }

        dq.TryEnqueue([this, outcome] {
            IsResolvingUrl = false;
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
            // Persist to database first — AddTrack returns the track
            // with its real database id (#39).
            auto saved = Library ? Library->AddTrack(outcome.track) : outcome.track;
            Tracks.insert(Tracks.begin(), saved);
            PlayTrack(saved);
        });
    }).detach();
}

} // namespace rhythm
