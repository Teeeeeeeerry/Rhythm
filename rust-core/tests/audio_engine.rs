//! AudioEngine state machine behavior tests
//! (docs/testing/behavior/audio-engine.md, AE-01..AE-30).
//!
//! The decoder/output seam (traits `Decoder` / `Sink` plus
//! `run_playback_with` / `play_file_with` / `play_url_with`) lets these tests
//! drive the real playback loop with scripted fakes: pre-canned packet
//! streams, controlled pacing, and failure injection — no audio device, no
//! real network.

mod common;

use common::{make_wav_bytes, RangeServer};

use rhythm_core::audio::decoder::AudioDecoder;
use rhythm_core::audio::{open_resolved_stream, stream_hint, AudioEngine, Decoder, Sink};
use rhythm_core::resolver::{ResolveErrorKind, ResolveFailure, ResolvedUrl};
use rhythm_core::{HttpError, HttpErrorKind, PlayerState, RhythmError, RhythmResult, SourceType};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ── fakes ───────────────────────────────────────────────────────────────────

/// One scripted step of a fake decoder.
enum Step {
    /// Hand out this interleaved PCM, reporting `position`.
    Packet { pcm: Vec<f32>, position: f64 },
    /// Fail the loop with a decode error.
    Fail(&'static str),
    /// End the stream.
    End,
}

/// Observations on a decoder that has moved into a playback thread.
#[derive(Default)]
struct DecoderProbe {
    /// `next_packet` calls.
    polls: AtomicUsize,
    /// Seek targets received, in order.
    seeks: Mutex<Vec<f64>>,
}

/// Scriptable decoder for the playback loop.
struct FakeDecoder {
    steps: Vec<Step>,
    next: usize,
    sample_rate: u32,
    channels: u16,
    duration: f64,
    position: f64,
    probe: Arc<DecoderProbe>,
    /// Packet repeated forever once the script is exhausted (endless streams).
    tail: Option<(Vec<f32>, f64)>,
    /// Sleep before handing out each tail packet (pacing for thread tests).
    tail_delay: Option<Duration>,
    /// When set, this `next_packet` call blocks until the test releases it —
    /// parks the loop at a deterministic point (e.g. so a pending seek can
    /// provably not be consumed before a stop).
    block_rx: Option<std::sync::mpsc::Receiver<()>>,
    /// Set by `seek`; the endless tail keeps reporting the seeked position
    /// instead of its canned one (a post-seek `position()` assertion must
    /// not be clobbered by the next tail packet).
    seeked: bool,
}

impl FakeDecoder {
    fn new(sample_rate: u32, channels: u16, duration: f64) -> Self {
        FakeDecoder {
            steps: Vec::new(),
            next: 0,
            sample_rate,
            channels,
            duration,
            position: 0.0,
            probe: Arc::new(DecoderProbe::default()),
            tail: None,
            tail_delay: None,
            block_rx: None,
            seeked: false,
        }
    }

    fn with_probe(mut self, probe: Arc<DecoderProbe>) -> Self {
        self.probe = probe;
        self
    }

    fn packet(mut self, pcm: Vec<f32>, position: f64) -> Self {
        self.steps.push(Step::Packet { pcm, position });
        self
    }

    fn fail(mut self, msg: &'static str) -> Self {
        self.steps.push(Step::Fail(msg));
        self
    }

    fn end(mut self) -> Self {
        self.steps.push(Step::End);
        self
    }

    /// Block the next `next_packet` call until `rx` receives a release.
    fn block_until_released(mut self, rx: std::sync::mpsc::Receiver<()>) -> Self {
        self.block_rx = Some(rx);
        self
    }

    /// A stream that never ends: one packet, repeated every `delay` at a fixed
    /// `position` — keeps a playback loop alive until a generation bump.
    fn endless_paced(
        sample_rate: u32,
        channels: u16,
        duration: f64,
        delay: Duration,
        position: f64,
    ) -> Self {
        let mut decoder = Self::new(sample_rate, channels, duration);
        decoder
            .steps
            .push(Step::Packet { pcm: vec![0.25f32; 8], position });
        decoder.tail = Some((vec![0.25f32; 8], position));
        decoder.tail_delay = Some(delay);
        decoder
    }
}

impl Decoder for FakeDecoder {
    fn next_packet(&mut self) -> RhythmResult<Option<Vec<f32>>> {
        self.probe.polls.fetch_add(1, Ordering::SeqCst);
        if let Some(rx) = self.block_rx.take() {
            // The sender drops when the test panics before releasing: end the
            // stream instead of panicking off-thread where the harness
            // cannot surface it.
            if rx.recv().is_err() {
                return Ok(None);
            }
        }
        loop {
            match self.steps.get(self.next) {
                Some(Step::Packet { pcm, position }) => {
                    self.next += 1;
                    self.position = *position;
                    return Ok(Some(pcm.clone()));
                }
                Some(Step::Fail(msg)) => {
                    self.next += 1;
                    return Err(RhythmError::Decode((*msg).to_string()));
                }
                Some(Step::End) => {
                    self.next += 1;
                    return Ok(None);
                }
                None => match &self.tail {
                    Some((pcm, position)) => {
                        if let Some(delay) = self.tail_delay {
                            thread::sleep(delay);
                        }
                        if !self.seeked {
                            self.position = *position;
                        }
                        return Ok(Some(pcm.clone()));
                    }
                    None => return Ok(None),
                },
            }
        }
    }

    fn seek(&mut self, seconds: f64) -> RhythmResult<()> {
        self.probe.seeks.lock().unwrap().push(seconds);
        self.position = seconds;
        self.seeked = true;
        Ok(())
    }

    fn position(&self) -> f64 {
        self.position
    }

    fn duration(&self) -> f64 {
        self.duration
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        self.channels
    }
}

/// Observations on a sink that has moved into a playback thread.
#[derive(Default)]
struct SinkProbe {
    /// Every sample ever written, flattened.
    samples: Mutex<Vec<f32>>,
    /// `write` calls.
    write_count: AtomicUsize,
    /// `drain` called.
    drained: AtomicBool,
}

/// In-memory audio sink: collects interleaved PCM instead of playing it.
struct FakeSink {
    sample_rate: u32,
    channels: u16,
    probe: Arc<SinkProbe>,
}

impl FakeSink {
    fn new(sample_rate: u32, channels: u16, probe: Arc<SinkProbe>) -> Self {
        FakeSink {
            sample_rate,
            channels,
            probe,
        }
    }
}

impl Sink for FakeSink {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn write(&mut self, data: &[f32]) -> RhythmResult<()> {
        self.probe.write_count.fetch_add(1, Ordering::SeqCst);
        self.probe.samples.lock().unwrap().extend_from_slice(data);
        Ok(())
    }

