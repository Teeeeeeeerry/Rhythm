pub mod audio;
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
#[derive(Debug, Clone, PartialEq)]
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
