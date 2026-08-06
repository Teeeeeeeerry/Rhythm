use crate::resolver::resolve_url;
use crate::{PlayerState, ProgressCallback, RhythmError, RhythmResult, StateCallback};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub mod decoder;
pub mod http_stream;
mod output;
mod resampler;

use decoder::AudioDecoder;
use http_stream::HttpStream;
use output::AudioOutput;
use resampler::Resampler;

/// Audio engine for playback of local files and network streams.
pub struct AudioEngine {
    inner: Arc<Mutex<EngineInner>>,
    state_callback: Arc<Mutex<Option<StateCallback>>>,
    progress_callback: Arc<Mutex<Option<ProgressCallback>>>,
    stop_flag: Arc<AtomicBool>,
}

struct EngineInner {
    current_source: Option<String>,
    state: PlayerState,
    volume: f32,
    duration: f64,
    position: f64,
    /// Pending seek requested by the user; consumed by the playback thread.
    desired_position: Option<f64>,
}

impl AudioEngine {
    pub fn new() -> Self {
        AudioEngine {
            inner: Arc::new(Mutex::new(EngineInner {
                current_source: None,
                state: PlayerState::Stopped,
                volume: 1.0,
                duration: 0.0,
                position: 0.0,
                desired_position: None,
            })),
            state_callback: Arc::new(Mutex::new(None)),
            progress_callback: Arc::new(Mutex::new(None)),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register a callback for state changes (Playing, Paused, Stopped, Error, etc.)
    pub fn on_state_change(&self, callback: StateCallback) {
        *self.state_callback.lock().unwrap() = Some(callback);
    }

    /// Register a callback for playback progress updates.
    pub fn on_progress(&self, callback: ProgressCallback) {
        *self.progress_callback.lock().unwrap() = Some(callback);
    }

    /// Play a local audio file.
    pub fn play_file(&self, path: &Path) -> RhythmResult<()> {
        let path_str = path.to_string_lossy().to_string();

        if !path.exists() {
            return Err(RhythmError::FileNotFound(path_str));
        }

        self.stop_flag.store(false, Ordering::SeqCst);

        let inner = self.inner.clone();
        let state_cb = self.state_callback.clone();
        let progress_cb = self.progress_callback.clone();
        let stop_flag = self.stop_flag.clone();

        thread::spawn(move || {
            if let Err(e) = play_file_impl(
                &path_str,
                inner.clone(),
                state_cb.clone(),
                progress_cb,
                stop_flag,
            ) {
                log::error!("Playback error: {e}");
                fail(&inner, &state_cb, e.to_string());
            }
        });

        Ok(())
    }

    /// Play from a network URL stream.
    pub fn play_url(&self, url: &str) -> RhythmResult<()> {
        self.stop_flag.store(false, Ordering::SeqCst);

        let url = url.to_string();
        let inner = self.inner.clone();
        let state_cb = self.state_callback.clone();
        let progress_cb = self.progress_callback.clone();
        let stop_flag = self.stop_flag.clone();

        thread::spawn(move || {
            if let Err(e) = play_url_impl(
                &url,
                inner.clone(),
                state_cb.clone(),
                progress_cb,
                stop_flag,
            ) {
                log::error!("Playback error: {e}");
                fail(&inner, &state_cb, e.to_string());
            }
        });

        Ok(())
    }

    /// Pause playback.
    pub fn pause(&self) {
        let mut inner = self.inner.lock().unwrap();
        if inner.state == PlayerState::Playing {
            inner.state = PlayerState::Paused;
            self.emit_state(PlayerState::Paused);
        }
    }

    /// Resume playback.
    pub fn resume(&self) {
        let mut inner = self.inner.lock().unwrap();
        if inner.state == PlayerState::Paused {
            inner.state = PlayerState::Playing;
            self.emit_state(PlayerState::Playing);
        }
    }

    /// Stop playback.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        let mut inner = self.inner.lock().unwrap();
        inner.state = PlayerState::Stopped;
        inner.current_source = None;
        inner.desired_position = None;
        self.emit_state(PlayerState::Stopped);
    }

    /// Seek to a position in seconds. The seek is applied by the playback
    /// thread at its next loop iteration.
    pub fn seek(&self, seconds: f64) -> RhythmResult<()> {
        let mut inner = self.inner.lock().unwrap();
        if seconds < 0.0 {
            return Err(RhythmError::InvalidInput(format!(
                "Seek position {seconds}s cannot be negative"
            )));
        }
        if inner.duration > 0.0 && seconds > inner.duration {
            return Err(RhythmError::InvalidInput(format!(
                "Seek position {seconds}s out of range [0, {}]",
                inner.duration
            )));
        }
        inner.desired_position = Some(seconds);
        Ok(())
    }

    /// Set volume (0.0 to 1.0).
    pub fn set_volume(&self, volume: f32) {
        let clamped = volume.clamp(0.0, 1.0);
        let mut inner = self.inner.lock().unwrap();
        inner.volume = clamped;
    }

    /// Get current volume.
    pub fn volume(&self) -> f32 {
        self.inner.lock().unwrap().volume
    }

    /// Get current player state.
    pub fn state(&self) -> PlayerState {
        self.inner.lock().unwrap().state.clone()
    }

    /// Get current playback duration.
    pub fn duration(&self) -> f64 {
        self.inner.lock().unwrap().duration
    }

    /// Get current playback position.
    pub fn position(&self) -> f64 {
        self.inner.lock().unwrap().position
    }

    fn emit_state(&self, state: PlayerState) {
        if let Some(ref cb) = *self.state_callback.lock().unwrap() {
            cb(state);
        }
    }
}

fn play_file_impl(
    path: &str,
    inner: Arc<Mutex<EngineInner>>,
    state_cb: Arc<Mutex<Option<StateCallback>>>,
    progress_cb: Arc<Mutex<Option<ProgressCallback>>>,
    stop_flag: Arc<AtomicBool>,
) -> RhythmResult<()> {
    // Update state
    inner.lock().unwrap().current_source = Some(path.to_string());
    set_state(&inner, &state_cb, PlayerState::Playing);

    let mut decoder = AudioDecoder::open_file(Path::new(path))?;
    set_duration(&inner, decoder.duration());

    let mut output = AudioOutput::new()?;
    let out_rate = output.sample_rate();
    let out_channels = output.channels();
    let mut resampler =
        Resampler::new(decoder.sample_rate(), decoder.channels(), out_rate, out_channels);

    run_playback_loop(
        &mut decoder,
        &mut output,
        &mut resampler,
        inner,
        state_cb,
        progress_cb,
        stop_flag,
    )
}

fn play_url_impl(
    url: &str,
    inner: Arc<Mutex<EngineInner>>,
    state_cb: Arc<Mutex<Option<StateCallback>>>,
    progress_cb: Arc<Mutex<Option<ProgressCallback>>>,
    stop_flag: Arc<AtomicBool>,
) -> RhythmResult<()> {
    // Buffering has to be recorded, not just emitted: the UI polls `state()`
    // rather than registering a callback, so an emit-only Buffering left it
    // showing the *previous* state for the whole resolve + connect + prebuffer
    // window — which is why a stuck stream read as an idle player at 0:00 with
    // no error (#23). Same reasoning as `fail()` below.
    set_state(&inner, &state_cb, PlayerState::Buffering);

    // 1. Resolve the real stream URL: yt-dlp for YouTube/Bilibili, direct for
    //    plain audio URLs.
    let resolved = resolve_url(url)?;

    // 2. Open the HTTP stream — this starts the prefetch downloader. The
    //    resolver's headers come along: Bilibili's CDN answers 403 without
    //    the Referer yt-dlp used.
    let stream = HttpStream::open_with_headers(&resolved.stream_url, &resolved.http_headers)?;

    // 3. Wait for the initial buffer so the probe/decoder doesn't stall on
    //    every read.
    stream.wait_initial_buffered()?;

    // 4. Open the decoder on the stream. Use the URL's file extension as a
    //    probe hint (helps when the stream lacks a standard container header).
    let hint = stream_hint(&resolved.stream_url);
    let mut decoder = AudioDecoder::open_source(Box::new(stream), hint)?;

    // 5. Play.
    inner.lock().unwrap().current_source = Some(url.to_string());
    set_state(&inner, &state_cb, PlayerState::Playing);

    // DASH segments (Bilibili's audio streams) carry no usable duration box,
    // so the decoder reports 0 and the UI would sit at "0:00 / 0:00". The
    // resolver already knows the real length — use it.
    let duration = if decoder.duration() > 0.0 {
        decoder.duration()
    } else {
        resolved.duration
    };
    set_duration(&inner, duration);

    let mut output = AudioOutput::new()?;
    let out_rate = output.sample_rate();
    let out_channels = output.channels();
    let mut resampler =
        Resampler::new(decoder.sample_rate(), decoder.channels(), out_rate, out_channels);

    run_playback_loop(
        &mut decoder,
        &mut output,
        &mut resampler,
        inner,
        state_cb,
        progress_cb,
        stop_flag,
    )
}

/// Shared decode → resample → output loop. Applies pending seeks, pause, and
/// volume, and reports progress via the callback.
fn run_playback_loop(
    decoder: &mut AudioDecoder,
    output: &mut AudioOutput,
    resampler: &mut Resampler,
    inner: Arc<Mutex<EngineInner>>,
    state_cb: Arc<Mutex<Option<StateCallback>>>,
    progress_cb: Arc<Mutex<Option<ProgressCallback>>>,
    stop_flag: Arc<AtomicBool>,
) -> RhythmResult<()> {
    let out_rate = output.sample_rate();
    let out_channels = output.channels();

    while !stop_flag.load(Ordering::SeqCst) {
        // Check pause state
        {
            let eng = inner.lock().unwrap();
            if eng.state == PlayerState::Paused {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        }

        // Apply a pending seek.
        let desired = { inner.lock().unwrap().desired_position.take() };
        if let Some(secs) = desired {
            decoder.seek(secs)?;
            resampler.reset();
            set_position(&inner, decoder.position());
            emit_progress(&progress_cb, decoder.position(), decoder.duration());
        }

        match decoder.next_packet() {
            Ok(Some(pcm)) => {
                // Resample the decoded chunk to the output device's format.
                let in_frames = pcm.len() / decoder.channels().max(1) as usize;
                let out_frames = (in_frames as f64 * f64::from(out_rate)
                    / f64::from(decoder.sample_rate().max(1)))
                .ceil() as usize
                    + 1;
                let mut out_buf = vec![0.0f32; out_frames * out_channels as usize];
                let frames = resampler.process(&pcm, &mut out_buf);

                // Apply volume.
                let volume = inner.lock().unwrap().volume;
                if volume < 1.0 {
                    for s in out_buf[..frames * out_channels as usize].iter_mut() {
                        *s *= volume;
                    }
                }

                output.write(&out_buf[..frames * out_channels as usize])?;

                let pos = decoder.position();
                set_position(&inner, pos);
                emit_progress(&progress_cb, pos, decoder.duration());
            }
            Ok(None) => break, // End of stream
            Err(e) => return Err(e),
        }
    }

    // Natural end vs. manual stop: `stop()` already emitted Stopped.
    if !stop_flag.load(Ordering::SeqCst) {
        emit(&state_cb, PlayerState::Finished);
    }
    Ok(())
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn emit(cb: &Arc<Mutex<Option<StateCallback>>>, state: PlayerState) {
    if let Some(ref cb) = *cb.lock().unwrap() {
        cb(state);
    }
}

/// Record a state transition where both the callback *and* `state()` can see
/// it.
///
/// Emitting alone only fires the callback; the UI polls `state()`, so any
/// transition that skips `inner.state` is invisible to it.
fn set_state(
    inner: &Arc<Mutex<EngineInner>>,
    state_cb: &Arc<Mutex<Option<StateCallback>>>,
    state: PlayerState,
) {
    inner.lock().unwrap().state = state.clone();
    emit(state_cb, state);
}

/// Record a playback failure. A failed stream used to look like a player
/// idling at 0:00 with nothing wrong.
fn fail(
    inner: &Arc<Mutex<EngineInner>>,
    state_cb: &Arc<Mutex<Option<StateCallback>>>,
    message: String,
) {
    set_state(inner, state_cb, PlayerState::Error(message));
}

fn emit_progress(cb: &Arc<Mutex<Option<ProgressCallback>>>, pos: f64, dur: f64) {
    if let Some(ref cb) = *cb.lock().unwrap() {
        cb(pos, dur);
    }
}

fn set_duration(inner: &Arc<Mutex<EngineInner>>, duration: f64) {
    let mut eng = inner.lock().unwrap();
    eng.duration = duration;
    eng.position = 0.0;
}

fn set_position(inner: &Arc<Mutex<EngineInner>>, position: f64) {
    let mut eng = inner.lock().unwrap();
    eng.position = position;
}

/// Guess a probe hint from the stream URL's file extension.
fn stream_hint(url: &str) -> Option<&'static str> {
    let path = url.split('?').next().unwrap_or(url);
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "mp3" => Some("mp3"),
        // `m4s` is a DASH segment — what Bilibili's CDN serves. The resolver
        // already special-cases it; this table used to not, so those streams
        // reached the probe with no hint at all (#23).
        "m4a" | "mp4" | "mov" | "m4s" => Some("m4a"),
        "aac" => Some("aac"),
        "flac" => Some("flac"),
        "ogg" | "opus" => Some("ogg"),
        "wav" => Some("wav"),
        "aiff" | "aif" => Some("aiff"),
        _ => None,
    }
}