    fn drain(&mut self) {
        self.probe.drained.store(true, Ordering::SeqCst);
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// The state / progress callback logs of one engine.
struct Recorders {
    states: Arc<Mutex<Vec<PlayerState>>>,
    progress: Arc<Mutex<Vec<(f64, f64)>>>,
}

impl Recorders {
    fn states(&self) -> Vec<PlayerState> {
        self.states.lock().unwrap().clone()
    }

    fn progress(&self) -> Vec<(f64, f64)> {
        self.progress.lock().unwrap().clone()
    }
}

fn attach_recorders(engine: &AudioEngine) -> Recorders {
    let states = Arc::new(Mutex::new(Vec::new()));
    let progress = Arc::new(Mutex::new(Vec::new()));
    let s = states.clone();
    engine.on_state_change(Box::new(move |state| s.lock().unwrap().push(state)));
    let p = progress.clone();
    engine.on_progress(Box::new(move |pos, dur| p.lock().unwrap().push((pos, dur))));
    Recorders { states, progress }
}

/// Poll `cond` every 5 ms until it holds or `timeout` elapses.
fn wait_for(what: &str, timeout: Duration, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {what}");
}

/// Like `wait_for`, but reports the engine's final state and callback log on
/// timeout — the flaky URL-pipeline failures would otherwise only say which
/// state never arrived, not where the engine actually landed.
fn wait_for_state(engine: &AudioEngine, rec: &Recorders, want: PlayerState, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let now = engine.state();
        if now == want {
            return;
        }
        if Instant::now() > deadline {
            panic!(
                "timed out waiting for {want:?}: state={now:?}, log={:?}",
                rec.states()
            );
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Like `wait_for_state`, but accepts any of `wants` — used where a short
/// stream can legitimately sprint through the wanted state before a poll
/// sees it (e.g. Playing for a fast fake/one-second WAV); the callback log
/// is then asserted separately.
fn wait_for_state_any(
    engine: &AudioEngine,
    rec: &Recorders,
    wants: &[PlayerState],
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    loop {
        let now = engine.state();
        if wants.contains(&now) {
            return;
        }
        if Instant::now() > deadline {
            panic!(
                "timed out waiting for any of {wants:?}: state={now:?}, log={:?}",
                rec.states()
            );
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Assert the engine and its callback log both landed in `Error` with a
/// non-empty message (the error-path discipline: report it, don't match text).
fn assert_error_reported(engine: &AudioEngine, rec: &Recorders) {
    match engine.state() {
        PlayerState::Error(msg) => assert!(!msg.is_empty(), "error message must not be empty"),
        other => panic!("expected Error state, got {other:?}"),
    }
    assert!(
        rec.states()
            .iter()
            .any(|s| matches!(s, PlayerState::Error(_))),
        "state callback never saw Error: {:?}",
        rec.states()
    );
}

fn stub_resolved(stream_url: &str, duration: f64) -> ResolvedUrl {
    ResolvedUrl {
        title: "stub".to_string(),
        artist: None,
        stream_url: stream_url.to_string(),
        duration,
        source_type: SourceType::DirectUrl,
        thumbnail_url: None,
        http_headers: BTreeMap::new(),
    }
}

/// Write a WAV file into a fresh temp dir; hold the returned `TempDir` for
/// the duration of the test so the fixture is cleaned up.
fn write_temp_wav(seconds: f64) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tone.wav");
    std::fs::write(&path, make_wav_bytes(seconds)).unwrap();
    (dir, path)
}

// ── 主路径 (P0) ─────────────────────────────────────────────────────────────

/// AE-01: a fresh engine is stopped, full volume, at 0:00.
#[test]
fn ae01_initial_state() {
    let engine = AudioEngine::new();
    assert_eq!(engine.state(), PlayerState::Stopped);
    assert_eq!(engine.volume(), 1.0);
    assert_eq!(engine.duration(), 0.0);
    assert_eq!(engine.position(), 0.0);
}

/// AE-02: playing a missing file fails fast — FileNotFound, still Stopped,
/// and no playback thread is started.
#[test]
fn ae02_play_file_missing_file_fails_fast() {
    let engine = AudioEngine::new();
    let err = engine
        .play_file(Path::new("/nonexistent/rhythm-tone.wav"))
        .unwrap_err();
    assert!(matches!(err, RhythmError::FileNotFound(_)));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(engine.state(), PlayerState::Stopped);
    assert_eq!(engine.current_source(), None);
}

/// AE-03: playing a real WAV file reaches Playing (state + callback) with
/// the decoder's duration, and audio flows into the sink.
#[test]
fn ae03_play_file_starts_playback() {
    let (_dir, path) = write_temp_wav(2.0);
    let engine = AudioEngine::new();
    let rec = attach_recorders(&engine);
    let probe = Arc::new(SinkProbe::default());
    let sink = FakeSink::new(44100, 2, probe.clone());

    let path2 = path.clone();
    engine
        .play_file_with(
            path.display().to_string(),
            move || {
                let decoder = AudioDecoder::open_file(&path2)?;
                Ok((decoder, None))
            },
            move |_| Ok(sink),
        )
        .unwrap();

    // Playing is emitted before the decoder opens, so duration lags it; a
    // short file can also sprint through Playing to Finished between polls.
    // Wait for the terminal states, then assert the log and duration.
    wait_for_state_any(
        &engine,
        &rec,
        &[PlayerState::Playing, PlayerState::Finished],
        Duration::from_secs(5),
    );
    assert!(rec.states().contains(&PlayerState::Playing));
    wait_for("duration known", Duration::from_secs(5), || engine.duration() > 0.0);
    assert!(
        (engine.duration() - 2.0).abs() < 0.05,
        "duration was {}",
        engine.duration()
    );
    wait_for("audio written", Duration::from_secs(5), || {
        probe.write_count.load(Ordering::SeqCst) > 0
    });
}

/// AE-04: play_url records Buffering (state + callback) for the whole
/// resolve / connect / prebuffer window (#23).
#[test]
fn ae04_play_url_shows_buffering_while_resolving() {
    let server = RangeServer::start_with_path(make_wav_bytes(2.0), "/tone.wav");
    let resolved = stub_resolved(&server.url(), 42.0);
    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| {
        // Slow resolve: the Buffering window must be observable for its whole
        // duration, not just flashed.
        thread::sleep(Duration::from_millis(300));
        Ok(resolved.clone())
    }));
    let rec = attach_recorders(&engine);
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            |resolved| open_resolved_stream(&resolved),
            move |_| Ok(sink),
        )
        .unwrap();

    wait_for_state(&engine, &rec, PlayerState::Buffering, Duration::from_secs(2));
    assert!(
        rec.states().contains(&PlayerState::Buffering),
        "Buffering must also reach the callback: {:?}",
        rec.states()
    );
    // Still resolving: the state must still read Buffering.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(engine.state(), PlayerState::Buffering);

    // A fast machine can decode the 2s WAV in a few poll intervals; accept
    // the sprint past Playing and assert the transition log instead.
    wait_for_state_any(
        &engine,
        &rec,
        &[PlayerState::Playing, PlayerState::Finished],
        Duration::from_secs(5),
    );
}

/// AE-05: play_url lands in Playing; when the decoder knows the duration it
/// wins over the resolver's (here a decoy 42.0), and when it doesn't the
/// resolved duration is the fallback.
#[test]
fn ae05_play_url_plays_and_uses_decoder_duration() {
    let server = RangeServer::start_with_path(make_wav_bytes(1.0), "/tone.wav");
    let resolved = stub_resolved(&server.url(), 42.0);
    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| Ok(resolved.clone())));
    let rec = attach_recorders(&engine);
    let probe = Arc::new(SinkProbe::default());
    let sink = FakeSink::new(44100, 2, probe.clone());

    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            |resolved| open_resolved_stream(&resolved),
            move |_| Ok(sink),
        )
        .unwrap();

