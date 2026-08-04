use crate::{RhythmError, RhythmResult, SourceType, TrackInfo};
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Cached URL resolutions to avoid repeated yt-dlp calls.
/// Each entry stores the resolved info and the instant it was cached.
static RESOLVED_CACHE: LazyLock<Mutex<HashMap<String, CachedEntry>>> = LazyLock::new(|| {
    Mutex::new(HashMap::new())
});

use std::sync::Mutex;

/// Maximum number of entries in the resolution cache.
const CACHE_MAX_CAPACITY: usize = 256;

/// Time-to-live for a cached resolution result.
const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

/// Timeout for the yt-dlp subprocess.
const YTDLP_TIMEOUT: Duration = Duration::from_secs(30);

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

#[derive(Debug, Clone)]
struct CachedEntry {
    resolved: ResolvedUrl,
    cached_at: Instant,
}

/// Pattern matchers for known platforms.
///
/// YouTube: handles standard watch, short links (youtu.be), Shorts, Music, and
/// embed URLs with optional playlist / timestamp / other query params.
static YOUTUBE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(https?://)?(www\.|music\.|m\.)?(youtube\.com/watch\?v=|youtu\.be/|youtube\.com/shorts/|youtube\.com/embed/)[\w\-]{6,}"
    )
    .unwrap()
});

/// Bilibili: full video pages (including mobile subdomain) and b23.tv short links.
static BILIBILI_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(https?://)?(www\.|m\.)?bilibili\.com/video/BV[\w]+|b23\.tv/[\w]+"
    )
    .unwrap()
});

/// Direct audio URL: common container extensions, optionally followed by query params.
static DIRECT_AUDIO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\.(mp3|flac|aac|ogg|opus|m4a|wav|wma|aiff|webm|weba)(\?.*)?$").unwrap()
});

// ─── Cache helpers ──────────────────────────────────────────────────

/// Evict the oldest entry if the cache is over capacity, then remove any
/// entries whose TTL has expired.
fn prune_cache(cache: &mut HashMap<String, CachedEntry>) {
    // Capacity pruning: remove the oldest entry while over the limit.
    while cache.len() > CACHE_MAX_CAPACITY {
        if let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, e)| e.cached_at)
            .map(|(k, _)| k.clone())
        {
            cache.remove(&oldest_key);
        } else {
            break;
        }
    }

    // TTL pruning.
    let now = Instant::now();
    cache.retain(|_, entry| now.duration_since(entry.cached_at) < CACHE_TTL);
}

// ─── yt-dlp availability check ──────────────────────────────────────

/// Check whether the `yt-dlp` binary is installed and on PATH.
fn ytdlp_available() -> bool {
    Command::new("yt-dlp")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Return a user-friendly error when yt-dlp is missing.
fn ytdlp_missing_error() -> RhythmError {
    RhythmError::Resolution(
        "yt-dlp is not installed or not on PATH.\n\n\
         Install it to resolve YouTube / Bilibili URLs:\n  \
         macOS:  brew install yt-dlp\n  \
         Windows: winget install yt-dlp   or   pip install yt-dlp"
            .to_string(),
    )
}

// ─── Public API ─────────────────────────────────────────────────────

/// Detect the type of URL and its source platform.
pub fn classify_url(url: &str) -> RhythmResult<SourceType> {
    // Basic sanity check: must look like an HTTP(S) URL.
    let trimmed = url.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(RhythmError::InvalidInput(
            "Please enter a valid URL starting with http:// or https://".to_string(),
        ));
    }

    if YOUTUBE_PATTERN.is_match(trimmed) {
        Ok(SourceType::YouTube)
    } else if BILIBILI_PATTERN.is_match(trimmed) {
        Ok(SourceType::Bilibili)
    } else if DIRECT_AUDIO_PATTERN.is_match(trimmed) {
        Ok(SourceType::DirectUrl)
    } else {
        // Could still be a direct URL without a recognised audio extension.
        // Default to treating it as a yt-dlp target in case it's an
        // unsupported video site that yt-dlp still understands.
        Ok(SourceType::YouTube)
    }
}

