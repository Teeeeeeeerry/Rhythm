use crate::{RhythmError, RhythmResult, TrackInfo, SourceType};
use lofty::picture::MimeType;
use std::path::Path;

mod formats;
mod scanner;

pub use scanner::*;

/// Supported audio file extensions
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "aac", "flac", "wav", "ogg", "oga", "opus", "alac", "ape", "wma",
    "mp4", "m4b", "m4p", "m4r", "aiff", "aif", "aifc", "wv",  // WavPack
];

/// Core metadata for a track, extracted from the audio file.
#[derive(Debug, Clone, Default)]
struct RawMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    genre: Option<String>,
    year: Option<u32>,
    duration: Option<f64>,
    format: Option<String>,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
}

/// Extract full track information from a local audio file.
pub fn extract_track_info(path: &Path) -> RhythmResult<TrackInfo> {
    if !path.exists() {
        return Err(RhythmError::FileNotFound(
            path.display().to_string(),
        ));
    }

    let raw = extract_raw_metadata(path)?;
    let file_size = std::fs::metadata(path).ok().map(|m| m.len());

    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    let title = raw.title.unwrap_or_else(|| filename.to_string());
    let duration = raw.duration.unwrap_or(0.0);
    let format = raw.format.or_else(|| {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
    });

    Ok(TrackInfo {
        id: None,
        file_path: Some(path.to_string_lossy().to_string()),
        source_type: SourceType::Local,
        source_url: None,
        title,
        artist: raw.artist,
        album: raw.album,
        album_artist: raw.album_artist,
        track_number: raw.track_number,
        disc_number: raw.disc_number,
        genre: raw.genre,
        year: raw.year,
        duration,
        format,
        bitrate: raw.bitrate,
        sample_rate: raw.sample_rate,
        channels: raw.channels,
        file_size,
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: None,
        is_available: true,
    })
}

/// Determine if a file extension is a supported audio format.
pub fn is_supported_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Determine if a file is an MP4/M4A container (video or audio).
/// For MP4 files with video, we only want the audio track.
pub fn is_mp4_container(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase()),
        Some(ref ext) if ext == "mp4" || ext == "m4a" || ext == "m4b" || ext == "m4v"
    )
}

/// Maximum artwork file size in bytes (1 MB) — skip larger embedded images.
const MAX_ARTWORK_SIZE: usize = 1_048_576;

/// Extract cover art from an audio file, saving to a cache directory.
/// Returns the path to the extracted artwork file, or None if no artwork found.
/// Skips artwork larger than `MAX_ARTWORK_SIZE` to avoid memory bloat.
pub fn extract_artwork(path: &Path, cache_dir: &Path) -> RhythmResult<Option<String>> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let tagged_file = Probe::open(path)
        .map_err(|e| RhythmError::Metadata(format!("Failed to open file: {e}")))?
        .read()
        .map_err(|e| RhythmError::Metadata(format!("Failed to read tags: {e}")))?;

    // Try to find embedded picture
    if let Some(tag) = tagged_file.primary_tag() {
        if let Some(picture) = tag.pictures().first() {
            let data = picture.data();
            if data.len() > MAX_ARTWORK_SIZE {
                log::warn!(
                    "Skipping oversized artwork ({} bytes) in {}",
                    data.len(),
                    path.display()
                );
                return Ok(None);
            }

            // Match lofty's MimeType variants directly (its Debug output is
            // capitalized, so string matching must be case-insensitive — rhythm#94).
            let ext = match picture.mime_type() {
                Some(MimeType::Png) => "png",
                Some(MimeType::Jpeg) => "jpg",
                Some(MimeType::Unknown(ref m)) if m.to_lowercase().contains("png") => "png",
                _ => "jpg",
            };

            let hash = blake3::hash(data);
            let filename = format!("{hash}.{ext}");
            let artwork_path = cache_dir.join(&filename);

            if !artwork_path.exists() {
                std::fs::create_dir_all(cache_dir)?;
                std::fs::write(&artwork_path, data)?;
            }

            return Ok(Some(artwork_path.to_string_lossy().to_string()));
        }
    }

    Ok(None)
}

/// Extract raw metadata from a local file using lofty (primary) or symphonia (fallback).
fn extract_raw_metadata(path: &Path) -> RhythmResult<RawMetadata> {
    // First attempt: use lofty for tag reading (faster, broader format support)
    if let Ok(raw) = extract_with_lofty(path) {
        if raw.title.is_some() || raw.duration.is_some() {
            return Ok(raw);
        }
    }

    // Fallback: use symphonia to probe the container
    extract_with_symphonia(path)
}

/// Map lofty's detected container type to the audio format name.
/// `FileType::Mpeg` covers MP3 (and MP2/MP1) — report the common name.
fn format_from_file_type(file_type: lofty::file::FileType) -> String {
    use lofty::file::FileType;
    match file_type {
        FileType::Mpeg => "mp3".to_string(),
        other => format!("{:?}", other).to_lowercase(),
    }
}

fn extract_with_lofty(path: &Path) -> RhythmResult<RawMetadata> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    // #96: format must be the container/encoding, not the tag type — probe
    // the file type before reading tags.
    let probe = Probe::open(path)
        .map_err(|e| RhythmError::Metadata(format!("Failed to open: {e}")))?;
    let format = probe
        .file_type()
        .map(format_from_file_type);
    let tagged_file = probe
        .read()
        .map_err(|e| RhythmError::Metadata(format!("Failed to read: {e}")))?;

    let properties = tagged_file.properties();
    let duration = properties.duration().as_secs_f64();
    let bitrate = properties.audio_bitrate().map(|b| b as u32);
    let sample_rate = properties.sample_rate().map(|s| s as u32);
    let channels = properties.channels().map(|c| c as u16);

    let tag = tagged_file.primary_tag();

    let raw = RawMetadata {
        title: tag.and_then(|t| t.title().map(|s| s.into_owned())),
        artist: tag.and_then(|t| t.artist().map(|s| s.into_owned())),
        album: tag.and_then(|t| t.album().map(|s| s.into_owned())),
        album_artist: None, // Use None for now; can be enhanced with ItemKey access
        track_number: tag.and_then(|t| t.track()),
        disc_number: tag.and_then(|t| t.disk()),
        genre: tag.and_then(|t| t.genre().map(|s| s.into_owned())),
        year: tag.and_then(|t| t.year()),
        duration: Some(duration),
        format,
        bitrate,
        sample_rate,
        channels,
    };

    Ok(raw)
}

fn extract_with_symphonia(path: &Path) -> RhythmResult<RawMetadata> {
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &Default::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| RhythmError::Decode(format!("Failed to probe format: {e}")))?;

    let format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| RhythmError::Decode("No default track found".to_string()))?;

    let codec_params = &track.codec_params;

    let duration = codec_params
        .time_base
        .and_then(|tb| {
            codec_params.n_frames.map(|frames| {
                frames as f64 * (tb.numer as f64 / tb.denom as f64)
            })
        });

    Ok(RawMetadata {
        title: None,
        artist: None,
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        genre: None,
        year: None,
        duration,
        format: Some(format!("{:?}", codec_params.codec).to_lowercase()),
        bitrate: codec_params.bits_per_sample.map(|b| b as u32),
        sample_rate: codec_params.sample_rate,
        channels: codec_params.channels.map(|c| c.count() as u16),
    })
}
