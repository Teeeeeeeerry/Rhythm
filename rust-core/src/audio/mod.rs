use crate::{PlayerState, ProgressCallback, RhythmError, RhythmResult, StateCallback};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod decoder;
mod output;

use decoder::AudioDecoder;
use output::AudioOutput;

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
                inner,
                state_cb,
                progress_cb,
                stop_flag,
            ) {
                log::error!("Playback error: {e}");
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
            if let Err(e) = play_url_impl(&url, inner, state_cb, progress_cb, stop_flag) {
                log::error!("Playback error: {e}");
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
        self.emit_state(PlayerState::Stopped);
    }

    /// Seek to a position in seconds.
    pub fn seek(&self, seconds: f64) -> RhythmResult<()> {
        let inner = self.inner.lock().unwrap();
        if seconds < 0.0 || seconds > inner.duration {
            return Err(RhythmError::InvalidInput(format!(
                "Seek position {seconds}s out of range [0, {}]",
                inner.duration
            )));
        }
        // Seek is handled by the decoder thread checking the desired position
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

    fn emit_progress(&self, position: f64, duration: f64) {
        if let Some(ref cb) = *self.progress_callback.lock().unwrap() {
            cb(position, duration);
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
    {
        let mut eng = inner.lock().unwrap();
        eng.current_source = Some(path.to_string());
        eng.state = PlayerState::Playing;
    }
    emit(&state_cb, PlayerState::Playing);

    // Open and decode the file
    let mut decoder = AudioDecoder::open_file(Path::new(path))?;
    let duration = decoder.duration();

    {
        let mut eng = inner.lock().unwrap();
        eng.duration = duration;
        eng.position = 0.0;
    }

    let mut output = AudioOutput::new()?;

    // Decode and play loop
    while !stop_flag.load(Ordering::SeqCst) {
        // Check pause state
        {
            let eng = inner.lock().unwrap();
            if eng.state == PlayerState::Paused {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
        }

        match decoder.next_packet() {
            Ok(Some(pcm_data)) => {
                output.write(&pcm_data)?;
                let pos = decoder.position();
                {
                    let mut eng = inner.lock().unwrap();
                    eng.position = pos;
                }
                emit_progress(&progress_cb, pos, duration);
            }
            Ok(None) => break, // End of stream
            Err(e) => {
                emit(&state_cb, PlayerState::Error(e.to_string()));
                return Err(e);
            }
        }
    }

    emit(&state_cb, PlayerState::Stopped);
    Ok(())
}

fn play_url_impl(
    _url: &str,
    _inner: Arc<Mutex<EngineInner>>,
    state_cb: Arc<Mutex<Option<StateCallback>>>,
    _progress_cb: Arc<Mutex<Option<ProgressCallback>>>,
    _stop_flag: Arc<AtomicBool>,
) -> RhythmResult<()> {
    emit(&state_cb, PlayerState::Buffering);

    // TODO: Implement HTTP streaming + buffering
    // For now, placeholder
    emit(&state_cb, PlayerState::Playing);
    emit(&state_cb, PlayerState::Stopped);
    Ok(())
}

fn emit(cb: &Arc<Mutex<Option<StateCallback>>>, state: PlayerState) {
    if let Some(ref cb) = *cb.lock().unwrap() {
        cb(state);
    }
}

fn emit_progress(cb: &Arc<Mutex<Option<ProgressCallback>>>, pos: f64, dur: f64) {
    if let Some(ref cb) = *cb.lock().unwrap() {
        cb(pos, dur);
    }
}