    wait_for_state_any(
        &engine,
        &rec,
        &[PlayerState::Playing, PlayerState::Finished],
        Duration::from_secs(5),
    );
    assert!(rec.states().contains(&PlayerState::Playing));
    assert!(
        (engine.duration() - 1.0).abs() < 0.05,
        "decoder duration must win over the resolved 42.0, got {}",
        engine.duration()
    );
    wait_for("audio written", Duration::from_secs(5), || {
        probe.write_count.load(Ordering::SeqCst) > 0
    });
}

/// AE-05b (DASH fallback): a decoder with no duration leaves the UI on the
/// resolver's duration instead of 0:00.
#[test]
fn ae05b_play_url_falls_back_to_resolved_duration() {
    let resolved = stub_resolved("http://127.0.0.1:1/unused.m4s", 42.0);
    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| Ok(resolved.clone())));
    let rec = attach_recorders(&engine);
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            // `open_decoder` is FnMut (#120 retry), so build the fake inside
            // the closure instead of moving one in.
            move |resolved| {
                let fake = FakeDecoder::new(44100, 2, 0.0)
                    .packet(vec![0.1f32; 8], 0.1)
                    .end();
                Ok((fake, Some(resolved.duration)))
            },
            move |_| Ok(sink),
        )
        .unwrap();

    // The fake decoder finishes instantly, so the intermediate Playing may
    // be gone before a poll sees it — assert the full transition sequence
    // through the callback log instead.
    wait_for_state(&engine, &rec, PlayerState::Finished, Duration::from_secs(5));
    assert!((engine.duration() - 42.0).abs() < 1e-9);
    assert_eq!(
        rec.states(),
        vec![
            PlayerState::Buffering,
            PlayerState::Playing,
            PlayerState::Finished
        ]
    );
}

