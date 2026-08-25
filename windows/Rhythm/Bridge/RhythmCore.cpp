#include "pch.h"
#include "RhythmCore.h"

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

static std::string WideToUtf8(const std::wstring& ws) {
    if (ws.empty()) return {};
    int len = WideCharToMultiByte(CP_UTF8, 0, ws.data(), (int)ws.size(), nullptr, 0, nullptr, nullptr);
    std::string result(len, '\0');
    WideCharToMultiByte(CP_UTF8, 0, ws.data(), (int)ws.size(), result.data(), len, nullptr, nullptr);
    return result;
}

static Track JsonToTrack(const json& j) {
    Track t;
    t.id = j.value("id", 0);
    if (j.contains("file_path") && !j["file_path"].is_null())
        t.filePath = Utf8ToWide(j["file_path"].get<std::string>());
    t.sourceType = Utf8ToWide(j.value("source_type", "local"));
    if (j.contains("source_url") && !j["source_url"].is_null())
        t.sourceUrl = Utf8ToWide(j["source_url"].get<std::string>());
    t.title = Utf8ToWide(j.value("title", "Unknown"));
    if (j.contains("artist") && !j["artist"].is_null())
        t.artist = Utf8ToWide(j["artist"].get<std::string>());
    if (j.contains("album") && !j["album"].is_null())
        t.album = Utf8ToWide(j["album"].get<std::string>());
    if (j.contains("album_artist") && !j["album_artist"].is_null())
        t.albumArtist = Utf8ToWide(j["album_artist"].get<std::string>());
    t.duration = j.value("duration", 0.0);
    if (j.contains("format") && !j["format"].is_null())
        t.format = Utf8ToWide(j["format"].get<std::string>());
    t.trackNumber = j.contains("track_number") && !j["track_number"].is_null()
        ? std::optional(j["track_number"].get<int32_t>()) : std::nullopt;
    t.discNumber = j.contains("disc_number") && !j["disc_number"].is_null()
        ? std::optional(j["disc_number"].get<int32_t>()) : std::nullopt;
    t.year = j.contains("year") && !j["year"].is_null()
        ? std::optional(j["year"].get<int32_t>()) : std::nullopt;
    t.bitrate = j.contains("bitrate") && !j["bitrate"].is_null()
        ? std::optional(j["bitrate"].get<int32_t>()) : std::nullopt;
    t.sampleRate = j.contains("sample_rate") && !j["sample_rate"].is_null()
        ? std::optional(j["sample_rate"].get<int32_t>()) : std::nullopt;
    t.channels = j.contains("channels") && !j["channels"].is_null()
        ? std::optional(j["channels"].get<int32_t>()) : std::nullopt;
    if (j.contains("genre") && !j["genre"].is_null())
        t.genre = Utf8ToWide(j["genre"].get<std::string>());
    if (j.contains("file_size") && !j["file_size"].is_null())
        t.fileSize = std::optional(j["file_size"].get<int64_t>());
    if (j.contains("date_added") && !j["date_added"].is_null())
        t.dateAdded = Utf8ToWide(j["date_added"].get<std::string>());
    if (j.contains("last_played") && !j["last_played"].is_null())
        t.lastPlayed = Utf8ToWide(j["last_played"].get<std::string>());
    t.playCount = j.value("play_count", 0);
    t.isAvailable = j.value("is_available", true);
    if (j.contains("artwork_path") && !j["artwork_path"].is_null())
        t.artworkPath = Utf8ToWide(j["artwork_path"].get<std::string>());
    return t;
}

Track ParseTrackJson(const std::string& json) {
    try {
        return JsonToTrack(json::parse(json));
    } catch (const json::exception&) {
        return Track{};
    }
}

static std::vector<Track> ParseTrackList(const char* json) {
    if (!json) return {};
    auto j = json::parse(json);
    std::vector<Track> tracks;
    for (const auto& item : j) {
        tracks.push_back(JsonToTrack(item));
    }
    return tracks;
}

