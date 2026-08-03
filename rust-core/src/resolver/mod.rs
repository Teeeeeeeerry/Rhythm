use crate::{RhythmError, RhythmResult, SourceType, TrackInfo};
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;
use std::sync::LazyLock;

/// Cached URL resolutions to avoid repeated yt-dlp calls.
/// Key: URL, Value: (title, stream_url, duration, uploader)
static RESOLVED_CACHE: LazyLock<Mutex<HashMap<String, ResolvedUrl>>> = LazyLock::new(|| {
    Mutex::new(HashMap::new())
});

use std::sync::Mutex;

/// The result of resolving a URL.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedUrl {
    pub title: String,
    pub artist: Option<String>,
    pub stream_url: String,
    pub duration: f64,
    pub source_type: SourceType,
    pub thumbnail_url: Option<String>,
}

/// Pattern matchers for known platforms
static YOUTUBE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(https?://)?(www\.)?(youtube\.com/watch\?v=|youtu\.be/|youtube\.com/shorts/|music\.youtube\.com/watch\?v=)[\w\-]+").unwrap()
});

static BILIBILI_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(https?://)?(www\.)?bilibili\.com/video/BV[\w]+|b23\.tv/[\w]+").unwrap()
});

static DIRECT_AUDIO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.(mp3|flac|aac|ogg|opus|m4a|wav|wma|aiff)(\?.*)?$").unwrap()
});

/// Detect the type of URL and its source platform.
pub fn classify_url(url: &str) -> RhythmResult<SourceType> {
    if YOUTUBE_PATTERN.is_match(url) {
        Ok(SourceType::YouTube)
    } else if BILIBILI_PATTERN.is_match(url) {
        Ok(SourceType::Bilibili)
    } else if DIRECT_AUDIO_PATTERN.is_match(url) {
        Ok(SourceType::DirectUrl)
    } else {
        // Could still be a direct URL without audio extension.
        // Default to trying yt-dlp as a fallback resolver.
        Ok(SourceType::YouTube) // Will be resolved by yt-dlp
    }
}

/// Resolve a URL to a playable audio stream.
/// Uses yt-dlp for YouTube/Bilibili, direct fetch for audio URLs.
pub fn resolve_url(url: &str) -> RhythmResult<ResolvedUrl> {
    // Check cache first
    {
        let cache = RESOLVED_CACHE.lock().unwrap();
        if let Some(resolved) = cache.get(url) {
            return Ok(resolved.clone());
        }
    }

    let source_type = classify_url(url)?;

    let resolved = match source_type {
        SourceType::DirectUrl => resolve_direct_url(url)?,
        SourceType::YouTube | SourceType::Bilibili => resolve_with_ytdlp(url, &source_type)?,
        _ => resolve_with_ytdlp(url, &source_type)?,
    };

    // Cache the result (with a TTL in production)
    {
        let mut cache = RESOLVED_CACHE.lock().unwrap();
        cache.insert(url.to_string(), resolved.clone());
    }

    Ok(resolved)
}

/// Simple resolution for direct audio URLs.
fn resolve_direct_url(url: &str) -> RhythmResult<ResolvedUrl> {
    let filename = url
        .rsplit('/')
        .next()
        .and_then(|s| s.split('?').next())
        .unwrap_or("Unknown");

    let title = urlencoding_if_needed(filename).to_string();

    Ok(ResolvedUrl {
        title,
        artist: None,
        stream_url: url.to_string(),
        duration: 0.0, // Unknown until playback starts
        source_type: SourceType::DirectUrl,
        thumbnail_url: None,
    })
}

/// Resolve using the yt-dlp binary.
fn resolve_with_ytdlp(url: &str, source_type: &SourceType) -> RhythmResult<ResolvedUrl> {
    let output = Command::new("yt-dlp")
        .args([
            "-f",
            "bestaudio/best",
            "--no-playlist",
            "--print-json",
            "--no-download",
            "--ignore-errors",
            "--no-check-certificates",
            url,
        ])
        .output()
        .map_err(|e| {
            RhythmError::Resolution(format!(
                "Failed to run yt-dlp: {e}. Is yt-dlp installed? Run: brew install yt-dlp"
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RhythmError::Resolution(format!(
            "yt-dlp failed: {stderr}"
        )));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(json_str.trim()).map_err(|e| {
        RhythmError::Resolution(format!("Failed to parse yt-dlp JSON output: {e}"))
    })?;

    let title = json["title"]
        .as_str()
        .unwrap_or("Unknown Title")
        .to_string();

    let artist = json["uploader"]
        .as_str()
        .or_else(|| json["channel"].as_str())
        .or_else(|| json["artist"].as_str())
        .map(|s| s.to_string());

    let duration = json["duration"].as_f64().unwrap_or(0.0);

    let stream_url = json["url"]
        .as_str()
        .or_else(|| json["requested_formats"].as_array()
            .and_then(|formats| {
                formats.iter().find_map(|f| f["url"].as_str())
            }))
        .ok_or_else(|| RhythmError::Resolution("No audio stream URL found".to_string()))?;

    let thumbnail_url = json["thumbnail"].as_str().map(|s| s.to_string());

    Ok(ResolvedUrl {
        title,
        artist,
        stream_url: stream_url.to_string(),
        duration,
        source_type: source_type.clone(),
        thumbnail_url,
    })
}

/// Build a TrackInfo from a resolved URL.
pub fn resolved_to_track(resolved: &ResolvedUrl, original_url: &str) -> TrackInfo {
    TrackInfo {
        id: None,
        file_path: None,
        source_type: resolved.source_type.clone(),
        source_url: Some(original_url.to_string()),
        title: resolved.title.clone(),
        artist: resolved.artist.clone(),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        genre: None,
        year: None,
        duration: resolved.duration,
        format: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_size: None,
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: resolved.thumbnail_url.clone(),
        is_available: true,
    }
}

/// Simple URL decoding helper.
fn urlencoding_if_needed(s: &str) -> String {
    if s.contains('%') {
        // Try to decode percent-encoded strings
        s.to_string() // Simplified — use `urlencoding` crate in production
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_youtube() {
        let result = classify_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(result, SourceType::YouTube);
    }

    #[test]
    fn test_classify_bilibili() {
        let result =
            classify_url("https://www.bilibili.com/video/BV1GJ411x7h7").unwrap();
        assert_eq!(result, SourceType::Bilibili);
    }

    #[test]
    fn test_classify_direct_audio() {
        let result = classify_url("https://example.com/music/song.mp3").unwrap();
        assert_eq!(result, SourceType::DirectUrl);

        let result = classify_url("https://cdn.example.com/track.flac?token=abc").unwrap();
        assert_eq!(result, SourceType::DirectUrl);
    }
}