/// AE-06/AE-07: pause flips Playing → Paused, resume flips back, both are
/// visible in state and callback, and pause sticks.
#[test]
fn ae06_ae07_pause_and_resume() {
    let engine = Arc::new(AudioEngine::new());
    let rec = attach_recorders(&engine);
    let fake = FakeDecoder::endless_paced(
        44100,
        2,
        10.0,
        Duration::from_millis(5),
        0.1,
    );
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    let e2 = engine.clone();
    let handle = thread::spawn(move || {
        e2.run_playback_with(
            "fake://paced".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();
    });
    wait_for("Playing state", Duration::from_secs(5), || {
        engine.state() == PlayerState::Playing
    });

    engine.pause();
    assert_eq!(engine.state(), PlayerState::Paused);
    assert!(rec.states().contains(&PlayerState::Paused));
    thread::sleep(Duration::from_millis(80));
    assert_eq!(engine.state(), PlayerState::Paused, "pause must stick");

    // resume() contends with the paused loop, which sleeps while holding the
    // engine mutex (related to #77), so this call may take a few cycles.
    engine.resume();
    assert_eq!(engine.state(), PlayerState::Playing);
    assert!(rec.states().contains(&PlayerState::Playing));

    engine.stop();
    handle.join().unwrap();
}

/// AE-08: stop lands Stopped (state + callback), clears the recorded source,
/// exits the old thread via generation mismatch, and nothing more is emitted
/// or produced afterwards.
#[test]
fn ae08_stop_halts_playback_and_clears_state() {
    let engine = Arc::new(AudioEngine::new());
    let rec = attach_recorders(&engine);
    let fake = FakeDecoder::endless_paced(
        44100,
        2,
        10.0,
        Duration::from_millis(5),
        0.1,
    );
    let probe = Arc::new(SinkProbe::default());
    let sink = FakeSink::new(44100, 2, probe.clone());

    let e2 = engine.clone();
    let handle = thread::spawn(move || {
        e2.run_playback_with(
            "fake://paced".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();
    });
    wait_for("audio flowing", Duration::from_secs(5), || {
        probe.write_count.load(Ordering::SeqCst) > 0
    });

    engine.stop();
    assert_eq!(engine.state(), PlayerState::Stopped);
    assert_eq!(engine.current_source(), None);
    handle.join().unwrap();

    let count_after_stop = probe.write_count.load(Ordering::SeqCst);
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        probe.write_count.load(Ordering::SeqCst),
        count_after_stop,
        "old thread kept producing audio after stop"
    );
    assert_eq!(
        rec.states(),
        vec![PlayerState::Playing, PlayerState::Stopped],
        "exactly one transition sequence, ending in Stopped"
    );
}

/// AE-09: when the stream ends naturally the output is drained and the state
/// lands on Finished (#35).
#[test]
fn ae09_natural_end_reaches_finished() {
    let engine = AudioEngine::new();
    let rec = attach_recorders(&engine);
    let fake = FakeDecoder::new(44100, 2, 0.5)
        .packet(vec![0.1f32; 8], 0.1)
        .packet(vec![0.2f32; 8], 0.2)
        .end();
    let probe = Arc::new(SinkProbe::default());
    let sink = FakeSink::new(44100, 2, probe.clone());

    engine
        .run_playback_with(
            "fake://src".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();

    assert_eq!(engine.state(), PlayerState::Finished);
    assert_eq!(
        rec.states(),
        vec![PlayerState::Playing, PlayerState::Finished]
    );
    assert!(
        probe.drained.load(Ordering::SeqCst),
        "output must be drained before Finished"
    );
}

/// AE-10: a valid seek is queued, consumed by the loop, and visible as an
/// updated position and a progress event.
#[test]
fn ae10_seek_queues_and_consumes() {
    let engine = Arc::new(AudioEngine::new());
    let rec = attach_recorders(&engine);
    let probe = Arc::new(DecoderProbe::default());
    // Wide pacing so the post-seek position is observable for a while before
    // the next packet overwrites it.
    let fake = FakeDecoder::endless_paced(
        44100,
        2,
        10.0,
        Duration::from_millis(50),
        0.1,
    )
    .with_probe(probe.clone());
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    let e2 = engine.clone();
    let handle = thread::spawn(move || {
        e2.run_playback_with(
            "fake://paced".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();
    });
    wait_for("Playing state", Duration::from_secs(5), || {
        engine.state() == PlayerState::Playing
    });

    engine.seek(0.5).unwrap();
    wait_for("seek consumed", Duration::from_secs(5), || {
        !probe.seeks.lock().unwrap().is_empty()
    });
    assert_eq!(*probe.seeks.lock().unwrap(), vec![0.5]);
    wait_for("position updated", Duration::from_secs(5), || {
        (engine.position() - 0.5).abs() < 1e-9
    });
    assert!(
        rec.progress().iter().any(|&(pos, _)| (pos - 0.5).abs() < 1e-9),
        "progress callback must see the seek position: {:?}",
        rec.progress()
    );

    engine.stop();
    handle.join().unwrap();
}

/// AE-11: a negative seek is rejected without touching the state.
#[test]
fn ae11_seek_negative_rejected() {
    let engine = AudioEngine::new();
    let err = engine.seek(-1.0).unwrap_err();
    assert!(matches!(err, RhythmError::InvalidInput(_)));
    assert_eq!(engine.state(), PlayerState::Stopped);
}

/// AE-12: once a duration is known, seeking past it is rejected.
#[test]
fn ae12_seek_beyond_duration_rejected() {
    let engine = Arc::new(AudioEngine::new());
    let fake = FakeDecoder::endless_paced(
        44100,
        2,
        10.0,
        Duration::from_millis(5),
        0.1,
    );
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    let e2 = engine.clone();
    let handle = thread::spawn(move || {
        e2.run_playback_with(
            "fake://paced".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();
    });
    wait_for("duration known", Duration::from_secs(5), || {
        engine.duration() > 0.0
    });
    assert_eq!(engine.duration(), 10.0);

    let err = engine.seek(10.5).unwrap_err();
    assert!(matches!(err, RhythmError::InvalidInput(_)));
    assert_eq!(engine.state(), PlayerState::Playing, "state must be untouched");

    engine.stop();
    handle.join().unwrap();
}

/// AE-13: volume is clamped to [0, 1].
#[test]
fn ae13_volume_clamped() {
    let engine = AudioEngine::new();
    engine.set_volume(-0.5);
    assert_eq!(engine.volume(), 0.0);
    engine.set_volume(1.5);
    assert_eq!(engine.volume(), 1.0);
}

/// AE-14: playing again while a stream is active stops the old stream via
/// generation mismatch — the old decoder is no longer polled, its audio no
/// longer produced, and the new stream owns playback (#51/#52).
#[test]
fn ae14_playing_again_replaces_old_stream() {
    let engine = Arc::new(AudioEngine::new());
    let old_decoder = Arc::new(DecoderProbe::default());
    let old_sink_probe = Arc::new(SinkProbe::default());
    let fake = FakeDecoder::endless_paced(
        44100,
        2,
        10.0,
        Duration::from_millis(5),
        0.1,
    )
    .with_probe(old_decoder.clone());
    let old_sink = FakeSink::new(44100, 2, old_sink_probe.clone());

    let e2 = engine.clone();
    let handle = thread::spawn(move || {
        e2.run_playback_with(
            "fake://old".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(old_sink),
        )
        .unwrap();
    });
    wait_for("old stream flowing", Duration::from_secs(5), || {
        old_sink_probe.write_count.load(Ordering::SeqCst) > 0
    });

    // Switch to a real WAV file into a fresh sink.
    let (_dir, path) = write_temp_wav(1.0);
    let new_sink_probe = Arc::new(SinkProbe::default());
    let new_sink = FakeSink::new(44100, 2, new_sink_probe.clone());
    let path2 = path.clone();
    engine
        .play_file_with(
            path.display().to_string(),
            move || {
                let decoder = AudioDecoder::open_file(&path2)?;
                Ok((decoder, None))
            },
            move |_| Ok(new_sink),
        )
        .unwrap();

    let old_writes_at_switch = old_sink_probe.write_count.load(Ordering::SeqCst);
    let old_polls_at_switch = old_decoder.polls.load(Ordering::SeqCst);

    wait_for("new stream finished", Duration::from_secs(10), || {
        engine.state() == PlayerState::Finished
    });
    handle.join().unwrap();

    // At most one packet was already in flight when the switch happened.
    let extra_writes =
        old_sink_probe.write_count.load(Ordering::SeqCst) - old_writes_at_switch;
    assert!(
        extra_writes <= 1,
        "old stream kept writing after replacement ({extra_writes} writes)"
    );
    let extra_polls = old_decoder.polls.load(Ordering::SeqCst) - old_polls_at_switch;
    assert!(
        extra_polls <= 1,
        "old decoder kept being polled after replacement ({extra_polls} polls)"
    );
    assert!(
        new_sink_probe.write_count.load(Ordering::SeqCst) > 0,
        "new stream must own the output"
    );
}

/// AE-15: a failure anywhere in playback lands on Error with a non-empty
/// message, visible in both state and callback — not an idle 0:00 player
/// (#23).
#[test]
fn ae15_failure_lands_error_state() {
    let engine = AudioEngine::new();
    let rec = attach_recorders(&engine);
    let fake = FakeDecoder::new(44100, 2, 0.5)
        .packet(vec![0.1f32; 8], 0.1)
        .fail("injected mid-stream decode failure");
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    engine
        .play_file_with(
            "fake://failing".to_string(),
            || Ok((fake, None)),
            move |_: &FakeDecoder| Ok(sink),
        )
        .unwrap();

    wait_for("Error state", Duration::from_secs(5), || {
        matches!(engine.state(), PlayerState::Error(_))
    });
    assert_error_reported(&engine, &rec);
}

/// AE-16: the URL-extension probe hint table (incl. the m4s DASH segment
/// special case, query stripping, and case insensitivity).
#[test]
fn ae16_stream_hint_extension_mapping() {
    assert_eq!(stream_hint("http://h/a.mp3"), Some("mp3"));
    for ext in ["m4a", "mp4", "mov", "m4s"] {
        assert_eq!(stream_hint(&format!("http://h/f.{ext}")), Some("m4a"), "{ext}");
    }
    assert_eq!(stream_hint("http://h/f.aac"), Some("aac"));
    assert_eq!(stream_hint("http://h/f.flac"), Some("flac"));
    assert_eq!(stream_hint("http://h/f.wav"), Some("wav"));
    assert_eq!(stream_hint("http://h/f.ogg"), Some("ogg"));
    assert_eq!(stream_hint("http://h/f.opus"), Some("ogg"));
    assert_eq!(stream_hint("http://h/f.aiff"), Some("aiff"));
    assert_eq!(stream_hint("http://h/f.aif"), Some("aiff"));

    // Unknown / missing extensions.
    assert_eq!(stream_hint("http://h/f.xyz"), None);
    assert_eq!(stream_hint("http://h/noext"), None);

    // Query strings are ignored; case is not significant.
    assert_eq!(stream_hint("http://h/a.MP3?sig=1&x=2"), Some("mp3"));
    assert_eq!(stream_hint("http://h/dir/seg.m4s?token=abc"), Some("m4a"));
}

/// AE-17: playback reports progress events with a monotonically advancing
/// position.
#[test]
fn ae17_progress_callback_reports_advancing_position() {
    let engine = AudioEngine::new();
    let rec = attach_recorders(&engine);
    let fake = FakeDecoder::new(44100, 2, 1.0)
        .packet(vec![0.1f32; 8], 0.25)
        .packet(vec![0.2f32; 8], 0.5)
        .end();
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    engine
        .run_playback_with(
            "fake://src".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();

    let progress = rec.progress();
    assert_eq!(
        progress,
        vec![(0.25, 1.0), (0.5, 1.0)],
        "one progress event per packet, position advancing"
    );
}

// ── 边界情况 (P1) ───────────────────────────────────────────────────────────

/// AE-18: pause outside Playing/Buffering is a no-op — no state change, no
/// callback (Buffering responds to pause, see AE-31/#111).
#[test]
fn ae18_pause_outside_playing_is_noop() {
    let engine = AudioEngine::new();
    let rec = attach_recorders(&engine);
    engine.pause();
    engine.pause();
    assert_eq!(engine.state(), PlayerState::Stopped);
    assert!(rec.states().is_empty(), "no callback for a no-op pause");
}

/// AE-19: resume outside Paused is a no-op — no state change, no callback.
#[test]
fn ae19_resume_outside_paused_is_noop() {
    let engine = AudioEngine::new();
    let rec = attach_recorders(&engine);
    engine.resume();
    engine.resume();
    assert_eq!(engine.state(), PlayerState::Stopped);
    assert!(rec.states().is_empty(), "no callback for a no-op resume");
}

/// AE-20: with no known duration (DASH streams) the upper bound check is
/// skipped and any non-negative position is accepted.
#[test]
fn ae20_seek_with_unknown_duration_accepted() {
    let engine = AudioEngine::new();
    engine.seek(123.0).unwrap();
    engine.seek(0.0).unwrap();
    assert_eq!(engine.state(), PlayerState::Stopped);
}

/// AE-21: seek during pause must apply immediately — consume the pending
/// position, update `position()`, and fire a progress event, so resume
/// continues from the dragged-to spot.
#[test]
fn ae21_seek_while_paused_applies_immediately() {
    let engine = Arc::new(AudioEngine::new());
    let rec = attach_recorders(&engine);
    let probe = Arc::new(DecoderProbe::default());
    let fake = FakeDecoder::endless_paced(
        44100,
        2,
        10.0,
        Duration::from_millis(5),
        0.1,
    )
    .with_probe(probe.clone());
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    let e2 = engine.clone();
    let handle = thread::spawn(move || {
        e2.run_playback_with(
            "fake://paced".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();
    });
    wait_for("Playing state", Duration::from_secs(5), || {
        engine.state() == PlayerState::Playing
    });
    engine.pause();
    wait_for("Paused state", Duration::from_secs(5), || {
        engine.state() == PlayerState::Paused
    });

    // The seek call itself contends with the paused loop's mutex-holding
    // sleep (related to #77) and may block for a few cycles before even
    // queueing; the assertion below is about the queued seek being applied.
    engine.seek(0.75).unwrap();

    // Wait for the decoder seek *and* the position publish: the loop records
    // the seek into the probe before `set_position`, so checking seeks alone
    // could race the position assertion on a loaded CI box.
    wait_for("seek applied while paused", Duration::from_secs(2), || {
        !probe.seeks.lock().unwrap().is_empty() && (engine.position() - 0.75).abs() < 1e-9
    });
    assert_eq!(*probe.seeks.lock().unwrap(), vec![0.75]);
    assert!(
        rec.progress().iter().any(|&(pos, _)| (pos - 0.75).abs() < 1e-9),
        "progress callback must see the paused seek: {:?}",
        rec.progress()
    );

    engine.stop();
    handle.join().unwrap();
}

/// AE-22: with no callbacks registered, playback and failures neither crash
/// nor lose the queryable state.
#[test]
fn ae22_missing_callbacks_are_safe() {
    let engine = AudioEngine::new();
    let fake = FakeDecoder::new(44100, 2, 0.5)
        .packet(vec![0.1f32; 8], 0.1)
        .end();
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine
        .run_playback_with(
            "fake://src".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();
    assert_eq!(engine.state(), PlayerState::Finished);

    let engine2 = AudioEngine::new();
    let bad_sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine2
        .play_file_with(
            "fake://failing".to_string(),
            || Err::<(FakeDecoder, Option<f64>), _>(RhythmError::Decode("boom".to_string())),
            move |_| Ok(bad_sink),
        )
        .unwrap();
    wait_for("Error state", Duration::from_secs(5), || {
        matches!(engine2.state(), PlayerState::Error(_))
    });
}

/// AE-23: while paused the sink receives nothing; audio resumes after resume.
#[test]
fn ae23_paused_loop_produces_no_audio() {
    let engine = Arc::new(AudioEngine::new());
    let fake = FakeDecoder::endless_paced(
        44100,
        2,
        10.0,
        Duration::from_millis(10),
        0.1,
    );
    let probe = Arc::new(SinkProbe::default());
    let sink = FakeSink::new(44100, 2, probe.clone());

    let e2 = engine.clone();
    let handle = thread::spawn(move || {
        e2.run_playback_with(
            "fake://paced".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();
    });
    wait_for("audio flowing", Duration::from_secs(5), || {
        probe.write_count.load(Ordering::SeqCst) > 0
    });

    engine.pause();
    wait_for("Paused state", Duration::from_secs(5), || {
        engine.state() == PlayerState::Paused
    });
    // Settle any in-flight packet, then the loop must go quiet.
    thread::sleep(Duration::from_millis(100));
    let paused_count = probe.write_count.load(Ordering::SeqCst);
    thread::sleep(Duration::from_millis(150));
    assert_eq!(
        probe.write_count.load(Ordering::SeqCst),
        paused_count,
        "paused loop must not write audio"
    );

    // Like resume(): contends with the paused loop's mutex-holding sleep
    // (related to #77) — eventual, but not instant.
    engine.resume();
    wait_for("audio resumes", Duration::from_secs(5), || {
        probe.write_count.load(Ordering::SeqCst) > paused_count
    });

    engine.stop();
    handle.join().unwrap();
}

/// AE-24: a seek queued before stop is discarded — the old stream never
/// applies it, and a fresh playback starts clean.
#[test]
fn ae24_stop_clears_pending_seek() {
    let engine = Arc::new(AudioEngine::new());
    let probe = Arc::new(DecoderProbe::default());
    // Park the loop inside `next_packet` (a gate the test controls) so the
    // pending seek provably cannot be consumed before the stop lands. Not
    // done via pause: the paused branch of the product loop sleeps while
    // holding the engine mutex, so any engine call from this thread would
    // randomly block for many sleep cycles (related to #77).
    let (gate_tx, gate_rx) = std::sync::mpsc::channel();
    let fake = FakeDecoder::new(44100, 2, 10.0)
        .block_until_released(gate_rx)
        .packet(vec![0.1f32; 8], 0.1)
        .end()
        .with_probe(probe.clone());
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    let e2 = engine.clone();
    let handle = thread::spawn(move || {
        e2.run_playback_with(
            "fake://gated".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();
    });

    // The loop has already passed the pending-seek check for this iteration
    // and is parked in `next_packet`; it cannot consume the seek.
    wait_for("loop parked in next_packet", Duration::from_secs(5), || {
        probe.polls.load(Ordering::SeqCst) == 1
    });

    engine.seek(1.0).unwrap();
    engine.stop();
    gate_tx.send(()).unwrap(); // release; the loop sees the generation mismatch and exits
    handle.join().unwrap();
    assert!(
        probe.seeks.lock().unwrap().is_empty(),
        "seek queued before stop must not be applied"
    );

    // A fresh playback starts from its own beginning, not the stale target.
    let probe2 = Arc::new(DecoderProbe::default());
    let fake2 = FakeDecoder::new(44100, 2, 10.0)
        .packet(vec![0.1f32; 8], 0.1)
        .end()
        .with_probe(probe2.clone());
    let sink2 = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine
        .run_playback_with(
            "fake://fresh".to_string(),
            PlayerState::Playing,
            || Ok((fake2, None)),
            |_| Ok(sink2),
        )
        .unwrap();
    assert!(
        probe2.seeks.lock().unwrap().is_empty(),
        "stale seek must not leak into the next stream"
    );
    assert_eq!(engine.state(), PlayerState::Finished);
}

/// AE-25: volume set on the engine scales the samples the loop writes.
#[test]
fn ae25_volume_applies_in_loop() {
    let engine = AudioEngine::new();
    engine.set_volume(0.5);
    let fake = FakeDecoder::new(44100, 2, 0.5)
        .packet(vec![1.0f32; 8], 0.1)
        .end();
    let probe = Arc::new(SinkProbe::default());
    let sink = FakeSink::new(44100, 2, probe.clone());

    engine
        .run_playback_with(
            "fake://src".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap();

    assert_eq!(*probe.samples.lock().unwrap(), vec![0.5f32; 8]);
}

// ── 错误路径 (P2) ───────────────────────────────────────────────────────────

/// AE-26: a decoder that fails to open lands on Error (reported, not idle).
#[test]
fn ae26_decoder_open_failure_reports_error() {
    let engine = AudioEngine::new();
    let rec = attach_recorders(&engine);
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine
        .play_file_with(
            "fake://bad".to_string(),
            || Err::<(FakeDecoder, Option<f64>), _>(RhythmError::Decode(
                "injected open failure".to_string(),
            )),
            move |_| Ok(sink),
        )
        .unwrap();
    wait_for("Error state", Duration::from_secs(5), || {
        matches!(engine.state(), PlayerState::Error(_))
    });
    assert_error_reported(&engine, &rec);
}

/// AE-27: an output device that fails to initialize lands on Error.
#[test]
fn ae27_output_open_failure_reports_error() {
    let engine = AudioEngine::new();
    let rec = attach_recorders(&engine);
    let fake = FakeDecoder::new(44100, 2, 1.0);
    engine
        .play_file_with(
            "fake://src".to_string(),
            || Ok((fake, None)),
            |_| {
                Err::<FakeSink, _>(RhythmError::Output(
                    "injected sink open failure".to_string(),
                ))
            },
        )
        .unwrap();
    wait_for("Error state", Duration::from_secs(5), || {
        matches!(engine.state(), PlayerState::Error(_))
    });
    assert_error_reported(&engine, &rec);
}

/// AE-28: a URL that fails to resolve lands on Error.
#[test]
fn ae28_resolution_failure_reports_error() {
    let engine = AudioEngine::new_with_resolver(Arc::new(|_url| {
        Err(ResolveFailure {
            kind: ResolveErrorKind::Unavailable,
            message: "stub: video is private".to_string(),
        })
    }));
    let rec = attach_recorders(&engine);
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            |_resolved| Ok((FakeDecoder::new(44100, 2, 1.0), None)),
            move |_| Ok(sink),
        )
        .unwrap();
    wait_for("Error state", Duration::from_secs(5), || {
        matches!(engine.state(), PlayerState::Error(_))
    });
    assert_error_reported(&engine, &rec);
}

/// AE-29: an HTTP stream that fails to open / buffer lands on Error.
#[test]
fn ae29_stream_open_failure_reports_error() {
    // A resolved URL pointing at a dead port: the open must fail fast.
    let resolved = stub_resolved("http://127.0.0.1:1/dead.m4a", 42.0);
    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| Ok(resolved.clone())));
    let rec = attach_recorders(&engine);
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            |resolved| open_resolved_stream(&resolved),
            move |_| Ok(sink),
        )
        .unwrap();
    wait_for("Error state", Duration::from_secs(10), || {
        matches!(engine.state(), PlayerState::Error(_))
    });
    assert_error_reported(&engine, &rec);
}

/// AE-30: a decode error mid-stream terminates the loop and propagates
/// (the spawned playback path reports it as AE-15 does).
#[test]
fn ae30_mid_stream_decode_error_terminates_loop() {
    let engine = AudioEngine::new();
    let fake = FakeDecoder::new(44100, 2, 0.5)
        .packet(vec![0.1f32; 8], 0.1)
        .fail("injected decode error");
    let probe = Arc::new(SinkProbe::default());
    let sink = FakeSink::new(44100, 2, probe.clone());

    let err = engine
        .run_playback_with(
            "fake://src".to_string(),
            PlayerState::Playing,
            || Ok((fake, None)),
            |_| Ok(sink),
        )
        .unwrap_err();
    assert!(matches!(err, RhythmError::Decode(_)));
    assert!(
        !probe.drained.load(Ordering::SeqCst),
        "an aborted stream must not drain as if it ended naturally"
    );
}

/// AE-31 (#111): pause during Buffering must stick — the buffered stream
/// must not force Playing and push audio while the UI shows paused.
#[test]
fn ae31_pause_during_buffering_blocks_audio() {
    let hold_open = Arc::new(AtomicBool::new(true));
    let released = hold_open.clone();
    let resolved = stub_resolved("https://example.com/tone.wav", 42.0);
    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| {
        // Hold the resolve open so the engine sits in Buffering.
        while released.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(5));
        }
        Ok(resolved.clone())
    }));
    let rec = attach_recorders(&engine);
    let sink_probe = Arc::new(SinkProbe::default());
    let sink = FakeSink::new(44100, 2, sink_probe.clone());

    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            move |_resolved| {
                Ok((
                    FakeDecoder::endless_paced(44100, 2, 42.0, Duration::from_millis(5), 0.1),
                    None,
                ))
            },
            move |_| Ok(sink),
        )
        .unwrap();

    wait_for_state(&engine, &rec, PlayerState::Buffering, Duration::from_secs(2));

    // Pause while the stream is still opening.
    engine.pause();
    assert_eq!(
        engine.state(),
        PlayerState::Paused,
        "pause must stick in Buffering"
    );

    // Now the open completes: the engine must stay Paused and the sink must
    // receive nothing until resume.
    hold_open.store(false, Ordering::SeqCst);
    wait_for("source recorded", Duration::from_secs(2), || {
        engine.current_source().is_some()
    });
    thread::sleep(Duration::from_millis(200));
    assert_eq!(
        engine.state(),
        PlayerState::Paused,
        "the opened stream must not force Playing"
    );
    assert!(
        rec.states().contains(&PlayerState::Paused),
        "Paused must reach the callback: {:?}",
        rec.states()
    );
    assert!(
        !rec.states().contains(&PlayerState::Playing),
        "no Playing transition may fire while paused: {:?}",
        rec.states()
    );
    assert_eq!(
        sink_probe.write_count.load(Ordering::SeqCst),
        0,
        "no audio may reach the sink while paused"
    );

    // Resume: audio flows.
    engine.resume();
    wait_for("audio after resume", Duration::from_secs(5), || {
        sink_probe.write_count.load(Ordering::SeqCst) > 0
    });
    engine.stop();
}

// ── AE-38…AE-41 (#120): HTTP 403 recovery ────────────────────────────

/// #120: a CDN-rejected stream URL (403 on a still-valid link) triggers
/// exactly one re-resolve + retry of the open; playback recovers.
#[test]
fn ae38_http_403_retries_with_fresh_resolution_once() {
    let resolve_calls = Arc::new(AtomicUsize::new(0));
    let resolve_calls2 = resolve_calls.clone();
    let resolved = stub_resolved("http://127.0.0.1:1/unused.m4s", 0.0);
    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| {
        resolve_calls2.fetch_add(1, Ordering::SeqCst);
        Ok(resolved.clone())
    }));
    let rec = attach_recorders(&engine);

    let open_calls = Arc::new(AtomicUsize::new(0));
    let open_calls2 = open_calls.clone();
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));

    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            move |_resolved| {
                let n = open_calls2.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    // First attempt: the CDN rejects a still-valid URL — the
                    // exact #120 shape (expire far in the future, yet 403).
                    Err(RhythmError::Http(HttpError::from_status(
                        403,
                        "https://rr1---sn-55goxu-hxas.googlevideo.com/videoplayback?expire=9999999999&mt=1&ip=1.2.3.4",
                    )))
                } else {
                    Ok((FakeDecoder::new(44100, 2, 0.0).end(), None))
                }
            },
            move |_| Ok(sink),
        )
        .unwrap();

    wait_for_state_any(
        &engine,
        &rec,
        &[PlayerState::Finished],
        Duration::from_secs(5),
    );
    assert_eq!(
        resolve_calls.load(Ordering::SeqCst),
        2,
        "exactly one re-resolve after the 403"
    );
    assert_eq!(open_calls.load(Ordering::SeqCst), 2, "open retried once");
    assert_eq!(
        engine.last_error_kind(),
        None,
        "recovered — no failure may be recorded"
    );
}

