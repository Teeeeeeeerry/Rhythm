#include "BehaviorPch.h"
#include "RhythmCore.h"
#include "GeneratedCodec.h"

#include <nlohmann/json.hpp>
using json = nlohmann::json;

namespace rhythm {

// ─── Helpers ────────────────────────────────────────────────────────

std::wstring Utf8ToWide(const std::string& s) {
    if (s.empty()) return {};
    int len = MultiByteToWideChar(CP_UTF8, 0, s.data(), (int)s.size(), nullptr, 0);
    std::wstring result(len, L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s.data(), (int)s.size(), result.data(), len);
    return result;
}

std::string WideToUtf8(const std::wstring& ws) {
    if (ws.empty()) return {};
    int len = WideCharToMultiByte(CP_UTF8, 0, ws.data(), (int)ws.size(), nullptr, 0, nullptr, nullptr);
    std::string result(len, '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws.data(), (int)ws.size(), result.data(), len, nullptr, nullptr);
    return result;
}

Track ParseTrackJson(const std::string& json) {
    try {
        return generated::TrackFromJson(json::parse(json));
    } catch (const json::exception&) {
        return Track{};
    }
}

static std::vector<Track> ParseTrackList(const char* json) {
    if (!json) return {};
    auto j = json::parse(json);
    std::vector<Track> tracks;
    for (const auto& item : j) {
        tracks.push_back(generated::TrackFromJson(item));
    }
    return tracks;
}

/// Track list -> core JSON array, one generated encoder call per track (#416).
static std::string TracksToJson(const std::vector<Track>& tracks) {
    json j = json::array();
    for (const auto& t : tracks) {
        j.push_back(generated::TrackToJson(t));
    }
    return j.dump();
}

/// Playlist list -> models, one generated decoder call per playlist (#367).
/// The fields come from the contract; this side only walks the array.
static std::vector<Playlist> ParsePlaylistList(const char* json) {
    if (!json) return {};
    auto j = json::parse(json);
    std::vector<Playlist> playlists;
    for (const auto& item : j) {
        playlists.push_back(generated::PlaylistFromJson(item));
    }
    return playlists;
}

// ─── Library ────────────────────────────────────────────────────────

Library::Library(const std::wstring& dbPath) {
    auto path = WideToUtf8(dbPath);
    ptr_ = rhythm_library_open(path.c_str());
}

Library::~Library() {
    if (ptr_) rhythm_library_close(ptr_);
}

/// Decode a named import outcome returned by the core (#241).
static std::optional<ImportOutcome> ParseImportOutcome(char* raw) {
    if (!raw) return std::nullopt;
    std::optional<ImportOutcome> outcome;
    try {
        outcome = generated::ImportOutcomeFromJson(json::parse(raw));
    } catch (const json::exception&) {
        // Malformed payload: nothing to report.
    }
    rhythm_free_string(raw);
    return outcome;
}

std::optional<ImportOutcome> Library::ImportDirectory(const std::wstring& path) {
    if (!ptr_) return std::nullopt;
    auto p = WideToUtf8(path);
    return ParseImportOutcome(rhythm_library_import_directory(ptr_, p.c_str()));
}

std::optional<ImportOutcome> Library::ImportFile(const std::wstring& path) {
    if (!ptr_) return std::nullopt;
    auto p = WideToUtf8(path);
    return ParseImportOutcome(rhythm_library_import_single_file(ptr_, p.c_str()));
}

std::optional<ImportOutcome> Library::ImportPaths(const std::vector<std::wstring>& paths) {
    if (!ptr_) return std::nullopt;
    json list = json::array();
    for (const auto& path : paths) {
        list.push_back(WideToUtf8(path));
    }
    auto payload = list.dump();
    return ParseImportOutcome(rhythm_library_import_paths(ptr_, payload.c_str()));
}

std::vector<Track> Library::AllTracks() {
    if (!ptr_) return {};
    char* json = rhythm_library_get_all_tracks(ptr_);
    auto tracks = ParseTrackList(json);
    if (json) rhythm_free_string(json);
    return tracks;
}

std::vector<Track> Library::Search(const std::wstring& query) {
    if (!ptr_) return {};
    auto q = WideToUtf8(query);
    char* json = rhythm_library_search(ptr_, q.c_str());
    auto tracks = ParseTrackList(json);
    if (json) rhythm_free_string(json);
    return tracks;
}

void Library::VerifyFiles() {
    if (!ptr_) return;
    char* json = rhythm_library_verify_files(ptr_);
    if (json) rhythm_free_string(json);
}

std::vector<Playlist> Library::AllPlaylists() {
    if (!ptr_) return {};
    char* json = rhythm_library_get_playlists(ptr_);
    auto playlists = ParsePlaylistList(json);
    if (json) rhythm_free_string(json);
    return playlists;
}

int64_t Library::CreatePlaylist(const std::wstring& name) {
    if (!ptr_) return -1;
    auto n = WideToUtf8(name);
    return rhythm_library_create_playlist(ptr_, n.c_str(), "");
}

void Library::AddToPlaylist(int64_t playlistId, int64_t trackId) {
    if (!ptr_) return;
    rhythm_library_playlist_add(ptr_, playlistId, trackId);
}

void Library::RemoveFromPlaylist(int64_t playlistId, int64_t trackId) {
    if (!ptr_) return;
    rhythm_library_playlist_remove(ptr_, playlistId, trackId);
}

void Library::DeletePlaylist(int64_t id) {
    if (!ptr_) return;
    rhythm_library_delete_playlist(ptr_, id);
}

void Library::RecordPlay(int64_t trackId) {
    if (!ptr_) return;
    rhythm_library_record_play(ptr_, trackId);
}

Track Library::AddTrack(const Track& track) {
    if (!ptr_) return track;
    auto json = generated::TrackToJson(track).dump();
    char* result = rhythm_library_add_track(ptr_, json.c_str());
    if (!result) return track;
    auto saved = generated::TrackFromJson(json::parse(result));
    rhythm_free_string(result);
    return saved;
}

bool Library::RemoveTrack(int64_t id) {
    if (!ptr_) return false;
    return rhythm_library_remove_track(ptr_, id) == 0;
}

M3u8ExportOutcome M3u8ExportOutcomeFromCode(int32_t code, int32_t trackCount) {
    if (code == 0) return { M3u8ExportStatus::Exported, trackCount, 0 };
    if (code == -1) return { M3u8ExportStatus::InvalidTracks, 0, code };
    return { M3u8ExportStatus::WriteFailed, 0, code };
}

M3u8ExportOutcome ExportM3U8(const std::wstring& path, const std::vector<Track>& tracks) {
    auto p = WideToUtf8(path);
    auto json = TracksToJson(tracks);
    return M3u8ExportOutcomeFromCode(rhythm_export_m3u8(p.c_str(), json.c_str()),
                                     static_cast<int32_t>(tracks.size()));
}

/// Decode the core's named import outcome (#236) — the counts are the whole
/// contract, the UI never re-derives "did this entry make it".
std::optional<M3u8ImportOutcome> Library::ImportM3U8(const std::wstring& path) {
    if (!ptr_) return std::nullopt;
    auto p = WideToUtf8(path);
    char* raw = rhythm_import_m3u8_into_library(ptr_, p.c_str());
    if (!raw) return std::nullopt;
    std::optional<M3u8ImportOutcome> outcome;
    try {
        outcome = generated::M3u8ImportOutcomeFromJson(json::parse(raw));
    } catch (const json::exception&) {
        // Malformed payload: nothing to report.
    }
    rhythm_free_string(raw);
    return outcome;
}

// ─── Coordinator ────────────────────────────────────────────────────

/// Parse the core's structured result JSON into a CoordinatorResult. The
/// fields come from the generated decoder (#364); only a missing or
/// malformed payload is handled here.
static CoordinatorResult ParseCoordinatorResult(const char* json) {
    if (!json) return {};
    try {
        return generated::CoordinatorResultFromJson(json::parse(json));
    } catch (const json::exception&) {
        CoordinatorResult result;
        result.errorKind = L"internal";
        result.errorMessage = L"Malformed coordinator response";
        return result;
    }
}

/// C trampoline: the core fires events on the playback thread; we hand the
/// JSON string to the wrapper (the caller marshals).
extern "C" void CoordinatorEventBridge(void* userdata, char* event_json) {
    auto* coordinator = static_cast<Coordinator*>(userdata);
    if (!coordinator || !event_json) return;
    std::string payload(event_json);
    rhythm_free_string(event_json);
    coordinator->DispatchEvent(payload);
}

void Coordinator::DispatchEvent(const std::string& utf8) {
    if (handler_) {
        handler_(Utf8ToWide(utf8));
    }
}

Coordinator::Coordinator() {
    ptr_ = rhythm_coordinator_create();
    if (ptr_) {
        rhythm_coordinator_set_event_callback(ptr_, CoordinatorEventBridge, this);
    }
}

Coordinator::~Coordinator() {
    if (ptr_) rhythm_coordinator_destroy(ptr_);
}

void Coordinator::SetLibrary(Library* library) {
    library_ = library;
    if (ptr_) rhythm_coordinator_set_library(ptr_, library ? library->Handle() : nullptr);
}

void Coordinator::SetEventHandler(std::function<void(const std::wstring&)> handler) {
    handler_ = std::move(handler);
}

CoordinatorResult Coordinator::Start(const Track& track,
                                     const std::vector<Track>& queueTracks,
                                     int32_t mode) {
    if (!ptr_) return {};
    auto trackJson = generated::TrackToJson(track).dump();
    auto queueJson = TracksToJson(queueTracks);
    char* raw = rhythm_coordinator_start(
        ptr_, library_ ? library_->Handle() : nullptr,
        trackJson.c_str(), queueJson.c_str(), mode);
    auto result = ParseCoordinatorResult(raw);
    if (raw) rhythm_free_string(raw);
    return result;
}

CoordinatorResult Coordinator::Next() {
    if (!ptr_) return {};
    char* raw = rhythm_coordinator_next(ptr_, library_ ? library_->Handle() : nullptr);
    auto result = ParseCoordinatorResult(raw);
    if (raw) rhythm_free_string(raw);
    return result;
}

CoordinatorResult Coordinator::Previous() {
    if (!ptr_) return {};
    char* raw = rhythm_coordinator_previous(ptr_, library_ ? library_->Handle() : nullptr);
    auto result = ParseCoordinatorResult(raw);
    if (raw) rhythm_free_string(raw);
    return result;
}

CoordinatorResult Coordinator::TogglePlayPause() {
    if (!ptr_) return {};
    char* raw = rhythm_coordinator_toggle_play_pause(ptr_, library_ ? library_->Handle() : nullptr);
    auto result = ParseCoordinatorResult(raw);
    if (raw) rhythm_free_string(raw);
    return result;
}

void Coordinator::SyncQueue(const std::vector<Track>& tracks) {
    if (!ptr_) return;
    auto queueJson = TracksToJson(tracks);
    rhythm_coordinator_sync_queue(ptr_, queueJson.c_str());
}

void Coordinator::Stop() {
    if (ptr_) rhythm_coordinator_stop(ptr_);
}

void Coordinator::SetVolume(float volume) {
    if (ptr_) rhythm_coordinator_set_volume(ptr_, volume);
}

void Coordinator::SetPlayMode(int32_t mode) {
    if (ptr_) rhythm_coordinator_set_play_mode(ptr_, mode);
}

bool Coordinator::HasNext() const {
    return ptr_ ? rhythm_coordinator_has_next(ptr_) != 0 : false;
}

bool Coordinator::HasPrevious() const {
    return ptr_ ? rhythm_coordinator_has_previous(ptr_) != 0 : false;
}

bool Coordinator::CanTogglePlayback() const {
    return ptr_ ? rhythm_coordinator_can_toggle_playback(ptr_) != 0 : false;
}

bool Coordinator::CanStop() const {
    return ptr_ ? rhythm_coordinator_can_stop(ptr_) != 0 : false;
}

double Coordinator::Position() const {
    return ptr_ ? rhythm_coordinator_get_position(ptr_) : 0.0;
}

double Coordinator::Duration() const {
    return ptr_ ? rhythm_coordinator_get_duration(ptr_) : 0.0;
}

int32_t Coordinator::State() const {
    return ptr_ ? rhythm_coordinator_get_state(ptr_) : -1;
}

std::wstring Coordinator::ErrorMessage() const {
    if (!ptr_) return {};
    char* raw = rhythm_coordinator_error(ptr_);
    if (!raw) return {};
    auto message = Utf8ToWide(raw);
    rhythm_free_string(raw);
    return message;
}

std::wstring Coordinator::ErrorKind() const {
    if (!ptr_) return {};
    char* raw = rhythm_coordinator_error_kind(ptr_);
    if (!raw) return {};
    auto kind = Utf8ToWide(raw);
    rhythm_free_string(raw);
    return kind;
}

std::optional<Track> Coordinator::CurrentTrack() const {
    if (!ptr_) return std::nullopt;
    char* raw = rhythm_coordinator_current_track(ptr_);
    if (!raw) return std::nullopt;
    auto track = generated::TrackFromJson(json::parse(raw));
    rhythm_free_string(raw);
    return track;
}

// ─── Resolver ───────────────────────────────────────────────────────

/// A resolved URL as a track that is not in the library yet (#364): what the
/// core resolved, plus the page URL the user pasted. Keep the page URL, not
/// the resolved CDN link: the core re-resolves (from cache) at playback time,
/// and those CDN links carry a deadline that expires. The rest keeps the
/// model's defaults (available, never played) -- decoding the payload as a
/// whole Track used to leave id 0 and mark the track unavailable.
static Track TrackFromResolved(const ResolvedUrl& resolved, const std::wstring& pageUrl) {
    Track track;
    track.id = -1;
    track.sourceType = resolved.sourceType;
    track.sourceUrl = pageUrl;
    track.title = resolved.title;
    track.artist = resolved.artist;
    track.duration = resolved.duration;
    return track;
}

/// A failed resolution with the given classification.
static ResolveOutcome ResolveFailure(std::wstring kind, std::wstring message = {}) {
    ResolveOutcome outcome;
    outcome.errorKind = std::move(kind);
    outcome.errorMessage = std::move(message);
    return outcome;
}

/// Parse the core's structured resolve result (#176): success payload +
/// classified error in a single return — the old "null, then query the
/// global error slot" two-step protocol is gone. The fields come from the
/// generated decoder (#364); only the checks a decoder cannot make (no
/// payload, malformed JSON, success without a payload) are handled here.
ResolveOutcome Resolver::ResolveURL(const std::wstring& url) {
    auto u = WideToUtf8(url);
    char* json_str = rhythm_resolve_url(u.c_str());
    if (!json_str) {
        // No payload at all: leave the message empty so the one fallback in
        // L10n::UrlResolveError picks the copy (#374, #412).
        return ResolveFailure(L"internal");
    }
    std::string payload(json_str);
    rhythm_free_string(json_str);

    ResolveResult result;
    try {
        result = generated::ResolveResultFromJson(json::parse(payload));
    } catch (const json::exception& e) {
        return ResolveFailure(L"internal", L"Malformed resolver response: " + Utf8ToWide(e.what()));
    }
    if (!result.ok) {
        return ResolveFailure(result.errorKind.value_or(L""), result.errorMessage.value_or(L""));
    }
    if (!result.resolved) {
        return ResolveFailure(L"internal", L"Malformed resolver response");
    }

    ResolveOutcome outcome;
    outcome.ok = true;
    outcome.track = TrackFromResolved(*result.resolved, url);
    return outcome;
}

std::wstring Resolver::Diagnostics() {
    char* raw = rhythm_resolver_diagnostics();
    if (!raw) return L"{}";
    auto result = Utf8ToWide(raw);
    rhythm_free_string(raw);
    return result;
}

ResolverStatus Resolver::Status() {
    ResolverStatus status;

    char* raw = rhythm_resolver_status();
    if (!raw) return status;

    std::string payload(raw);
    rhythm_free_string(raw);

    try {
        auto j = json::parse(payload);
        if (j.contains("phase") && !j["phase"].is_null()) {
            status.phase = Utf8ToWide(j["phase"].get<std::string>());
        }
        status.received = j.value("received", (int64_t)0);
        status.total = j.value("total", (int64_t)0);
    } catch (const json::exception&) {
        // Leave the idle default.
    }
    return status;
}

std::wstring Resolver::StatusText(const ResolverStatus& status) {
    // #141: all copy lives in L10n (system language, manual override).
    return L10n::ResolverStatusText(status.phase, status.received, status.total);
}

std::wstring Resolver::ClassifyURL(const std::wstring& url) {
    // #181: structured result — no global error slot.
    auto u = WideToUtf8(url);
    char* result = rhythm_classify_url(u.c_str());
    if (!result) return L"";
    std::wstring sourceType;
    try {
        auto j = json::parse(result);
        rhythm_free_string(result);
        if (j.value("ok", false) && j.contains("source_type") && !j["source_type"].is_null()) {
            sourceType = Utf8ToWide(j["source_type"].get<std::string>());
        }
    } catch (const json::exception&) {
        rhythm_free_string(result);
    }
    return sourceType;
}

} // namespace rhythm
