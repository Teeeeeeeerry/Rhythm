#include "BehaviorPch.h"
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

    // #355: the refresh replaces the list, so the selection is re-resolved by
    // id against the new one -- its content must come from the reloaded
    // playlists, never from the copy taken before the refresh.
    // #356: a selection the new list no longer carries is cleared -- the UI
    // goes to its empty state instead of keeping a dead pointer.
    if (CurrentPlaylist) {
        auto reloaded = CurrentPlaylist->id ? FindPlaylist(*CurrentPlaylist->id) : nullptr;
        if (reloaded) {
            CurrentPlaylist = *reloaded;
        } else {
            ClearPlaylistSelection();
        }
    }

    // #69: keep the play queue in sync so newly imported tracks are
    // reachable via "next" and deleted tracks are removed — inside the
    // coordinator (ticket #173).
    Coordinator->SyncQueue(Tracks);
}

int64_t AppState::CreatePlaylist(const std::wstring& name) {
    if (!Library || name.empty()) return -1;
    auto id = Library->CreatePlaylist(name);
    if (id < 0) return -1;
    RefreshLibrary();
    return id;
}

void AppState::ImportDirectory(const std::wstring& path) {
    if (!Library) return;
    auto outcome = Library->ImportDirectory(path);
    if (!outcome) return;
    // WA-23: mirror the macOS import alert. The counts are named (#241) --
    // no reverse-engineering a magic integer.
    if (outcome->imported > 0) {
        RefreshLibrary();
        ImportAlertMessage = L10n::ImportedTracks(outcome->imported);
    } else if (outcome->failed > 0) {
        ImportAlertMessage = L10n::ImportFailed();
    } else {
        ImportAlertMessage = L10n::ImportNoFiles();
    }
    ShowImportAlert = true;
}

void AppState::ImportFile(const std::wstring& path) {
    if (!Library) return;
    auto outcome = Library->ImportFile(path);
    if (!outcome) return;
    if (outcome->imported > 0) {
        RefreshLibrary();
        ImportAlertMessage = L10n::ImportedTracks(outcome->imported);
    } else if (outcome->unsupported > 0) {
        ImportAlertMessage = L10n::ImportFileUnsupported();
    } else {
        ImportAlertMessage = L10n::ImportFileFailed();
    }
    ShowImportAlert = true;
}

void AppState::ImportPaths(const std::vector<std::wstring>& paths) {
    if (!Library) return;
    auto outcome = Library->ImportPaths(paths);
    if (!outcome) return;
    // Same four arms as macOS, in the same order, from the same counts --
    // both platforms must render one batch identically (#243).
    if (outcome->imported > 0) RefreshLibrary();
    if (outcome->imported > 0 && outcome->failed == 0) {
        ImportAlertMessage = L10n::ImportedTracks(outcome->imported);
    } else if (outcome->imported > 0) {
        ImportAlertMessage = L10n::ImportSomeFailed(outcome->imported, outcome->failed);
    } else if (outcome->failed > 0) {
        ImportAlertMessage = L10n::ImportAllFailed();
    } else {
        ImportAlertMessage = L10n::ImportNoneFound();
    }
    ShowImportAlert = true;
}

void AppState::ImportM3U8(const std::wstring& path) {
    if (!Library) return;
    // #236: parsing and storing are one core entry point — this layer only
    // picks the alert text and reloads the list from the database.
    DismissAlerts();  // only this import's feedback is pending (#321 review)
    auto outcome = Library->ImportM3U8(path);
    if (!outcome) return;
    RefreshLibrary();
    if (outcome->failed > 0) {
        ImportAlertMessage = L10n::ImportSomeFailed(outcome->imported, outcome->failed);
    } else if (outcome->imported > 0) {
        ImportAlertMessage = L10n::ImportedTracks(outcome->imported);
    }
    if (outcome->imported > 0 || outcome->failed > 0) {
        ShowImportAlert = true;
    }
}

