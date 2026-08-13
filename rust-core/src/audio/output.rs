use crate::{RhythmError, RhythmResult};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

use super::Sink;

/// Audio output abstraction over cpal.
/// Connects to the platform's native audio API (CoreAudio on macOS, WASAPI on
/// Windows). PCM (interleaved f32) is pushed through a bounded channel that
/// the audio callback drains in real time; the bound provides backpressure so
/// decoding never runs ahead of playback.
pub struct AudioOutput {
    // `tx` is declared *before* `_stream` so the Drop glue runs sender-drop
    // first: the callback sees the disconnect and can drain any remaining
    // blocks before the stream itself is torn down.
    tx: Option<SyncSender<Vec<f32>>>,
    // Keep the stream alive for the lifetime of the output.
    _stream: cpal::Stream,
    sample_rate: u32,
    channels: u16,
}

/// How many decoded buffers can be queued ahead of the audio callback.
const QUEUE_DEPTH: usize = 16;
/// How long the callback waits for a buffer before playing silence.
const CALLBACK_TIMEOUT: Duration = Duration::from_millis(10);

/// Feeds the device callback from the decode thread's channel.
///
/// Decoded blocks and device callback buffers never line up: symphonia hands
/// back one packet at a time (1024 frames per AAC packet, and more after
/// resampling), while the device asks for whatever its buffer size happens to
/// be. Whatever a callback doesn't consume has to survive until the next one.
///
/// The previous implementation took one block per callback and filled the
/// device buffer from its head, so every sample past `data.len()` was dropped
/// on the floor — most of the audio, for a typical block/callback size
/// mismatch — while the position kept advancing off packet timestamps (#23).
struct PcmPump {
    rx: Receiver<Vec<f32>>,
    /// The block currently being handed out.
    block: Vec<f32>,
    /// How much of `block` has already been written out.
    cursor: usize,
}

impl PcmPump {
    fn new(rx: Receiver<Vec<f32>>) -> Self {
        PcmPump {
            rx,
            block: Vec::new(),
            cursor: 0,
        }
    }

