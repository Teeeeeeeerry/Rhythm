#include "pch.h"
#include "RhythmCore.h"

#include <nlohmann/json.hpp>
using json = nlohmann::json;

namespace rhythm {

// ─── Helpers ────────────────────────────────────────────────────────

static std::wstring Utf8ToWide(const std::string& s) {
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
    t.playCount = j.value("play_count", 0);
    t.isAvailable = j.value("is_available", true);
    if (j.contains("artwork_path") && !j["artwork_path"].is_null())
        t.artworkPath = Utf8ToWide(j["artwork_path"].get<std::string>());
    return t;
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
double Player::Position() const { return ptr_ ? rhythm_player_get_position(ptr_) : 0.0; }
double Player::Duration() const { return ptr_ ? rhythm_player_get_duration(ptr_) : 0.0; }

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
        // ResolvedUrl uses `stream_url` (the playable media URL), which
        // JsonToTrack maps to Track::sourceUrl.
        if (j.contains("stream_url") && !j["stream_url"].is_null()) {
            outcome.track.sourceUrl = Utf8ToWide(j["stream_url"].get<std::string>());
        } else {
            outcome.track.sourceUrl = url;
        }
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

std::wstring Resolver::ClassifyURL(const std::wstring& url) {
    auto u = WideToUtf8(url);
    char* result = rhythm_classify_url(u.c_str());
    if (!result) return L"";
    auto s = Utf8ToWide(result);
    rhythm_free_string(result);
    return s;
}

} // namespace rhythm
