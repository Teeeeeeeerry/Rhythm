use crate::RhythmResult;
use cpal::traits::{DeviceTrait, HostTrait};
use std::sync::mpsc;

/// Audio output abstraction over cpal.
/// Connects to the platform's native audio API (CoreAudio on macOS, WASAPI on Windows).
pub struct AudioOutput {
    // Using mpsc to send PCM data to the audio callback
    tx: mpsc::Sender<Vec<f32>>,
}

impl AudioOutput {
    /// Create a new audio output stream.
    pub fn new() -> RhythmResult<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| crate::RhythmError::Output("No output device found".to_string()))?;

        let config = device.default_output_config().map_err(|e| {
            crate::RhythmError::Output(format!("Failed to get default output config: {e}"))
        })?;

        let (tx, _rx) = mpsc::channel::<Vec<f32>>();

        // TODO: Build the actual output stream with the cpal callback
        // The callback will pull from rx and write to the device buffer

        let _sample_rate = config.sample_rate();

        Ok(AudioOutput { tx })
    }

    /// Write PCM data to the audio output.
    pub fn write(&mut self, data: &[f32]) -> RhythmResult<()> {
        self.tx
            .send(data.to_vec())
            .map_err(|e| crate::RhythmError::Output(format!("Audio output channel closed: {e}")))?;
        Ok(())
    }
}