    /// Fill `out` with as much audio as is available, pulling further blocks
    /// from the channel when the current one runs out. Any tail that couldn't
    /// be filled is zeroed, so the caller always gets a fully initialized
    /// buffer. Returns the number of real (non-silence) samples written.
    ///
    /// Waits up to `timeout` for the *first* sample only. Once something has
    /// been written, a starved channel ends the call rather than stalling the
    /// callback — a partially filled buffer now beats a late one.
    fn fill(&mut self, out: &mut [f32], timeout: Duration) -> usize {
        let mut written = 0;

        while written < out.len() {
            if self.cursor < self.block.len() {
                let take = (out.len() - written).min(self.block.len() - self.cursor);
                out[written..written + take]
                    .copy_from_slice(&self.block[self.cursor..self.cursor + take]);
                self.cursor += take;
                written += take;
                continue;
            }

            // Current block drained — try to pull the next one.
            let next = if written == 0 {
                self.rx.recv_timeout(timeout)
            } else {
                self.rx.try_recv().map_err(|e| match e {
                    mpsc::TryRecvError::Empty => RecvTimeoutError::Timeout,
                    mpsc::TryRecvError::Disconnected => RecvTimeoutError::Disconnected,
                })
            };
            match next {
                Ok(block) => {
                    self.block = block;
                    self.cursor = 0;
                }
                Err(_) => break,
            }
        }

        for slot in out[written..].iter_mut() {
            *slot = 0.0;
        }
        written
    }
}

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
            tx: Some(tx),
            _stream: stream,
            sample_rate,
            channels,
        })
    }

    /// Write interleaved f32 PCM to the audio output. Blocks when the output
    /// queue is full (backpressure against the decode loop).
    pub fn write(&mut self, data: &[f32]) -> RhythmResult<()> {
        if let Some(ref tx) = self.tx {
            tx.send(data.to_vec()).map_err(|e| {
                RhythmError::Output(format!("Audio output channel closed: {e}"))
            })?;
        }
        Ok(())
    }

    /// Drain the output queue before the stream is torn down.
    ///
    /// After the decode loop ends naturally, there may be up to
    /// `QUEUE_DEPTH` blocks still buffered in the channel plus whatever the
    /// `PcmPump` callback hasn't consumed yet. Dropping the sender signals
    /// end-of-stream to the callback; waiting here lets those blocks play
    /// out before the caller drops `_stream` and stops the device.
    pub fn drain(&mut self) {
        // Drop the sender — the callback will finish consuming whatever is
        // still queued, then output silence until the stream is dropped.
        self.tx.take();
        // Each block is at most a decoded packet (~1024 frames); the channel
        // holds at most QUEUE_DEPTH blocks. Add 50 ms safety margin.
        let max_ms = QUEUE_DEPTH as u64 * 1024 * 1000 / self.sample_rate as u64 + 50;
        std::thread::sleep(std::time::Duration::from_millis(max_ms));
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

/// The playback loop's view of an audio sink (Wave 1 seam, see `audio::Sink`).
impl Sink for AudioOutput {
    fn sample_rate(&self) -> u32 {
        AudioOutput::sample_rate(self)
    }

    fn channels(&self) -> u16 {
        AudioOutput::channels(self)
    }

    fn write(&mut self, data: &[f32]) -> RhythmResult<()> {
        AudioOutput::write(self, data)
    }

    fn drain(&mut self) {
        AudioOutput::drain(self)
    }
}

/// Build the device stream for a concrete sample type. The callback pulls
/// decoded audio through a `PcmPump`, which carries whatever the previous
/// callback didn't consume, then converts f32 → the device's sample type.
fn build_stream<F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<Vec<f32>>,
) -> RhythmResult<cpal::Stream>
where
    F: cpal::SizedSample + cpal::FromSample<f32>,
{
    let err_fn = |err| log::error!("Audio stream error: {err}");

    let mut pump = PcmPump::new(rx);
    // Reused across callbacks: allocating inside a realtime callback is a
    // recipe for dropouts.
    let mut scratch: Vec<f32> = Vec::new();

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [F], _: &cpal::OutputCallbackInfo| {
                if scratch.len() < data.len() {
                    scratch.resize(data.len(), 0.0);
                }
                let staging = &mut scratch[..data.len()];
                pump.fill(staging, CALLBACK_TIMEOUT);
                for (slot, sample) in data.iter_mut().zip(staging.iter()) {
                    *slot = F::from_sample(*sample);
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| RhythmError::Output(format!("Failed to build output stream: {e}")))?;

    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: Duration = Duration::from_millis(50);

    /// Drain `pump` through callback buffers of `chunk` samples until it stops
    /// producing audio, returning everything it handed out.
    fn drain(pump: &mut PcmPump, chunk: usize) -> Vec<f32> {
        let mut got = Vec::new();
        let mut buf = vec![0.0f32; chunk];
        loop {
            let n = pump.fill(&mut buf, TIMEOUT);
            if n == 0 {
                return got;
            }
            got.extend_from_slice(&buf[..n]);
        }
    }

    /// The regression this whole rewrite is about: a decoded block larger than
    /// the device buffer must come out whole, across as many callbacks as it
    /// takes.
    #[test]
    fn block_larger_than_callback_survives_intact() {
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(QUEUE_DEPTH);
        let block: Vec<f32> = (0..2230).map(|i| i as f32).collect();
        tx.send(block.clone()).unwrap();
        drop(tx);

        // 2230 samples through 512-sample callbacks: the old code kept 512 and
        // discarded 1718.
        let got = drain(&mut PcmPump::new(rx), 512);
        assert_eq!(got, block);
    }

    /// The other direction: several small blocks must pack into one callback
    /// rather than each wasting a whole buffer.
    #[test]
    fn blocks_smaller_than_callback_are_packed() {
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(QUEUE_DEPTH);
        let mut expected = Vec::new();
        for b in 0..4 {
            let block: Vec<f32> = (0..100).map(|i| (b * 100 + i) as f32).collect();
            expected.extend_from_slice(&block);
            tx.send(block).unwrap();
        }
        drop(tx);

        let got = drain(&mut PcmPump::new(rx), 512);
        assert_eq!(got, expected);
    }

    /// Sizes that share no common factor — the case where an off-by-one in the
    /// cursor arithmetic would show up.
    #[test]
    fn ragged_sizes_lose_nothing() {
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(QUEUE_DEPTH);
        let mut expected = Vec::new();
        for b in 0..8 {
            let block: Vec<f32> = (0..377).map(|i| (b * 377 + i) as f32).collect();
            expected.extend_from_slice(&block);
            tx.send(block).unwrap();
        }
        drop(tx);

        let got = drain(&mut PcmPump::new(rx), 149);
        assert_eq!(got, expected);
    }

    /// A starved channel must yield silence, not stale audio or garbage.
    #[test]
    fn starved_channel_fills_silence() {
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(QUEUE_DEPTH);
        tx.send(vec![1.0; 3]).unwrap();
        let mut pump = PcmPump::new(rx);

        let mut buf = vec![9.9f32; 8];
        let n = pump.fill(&mut buf, TIMEOUT);

        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[1.0, 1.0, 1.0]);
        assert_eq!(&buf[3..], &[0.0; 5], "tail must be zeroed, not left dirty");
    }

    /// Playback ending mid-block must still hand out the remainder.
    #[test]
    fn disconnect_drains_remaining_block() {
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(QUEUE_DEPTH);
        tx.send((0..1000).map(|i| i as f32).collect()).unwrap();
        drop(tx);

        let got = drain(&mut PcmPump::new(rx), 256);
        assert_eq!(got.len(), 1000);
        assert_eq!(got[999], 999.0);
    }
}
