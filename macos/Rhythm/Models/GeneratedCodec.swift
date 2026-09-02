// 本文件由 scripts/gen-ffi-bindings.py 从 contracts/ffi-contract.json 生成（#180）。
// 请勿手改——改契约后重新生成。

import Foundation

/// 契约驱动的编解码（#180）：与 Codable+convertFromSnakeCase 路径等价，
/// 由契约测试锁定两者产物一致。
enum GeneratedCodec {

    /// Decode a Track from the core's snake_case JSON.
    static func decodeTrack(_ json: String) -> Track? {
        guard let data = json.data(using: .utf8),
              let obj = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { return nil }
        return Track(
            id: (obj["id"] as? NSNumber)?.int64Value ?? 0,
            filePath: obj["file_path"] as? String,
            sourceType: (obj["source_type"] as? String) ?? "direct_url",
            sourceUrl: obj["source_url"] as? String,
            title: (obj["title"] as? String) ?? "",
            artist: obj["artist"] as? String,
            album: obj["album"] as? String,
            albumArtist: obj["album_artist"] as? String,
            trackNumber: (obj["track_number"] as? NSNumber)?.intValue,
            discNumber: (obj["disc_number"] as? NSNumber)?.intValue,
            genre: obj["genre"] as? String,
            year: (obj["year"] as? NSNumber)?.intValue,
            duration: (obj["duration"] as? NSNumber)?.doubleValue ?? 0,
            format: obj["format"] as? String,
            bitrate: (obj["bitrate"] as? NSNumber)?.intValue,
            sampleRate: (obj["sample_rate"] as? NSNumber)?.intValue,
            channels: (obj["channels"] as? NSNumber)?.intValue,
            fileSize: (obj["file_size"] as? NSNumber)?.int64Value,
            dateAdded: obj["date_added"] as? String,
            lastPlayed: obj["last_played"] as? String,
            playCount: (obj["play_count"] as? NSNumber)?.intValue ?? 0,
            artworkPath: obj["artwork_path"] as? String,
            isAvailable: (obj["is_available"] as? NSNumber)?.boolValue ?? false
        )
    }

    /// Encode a Track with snake_case keys (mirror of the core's JSON).
    static func encodeTrack(_ value: Track) -> String {
        var obj: [String: Any] = [:]
        obj["id"] = value.id
        if let v = value.filePath { obj["file_path"] = v }
        obj["source_type"] = value.sourceType
        if let v = value.sourceUrl { obj["source_url"] = v }
        obj["title"] = value.title
        if let v = value.artist { obj["artist"] = v }
        if let v = value.album { obj["album"] = v }
        if let v = value.albumArtist { obj["album_artist"] = v }
        if let v = value.trackNumber { obj["track_number"] = v }
        if let v = value.discNumber { obj["disc_number"] = v }
        if let v = value.genre { obj["genre"] = v }
        if let v = value.year { obj["year"] = v }
        obj["duration"] = value.duration
        if let v = value.format { obj["format"] = v }
        if let v = value.bitrate { obj["bitrate"] = v }
        if let v = value.sampleRate { obj["sample_rate"] = v }
        if let v = value.channels { obj["channels"] = v }
        if let v = value.fileSize { obj["file_size"] = v }
        if let v = value.dateAdded { obj["date_added"] = v }
        if let v = value.lastPlayed { obj["last_played"] = v }
        obj["play_count"] = value.playCount
        if let v = value.artworkPath { obj["artwork_path"] = v }
        obj["is_available"] = value.isAvailable
        guard let data = try? JSONSerialization.data(withJSONObject: obj) else { return "{}" }
        return String(data: data, encoding: .utf8) ?? "{}"
    }

    /// Decode a M3u8Entry from the core's snake_case JSON.
    static func decodeM3u8Entry(_ json: String) -> M3u8Entry? {
        guard let data = json.data(using: .utf8),
              let obj = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { return nil }
        return M3u8Entry(
            title: (obj["title"] as? String) ?? "",
            artist: obj["artist"] as? String,
            location: (obj["location"] as? String) ?? ""
        )
    }

    /// Encode a M3u8Entry with snake_case keys (mirror of the core's JSON).
    static func encodeM3u8Entry(_ value: M3u8Entry) -> String {
        var obj: [String: Any] = [:]
        obj["title"] = value.title
        if let v = value.artist { obj["artist"] = v }
        obj["location"] = value.location
        guard let data = try? JSONSerialization.data(withJSONObject: obj) else { return "{}" }
        return String(data: data, encoding: .utf8) ?? "{}"
    }

    /// Decode a M3u8ImportOutcome from the core's snake_case JSON.
    static func decodeM3u8ImportOutcome(_ json: String) -> M3u8ImportOutcome? {
        guard let data = json.data(using: .utf8),
              let obj = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else { return nil }
        return M3u8ImportOutcome(
            imported: (obj["imported"] as? NSNumber)?.intValue ?? 0,
            failed: (obj["failed"] as? NSNumber)?.intValue ?? 0
        )
    }

    /// Encode a M3u8ImportOutcome with snake_case keys (mirror of the core's JSON).
    static func encodeM3u8ImportOutcome(_ value: M3u8ImportOutcome) -> String {
        var obj: [String: Any] = [:]
        obj["imported"] = value.imported
        obj["failed"] = value.failed
        guard let data = try? JSONSerialization.data(withJSONObject: obj) else { return "{}" }
        return String(data: data, encoding: .utf8) ?? "{}"
    }

    /// Decode a list of M3u8Entry objects (the M3U8 import path).
    static func decodeM3u8Entries(_ json: String) -> [M3u8Entry]? {
        guard let data = json.data(using: .utf8),
              let objects = (try? JSONSerialization.jsonObject(with: data)) as? [[String: Any]] else { return nil }
        return objects.compactMap { obj in
            M3u8Entry(
                title: (obj["title"] as? String) ?? "",
                artist: obj["artist"] as? String,
                location: (obj["location"] as? String) ?? ""
            )
        }
    }


}