std::optional<M3u8ExportOutcome> AppState::ExportPlaylist(int64_t playlistId, const std::wstring& path) {
    if (!Library) return std::nullopt;
    auto playlist = FindPlaylist(playlistId);
    if (!playlist) return std::nullopt;
    DismissAlerts();  // only this export's feedback is pending
    auto outcome = ExportM3U8(path, playlist->tracks);
    // #352: the failure copy finally reaches the UI; the code tells a bad
    // payload (-1) from an unwritable target (-2).
    if (outcome.status == M3u8ExportStatus::Exported) {
        ExportAlertTitle = L10n::ExportResultTitle();
        ExportAlertMessage = L10n::ExportedTracks(outcome.exported);
    } else {
        ExportAlertTitle = L10n::ExportFailedTitle();
        ExportAlertMessage = ExportFailureText(outcome);
    }
    ShowExportAlert = true;
    return outcome;
}

std::wstring ExportFailureText(const M3u8ExportOutcome& outcome) {
    return outcome.status == M3u8ExportStatus::InvalidTracks
        ? L10n::ExportInvalidTracks(outcome.code)
        : L10n::ExportFailed(outcome.code);
}

void AppState::DismissAlerts() {
    ShowImportAlert = false;
    ShowExportAlert = false;
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

const Playlist* AppState::FindPlaylist(int64_t id) const {
    for (const auto& playlist : Playlists) {
        if (playlist.id == id) return &playlist;
    }
    return nullptr;
}

void AppState::SelectPlaylist(int64_t id) {
    // #354: the selection is a value, not an address inside `Playlists`.
    // An unknown id clears it rather than leaving a stale selection.
    auto playlist = FindPlaylist(id);
    if (!playlist) {
        ClearPlaylistSelection();
        return;
    }
    CurrentPlaylist = *playlist;
}

void AppState::ClearPlaylistSelection() {
    CurrentPlaylist.reset();
}

std::wstring AppState::ResolverStatusText() const {
    // The status is an FFI poll: only worth taking while a link resolves.
    if (!IsResolvingUrl) return {};
    auto status = PollResolverStatus();
    return status.IsQuiet() ? std::wstring{} : Resolver::StatusText(status);
}

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

    auto post = uiPost_;
    std::thread([this, trimmed, post] {
        auto outcome = rhythm::Resolver::ResolveURL(trimmed);
        if (!post) {
            IsResolvingUrl = false;
            return;
        }

        post([this, outcome] {
            IsResolvingUrl = false;
            if (!outcome.ok) {
                // Report the reason rather than queueing a track that cannot
                // play — the core distinguishes a missing yt-dlp from a
                // timeout, a private video, and so on (#21).
                // #230: 每个失败处一次性本地化（分派在核心），视图只渲染。
                UrlError = L10n::UrlResolveError(outcome.errorKind, outcome.errorMessage);
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
    auto post = uiPost_;
    if (post) {
        post([this, json] { ApplyCoordinatorEvent(json); });
    } else {
        // No UI thread (tests): apply synchronously on the caller thread.
        ApplyCoordinatorEvent(json);
    }
}

void AppState::ApplyCoordinatorEvent(const std::wstring& json) {
    try {
        // #432: UTF-8 both ways -- a per-character cast garbled non-ASCII
        // titles and paths, or made the JSON invalid so the event was dropped.
        auto j = json::parse(WideToUtf8(json));
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
            auto detail = Utf8ToWide(message);
            std::wstring kind;
            if (j.contains("kind") && !j["kind"].is_null()) {
                kind = Utf8ToWide(j["kind"].get<std::string>());
            }
            UrlError = L10n::PlaybackFailed(kind, detail);
            OutputDebugStringW((L"Playback failed: " + detail + L"\n").c_str());
            // #226: the event carries the core's own classification value
            // (#120 expired / cdn_rejected / other, empty when the failure
            // was not HTTP) — no UI-side prefix to encode and decode.
            if (OnUrlError) OnUrlError(kind, detail);
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
