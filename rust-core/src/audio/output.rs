use crate::{RhythmError, RhythmResult};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Audio output abstraction over cpal.
/// Connects to the platform's native audio API (CoreAudio on macOS, WASAPI on
/// Windows). PCM (interleaved f32) is pushed through a bounded channel that
/// the audio callback drains in real time; the bound provides backpressure so
/// decoding never runs ahead of playback.
pub struct AudioOutput {
    // Keep the stream alive for the lifetime of the output.
    _stream: cpal::Stream,
    tx: SyncSender<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
}

/// How many decoded buffers can be queued ahead of the audio callback.
const QUEUE_DEPTH: usize = 16;
/// How long the callback waits for a buffer before playing silence.
const CALLBACK_TIMEOUT: Duration = Duration::from_millis(10);

impl AudioOutput {
    /// Create a new audio output stream on the default output device.
    /// Returns the device's native sample rate and channel count; decoded PCM
    /// must be resampled to match before calling `write`.
    pub fn new() -> RhythmResult<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| RhythmError::Output("No output device found".to_string()))?;

        let config = device.default_output_config().map_err(|e| {
            RhythmError::Output(format!("Failed to get default output config: {e}"))
        })?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(QUEUE_DEPTH);
        let rx = Arc::new(Mutex::new(rx));

        let stream_config: cpal::StreamConfig = config.clone().into();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => build_stream::<f32>(&device, &stream_config, rx)?,
            cpal::SampleFormat::I16 => build_stream::<i16>(&device, &stream_config, rx)?,
            cpal::SampleFormat::U16 => build_stream::<u16>(&device, &stream_config, rx)?,
            other => {
                return Err(RhythmError::Output(format!(
                    "Unsupported sample format: {other:?}"
                )));
            }
        };
        stream.play().map_err(|e| {
            RhythmError::Output(format!("Failed to start audio stream: {e}"))
        })?;

        Ok(AudioOutput {
            _stream: stream,
            tx,
            sample_rate,
            channels,
        })
    }

    /// Write interleaved f32 PCM to the audio output. Blocks when the output
    /// queue is full (backpressure against the decode loop).
    pub fn write(&mut self, data: &[f32]) -> RhythmResult<()> {
        self.tx.send(data.to_vec()).map_err(|e| {
            RhythmError::Output(format!("Audio output channel closed: {e}"))
        })?;
        Ok(())
    }

    /// The device's sample rate; decoded audio must be resampled to this.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The device's channel count; decoded audio must be mapped to this.
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

/// Build the device stream for a concrete sample type. The callback pulls
/// decoded buffers from `rx` and converts f32 → the device's sample type;
/// it plays silence when nothing is queued.
fn build_stream<F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Arc<Mutex<Receiver<Vec<f32>>>>,
) -> RhythmResult<cpal::Stream>
where
    F: cpal::SizedSample + cpal::FromSample<f32>,
{
    let err_fn = |err| log::error!("Audio stream error: {err}");

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [F], _: &cpal::OutputCallbackInfo| {
                let pending = rx.lock().unwrap();
                match pending.recv_timeout(CALLBACK_TIMEOUT) {
                    Ok(pcm) => {
                        for (i, slot) in data.iter_mut().enumerate() {
                            let sample = pcm.get(i).copied().unwrap_or(0.0f32);
                            *slot = F::from_sample(sample);
                        }
                    }
                    Err(_) => {
                        for slot in data.iter_mut() {
                            *slot = F::from_sample(0.0f32);
                        }
                    }
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| RhythmError::Output(format!("Failed to build output stream: {e}")))?;

    Ok(stream)
}