/// #120: a genuinely expired link is re-resolved once too — the fresh
/// resolution returns a new (valid) URL and playback proceeds.
#[test]
fn ae39_expired_link_retries_with_fresh_resolution() {
    let resolve_calls = Arc::new(AtomicUsize::new(0));
    let resolve_calls2 = resolve_calls.clone();
    let resolved = stub_resolved("http://127.0.0.1:1/unused.m4s", 0.0);
    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| {
        resolve_calls2.fetch_add(1, Ordering::SeqCst);
        Ok(resolved.clone())
    }));
    let rec = attach_recorders(&engine);

    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            move |_resolved| {
                // expire=1 is long past → Expired.
                Err(RhythmError::Http(HttpError::from_status(
                    403,
                    "https://rr.example/videoplayback?expire=1&mt=0&ip=1.2.3.4",
                )))
            },
            move |_: &FakeDecoder| Ok(sink),
        )
        .unwrap();

    wait_for("Error state", Duration::from_secs(5), || {
        matches!(engine.state(), PlayerState::Error(_))
    });
    // The retry re-resolved but the open failed again (this stub never
    // succeeds) — the failure must surface with the Expired classification.
    assert_eq!(resolve_calls.load(Ordering::SeqCst), 2);
    assert_eq!(engine.last_error_kind(), Some(HttpErrorKind::Expired));
}

