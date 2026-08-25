// 本文件由 scripts/gen-ffi-bindings.py 从 contracts/ffi-contract.json 生成（#180）。
// 请勿手改——改契约后重新生成。
#pragma once

#include <nlohmann/json.hpp>

namespace rhythm::generated {

// Utf8ToWide / WideToUtf8 由 RhythmCore.cpp 提供（见 RhythmCore.h）。
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
    if (t.trackNumber) j["track_number"] = WideToUtf8(*t.trackNumber);
    if (t.discNumber) j["disc_number"] = WideToUtf8(*t.discNumber);
    if (t.genre) j["genre"] = WideToUtf8(*t.genre);
    if (t.year) j["year"] = WideToUtf8(*t.year);
    j["duration"] = t.duration;
    if (t.format) j["format"] = WideToUtf8(*t.format);
    if (t.bitrate) j["bitrate"] = WideToUtf8(*t.bitrate);
    if (t.sampleRate) j["sample_rate"] = WideToUtf8(*t.sampleRate);
    if (t.channels) j["channels"] = WideToUtf8(*t.channels);
    if (t.fileSize) j["file_size"] = WideToUtf8(*t.fileSize);
    if (t.dateAdded) j["date_added"] = WideToUtf8(*t.dateAdded);
    if (t.lastPlayed) j["last_played"] = WideToUtf8(*t.lastPlayed);
    j["play_count"] = t.playCount;
    if (t.artworkPath) j["artwork_path"] = WideToUtf8(*t.artworkPath);
    j["is_available"] = t.isAvailable;
    return j;
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


} // namespace rhythm::generated
