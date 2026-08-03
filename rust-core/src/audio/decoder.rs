use crate::{RhythmError, RhythmResult};
use std::path::Path;

/// Audio decoder using symphonia.
/// Handles demuxing and decoding of local audio files.
pub struct AudioDecoder {
    // Placeholder — will be fleshed out with symphonia types
    file_path: String,
    duration: f64,
    position: f64,
    sample_rate: u32,
    channels: u16,
}

impl AudioDecoder {
    /// Open a local audio file for decoding.
    pub fn open_file(path: &Path) -> RhythmResult<Self> {
        if !path.exists() {
            return Err(RhythmError::FileNotFound(
                path.display().to_string(),
            ));
        }

        // TODO: Full symphonia integration
        Ok(AudioDecoder {
            file_path: path.display().to_string(),
            duration: 0.0,
            position: 0.0,
            sample_rate: 44100,
            channels: 2,
        })
    }

    /// Get the total duration in seconds.
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// Get the current playback position in seconds.
    pub fn position(&self) -> f64 {
        self.position
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of channels.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Read the next decoded PCM packet.
    /// Returns `None` when the stream ends.
    pub fn next_packet(&mut self) -> RhythmResult<Option<Vec<f32>>> {
        // TODO: Full symphonia integration
        Ok(None)
    }
}