static std::vector<Playlist> ParsePlaylistList(const char* json) {
    if (!json) return {};
    auto j = json::parse(json);
    std::vector<Playlist> playlists;
    for (const auto& item : j) {
        Playlist p;
        p.id = item.value("id", 0);
        p.name = Utf8ToWide(item.value("name", ""));
        if (item.contains("description") && !item["description"].is_null())
            p.description = Utf8ToWide(item["description"].get<std::string>());
        for (const auto& tj : item["tracks"]) {
            p.tracks.push_back(JsonToTrack(tj));
        }
        playlists.push_back(p);
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

int32_t Library::ImportDirectory(const std::wstring& path) {
    if (!ptr_) return -1;
    auto p = WideToUtf8(path);
    return rhythm_library_import(ptr_, p.c_str());
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

static std::string TrackToJson(const Track& t) {
    json j;
    j["id"] = t.id;
    if (t.filePath) j["file_path"] = WideToUtf8(*t.filePath);
    j["source_type"] = WideToUtf8(t.sourceType);
    if (t.sourceUrl) j["source_url"] = WideToUtf8(*t.sourceUrl);
    j["title"] = WideToUtf8(t.title);
    if (t.artist) j["artist"] = WideToUtf8(*t.artist);
    if (t.album) j["album"] = WideToUtf8(*t.album);
    if (t.albumArtist) j["album_artist"] = WideToUtf8(*t.albumArtist);
    if (t.trackNumber) j["track_number"] = *t.trackNumber;
    if (t.discNumber) j["disc_number"] = *t.discNumber;
    if (t.genre) j["genre"] = WideToUtf8(*t.genre);
    if (t.year) j["year"] = *t.year;
    j["duration"] = t.duration;
    if (t.format) j["format"] = WideToUtf8(*t.format);
    if (t.bitrate) j["bitrate"] = *t.bitrate;
    if (t.sampleRate) j["sample_rate"] = *t.sampleRate;
    if (t.channels) j["channels"] = *t.channels;
    if (t.fileSize) j["file_size"] = *t.fileSize;
    if (t.dateAdded) j["date_added"] = WideToUtf8(*t.dateAdded);
    if (t.lastPlayed) j["last_played"] = WideToUtf8(*t.lastPlayed);
    j["play_count"] = t.playCount;
    if (t.artworkPath) j["artwork_path"] = WideToUtf8(*t.artworkPath);
    j["is_available"] = t.isAvailable;
    return j.dump();
}

Track Library::AddTrack(const Track& track) {
    if (!ptr_) return track;
    auto json = TrackToJson(track);
    char* result = rhythm_library_add_track(ptr_, json.c_str());
    if (!result) return track;
    auto saved = JsonToTrack(json::parse(result));
    rhythm_free_string(result);
    return saved;
}

bool Library::RemoveTrack(int64_t id) {
    if (!ptr_) return false;
    return rhythm_library_remove_track(ptr_, id) == 0;
}

/// Decode the core's positional M3U8 entries ([title, artist, location]).
static std::vector<M3u8Entry> ParseM3u8Entries(const char* json) {
    std::vector<M3u8Entry> entries;
    if (!json) return entries;
    try {
        auto j = json::parse(json);
        for (const auto& item : j) {
            M3u8Entry entry;
            if (item.size() > 0 && !item[0].is_null()) {
                entry.title = Utf8ToWide(item[0].get<std::string>());
            }
            if (item.size() > 1 && !item[1].is_null()) {
                entry.artist = Utf8ToWide(item[1].get<std::string>());
            }
            if (item.size() > 2 && !item[2].is_null()) {
                entry.location = Utf8ToWide(item[2].get<std::string>());
            }
            entries.push_back(std::move(entry));
        }
    } catch (const json::exception&) {
        // Malformed payload: nothing to import.
    }
    return entries;
}

std::vector<M3u8Entry> Library::ImportM3U8(const std::wstring& path) {
    if (!ptr_) return {};
    auto p = WideToUtf8(path);
    char* raw = rhythm_import_m3u8(p.c_str());
    auto entries = ParseM3u8Entries(raw);
    if (raw) rhythm_free_string(raw);
    return entries;
}

// ─── Player ─────────────────────────────────────────────────────────

Player::Player() {
    ptr_ = rhythm_player_create();
}

Player::~Player() {
    if (ptr_) rhythm_player_destroy(ptr_);
}

void Player::PlayFile(const std::wstring& path) {
    if (!ptr_) return;
    auto p = WideToUtf8(path);
    rhythm_player_play_file(ptr_, p.c_str());
}

void Player::PlayURL(const std::wstring& url) {
    if (!ptr_) return;
    auto u = WideToUtf8(url);
    rhythm_player_play_url(ptr_, u.c_str());
}

void Player::Pause() { if (ptr_) rhythm_player_pause(ptr_); }
void Player::Resume() { if (ptr_) rhythm_player_resume(ptr_); }
void Player::Stop() { if (ptr_) rhythm_player_stop(ptr_); }
void Player::SetVolume(float v) { if (ptr_) rhythm_player_set_volume(ptr_, v); }
float Player::Volume() const { return ptr_ ? rhythm_player_get_volume(ptr_) : 0.0f; }
int32_t Player::State() const { return ptr_ ? rhythm_player_get_state(ptr_) : -1; }

std::wstring Player::ErrorMessage() const {
    if (!ptr_) return {};
    char* raw = rhythm_player_error(ptr_);
    if (!raw) return {};
    auto message = Utf8ToWide(raw);
    rhythm_free_string(raw);
    return message;
}

std::wstring Player::ErrorKind() const {
    if (!ptr_) return {};
    char* raw = rhythm_player_error_kind(ptr_);
    if (!raw) return {};
    auto kind = Utf8ToWide(raw);
    rhythm_free_string(raw);
    return kind;
}
double Player::Position() const { return ptr_ ? rhythm_player_get_position(ptr_) : 0.0; }
double Player::Duration() const { return ptr_ ? rhythm_player_get_duration(ptr_) : 0.0; }

// ─── Coordinator ────────────────────────────────────────────────────

/// Parse the core's structured result JSON into a CoordinatorResult.
static CoordinatorResult ParseCoordinatorResult(const char* json) {
    CoordinatorResult result;
    if (!json) return result;
    try {
        auto j = json::parse(json);
        result.ok = j.value("ok", false);
        if (j.contains("error_kind") && !j["error_kind"].is_null()) {
            result.errorKind = Utf8ToWide(j["error_kind"].get<std::string>());
        }
        if (j.contains("error_message") && !j["error_message"].is_null()) {
            result.errorMessage = Utf8ToWide(j["error_message"].get<std::string>());
        }
        if (j.contains("current_track") && !j["current_track"].is_null()) {
            result.currentTrack = JsonToTrack(j["current_track"]);
        }
        result.playbackActive = j.value("playback_active", false);
    } catch (const json::exception&) {
        result.ok = false;
        result.errorKind = L"internal";
        result.errorMessage = L"Malformed coordinator response";
    }
    return result;
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
    auto trackJson = TrackToJson(track);
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
    auto track = JsonToTrack(json::parse(raw));
    rhythm_free_string(raw);
    return track;
}

// ─── Resolver ───────────────────────────────────────────────────────

/// Read the core's last resolution failure. Falls back to a generic message
/// when no payload is available.
static ResolveOutcome LastResolveFailure() {
    ResolveOutcome outcome;
    outcome.ok = false;
    outcome.errorKind = L"internal";
    outcome.errorMessage = L"Failed to resolve the URL.";

    char* raw = rhythm_last_error();
    if (!raw) return outcome;

    std::string payload(raw);
    rhythm_free_string(raw);

    try {
        auto j = json::parse(payload);
        if (j.contains("kind") && !j["kind"].is_null()) {
            outcome.errorKind = Utf8ToWide(j["kind"].get<std::string>());
        }
        if (j.contains("message") && !j["message"].is_null()) {
            outcome.errorMessage = Utf8ToWide(j["message"].get<std::string>());
        }
    } catch (const json::exception&) {
        // Keep the generic message.
    }
    return outcome;
}

ResolveOutcome Resolver::ResolveURL(const std::wstring& url) {
    auto u = WideToUtf8(url);
    char* json_str = rhythm_resolve_url(u.c_str());
    // A null return means the core recorded a reason — surface it instead of
    // handing the UI an unplayable placeholder track (#21).
    if (!json_str) return LastResolveFailure();

    ResolveOutcome outcome;
    try {
        auto j = json::parse(json_str);
        rhythm_free_string(json_str);

        outcome.track = JsonToTrack(j);
        // Keep the page URL, not the resolved CDN link: the core re-resolves
        // (from cache) at playback time, and those CDN links carry a deadline
        // that expires.
        outcome.track.sourceUrl = url;
        outcome.ok = true;
    } catch (const json::exception& e) {
        rhythm_free_string(json_str);
        outcome.ok = false;
        outcome.errorKind = L"internal";
        outcome.errorMessage = L"Malformed resolver response: " + Utf8ToWide(e.what());
    }
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
    auto u = WideToUtf8(url);
    char* result = rhythm_classify_url(u.c_str());
    if (!result) return L"";
    auto s = Utf8ToWide(result);
    rhythm_free_string(result);
    return s;
}

} // namespace rhythm
