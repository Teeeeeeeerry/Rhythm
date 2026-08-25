#include "pch.h"
#include "AppState.h"
#include "L10n.h"

#include <nlohmann/json.hpp>

#include <thread>

using json = nlohmann::json;

namespace rhythm {

AppState::AppState() {
    Coordinator = std::make_unique<rhythm::Coordinator>();
    Coordinator->SetEventHandler(
        [this](const std::wstring& json) { OnCoordinatorEvent(json); });
}

void AppState::OpenDatabase(const std::wstring& path) {
    Library = std::make_unique<rhythm::Library>(path);
    Coordinator->SetLibrary(Library.get());
    RefreshLibrary();
}

void AppState::RefreshLibrary() {
    if (!Library) return;
    Tracks = Library->AllTracks();
    Playlists = Library->AllPlaylists();

    // #69: keep the play queue in sync so newly imported tracks are
    // reachable via "next" and deleted tracks are removed — inside the
    // coordinator (ticket #173).
    Coordinator->SyncQueue(Tracks);
}

void AppState::ImportDirectory(const std::wstring& path) {
    if (!Library) return;
    int32_t count = Library->ImportDirectory(path);
    // WA-23: mirror the macOS import alert, including its zero/failure arms.
    if (count > 0) {
        RefreshLibrary();
        ImportAlertMessage = L10n::ImportedTracks(count);
        ShowImportAlert = true;
    } else if (count == 0) {
        ImportAlertMessage = L10n::ImportNoFiles();
        ShowImportAlert = true;
    } else {
        ImportAlertMessage = L10n::ImportFailed();
        ShowImportAlert = true;
    }
}

void AppState::ImportM3U8(const std::wstring& path) {
    if (!Library) return;
    auto entries = Library->ImportM3U8(path);
    int32_t imported = 0;
    int32_t failed = 0;
    for (const auto& entry : entries) {
        // Entries without a location are not importable (macOS parity).
        if (entry.location.empty()) {
            failed += 1;
            continue;
        }
        // A location that looks like an http(s) URL is stored as a
        // direct_url; anything else is a local file path (macOS parity,
        // #136 semantics).
        bool isUrl = entry.location.rfind(L"http://", 0) == 0 ||
                     entry.location.rfind(L"https://", 0) == 0;
        Track track;
        track.id = 0;
        track.title = entry.title.empty() ? L"Unknown" : entry.title;
        track.artist = entry.artist;
        track.sourceType = isUrl ? L"direct_url" : L"local";
        if (isUrl) {
            track.sourceUrl = entry.location;
        } else {
            track.filePath = entry.location;
        }
        auto saved = Library->AddTrack(track);
        if (saved.id != 0) {
            imported += 1;
        } else {
            failed += 1;
        }
    }
    RefreshLibrary();
    if (failed > 0) {
        ImportAlertMessage = L10n::ImportSomeFailed(imported, failed);
    } else if (imported > 0) {
        ImportAlertMessage = L10n::ImportedTracks(imported);
    }
    if (imported > 0 || failed > 0) {
        ShowImportAlert = true;
    }
}

void AppState::DoSearch() {
    if (!Library) return;
    Tracks = SearchQuery.empty() ? Library->AllTracks() : Library->Search(SearchQuery);
}

void AppState::PlayTrack(const Track& track) {
    // #81: the no-playable-location guard lives in the coordinator — a track
    // without a location comes back as a classified failure and nothing
    // changes (silent fake playback is impossible).
    auto outcome = Coordinator->Start(track, Tracks, static_cast<int32_t>(CurrentMode));
    if (!outcome.ok) return;

    CurrentTrack = track;
    IsPlaying = true;
}

void AppState::TogglePlayPause() {
    // The full transport semantics live in the coordinator (ticket #171):
    // pause while playing/buffering, resume only when paused, idle-start the
    // first playable library track.
    auto outcome = Coordinator->TogglePlayPause();
    if (!outcome.ok) return;
    if (outcome.currentTrack) {
        CurrentTrack = outcome.currentTrack;
    }
    IsPlaying = outcome.playbackActive;
    if (!IsPlaying) {
        // Nothing polls while paused, so this would otherwise stay stuck on
        // whatever it was when the user hit pause.
        IsBuffering = false;
    }
}

void AppState::SetVolume(double v) {
    Volume = v;
    Coordinator->SetVolume(static_cast<float>(v));
}

// ─── Transport availability (WA-22) ────────────────────────────────

bool AppState::CanTogglePlayback() const {
    return Coordinator->CanTogglePlayback();
}

bool AppState::CanPlayNext() const {
    return Coordinator->HasNext();
}

bool AppState::CanPlayPrevious() const {
    return Coordinator->HasPrevious();
}

bool AppState::CanStop() const {
    return Coordinator->CanStop();
}

// ─── Queue transport (WA-19) ───────────────────────────────────────

void AppState::PlayNext() {
    auto outcome = Coordinator->Next();
    if (!outcome.ok) return;
    if (outcome.currentTrack) {
        CurrentTrack = outcome.currentTrack;
        IsPlaying = true;
        IsBuffering = false;
    }
}

void AppState::PlayPrevious() {
    auto outcome = Coordinator->Previous();
    if (!outcome.ok) return;
    if (outcome.currentTrack) {
        CurrentTrack = outcome.currentTrack;
        IsPlaying = true;
        IsBuffering = false;
    }
}

// ─── Play mode (WA-21) ─────────────────────────────────────────────

void AppState::CyclePlayMode() {
    CurrentMode = static_cast<PlayMode>((static_cast<int32_t>(CurrentMode) + 1) % 4);
    Coordinator->SetPlayMode(static_cast<int32_t>(CurrentMode));
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
            // #139: reload the list from DB instead of a manual front-insert
            // so the list and play queue stay in sync (macOS #66/#69 parity).
            RefreshLibrary();
            PlayTrack(saved);
        });
    }).detach();
}

