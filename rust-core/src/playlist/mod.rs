//! Playlist manager — import/export M3U8 and standalone playlist CRUD.
//!
//! Most playlist CRUD is handled by `Library` since playlists are
//! tightly coupled to the track database. This module provides
//! import/export functionality and standalone playlist operations.

use crate::{RhythmResult, TrackInfo};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// One imported M3U8 entry — named fields across the seam (title, artist,
/// location), so callers never index by position (#177).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct M3u8Entry {
    pub title: String,
    pub artist: Option<String>,
    pub location: String,
}

/// Export tracks to an M3U8 playlist file.
pub fn export_m3u8(path: &Path, tracks: &[TrackInfo]) -> RhythmResult<()> {
    let mut file = std::fs::File::create(path)?;

    // M3U8 header
    writeln!(file, "#EXTM3U")?;

    for track in tracks {
        // Write EXTINF line
        writeln!(
            file,
            "#EXTINF:{},{} - {}",
            track.duration as i64,
            track.artist.as_deref().unwrap_or("Unknown Artist"),
            track.title,
        )?;

        // Write the path or URL
        let location = match &track.source_type {
            crate::SourceType::Local => track.file_path.clone().unwrap_or_default(),
            _ => track.source_url.clone().unwrap_or_default(),
        };
        writeln!(file, "{location}")?;
    }

    Ok(())
}

/// Import an M3U8 playlist file.
/// Returns a list of (title, file_path_or_url) pairs.
/// Does NOT add tracks to the database — the caller should do that.
pub fn import_m3u8(path: &Path) -> RhythmResult<Vec<M3u8Entry>> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut entries: Vec<M3u8Entry> = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_artist: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed == "#EXTM3U" {
            continue;
        }

        if let Some(info) = trimmed.strip_prefix("#EXTINF:") {
            // Parse #EXTINF:duration,artist - title
            let info_part = info
                .split(',')
                .nth(1)
                .unwrap_or("")
                .trim();

            if let Some((artist, title)) = info_part.split_once(" - ") {
                current_artist = Some(artist.to_string());
                current_title = Some(title.to_string());
            } else {
                current_title = Some(info_part.to_string());
                current_artist = None;
            }
        } else if !trimmed.starts_with('#') {
            // This is the file path / URL line
            let location = trimmed.to_string();
            let title = current_title.take().unwrap_or_else(|| {
                Path::new(trimmed)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(trimmed)
                    .to_string()
            });
            let artist = current_artist.take();
            entries.push(M3u8Entry {
                title,
                artist,
                location,
            });
        }
    }

    Ok(entries)
}
