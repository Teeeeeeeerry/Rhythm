//! URL classification (ticket #187 split): which source type a pasted URL
//! belongs to.

use crate::resolver::{ResolveErrorKind, ResolveFailure, ResolveResult, SourceType};
use regex::Regex;
use std::sync::LazyLock;

static YOUTUBE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(https?://)?(www\.|music\.|m\.)?(youtube\.com/watch\?v=|youtu\.be/|youtube\.com/shorts/|youtube\.com/embed/)[\w\-]{6,}"
    )
    .unwrap()
});

/// Bilibili: full video pages (including mobile subdomain) and b23.tv short links.
static BILIBILI_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(https?://)?(www\.|m\.)?bilibili\.com/video/BV[\w]+|b23\.tv/[\w]+").unwrap()
});

/// Direct audio URL: common container extensions, optionally followed by query params.
static DIRECT_AUDIO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // `m4s` covers DASH segments: those are what Bilibili's CDN serves, and
    // handing one back to yt-dlp only earns a 403 from its generic extractor.
    Regex::new(r"(?i)\.(mp3|flac|aac|ogg|opus|m4a|m4s|mp4|wav|wma|aiff|webm|weba)(\?.*)?$")
        .unwrap()
});

/// Detect the type of URL and its source platform.
pub fn classify_url(url: &str) -> ResolveResult<SourceType> {
    // Basic sanity check: must look like an HTTP(S) URL.
    let trimmed = url.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err(ResolveFailure::new(
            ResolveErrorKind::InvalidUrl,
            "Please enter a valid URL starting with http:// or https://",
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
