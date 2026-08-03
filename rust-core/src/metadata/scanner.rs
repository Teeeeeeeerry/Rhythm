use crate::metadata::{extract_track_info, is_supported_audio};
use crate::{RhythmResult, TrackInfo};
use std::path::Path;

/// Scan a directory recursively for supported audio files.
/// Returns a list of TrackInfo for all found audio files.
pub fn scan_directory(dir: &Path) -> RhythmResult<Vec<TrackInfo>> {
    let mut tracks = Vec::new();

    if !dir.is_dir() {
        return Err(crate::RhythmError::InvalidInput(format!(
            "Path is not a directory: {}",
            dir.display()
        )));
    }

    scan_dir_recursive(dir, &mut tracks)?;
    Ok(tracks)
}

fn scan_dir_recursive(dir: &Path, tracks: &mut Vec<TrackInfo>) -> RhythmResult<()> {
    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Skip hidden directories
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
            scan_dir_recursive(&path, tracks)?;
        } else if is_supported_audio(&path) {
            match extract_track_info(&path) {
                Ok(track) => tracks.push(track),
                Err(e) => {
                    log::warn!("Skipping {}: {e}", path.display());
                }
            }
        }
    }

    Ok(())
}