// ─── Coordinator events (ticket #172/#173) ─────────────────────────

void AppState::OnCoordinatorEvent(const std::wstring& json) {
    auto dq = dispatcher_;
    if (dq) {
        dq.TryEnqueue([this, json] { ApplyCoordinatorEvent(json); });
    } else {
        // No dispatcher (tests): apply synchronously on the caller thread.
        ApplyCoordinatorEvent(json);
    }
}

void AppState::ApplyCoordinatorEvent(const std::wstring& json) {
    try {
        auto j = json::parse(std::string(json.begin(), json.end()));
        std::string type = j.value("type", "");

        if (type == "progress") {
            Position = j.value("position", 0.0);
            Duration = j.value("duration", 0.0);
        } else if (type == "state") {
            std::string state = j.value("state", "");
            IsBuffering = state == "buffering";
            IsPlaying = state == "playing" || state == "buffering";
        } else if (type == "finished") {
            // The coordinator already auto-advanced if possible (a
            // track_changed event follows); when the queue is exhausted,
            // stop claiming playback.
            IsPlaying = false;
            IsBuffering = false;
        } else if (type == "error") {
            IsPlaying = false;
            IsBuffering = false;
            std::string message = j.value("message", "");
            auto detail = std::wstring(message.begin(), message.end());
            std::wstring kind;
            if (j.contains("kind") && !j["kind"].is_null()) {
                kind = Utf8ToWide(j["kind"].get<std::string>());
            }
            UrlError = L10n::PlaybackFailed(kind, detail);
            OutputDebugStringW((L"Playback failed: " + detail + L"\n").c_str());
            if (OnUrlError) {
                // #120: classify HTTP failures so the dialog can tell a
                // genuinely expired link from a CDN rejection.
                std::wstring kindCode = L"playback_failed";
                if (j.contains("kind") && !j["kind"].is_null()) {
                    auto kind = j["kind"].get<std::string>();
                    if (kind == "expired") {
                        kindCode = L"playback_expired";
                    } else if (kind == "cdn_rejected") {
                        kindCode = L"playback_cdn_rejected";
                    }
                }
                OnUrlError(kindCode, detail);
            }
        } else if (type == "track_changed") {
            if (j.contains("track") && !j["track"].is_null()) {
                CurrentTrack = rhythm::ParseTrackJson(j["track"].dump());
            }
            IsPlaying = true;
            IsBuffering = false;
        }
    } catch (const json::exception&) {
        // Malformed event: ignore.
    }
    if (OnStateChanged) {
        OnStateChanged();
    }
}

} // namespace rhythm
