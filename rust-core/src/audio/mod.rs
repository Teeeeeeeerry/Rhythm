use crate::resolver::{evict_resolution, resolve_url, resolve_url_fresh, ResolveResult, ResolvedUrl};
use crate::{HttpErrorKind, PlayerState, ProgressCallback, RhythmError, RhythmResult, StateCallback};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
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

/// URL resolution seam: turns a user-entered URL into a playable stream URL.
///
/// The production resolver (`resolve_url`) shells out to yt-dlp for YouTube /
/// Bilibili links and fetches direct audio URLs itself. Tests inject stubs so
/// the `play_url` path can be exercised without network or subprocess access.
/// Introduced as part of the Wave 1 minimum seam
/// (docs/testing/behavior/audio-engine.md, AE-04/AE-05/AE-28).
pub type UrlResolver = Arc<dyn Fn(&str) -> ResolveResult<ResolvedUrl> + Send + Sync>;

/// Decode seam: everything the playback loop needs from a decoder.
///
/// `AudioDecoder` (symphonia) is the production implementation; tests inject
/// fakes that hand out pre-canned packets with scripted failures and pacing.
/// Abstracting this lets the playback loop run on a test thread with no audio
/// device or real media (docs/testing/behavior/audio-engine.md).
pub trait Decoder {
    fn next_packet(&mut self) -> RhythmResult<Option<Vec<f32>>>;
    fn seek(&mut self, seconds: f64) -> RhythmResult<()>;
    fn position(&self) -> f64;
    fn duration(&self) -> f64;
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
}

/// Output seam: everything the playback loop needs from an audio sink.
///
/// `AudioOutput` (cpal) is the production implementation; tests inject an
/// in-memory collector so decoded audio is assertable.
pub trait Sink {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
    fn write(&mut self, data: &[f32]) -> RhythmResult<()>;
    fn drain(&mut self);
}

/// Audio engine for playback of local files and network streams.
///
/// Uses a generation counter so that a new `play_file` / `play_url` call
/// automatically stops the previous playback thread — the old thread sees a
/// mismatch and exits its loop, preventing two cpal output streams from
/// mixing on the same device (#51).
pub struct AudioEngine {
    inner: Arc<Mutex<EngineInner>>,
    state_callback: Arc<Mutex<Option<StateCallback>>>,
    progress_callback: Arc<Mutex<Option<ProgressCallback>>>,
    generation: Arc<AtomicU64>,
    resolver: UrlResolver,
    /// Cache-bypass re-resolution, wired only in production (#120). When a
    /// stream URL is rejected, the engine evicts the cached resolution and
    /// re-resolves with this (freshly signed URL) instead of the cache that
    /// would hand back the same dead link.
    fresh_resolver: Option<UrlResolver>,
    /// Cache eviction for the #120 recovery; wired only in production.
    evictor: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

struct EngineInner {
    current_source: Option<String>,
    state: PlayerState,
    volume: f32,
    duration: f64,
    position: f64,
    /// Pending seek requested by the user; consumed by the playback thread.
    desired_position: Option<f64>,
    /// Classification of the last playback failure, when it was an HTTP one
    /// (#120). Lets the UI say "the link expired" vs "the CDN rejected it".
    last_http_error: Option<HttpErrorKind>,
}

impl AudioEngine {
    pub fn new() -> Self {
        // #120 recovery: a rejected stream URL (403 on a still-valid URL, or
        // a genuinely expired one) is retried once with a fresh resolution.
        AudioEngine::new_with_resolver(Arc::new(resolve_url))
            .with_recovery(Arc::new(resolve_url_fresh), Arc::new(evict_resolution))
    }

