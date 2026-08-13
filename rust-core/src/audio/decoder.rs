use crate::{RhythmError, RhythmResult};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use symphonia::core::audio::AudioBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Local file wrapper implementing `MediaSource` (Read + Seek + length info).
struct FileSource {
    file: File,
    len: Option<u64>,
}

impl Read for FileSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for FileSource {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl MediaSource for FileSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.len
    }
}

/// Audio decoder using symphonia.
/// Handles demuxing and decoding of local files and network streams.
/// `next_packet` returns interleaved f32 PCM at the stream's native sample rate.
pub struct AudioDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    duration: f64,
    position: f64,
    time_base: Option<symphonia::core::units::TimeBase>,
}

impl AudioDecoder {
    /// Open a local audio file for decoding.
    pub fn open_file(path: &Path) -> RhythmResult<Self> {
        if !path.exists() {
            return Err(RhythmError::FileNotFound(path.display().to_string()));
        }
        let file = File::open(path).map_err(|e| {
            RhythmError::Decode(format!("Failed to open {}: {e}", path.display()))
        })?;
        let len = file.metadata().ok().map(|m| m.len());
        Self::open_source(Box::new(FileSource { file, len }), None)
    }

    /// Open any seekable media source (file, HTTP stream, ...) for decoding.
    ///
    /// `hint_ext` optionally gives the file extension (e.g. "mp3", "m4a") so the
    /// format probe can skip incompatible readers. `None` lets symphonia probe
    /// all registered formats.
    pub fn open_source(
        source: Box<dyn MediaSource>,
        hint_ext: Option<&str>,
    ) -> RhythmResult<Self> {
        let mss = MediaSourceStream::new(source, Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = hint_ext {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| RhythmError::Decode(format!("Failed to probe stream: {e}")))?;

        let format = probed.format;

        // Capture track info and build the decoder before moving `format` around.
        let (track_id, sample_rate, channels, duration, time_base, decoder) = {
            let track = format
                .default_track()
                .ok_or_else(|| RhythmError::Decode("No default audio track found".to_string()))?;
            let codec_params = &track.codec_params;

            let decoder = symphonia::default::get_codecs()
                .make(codec_params, &DecoderOptions::default())
                .map_err(|e| RhythmError::Decode(format!("Unsupported codec: {e}")))?;

            let duration = codec_params
                .n_frames
                .zip(codec_params.time_base)
                .map(|(n_frames, tb)| {
                    n_frames as f64 * f64::from(tb.numer) / f64::from(tb.denom)
                })
                .unwrap_or(0.0);

            let sample_rate = codec_params.sample_rate.unwrap_or(44100);
            let channels = codec_params
                .channels
                .map(|c| c.count() as u16)
                .unwrap_or(2);

            (
                track.id,
                sample_rate,
                channels,
                duration,
                codec_params.time_base,
                decoder,
            )
        };

        Ok(AudioDecoder {
            format,
            decoder,
            track_id,
            sample_rate,
            channels,
            duration,
            position: 0.0,
            time_base,
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

    /// Read the next decoded PCM packet as interleaved f32.
    /// Returns `None` when the stream ends.
    pub fn next_packet(&mut self) -> RhythmResult<Option<Vec<f32>>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(RhythmError::Decode(format!("Demux error: {e}"))),
            };

            // Skip packets from other tracks.
            if packet.track_id() != self.track_id {
                continue;
            }

            // Track position from the packet timestamp.
            if let Some(tb) = self.time_base {
                self.position = packet.ts as f64 * f64::from(tb.numer) / f64::from(tb.denom);
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    return Ok(Some(convert_to_interleaved(decoded)?));
                }
                // Skip a single undecodable packet and continue.
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                // Stream parameters changed (e.g. after seek); reset the decoder.
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => return Err(RhythmError::Decode(format!("Decode error: {e}"))),
            }
        }
    }

    /// Seek to a position in seconds.
    pub fn seek(&mut self, seconds: f64) -> RhythmResult<()> {
        let seek_to = SeekTo::Time {
            time: symphonia::core::units::Time::from(seconds),
            track_id: None,
        };
        let seeked = self
            .format
            .seek(SeekMode::Accurate, seek_to)
            .map_err(|e| RhythmError::Decode(format!("Seek failed: {e}")))?;
        self.decoder.reset();

        if let Some(tb) = self.time_base {
            self.position =
                seeked.required_ts as f64 * f64::from(tb.numer) / f64::from(tb.denom);
        } else {
            self.position = seconds;
        }
        Ok(())
    }
}

/// The playback loop's view of a decoder (Wave 1 seam, see `audio::Decoder`).
impl super::Decoder for AudioDecoder {
    fn next_packet(&mut self) -> RhythmResult<Option<Vec<f32>>> {
        AudioDecoder::next_packet(self)
    }

    fn seek(&mut self, seconds: f64) -> RhythmResult<()> {
        AudioDecoder::seek(self, seconds)
    }

    fn position(&self) -> f64 {
        AudioDecoder::position(self)
    }

    fn duration(&self) -> f64 {
        AudioDecoder::duration(self)
    }

    fn sample_rate(&self) -> u32 {
        AudioDecoder::sample_rate(self)
    }

    fn channels(&self) -> u16 {
        AudioDecoder::channels(self)
    }
}

/// Convert a decoded `AudioBufferRef` into interleaved f32 PCM.
fn convert_to_interleaved(decoded: symphonia::core::audio::AudioBufferRef<'_>) -> RhythmResult<Vec<f32>> {
    let mut buf = AudioBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
    decoded.convert(&mut buf);

    let planes = buf.planes();
    let frames = planes
        .planes()
        .first()
        .map(|p| p.len())
        .unwrap_or(0);
    let channels = buf.spec().channels.count();
    if channels == 0 || frames == 0 {
        return Ok(Vec::new());
    }

    let mut out = vec![0.0f32; frames * channels];
    for (ch, plane) in planes.planes().iter().enumerate().take(channels) {
        for (i, sample) in plane.iter().take(frames).enumerate() {
            out[i * channels + ch] = *sample;
        }
    }
    Ok(out)
}