/// #120: non-HTTP failures do NOT re-resolve — the resolver runs exactly
/// once, and the error surfaces unclassified.
#[test]
fn ae40_non_http_failure_does_not_retry() {
    let resolve_calls = Arc::new(AtomicUsize::new(0));
    let resolve_calls2 = resolve_calls.clone();
    let resolved = stub_resolved("http://127.0.0.1:1/unused.m4s", 0.0);
    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| {
        resolve_calls2.fetch_add(1, Ordering::SeqCst);
        Ok(resolved.clone())
    }));
    let rec = attach_recorders(&engine);

    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            move |_resolved| Err(RhythmError::Network("connection refused".into())),
            move |_: &FakeDecoder| Ok(sink),
        )
        .unwrap();

    wait_for("Error state", Duration::from_secs(5), || {
        matches!(engine.state(), PlayerState::Error(_))
    });
    assert_eq!(resolve_calls.load(Ordering::SeqCst), 1, "no re-resolve");
    assert_eq!(engine.last_error_kind(), None, "not an HTTP failure");
}

/// #120: with the production recovery hooks wired, the evictor and the fresh
/// resolver run — and the cached resolver is NOT consulted again.
#[test]
fn ae41_recovery_hooks_evict_and_resolve_fresh() {
    let resolve_calls = Arc::new(AtomicUsize::new(0));
    let resolve_calls2 = resolve_calls.clone();
    let evict_calls = Arc::new(AtomicUsize::new(0));
    let evict_calls2 = evict_calls.clone();
    let fresh_calls = Arc::new(AtomicUsize::new(0));
    let fresh_calls2 = fresh_calls.clone();

    let resolved = stub_resolved("http://127.0.0.1:1/unused.m4s", 0.0);
    let fresh_resolved = stub_resolved("http://127.0.0.1:1/fresh.m4s", 0.0);

    let engine = AudioEngine::new_with_resolver(Arc::new(move |_url| {
        resolve_calls2.fetch_add(1, Ordering::SeqCst);
        Ok(resolved.clone())
    }))
    .with_recovery(
        Arc::new(move |_url| {
            fresh_calls2.fetch_add(1, Ordering::SeqCst);
            Ok(fresh_resolved.clone())
        }),
        Arc::new(move |_url| {
            evict_calls2.fetch_add(1, Ordering::SeqCst);
        }),
    );
    let rec = attach_recorders(&engine);

    let open_calls = Arc::new(AtomicUsize::new(0));
    let open_calls2 = open_calls.clone();
    let sink = FakeSink::new(44100, 2, Arc::new(SinkProbe::default()));
    engine
        .play_url_with(
            "https://example.com/watch?v=stub",
            move |_resolved| {
                let n = open_calls2.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Err(RhythmError::Http(HttpError::from_status(
                        403,
                        "https://rr.example/videoplayback?expire=9999999999&mt=1&ip=1.2.3.4",
                    )))
                } else {
                    // The fresh stream URL is the one that opens.
                    assert_eq!(_resolved.stream_url, "http://127.0.0.1:1/fresh.m4s");
                    Ok((FakeDecoder::new(44100, 2, 0.0).end(), None))
                }
            },
            move |_| Ok(sink),
        )
        .unwrap();

    wait_for_state_any(
        &engine,
        &rec,
        &[PlayerState::Finished],
        Duration::from_secs(5),
    );
    assert_eq!(resolve_calls.load(Ordering::SeqCst), 1, "cached resolver once");
    assert_eq!(evict_calls.load(Ordering::SeqCst), 1, "cache entry evicted");
    assert_eq!(fresh_calls.load(Ordering::SeqCst), 1, "fresh resolve once");
    assert_eq!(open_calls.load(Ordering::SeqCst), 2);
}

