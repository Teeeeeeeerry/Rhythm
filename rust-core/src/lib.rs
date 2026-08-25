pub mod audio;
pub mod coordinator;
pub mod ffi;
pub mod library;
pub mod metadata;
pub mod playlist;
pub mod queue;
pub mod resolver;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RhythmError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Audio decode error: {0}")]
    Decode(String),

    #[error("Audio output error: {0}")]
    Output(String),

    #[error("Metadata error: {0}")]
    Metadata(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Network error: {0}")]
    Network(String),

    /// An HTTP request to a stream URL failed, with enough context to tell a
    /// genuinely expired link from a CDN rejecting a still-valid one (#120).
    #[error("Network error: {0}")]
    Http(HttpError),

    #[error("URL resolution error: {0}")]
    Resolution(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type RhythmResult<T> = Result<T, RhythmError>;

/// Why an HTTP request to a stream URL failed, so the UI can stop blaming
/// every failure on an "expired link" (#120).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpErrorKind {
    /// The URL's `expire` timestamp has passed — the link really is stale.
    Expired,
    /// The URL is still valid (not past `expire`) but the server refused it,
    /// e.g. HTTP 403 from a CDN. Typically the network side: an ISP-hosted
    /// cache node or an exit IP YouTube has blocked. Re-pasting the link
    /// cannot help.
    CdnRejected,
    /// Anything else: 5xx, DNS, TLS, malformed response…
    Other,
}

/// A failed HTTP request to a stream URL, carrying the fields that decide
/// whether the link was genuinely expired.
#[derive(Debug, Clone)]
pub struct HttpError {
    /// HTTP status code, when the server answered.
    pub status: Option<u16>,
    /// The URL that failed.
    pub url: String,
    /// Human-readable detail (kept in the error message shown to the user).
    pub message: String,
    /// The URL's `expire` query parameter, when present (YouTube signed URLs).
    pub expire: Option<i64>,
    /// The URL's `mt` query parameter — when the URL was issued.
    pub issued_at: Option<i64>,
    /// The URL's `ip` query parameter — the client IP the URL was signed for.
    pub ip: Option<String>,
    pub kind: HttpErrorKind,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl HttpError {
    /// Build from a non-2xx status, decoding `expire` / `mt` / `ip` out of the
    /// URL's query string so the failure can be classified.
    pub fn from_status(status: u16, url: &str) -> Self {
        let (expire, issued_at, ip) = signed_url_params(url);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let kind = if expire.is_some_and(|e| now > e) {
            HttpErrorKind::Expired
        } else if status == 403 {
            HttpErrorKind::CdnRejected
        } else {
            HttpErrorKind::Other
        };

        Self {
            status: Some(status),
            url: url.to_string(),
            message: format!("GET {url} failed: HTTP {status}"),
            expire,
            issued_at,
            ip,
            kind,
        }
    }
}

/// Pull `expire`, `mt`, and `ip` out of a URL's query string.
///
/// YouTube's signed googlevideo URLs carry them: `expire` is the hard
/// deadline, `mt` the issue time, `ip` the client IP the signature binds.
fn signed_url_params(url: &str) -> (Option<i64>, Option<i64>, Option<String>) {
    let query = match url::Url::parse(url) {
        Ok(parsed) => parsed.query().unwrap_or("").to_string(),
        Err(_) => return (None, None, None),
    };
    let mut expire = None;
    let mut issued_at = None;
    let mut ip = None;
    for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
        match k.as_ref() {
            "expire" => expire = v.parse::<i64>().ok(),
            "mt" => issued_at = v.parse::<i64>().ok(),
            "ip" => ip = Some(v.into_owned()),
            _ => {}
        }
    }
    (expire, issued_at, ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pure parse fixture for `signed_url_params` / display tests (no clock
    // dependency — only the classification test above needs live timestamps).
    const SIGNED: &str = "https://rr2---sn-55goxu-hxas.googlevideo.com/videoplayback?mt=1787020361&expire=1787042504&ip=138.25.4.51&itag=140&c=ANDROID_VR";

    #[test]
    fn http_error_403_on_valid_url_is_cdn_rejected() {
        // The #120 case: URL signed minutes ago, expire hours away, yet 403.
        // Timestamps are computed so the test never goes stale (the original
        // hardcoded 2026-08-18 URL expired and flipped this to `Expired`).
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let url = format!(
            "https://rr2---sn-55goxu-hxas.googlevideo.com/videoplayback?mt={}&expire={}&ip=138.25.4.51&itag=140&c=ANDROID_VR",
            now - 120,
            now + 3600
        );
        let error = HttpError::from_status(403, &url);
        assert_eq!(error.kind, HttpErrorKind::CdnRejected);
        assert_eq!(error.expire, Some(now + 3600));
        assert_eq!(error.issued_at, Some(now - 120));
        assert_eq!(error.ip.as_deref(), Some("138.25.4.51"));
        assert_eq!(error.status, Some(403));
        assert!(error.message.contains("HTTP 403"));
    }

    #[test]
    fn http_error_403_on_expired_url_is_expired() {
        // Same URL shape, but `expire` is long past: the link really is stale.
        let url = "https://rr.example/videoplayback?expire=1&mt=0&ip=1.2.3.4";
        let error = HttpError::from_status(403, url);
        assert_eq!(error.kind, HttpErrorKind::Expired);
    }

    #[test]
    fn http_error_5xx_without_expire_is_other() {
        let error = HttpError::from_status(503, "https://cdn.example/audio.m4a");
        assert_eq!(error.kind, HttpErrorKind::Other);
        assert_eq!(error.expire, None);
        assert_eq!(error.ip, None);
    }

    #[test]
    fn http_error_403_without_expire_info_is_cdn_rejected() {
        // No `expire` to prove staleness → treat a 403 as a CDN rejection.
        let error = HttpError::from_status(403, "not a url");
        assert_eq!(error.kind, HttpErrorKind::CdnRejected);
        assert_eq!(error.status, Some(403));
    }

    #[test]
    fn signed_url_params_extracts_expire_mt_ip() {
        let (expire, issued_at, ip) = signed_url_params(SIGNED);
        assert_eq!(expire, Some(1787042504));
        assert_eq!(issued_at, Some(1787020361));
        assert_eq!(ip.as_deref(), Some("138.25.4.51"));
    }

    #[test]
    fn http_error_display_reads_like_the_old_network_error() {
        let error = HttpError::from_status(403, SIGNED);
        let text = error.to_string();
        assert!(text.contains("failed: HTTP 403"), "{text}");
    }
}

/// Track information used across all modules
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrackInfo {
    pub id: Option<i64>,
    pub file_path: Option<String>,
    pub source_type: SourceType,
    pub source_url: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub duration: f64,
    pub format: Option<String>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub file_size: Option<u64>,
    pub date_added: Option<String>,
    pub last_played: Option<String>,
    pub play_count: u32,
    pub artwork_path: Option<String>,
    pub is_available: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SourceType {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "youtube")]
    YouTube,
    #[serde(rename = "bilibili")]
    Bilibili,
    #[serde(rename = "direct_url")]
    DirectUrl,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Local => write!(f, "local"),
            SourceType::YouTube => write!(f, "youtube"),
            SourceType::Bilibili => write!(f, "bilibili"),
            SourceType::DirectUrl => write!(f, "direct_url"),
        }
    }
}

impl TryFrom<&str> for SourceType {
    type Error = RhythmError;

    fn try_from(s: &str) -> RhythmResult<Self> {
        match s {
            "local" => Ok(SourceType::Local),
            "youtube" => Ok(SourceType::YouTube),
            "bilibili" => Ok(SourceType::Bilibili),
            "direct_url" => Ok(SourceType::DirectUrl),
            _ => Err(RhythmError::InvalidInput(format!("Unknown source type: {s}"))),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Playlist {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub date_created: Option<String>,
    pub date_modified: Option<String>,
    pub tracks: Vec<TrackInfo>,
}

/// Player state for FFI callbacks
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
    Buffering,
    /// Track ended naturally (not manually stopped).
    Finished,
    Error(String),
}

/// Callback types for FFI layer
pub type StateCallback = Box<dyn Fn(PlayerState) + Send + Sync>;
pub type ProgressCallback = Box<dyn Fn(f64, f64) + Send + Sync>; // (position_sec, duration_sec)
