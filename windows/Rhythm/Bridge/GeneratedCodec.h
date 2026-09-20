// 本文件由 scripts/gen-ffi-bindings.py 从 contracts/ffi-contract.json 生成（#180）。
// 请勿手改——改契约后重新生成。
#pragma once

#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "RhythmCore.h"
#include "MessageSpec.h"

namespace rhythm::generated {

// 模型与 Utf8ToWide 声明在 RhythmCore.h，WideToUtf8 声明在 MessageSpec.h（#413）。
using nlohmann::json;


/// Decode a Track from the core's snake_case JSON (contract #Track).
inline Track TrackFromJson(const json& j) {
    Track t;
    t.id = j.value("id", (int64_t)0);
    if (j.contains("file_path") && !j["file_path"].is_null()) {
        t.filePath = Utf8ToWide(j["file_path"].get<std::string>());
    }
    t.sourceType = Utf8ToWide(j.value("source_type", std::string("local")));
    if (j.contains("source_url") && !j["source_url"].is_null()) {
        t.sourceUrl = Utf8ToWide(j["source_url"].get<std::string>());
    }
    t.title = Utf8ToWide(j.value("title", std::string("")));
    if (j.contains("artist") && !j["artist"].is_null()) {
        t.artist = Utf8ToWide(j["artist"].get<std::string>());
    }
    if (j.contains("album") && !j["album"].is_null()) {
        t.album = Utf8ToWide(j["album"].get<std::string>());
    }
    if (j.contains("album_artist") && !j["album_artist"].is_null()) {
        t.albumArtist = Utf8ToWide(j["album_artist"].get<std::string>());
    }
    if (j.contains("track_number") && !j["track_number"].is_null()) {
        t.trackNumber = j["track_number"].get<int32_t>();
    }
    if (j.contains("disc_number") && !j["disc_number"].is_null()) {
        t.discNumber = j["disc_number"].get<int32_t>();
    }
    if (j.contains("genre") && !j["genre"].is_null()) {
        t.genre = Utf8ToWide(j["genre"].get<std::string>());
    }
    if (j.contains("year") && !j["year"].is_null()) {
        t.year = j["year"].get<int32_t>();
    }
    t.duration = j.value("duration", 0.0);
    if (j.contains("format") && !j["format"].is_null()) {
        t.format = Utf8ToWide(j["format"].get<std::string>());
    }
    if (j.contains("bitrate") && !j["bitrate"].is_null()) {
        t.bitrate = j["bitrate"].get<int32_t>();
    }
    if (j.contains("sample_rate") && !j["sample_rate"].is_null()) {
        t.sampleRate = j["sample_rate"].get<int32_t>();
    }
    if (j.contains("channels") && !j["channels"].is_null()) {
        t.channels = j["channels"].get<int32_t>();
    }
    if (j.contains("file_size") && !j["file_size"].is_null()) {
        t.fileSize = j["file_size"].get<int64_t>();
    }
    if (j.contains("date_added") && !j["date_added"].is_null()) {
        t.dateAdded = Utf8ToWide(j["date_added"].get<std::string>());
    }
    if (j.contains("last_played") && !j["last_played"].is_null()) {
        t.lastPlayed = Utf8ToWide(j["last_played"].get<std::string>());
    }
    t.playCount = j.value("play_count", (int32_t)0);
    if (j.contains("artwork_path") && !j["artwork_path"].is_null()) {
        t.artworkPath = Utf8ToWide(j["artwork_path"].get<std::string>());
    }
    t.isAvailable = j.value("is_available", false);
    return t;
}

/// Encode a Track with snake_case keys (contract #Track).
inline json TrackToJson(const Track& t) {
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
    return j;
}

/// Visit every contract field of a Track as visit(contract key, member) (#365).
template <typename Visit>
void ForEachField(Track& t, Visit&& visit) {
    visit("id", t.id);
    visit("file_path", t.filePath);
    visit("source_type", t.sourceType);
    visit("source_url", t.sourceUrl);
    visit("title", t.title);
    visit("artist", t.artist);
    visit("album", t.album);
    visit("album_artist", t.albumArtist);
    visit("track_number", t.trackNumber);
    visit("disc_number", t.discNumber);
    visit("genre", t.genre);
    visit("year", t.year);
    visit("duration", t.duration);
    visit("format", t.format);
    visit("bitrate", t.bitrate);
    visit("sample_rate", t.sampleRate);
    visit("channels", t.channels);
    visit("file_size", t.fileSize);
    visit("date_added", t.dateAdded);
    visit("last_played", t.lastPlayed);
    visit("play_count", t.playCount);
    visit("artwork_path", t.artworkPath);
    visit("is_available", t.isAvailable);
}

/// Decode a Playlist from the core's snake_case JSON (contract #Playlist).
inline Playlist PlaylistFromJson(const json& j) {
    Playlist t;
    if (j.contains("id") && !j["id"].is_null()) {
        t.id = j["id"].get<int64_t>();
    }
    t.name = Utf8ToWide(j.value("name", std::string("")));
    if (j.contains("description") && !j["description"].is_null()) {
        t.description = Utf8ToWide(j["description"].get<std::string>());
    }
    if (j.contains("tracks") && !j["tracks"].is_null()) {
        for (const auto& item : j["tracks"]) {
            t.tracks.push_back(TrackFromJson(item));
        }
    }
    return t;
}

/// Encode a Playlist with snake_case keys (contract #Playlist).
inline json PlaylistToJson(const Playlist& t) {
    json j;
    if (t.id) j["id"] = *t.id;
    j["name"] = WideToUtf8(t.name);
    if (t.description) j["description"] = WideToUtf8(*t.description);
    j["tracks"] = json::array();
    for (const auto& item : t.tracks) {
        j["tracks"].push_back(TrackToJson(item));
    }
    return j;
}

/// Visit every contract field of a Playlist as visit(contract key, member) (#365).
template <typename Visit>
void ForEachField(Playlist& t, Visit&& visit) {
    visit("id", t.id);
    visit("name", t.name);
    visit("description", t.description);
    visit("tracks", t.tracks);
}

/// Decode a M3u8Entry from the core's snake_case JSON (contract #M3u8Entry).
inline M3u8Entry M3u8EntryFromJson(const json& j) {
    M3u8Entry t;
    t.title = Utf8ToWide(j.value("title", std::string("")));
    if (j.contains("artist") && !j["artist"].is_null()) {
        t.artist = Utf8ToWide(j["artist"].get<std::string>());
    }
    t.location = Utf8ToWide(j.value("location", std::string("")));
    return t;
}

/// Encode a M3u8Entry with snake_case keys (contract #M3u8Entry).
inline json M3u8EntryToJson(const M3u8Entry& t) {
    json j;
    j["title"] = WideToUtf8(t.title);
    if (t.artist) j["artist"] = WideToUtf8(*t.artist);
    j["location"] = WideToUtf8(t.location);
    return j;
}

/// Visit every contract field of a M3u8Entry as visit(contract key, member) (#365).
template <typename Visit>
void ForEachField(M3u8Entry& t, Visit&& visit) {
    visit("title", t.title);
    visit("artist", t.artist);
    visit("location", t.location);
}

/// Decode a M3u8ImportOutcome from the core's snake_case JSON (contract #M3u8ImportOutcome).
inline M3u8ImportOutcome M3u8ImportOutcomeFromJson(const json& j) {
    M3u8ImportOutcome t;
    t.imported = j.value("imported", (int32_t)0);
    t.failed = j.value("failed", (int32_t)0);
    return t;
}

/// Encode a M3u8ImportOutcome with snake_case keys (contract #M3u8ImportOutcome).
inline json M3u8ImportOutcomeToJson(const M3u8ImportOutcome& t) {
    json j;
    j["imported"] = t.imported;
    j["failed"] = t.failed;
    return j;
}

/// Visit every contract field of a M3u8ImportOutcome as visit(contract key, member) (#365).
template <typename Visit>
void ForEachField(M3u8ImportOutcome& t, Visit&& visit) {
    visit("imported", t.imported);
    visit("failed", t.failed);
}

/// Decode a ImportOutcome from the core's snake_case JSON (contract #ImportOutcome).
inline ImportOutcome ImportOutcomeFromJson(const json& j) {
    ImportOutcome t;
    t.imported = j.value("imported", (int32_t)0);
    t.unsupported = j.value("unsupported", (int32_t)0);
    t.failed = j.value("failed", (int32_t)0);
    return t;
}

/// Encode a ImportOutcome with snake_case keys (contract #ImportOutcome).
inline json ImportOutcomeToJson(const ImportOutcome& t) {
    json j;
    j["imported"] = t.imported;
    j["unsupported"] = t.unsupported;
    j["failed"] = t.failed;
    return j;
}

/// Visit every contract field of a ImportOutcome as visit(contract key, member) (#365).
template <typename Visit>
void ForEachField(ImportOutcome& t, Visit&& visit) {
    visit("imported", t.imported);
    visit("unsupported", t.unsupported);
    visit("failed", t.failed);
}

/// Decode a ResolvedUrl from the core's snake_case JSON (contract #ResolvedUrl).
inline ResolvedUrl ResolvedUrlFromJson(const json& j) {
    ResolvedUrl t;
    t.title = Utf8ToWide(j.value("title", std::string("")));
    if (j.contains("artist") && !j["artist"].is_null()) {
        t.artist = Utf8ToWide(j["artist"].get<std::string>());
    }
    t.streamUrl = Utf8ToWide(j.value("stream_url", std::string("")));
    t.duration = j.value("duration", 0.0);
    t.sourceType = Utf8ToWide(j.value("source_type", std::string("local")));
    if (j.contains("thumbnail_url") && !j["thumbnail_url"].is_null()) {
        t.thumbnailUrl = Utf8ToWide(j["thumbnail_url"].get<std::string>());
    }
    if (j.contains("http_headers") && !j["http_headers"].is_null()) {
        for (const auto& [k, v] : j["http_headers"].items()) {
            t.httpHeaders[Utf8ToWide(k)] = Utf8ToWide(v.get<std::string>());
        }
    }
    return t;
}

/// Encode a ResolvedUrl with snake_case keys (contract #ResolvedUrl).
inline json ResolvedUrlToJson(const ResolvedUrl& t) {
    json j;
    j["title"] = WideToUtf8(t.title);
    if (t.artist) j["artist"] = WideToUtf8(*t.artist);
    j["stream_url"] = WideToUtf8(t.streamUrl);
    j["duration"] = t.duration;
    j["source_type"] = WideToUtf8(t.sourceType);
    if (t.thumbnailUrl) j["thumbnail_url"] = WideToUtf8(*t.thumbnailUrl);
    for (const auto& [k, v] : t.httpHeaders) {
        j["http_headers"][WideToUtf8(k)] = WideToUtf8(v);
    }
    return j;
}

/// Visit every contract field of a ResolvedUrl as visit(contract key, member) (#365).
template <typename Visit>
void ForEachField(ResolvedUrl& t, Visit&& visit) {
    visit("title", t.title);
    visit("artist", t.artist);
    visit("stream_url", t.streamUrl);
    visit("duration", t.duration);
    visit("source_type", t.sourceType);
    visit("thumbnail_url", t.thumbnailUrl);
    visit("http_headers", t.httpHeaders);
}

/// Decode a ResolveResult from the core's snake_case JSON (contract #ResolveResult).
inline ResolveResult ResolveResultFromJson(const json& j) {
    ResolveResult t;
    t.ok = j.value("ok", false);
    if (j.contains("resolved") && !j["resolved"].is_null()) {
        t.resolved = ResolvedUrlFromJson(j["resolved"]);
    }
    if (j.contains("error_kind") && !j["error_kind"].is_null()) {
        t.errorKind = Utf8ToWide(j["error_kind"].get<std::string>());
    }
    if (j.contains("error_message") && !j["error_message"].is_null()) {
        t.errorMessage = Utf8ToWide(j["error_message"].get<std::string>());
    }
    return t;
}

/// Encode a ResolveResult with snake_case keys (contract #ResolveResult).
inline json ResolveResultToJson(const ResolveResult& t) {
    json j;
    j["ok"] = t.ok;
    if (t.resolved) j["resolved"] = ResolvedUrlToJson(*t.resolved);
    if (t.errorKind) j["error_kind"] = WideToUtf8(*t.errorKind);
    if (t.errorMessage) j["error_message"] = WideToUtf8(*t.errorMessage);
    return j;
}

/// Visit every contract field of a ResolveResult as visit(contract key, member) (#365).
template <typename Visit>
void ForEachField(ResolveResult& t, Visit&& visit) {
    visit("ok", t.ok);
    visit("resolved", t.resolved);
    visit("error_kind", t.errorKind);
    visit("error_message", t.errorMessage);
}

/// Decode a CoordinatorResult from the core's snake_case JSON (contract #CoordinatorResult).
inline CoordinatorResult CoordinatorResultFromJson(const json& j) {
    CoordinatorResult t;
    t.ok = j.value("ok", false);
    if (j.contains("error_kind") && !j["error_kind"].is_null()) {
        t.errorKind = Utf8ToWide(j["error_kind"].get<std::string>());
    }
    if (j.contains("error_message") && !j["error_message"].is_null()) {
        t.errorMessage = Utf8ToWide(j["error_message"].get<std::string>());
    }
    if (j.contains("current_track") && !j["current_track"].is_null()) {
        t.currentTrack = TrackFromJson(j["current_track"]);
    }
    t.playbackActive = j.value("playback_active", false);
    return t;
}

/// Encode a CoordinatorResult with snake_case keys (contract #CoordinatorResult).
inline json CoordinatorResultToJson(const CoordinatorResult& t) {
    json j;
    j["ok"] = t.ok;
    if (t.errorKind) j["error_kind"] = WideToUtf8(*t.errorKind);
    if (t.errorMessage) j["error_message"] = WideToUtf8(*t.errorMessage);
    if (t.currentTrack) j["current_track"] = TrackToJson(*t.currentTrack);
    j["playback_active"] = t.playbackActive;
    return j;
}

/// Visit every contract field of a CoordinatorResult as visit(contract key, member) (#365).
template <typename Visit>
void ForEachField(CoordinatorResult& t, Visit&& visit) {
    visit("ok", t.ok);
    visit("error_kind", t.errorKind);
    visit("error_message", t.errorMessage);
    visit("current_track", t.currentTrack);
    visit("playback_active", t.playbackActive);
}

/// Every contract object's codec in contract order, as visit(contract key,
/// decoder, encoder) (#365). Cover every object by walking this list rather
/// than naming them: an object added to the contract joins on regeneration.
template <typename Visit>
void ForEachContractObject(Visit&& visit) {
    visit("track", TrackFromJson, TrackToJson);
    visit("playlist", PlaylistFromJson, PlaylistToJson);
    visit("m3u8_entry", M3u8EntryFromJson, M3u8EntryToJson);
    visit("m3u8_import_outcome", M3u8ImportOutcomeFromJson, M3u8ImportOutcomeToJson);
    visit("import_outcome", ImportOutcomeFromJson, ImportOutcomeToJson);
    visit("resolved_url", ResolvedUrlFromJson, ResolvedUrlToJson);
    visit("resolve_result", ResolveResultFromJson, ResolveResultToJson);
    visit("coordinator_result", CoordinatorResultFromJson, CoordinatorResultToJson);
}

} // namespace rhythm::generated