/// AE-42 (#134): a stale playback thread's failure must not clobber the newer
/// playback. Track A's URL open blocks (the tens-of-seconds resolve window);
/// track B starts and is audibly Playing; A's open then fails — the old
/// thread must NOT write Error over B's state, emit an Error callback, or
/// leak its HTTP classification into `last_error_kind()`.
#[test]
fn ae42_stale_failure_does_not_clobber_new_playback() {
    let (a_entered, a_entered_rx) = std::sync::mpsc::channel();
    let (a_release, a_release_rx) = std::sync::mpsc::channel();

    let resolved = stub_resolved("http://127.0.0.1:1/slow.m4s", 0.0);
    let engine = Arc::new(AudioEngine::new_with_resolver(Arc::new(move |_url| Ok(resolved.clone()))));
    let rec = attach_recorders(&engine);

    // Track A: the slow URL — its open blocks until released, then fails with
    // a non-retryable HTTP error (500 → `Other`, no #120 re-resolve).
    let e1 = engine.clone();
    let a_handle = thread::spawn(move || {
        e1.play_url_with(
            "https://example.com/slow-a",
            move |_resolved| {
                a_entered.send(()).unwrap();
                a_release_rx.recv().unwrap();
                Err(RhythmError::Http(HttpError::from_status(
                    500,
                    "https://rr.example/videoplayback",
                )))
            },
            |_: &FakeDecoder| -> RhythmResult<FakeSink> {
                panic!("stale A must never open a sink");
            },
        )
        .unwrap();
    });
    a_entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    // Track B: normal playback on the same engine — bumps the generation.
    let b_sink_probe = Arc::new(SinkProbe::default());
    let b_sink = FakeSink::new(44100, 2, b_sink_probe.clone());
    engine
        .play_url_with(
            "https://example.com/fast-b",
            move |_resolved| Ok((FakeDecoder::endless_paced(44100, 2, 10.0, Duration::from_millis(5), 0.1), None)),
            move |_| Ok(b_sink),
        )
        .unwrap();
    wait_for("B flowing", Duration::from_secs(5), || {
        b_sink_probe.write_count.load(Ordering::SeqCst) > 0
    });
    assert_eq!(engine.state(), PlayerState::Playing);

    // Release A's slow open — it fails. The stale thread must not touch B.
    a_release.send(()).unwrap();
    a_handle.join().unwrap();
    thread::sleep(Duration::from_millis(300));

    assert_eq!(
        engine.state(),
        PlayerState::Playing,
        "stale A's failure must not clobber B's Playing state"
    );
    assert!(
        !rec.states()
            .iter()
            .any(|s| matches!(s, PlayerState::Error(_))),
        "stale A's failure must not emit an Error callback: {:?}",
        rec.states()
    );
    assert_eq!(
        engine.last_error_kind(),
        None,
        "stale A's HTTP classification must not leak into B's playback"
    );

    // B is still audibly flowing after A's stale failure.
    let writes_after = b_sink_probe.write_count.load(Ordering::SeqCst);
    wait_for("B still flowing after A failed", Duration::from_secs(5), || {
        b_sink_probe.write_count.load(Ordering::SeqCst) > writes_after
    });
}