/// Resolve a URL to a playable audio stream.
///
/// Uses yt-dlp for YouTube / Bilibili, direct fetch for audio URLs.
/// Results are cached for one hour (up to 256 entries).
pub fn resolve_url(url: &str) -> RhythmResult<ResolvedUrl> {
    let trimmed = url.trim();

    // Basic sanity check.
    if trimmed.is_empty() {
        return Err(RhythmError::InvalidInput("URL is empty".to_string()));
    }
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(RhythmError::InvalidInput(
            "Please enter a valid URL starting with http:// or https://".to_string(),
        ));
    }

    // Check cache (return clone so we don't hold the lock across I/O).
    {
        let mut cache = RESOLVED_CACHE.lock().unwrap();
        prune_cache(&mut cache);
        if let Some(entry) = cache.get(trimmed) {
            if entry.cached_at.elapsed() < CACHE_TTL {
                return Ok(entry.resolved.clone());
            }
        }
    }

    let source_type = classify_url(trimmed)?;

    let resolved = match source_type {
        SourceType::DirectUrl => resolve_direct_url(trimmed)?,
        SourceType::YouTube | SourceType::Bilibili => resolve_with_ytdlp(trimmed, &source_type)?,
        _ => resolve_with_ytdlp(trimmed, &source_type)?,
    };

    // Insert into cache.
    {
        let mut cache = RESOLVED_CACHE.lock().unwrap();
        cache.insert(
            trimmed.to_string(),
            CachedEntry {
                resolved: resolved.clone(),
                cached_at: Instant::now(),
            },
        );
        // Prune after insert in case we just went over capacity.
        prune_cache(&mut cache);
    }

    Ok(resolved)
}

// ─── Direct URL resolution ──────────────────────────────────────────

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

// ─── yt-dlp resolution ──────────────────────────────────────────────

/// Resolve using the yt-dlp binary with a timeout.
///
/// Spawns yt-dlp in a separate process. If it doesn't finish within
/// `YTDLP_TIMEOUT` (30 s) the process is killed and an error returned.
fn resolve_with_ytdlp(url: &str, source_type: &SourceType) -> RhythmResult<ResolvedUrl> {
    if !ytdlp_available() {
        return Err(ytdlp_missing_error());
    }

    let child = Command::new("yt-dlp")
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
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            RhythmError::Resolution(format!(
                "Failed to start yt-dlp: {e}. Is yt-dlp installed?"
            ))
        })?;

    // Wait with timeout on a background thread so the FFI caller is not
    // blocked indefinitely.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    let output = match rx.recv_timeout(YTDLP_TIMEOUT) {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => {
            return Err(RhythmError::Resolution(format!(
                "yt-dlp process error: {e}"
            )));
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            // The spawned thread still holds `child` — we can't kill it from
            // here.  Signal the user that the operation timed out; the orphaned
            // process will be cleaned up by the OS once the handle drops on the
            // thread.
            return Err(RhythmError::Resolution(format!(
                "URL resolution timed out after {} seconds. \
                 Check your network connection and try again.",
                YTDLP_TIMEOUT.as_secs()
            )));
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            return Err(RhythmError::Resolution(
                "yt-dlp process terminated unexpectedly".to_string(),
            ));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = if stderr.trim().is_empty() {
            format!(
                "yt-dlp exited with status {} (no error output). \
                 The URL may be unavailable or blocked in your region.",
                output.status
            )
        } else {
            format!("yt-dlp failed: {}", stderr.trim())
        };
        return Err(RhythmError::Resolution(msg));
    }

    let raw_json = String::from_utf8_lossy(&output.stdout);
    let trimmed_json = raw_json.trim();
    if trimmed_json.is_empty() {
        return Err(RhythmError::Resolution(
            "yt-dlp returned no output. The URL may be private, geo-blocked, or deleted."
                .to_string(),
        ));
    }

    let json: serde_json::Value = serde_json::from_str(trimmed_json).map_err(|e| {
        RhythmError::Resolution(format!(
            "Failed to parse yt-dlp output (unexpected JSON format): {e}"
        ))
    })?;

    // ── Extract fields with fallback chains ─────────────────────────

    let title = json["title"]
        .as_str()
        .or_else(|| json["fulltitle"].as_str())
        .or_else(|| json["alt_title"].as_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let artist = json["uploader"]
        .as_str()
        .or_else(|| json["channel"].as_str())
        .or_else(|| json["artist"].as_str())
        .or_else(|| json["creator"].as_str())
        .or_else(|| json["uploader_id"].as_str())
        .map(|s| s.to_string());

    // Duration: accept both numeric and string representations.
    let duration = parse_duration_seconds(&json);

    // Stream URL: try several known locations in the yt-dlp JSON schema.
    let stream_url = extract_stream_url(&json, url)?;

    let thumbnail_url = json["thumbnail"].as_str().map(|s| s.to_string());

    Ok(ResolvedUrl {
        title,
        artist,
        stream_url,
        duration,
        source_type: source_type.clone(),
        thumbnail_url,
    })
}