    /// Test seam: construct an engine whose URL resolution goes through
    /// `resolver` instead of the real yt-dlp / direct-fetch pipeline.
    #[doc(hidden)]
    pub fn new_with_resolver(resolver: UrlResolver) -> Self {
        AudioEngine {
            inner: Arc::new(Mutex::new(EngineInner {
                current_source: None,
                state: PlayerState::Stopped,
                volume: 1.0,
                duration: 0.0,
                position: 0.0,
                desired_position: None,
                last_http_error: None,
            })),
            state_callback: Arc::new(Mutex::new(None)),
            progress_callback: Arc::new(Mutex::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            resolver,
            fresh_resolver: None,
            evictor: None,
        }
    }

    /// Test seam: wire the #120 cache-bypass recovery (fresh re-resolution +
    /// eviction) onto an engine built with a stub resolver.
    #[doc(hidden)]
    pub fn with_recovery(
        mut self,
        fresh_resolver: UrlResolver,
        evictor: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> Self {
        self.fresh_resolver = Some(fresh_resolver);
        self.evictor = Some(evictor);
        self
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

        let path = path.to_path_buf();
        self.play_file_with(
            path_str,
            move || {
                let decoder = AudioDecoder::open_file(&path)?;
                Ok((decoder, None))
            },
            |_| AudioOutput::new(),
        )
    }

    /// Test seam: like `play_file`, but `open_decoder` / `open_sink` build the
    /// decoder and output (e.g. fakes) instead of the production
    /// `AudioDecoder` / `AudioOutput`. The decoder may hand back a fallback
    /// duration used when it reports none (DASH streams).
    #[doc(hidden)]
    pub fn play_file_with<D, S>(
        &self,
        source: String,
        open_decoder: impl FnOnce() -> RhythmResult<(D, Option<f64>)> + Send + 'static,
        open_sink: impl FnOnce(&D) -> RhythmResult<S> + Send + 'static,
    ) -> RhythmResult<()>
    where
        D: Decoder,
        S: Sink,
    {
        self.spawn_playback(source, PlayerState::Playing, open_decoder, open_sink)
    }

    /// Play from a network URL stream.
    pub fn play_url(&self, url: &str) -> RhythmResult<()> {
        self.play_url_with(url, |resolved| open_resolved_stream(&resolved), |_| {
            AudioOutput::new()
        })
    }

    /// Test seam: like `play_url`, but `open_decoder` receives the resolved
    /// URL and may substitute a fake decoder / stream instead of the real
    /// `HttpStream` + `AudioDecoder` pipeline. The engine's resolver (see
    /// `new_with_resolver`) still runs first on the playback thread.
    ///
    /// `FnMut`, not `FnOnce`: the #120 recovery calls it a second time with a
    /// fresh resolution when the first stream URL was rejected.
    #[doc(hidden)]
    pub fn play_url_with<D, S>(
        &self,
        url: &str,
        mut open_decoder: impl FnMut(ResolvedUrl) -> RhythmResult<(D, Option<f64>)> + Send + 'static,
        open_sink: impl FnOnce(&D) -> RhythmResult<S> + Send + 'static,
    ) -> RhythmResult<()>
    where
        D: Decoder,
        S: Sink,
    {
        let url = url.to_string();
        let resolver = self.resolver.clone();
        let fresh_resolver = self.fresh_resolver.clone();
        let evictor = self.evictor.clone();
        self.spawn_playback(
            url.clone(),
            PlayerState::Buffering,
            move || {
                let resolved = resolver(&url)?;
                let first = open_decoder(resolved);
                match first {
                    Ok(pair) => Ok(pair),
                    Err(e) if is_retryable_http(&e) => {
                        // #120: the stream URL was rejected — either the link
                        // genuinely expired or the CDN refused a valid one.
                        // Either way the cached resolution is useless: evict
                        // it, re-resolve bypassing the cache (freshly signed
                        // URL), and retry the open once. Stub resolvers don't
                        // cache, so an unwired engine just resolves again.
                        if let RhythmError::Http(http) = &e {
                            crate::resolver::log_playback_http(&url, http);
                        }
                        log::warn!("audio: stream URL rejected ({e}); re-resolving {url}");
                        if let Some(evict) = &evictor {
                            evict(&url);
                        }
                        let fresh = match &fresh_resolver {
                            Some(fresh) => fresh(&url)?,
                            None => resolver(&url)?,
                        };
                        open_decoder(fresh)
                    }
                    Err(e) => Err(e),
                }
            },
            open_sink,
        )
    }

    /// Bump the generation and start a playback thread running
    /// `drive_playback`. The previous thread (if any) sees the mismatch and
    /// exits its loop (#51).
    /// `D` / `S` are created on the playback thread itself, so unlike the
    /// factory closures they need no `Send` bound (cpal's stream handle is
    /// not `Send`, and it never has to be).
    fn spawn_playback<D, S>(
        &self,
        source: String,
        pre_state: PlayerState,
        open_decoder: impl FnOnce() -> RhythmResult<(D, Option<f64>)> + Send + 'static,
        open_sink: impl FnOnce(&D) -> RhythmResult<S> + Send + 'static,
    ) -> RhythmResult<()>
    where
        D: Decoder,
        S: Sink,
    {
        let my_gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

        let inner = self.inner.clone();
        let state_cb = self.state_callback.clone();
        let progress_cb = self.progress_callback.clone();
        let generation = self.generation.clone();

        thread::spawn(move || {
            if let Err(e) = drive_playback(
                source,
                pre_state,
                open_decoder,
                open_sink,
                inner.clone(),
                state_cb.clone(),
                progress_cb,
                generation,
                my_gen,
            ) {
                log::error!("Playback error: {e}");
                fail(&inner, &state_cb, &e);
            }
        });

        Ok(())
    }

    /// Test seam: run the playback loop synchronously on the caller's thread
    /// with injected fakes — no spawned thread, no audio device. Engine
    /// controls (`pause` / `resume` / `seek` / `stop`) can be driven from a
    /// sibling thread while the loop runs, exactly as they are in production.
    ///
    /// Bumps the generation like `spawn_playback`, so it also terminates any
    /// live playback loop on the same engine (the #51 contract works in both
    /// directions), and `stop()` / a later `play_*` ends this loop the same
    /// way it ends a spawned playback thread.
    #[doc(hidden)]
    pub fn run_playback_with<D, S>(
        &self,
        source: String,
        pre_state: PlayerState,
        open_decoder: impl FnOnce() -> RhythmResult<(D, Option<f64>)>,
        open_sink: impl FnOnce(&D) -> RhythmResult<S>,
    ) -> RhythmResult<()>
    where
        D: Decoder,
        S: Sink,
    {
        let my_gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        drive_playback(
            source,
            pre_state,
            open_decoder,
            open_sink,
            self.inner.clone(),
            self.state_callback.clone(),
            self.progress_callback.clone(),
            self.generation.clone(),
            my_gen,
        )
    }

    /// Pause playback.
    ///
    /// #111: also accepts `Buffering` — a pause during a slow URL resolve /
    /// prebuffer must stick, otherwise the buffered stream starts `Playing`
    /// and pushes audio while the UI shows paused.
    pub fn pause(&self) {
        let mut inner = self.inner.lock().unwrap();
        if inner.state == PlayerState::Playing || inner.state == PlayerState::Buffering {
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
        // Bump the generation so the playback thread sees a mismatch and
        // exits its loop (#51).
        self.generation.fetch_add(1, Ordering::SeqCst);
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

    /// Classification of the last playback failure, when it was an HTTP one
    /// (expired link vs CDN rejection vs other). `None` when the last failure
    /// was not HTTP, or there was none (#120).
    pub fn last_error_kind(&self) -> Option<HttpErrorKind> {
        self.inner.lock().unwrap().last_http_error
    }

    /// Test seam: the source recorded for the current playback request.
    #[doc(hidden)]
    pub fn current_source(&self) -> Option<String> {
        self.inner.lock().unwrap().current_source.clone()
    }

    fn emit_state(&self, state: PlayerState) {
        if let Some(ref cb) = *self.state_callback.lock().unwrap() {
            cb(state);
        }
    }
}

/// Shared playback driver for files and URLs: record the source, emit the
/// pre-open state, open decoder and sink, then run the loop.
///
/// Files emit `Playing` before the (fast) open. URLs emit `Buffering` for the
/// slow resolve + connect + prebuffer window and only then `Playing` — the
/// Buffering state has to be recorded, not just emitted, because the UI polls
/// `state()` rather than registering a callback (#23).
/// The production URL stream → decoder pipeline, shared by `play_url` and the
/// AE tests so the latter always exercise the real open path (headers,
/// prebuffer wait, probe hint).
#[doc(hidden)]
pub fn open_resolved_stream(
    resolved: &ResolvedUrl,
) -> RhythmResult<(AudioDecoder, Option<f64>)> {
    // Open the HTTP stream — this starts the prefetch downloader. The
    // resolver's headers come along: Bilibili's CDN answers 403 without
    // the Referer yt-dlp used.
    let stream = HttpStream::open_with_headers(&resolved.stream_url, &resolved.http_headers)?;

    // Wait for the initial buffer so the probe/decoder doesn't stall on
    // every read.
    stream.wait_initial_buffered()?;

    // Use the URL's file extension as a probe hint (helps when the stream
    // lacks a standard container header).
    let hint = stream_hint(&resolved.stream_url);
    let decoder = AudioDecoder::open_source(Box::new(stream), hint)?;
    Ok((decoder, Some(resolved.duration)))
}

fn drive_playback<D, S>(
    source: String,
    pre_state: PlayerState,
    open_decoder: impl FnOnce() -> RhythmResult<(D, Option<f64>)>,
    open_sink: impl FnOnce(&D) -> RhythmResult<S>,
    inner: Arc<Mutex<EngineInner>>,
    state_cb: Arc<Mutex<Option<StateCallback>>>,
    progress_cb: Arc<Mutex<Option<ProgressCallback>>>,
    generation: Arc<AtomicU64>,
    my_gen: u64,
) -> RhythmResult<()>
where
    D: Decoder,
    S: Sink,
{
    // Files record their source before the (fast) open, URLs only once the
    // stream actually opened — a failed resolve must not clobber the source
    // of the track that was playing.
    if pre_state == PlayerState::Playing {
        inner.lock().unwrap().current_source = Some(source.clone());
    }
    set_state(&inner, &state_cb, pre_state.clone());

    let (mut decoder, fallback_duration) = open_decoder()?;

    // The URL path already emitted Buffering; now that the stream is open,
    // announce the real transition to Playing.
    if pre_state != PlayerState::Playing {
        // #111: a pause during Buffering must stick — only announce Playing
        // if the state is still the pre-open one. Check and set under one
        // lock so a pause() landing right here cannot be clobbered by an
        // unconditional Playing transition (the exact divergence #111 fixes).
        let mut eng = inner.lock().unwrap();
        eng.current_source = Some(source);
        if eng.state == pre_state {
            eng.state = PlayerState::Playing;
            drop(eng);
            emit(&state_cb, PlayerState::Playing);
        }
    }

    // DASH segments (Bilibili's audio streams) carry no usable duration box,
    // so the decoder reports 0 and the UI would sit at "0:00 / 0:00". The
    // resolver already knows the real length — use it as the fallback.
    let duration = if decoder.duration() > 0.0 {
        decoder.duration()
    } else {
        fallback_duration.unwrap_or(0.0)
    };
    set_duration(&inner, duration);

    let mut output = open_sink(&decoder)?;
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
        generation,
        my_gen,
    )
}

/// Shared decode → resample → output loop. Applies pending seeks, pause, and
/// volume, and reports progress via the callback.
///
/// The loop exits when the global generation counter no longer matches
/// `my_gen` — this happens when `stop()`, `play_file()`, or `play_url()` is
/// called while the thread is running (#51).
fn run_playback_loop<D: Decoder + ?Sized, S: Sink + ?Sized>(
    decoder: &mut D,
    output: &mut S,
    resampler: &mut Resampler,
    inner: Arc<Mutex<EngineInner>>,
    state_cb: Arc<Mutex<Option<StateCallback>>>,
    progress_cb: Arc<Mutex<Option<ProgressCallback>>>,
    generation: Arc<AtomicU64>,
    my_gen: u64,
) -> RhythmResult<()> {
    let out_rate = output.sample_rate();
    let out_channels = output.channels();

    while generation.load(Ordering::SeqCst) == my_gen {
        // Check pause state. A paused loop still consumes pending seeks so a
        // drag while paused applies immediately (#77): the decoder is idle
        // here, so seeking is safe, and resume then continues from the new
        // position (the resampler is reset so no stale audio is replayed).
        {
            let mut eng = inner.lock().unwrap();
            if eng.state == PlayerState::Paused {
                if let Some(secs) = eng.desired_position.take() {
                    drop(eng);
                    decoder.seek(secs)?;
                    resampler.reset();
                    set_position(&inner, decoder.position());
                    emit_progress(&progress_cb, decoder.position(), decoder.duration());
                } else {
                    drop(eng);
                    thread::sleep(Duration::from_millis(50));
                }
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
    if generation.load(Ordering::SeqCst) == my_gen {
        // Drain the output queue before the stream is torn down so the
        // listener hears the tail of the last decoded packet instead of
        // having it clipped by the cpal stream drop (#28).
        output.drain();
        set_state(&inner, &state_cb, PlayerState::Finished);
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
///
/// HTTP failures also record their classification so the UI can pick a
/// truthful message ("link expired" vs "CDN rejected your network") (#120).
fn fail(
    inner: &Arc<Mutex<EngineInner>>,
    state_cb: &Arc<Mutex<Option<StateCallback>>>,
    error: &RhythmError,
) {
    let mut eng = inner.lock().unwrap();
    eng.last_http_error = match error {
        RhythmError::Http(http) => Some(http.kind),
        _ => None,
    };
    drop(eng);
    set_state(inner, state_cb, PlayerState::Error(error.to_string()));
}

/// Should a failed stream open trigger a cache-bypass re-resolve + retry?
///
/// Yes for a genuinely expired link (a fresh resolve returns a new signed
/// URL) and for a CDN-rejected one (the cached URL may be the problem); no
/// for anything else — 5xx/DNS/TLS re-resolution cannot help (#120).
fn is_retryable_http(error: &RhythmError) -> bool {
    matches!(
        error,
        RhythmError::Http(http)
            if http.kind == HttpErrorKind::Expired || http.kind == HttpErrorKind::CdnRejected
    )
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
pub fn stream_hint(url: &str) -> Option<&'static str> {
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