/// AE-43 (#134): a stale playback thread whose slow open eventually SUCCEEDS
/// must not claim the source, announce Playing, or open a sink — the newer
/// playback owns all of those. Tracks A and B are both URL playbacks
/// (Buffering); A's open finishes while B is still resolving.
#[test]
fn ae43_stale_success_does_not_touch_new_playback() {
    let (a_entered, a_entered_rx) = std::sync::mpsc::channel();
    let (a_release, a_release_rx) = std::sync::mpsc::channel();
    let (b_entered, b_entered_rx) = std::sync::mpsc::channel();
    let (b_release, b_release_rx) = std::sync::mpsc::channel();

    let resolved = stub_resolved("http://127.0.0.1:1/slow.m4s", 0.0);
    let engine = Arc::new(AudioEngine::new_with_resolver(Arc::new(move |_url| Ok(resolved.clone()))));

    let a_sink_opens = Arc::new(AtomicUsize::new(0));
    let a_sink_opens2 = a_sink_opens.clone();
    let e1 = engine.clone();
    let a_handle = thread::spawn(move || {
        e1.play_url_with(
            "https://example.com/slow-a",
            move |_resolved| {
                a_entered.send(()).unwrap();
                a_release_rx.recv().unwrap();
                Ok((FakeDecoder::new(44100, 2, 999.0).end(), None))
            },
            move |_: &FakeDecoder| {
                a_sink_opens2.fetch_add(1, Ordering::SeqCst);
                Ok(FakeSink::new(44100, 2, Arc::new(SinkProbe::default())))
            },
        )
        .unwrap();
    });
    a_entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    let b_sink_probe = Arc::new(SinkProbe::default());
    let b_sink = FakeSink::new(44100, 2, b_sink_probe.clone());
    let e2 = engine.clone();
    let b_handle = thread::spawn(move || {
        e2.play_url_with(
            "https://example.com/slow-b",
            move |_resolved| {
                b_entered.send(()).unwrap();
                b_release_rx.recv().unwrap();
                Ok((
                    FakeDecoder::endless_paced(
                        44100,
                        2,
                        10.0,
                        Duration::from_millis(5),
                        0.1,
                    ),
                    None,
                ))
            },
            move |_| Ok(b_sink),
        )
        .unwrap();
    });
    b_entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(engine.state(), PlayerState::Buffering);

    // A's open succeeds first — but A is stale (B bumped the generation).
    a_release.send(()).unwrap();
    a_handle.join().unwrap();
    thread::sleep(Duration::from_millis(300));

    assert_eq!(
        engine.state(),
        PlayerState::Buffering,
        "stale A must not announce Playing over B's still-resolving state"
    );
    assert_eq!(
        engine.current_source(),
        None,
        "stale A must not claim the source while B is resolving"
    );
    assert_eq!(
        a_sink_opens.load(Ordering::SeqCst),
        0,
        "stale A must not open a sink"
    );

    // B's open completes: it owns Playing, the source, and the output.
    b_release.send(()).unwrap();
    b_handle.join().unwrap();
    wait_for("B playing", Duration::from_secs(5), || {
        engine.state() == PlayerState::Playing
    });
    assert_eq!(
        engine.current_source(),
        Some("https://example.com/slow-b".to_string()),
        "B must own the source"
    );
    assert_eq!(engine.duration(), 10.0, "B's duration must survive");
    wait_for("B flowing", Duration::from_secs(5), || {
        b_sink_probe.write_count.load(Ordering::SeqCst) > 0
    });
    assert_eq!(
        a_sink_opens.load(Ordering::SeqCst),
        0,
        "stale A must never open a sink"
    );
}