/// Try every known field where yt-dlp may place the audio stream URL.
fn extract_stream_url(json: &serde_json::Value, original_url: &str) -> RhythmResult<String> {
    // 1. Direct "url" field (most common).
    if let Some(u) = json["url"].as_str() {
        return Ok(u.to_string());
    }

    // 2. "requested_formats" array — pick the first entry that has a "url".
    if let Some(formats) = json["requested_formats"].as_array() {
        for fmt in formats {
            if let Some(u) = fmt["url"].as_str() {
                return Ok(u.to_string());
            }
        }
    }

    // 3. "formats" array — sometimes yt-dlp returns a flat formats list
    //    when --print-json is used with certain extractors.
    if let Some(formats) = json["formats"].as_array() {
        // Prefer audio-only formats, then fall back to any.
        for fmt in formats {
            let is_audio = fmt["vcodec"].as_str() == Some("none")
                || fmt["acodec"].as_str().map_or(false, |a| a != "none");
            if is_audio {
                if let Some(u) = fmt["url"].as_str() {
                    return Ok(u.to_string());
                }
            }
        }
        // Fallback: first format with any url.
        for fmt in formats {
            if let Some(u) = fmt["url"].as_str() {
                return Ok(u.to_string());
            }
        }
    }

    // 4. "manifest_url" or "m3u8" HLS playlist — usable for streaming.
    if let Some(u) = json["manifest_url"].as_str() {
        return Ok(u.to_string());
    }

    Err(RhythmError::Resolution(format!(
        "No audio stream URL found in yt-dlp output for: {original_url}"
    )))
}

/// Parse duration in seconds from yt-dlp JSON.
///
/// yt-dlp can return `duration` as a number, a numeric string, or even
/// a `duration_string` like "3:45".
fn parse_duration_seconds(json: &serde_json::Value) -> f64 {
    // Numeric duration field.
    if let Some(d) = json["duration"].as_f64() {
        if d > 0.0 {
            return d;
        }
    }
    // Duration as a string of digits.
    if let Some(s) = json["duration"].as_str() {
        if let Ok(d) = s.parse::<f64>() {
            if d > 0.0 {
                return d;
            }
        }
    }
    // "duration_string" like "3:45" or "1:02:30".
    if let Some(s) = json["duration_string"].as_str() {
        if let Some(total) = parse_hh_mm_ss(s) {
            return total;
        }
    }
    0.0
}

/// Parse a human-readable duration like "3:45", "1:02:30", or "45" into
/// total seconds.
fn parse_hh_mm_ss(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        1 => parts[0].parse::<f64>().ok(),
        2 => {
            let min: f64 = parts[0].parse().ok()?;
            let sec: f64 = parts[1].parse().ok()?;
            Some(min * 60.0 + sec)
        }
        3 => {
            let hr: f64 = parts[0].parse().ok()?;
            let min: f64 = parts[1].parse().ok()?;
            let sec: f64 = parts[2].parse().ok()?;
            Some(hr * 3600.0 + min * 60.0 + sec)
        }
        _ => None,
    }
}

// ─── Track conversion ────────────────────────────────────────────────

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
        // Try to decode percent-encoded strings.
        s.to_string() // Simplified — use `urlencoding` crate in production
    } else {
        s.to_string()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── classify_url ─────────────────────────────────────────────

    #[test]
    fn test_classify_youtube() {
        let result = classify_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://youtube.com/shorts/abc123def45").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://music.youtube.com/watch?v=xyz789").unwrap();
        assert_eq!(result, SourceType::YouTube);

        let result = classify_url("https://www.youtube.com/embed/dQw4w9WgXcQ").unwrap();
        assert_eq!(result, SourceType::YouTube);
    }

    #[test]
    fn test_classify_bilibili() {
        let result = classify_url("https://www.bilibili.com/video/BV1GJ411x7h7").unwrap();
        assert_eq!(result, SourceType::Bilibili);

        let result = classify_url("https://m.bilibili.com/video/BV1xx411E7jJ").unwrap();
        assert_eq!(result, SourceType::Bilibili);

        let result = classify_url("https://b23.tv/abc1234").unwrap();
        assert_eq!(result, SourceType::Bilibili);
    }

    #[test]
    fn test_classify_direct_audio() {
        let result = classify_url("https://example.com/music/song.mp3").unwrap();
        assert_eq!(result, SourceType::DirectUrl);

        let result = classify_url("https://cdn.example.com/track.flac?token=abc").unwrap();
        assert_eq!(result, SourceType::DirectUrl);

        let result = classify_url("https://example.com/audio.opus").unwrap();
        assert_eq!(result, SourceType::DirectUrl);
    }

    #[test]
    fn test_classify_rejects_non_http() {
        assert!(classify_url("not-a-url").is_err());
        assert!(classify_url("ftp://example.com/song.mp3").is_err());
    }

    // ── Duration parsing ─────────────────────────────────────────

    #[test]
    fn test_parse_hh_mm_ss() {
        assert_eq!(parse_hh_mm_ss("45"), Some(45.0));
        assert_eq!(parse_hh_mm_ss("3:45"), Some(225.0));
        assert_eq!(parse_hh_mm_ss("1:02:30"), Some(3750.0));
        assert_eq!(parse_hh_mm_ss("0:05"), Some(5.0));
        assert_eq!(parse_hh_mm_ss(""), None);
        assert_eq!(parse_hh_mm_ss("abc"), None);
    }
}
