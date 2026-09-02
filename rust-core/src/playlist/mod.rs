//! Playlist manager — import/export M3U8 and standalone playlist CRUD.
//!
//! Most playlist CRUD is handled by `Library` since playlists are
//! tightly coupled to the track database. This module provides
//! import/export functionality and standalone playlist operations.

use crate::library::Library;
use crate::{RhythmResult, SourceType, TrackInfo};
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

/// Named outcome of importing a playlist into the library (#233): how many
/// entries were persisted and how many could not be. Counts are the whole
/// contract — callers never re-derive "was this entry stored" themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct M3u8ImportOutcome {
    pub imported: i32,
    pub failed: i32,
}

/// Placeholder title for an entry whose `#EXTINF` carried no title (#233).
const UNTITLED: &str = "Unknown";

/// Parse an M3U8 playlist file and persist every entry into `library` (#217).
///
/// Parsing and storing are one entry point so the storage rules live in a
/// single place: an empty location counts as a failure, an http(s) location
/// becomes a `direct_url` track and anything else a local file path, a
/// missing title falls back to a placeholder. Whether a write succeeded is
/// decided here, never across the seam — the two UI layers used to answer it
/// differently (#136 on macOS, #173 on Windows: one defect, fixed twice).
pub fn import_m3u8_into_library(path: &Path, library: &Library) -> RhythmResult<M3u8ImportOutcome> {
    let entries = import_m3u8(path)?;
    Ok(import_entries_into_library(&entries, library))
}

/// The storage half of [`import_m3u8_into_library`], for callers that already
/// hold parsed entries. Not exported across the FFI seam — the UI layers only
/// ever see the file-level entry point.
pub fn import_entries_into_library(
    entries: &[M3u8Entry],
    library: &Library,
) -> M3u8ImportOutcome {
    let mut outcome = M3u8ImportOutcome {
        imported: 0,
        failed: 0,
    };

    for entry in entries {
        let Some(track) = entry_to_track(entry) else {
            outcome.failed += 1;
            continue;
        };
        match library.add_track(&track) {
            Ok(_) => outcome.imported += 1,
            Err(e) => {
                log::warn!("M3U8 import failed for {}: {e}", entry.location);
                outcome.failed += 1;
            }
        }
    }

    outcome
}

/// Map one parsed entry onto a track, or `None` when it has no location to
/// play from. An http(s) location is a `direct_url` track; anything else is a
/// local file path.
fn entry_to_track(entry: &M3u8Entry) -> Option<TrackInfo> {
    if entry.location.is_empty() {
        return None;
    }
    let is_url =
        entry.location.starts_with("http://") || entry.location.starts_with("https://");
    let title = if entry.title.is_empty() {
        UNTITLED.to_string()
    } else {
        entry.title.clone()
    };

    Some(TrackInfo {
        id: None,
        file_path: (!is_url).then(|| entry.location.clone()),
        source_type: if is_url {
            SourceType::DirectUrl
        } else {
            SourceType::Local
        },
        source_url: is_url.then(|| entry.location.clone()),
        title,
        artist: entry.artist.clone(),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        genre: None,
        year: None,
        duration: 0.0,
        format: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_size: None,
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: None,
        is_available: true,
    })
}
