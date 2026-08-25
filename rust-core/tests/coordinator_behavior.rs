//! Behavior tests for the playback coordinator
//! (docs/testing/behavior/coordinator.md).
//!
//! The coordinator is the seam where the macOS/Windows orchestration rules
//! (stop old playback, dispatch by source type, record plays, queue build +
//! positioning, bounded skip of unplayable tracks) converge into one place
//! (parent issue #165). Tests drive the coordinator interface with a
//! call-recording fake player surface — no audio device required.

use rhythm_core::coordinator::{
    CoordinatorErrorKind, CoordinatorEvent, PlaybackCoordinator, PlayerSurface,
};
use rhythm_core::library::Library;
use rhythm_core::queue::PlayMode;
use rhythm_core::{
    HttpErrorKind, PlayerState, ProgressCallback, RhythmResult, SourceType, StateCallback, TrackInfo,
};
use std::ffi::CString;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// ─── Fake player surface ──────────────────────────────────────────

/// Call-recording fake: records every engine call so tests can assert the
/// exact orchestration sequence (e.g. `stop` before `play_file` — #51).
struct FakePlayer {
    calls: CallLog,
    state: EngineState,
    fail_play_file: AtomicBool,
    fail_play_url: AtomicBool,
    error_kind: Mutex<Option<HttpErrorKind>>,
    /// Shared event bus: the coordinator registers its engine callbacks
    /// here, and tests fire engine transitions through it.
    bus: Arc<FakeEventBus>,
}

/// The engine's event side, shared with the test: `fire_state` /
/// `fire_progress` simulate what the real engine's callbacks deliver.
struct FakeEventBus {
    state_cb: Mutex<Option<StateCallback>>,
    progress_cb: Mutex<Option<ProgressCallback>>,
}

impl FakePlayer {
    /// Returns the player plus shared handles for the call log, the
    /// engine-mirror state, and the event bus.
    fn new() -> (FakePlayer, CallLog, EngineState, Arc<FakeEventBus>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let state = Arc::new(Mutex::new(PlayerState::Stopped));
        let bus = Arc::new(FakeEventBus {
            state_cb: Mutex::new(None),
            progress_cb: Mutex::new(None),
        });
        let player = FakePlayer {
            calls: calls.clone(),
            state: state.clone(),
            fail_play_file: AtomicBool::new(false),
            fail_play_url: AtomicBool::new(false),
            error_kind: Mutex::new(None),
            bus: bus.clone(),
        };
        (player, calls, state, bus)
    }

    fn record(&self, call: &str) {
        self.calls.lock().unwrap().push(call.to_string());
    }
}

impl FakeEventBus {
    /// Test helper: fire an engine state transition through the registered
    /// callback (the coordinator forwards it as an event).
    fn fire_state(&self, state: PlayerState) {
        if let Some(cb) = self.state_cb.lock().unwrap().as_ref() {
            cb(state);
        }
    }

    /// Test helper: fire a progress update through the registered callback.
    fn fire_progress(&self, position: f64, duration: f64) {
        if let Some(cb) = self.progress_cb.lock().unwrap().as_ref() {
            cb(position, duration);
        }
    }
}

impl PlayerSurface for FakePlayer {
    fn play_file(&self, path: &Path) -> RhythmResult<()> {
        self.record(&format!("play_file:{}", path.display()));
        if self.fail_play_file.load(Ordering::SeqCst) {
            return Err(rhythm_core::RhythmError::FileNotFound(path.display().to_string()));
        }
        *self.state.lock().unwrap() = PlayerState::Playing;
        Ok(())
    }

    fn play_url(&self, url: &str) -> RhythmResult<()> {
        self.record(&format!("play_url:{url}"));
        if self.fail_play_url.load(Ordering::SeqCst) {
            return Err(rhythm_core::RhythmError::Network(url.to_string()));
        }
        *self.state.lock().unwrap() = PlayerState::Playing;
        Ok(())
    }

    fn pause(&self) {
        self.record("pause");
        *self.state.lock().unwrap() = PlayerState::Paused;
    }

    fn resume(&self) {
        self.record("resume");
        *self.state.lock().unwrap() = PlayerState::Playing;
    }

    fn stop(&self) {
        self.record("stop");
        *self.state.lock().unwrap() = PlayerState::Stopped;
    }

    fn seek(&self, seconds: f64) -> RhythmResult<()> {
        self.record(&format!("seek:{seconds}"));
        Ok(())
    }

    fn set_volume(&self, volume: f32) {
        self.record(&format!("set_volume:{volume}"));
    }

    fn volume(&self) -> f32 {
        1.0
    }

    fn state(&self) -> PlayerState {
        self.state.lock().unwrap().clone()
    }

    fn position(&self) -> f64 {
        0.0
    }

    fn duration(&self) -> f64 {
        0.0
    }

    fn error_message(&self) -> Option<String> {
        match self.state() {
            PlayerState::Error(message) => Some(message),
            _ => None,
        }
    }

    fn error_kind(&self) -> Option<HttpErrorKind> {
        *self.error_kind.lock().unwrap()
    }

    fn on_state_change(&self, callback: StateCallback) {
        *self.bus.state_cb.lock().unwrap() = Some(callback);
    }

    fn on_progress(&self, callback: ProgressCallback) {
        *self.bus.progress_cb.lock().unwrap() = Some(callback);
    }
}

/// Shared call log handle.
type CallLog = Arc<Mutex<Vec<String>>>;
/// Shared engine-mirror state handle (tests force states like Paused).
type EngineState = Arc<Mutex<PlayerState>>;

fn dummy_track(id: i64, title: &str) -> TrackInfo {
    TrackInfo {
        id: Some(id),
        file_path: Some(format!("/music/{title}.mp3")),
        source_type: SourceType::Local,
        source_url: None,
        title: title.to_string(),
        artist: Some("Test Artist".to_string()),
        album: Some("Test Album".to_string()),
        album_artist: None,
        track_number: Some(1),
        disc_number: Some(1),
        genre: None,
        year: Some(2024),
        duration: 180.0,
        format: Some("mp3".to_string()),
        bitrate: Some(320),
        sample_rate: Some(44100),
        channels: Some(2),
        file_size: Some(5000000),
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: None,
        is_available: true,
    }
}

/// A track with no playable location (no path, no URL).
fn unplayable_track(id: i64, title: &str) -> TrackInfo {
    let mut t = dummy_track(id, title);
    t.file_path = None;
    t.source_url = None;
    t
}

fn url_track(id: i64, title: &str, url: &str) -> TrackInfo {
    let mut t = dummy_track(id, title);
    t.source_type = SourceType::DirectUrl;
    t.file_path = None;
    t.source_url = Some(url.to_string());
    t
}

fn make_coordinator(player: FakePlayer) -> PlaybackCoordinator {
    PlaybackCoordinator::with_player(Box::new(player))
}

// ─── CO-01: start local track ─────────────────────────────────────

#[test]
fn co01_start_local_stops_then_plays_and_positions_queue() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let track = dummy_track(1, "A");
    let queue = vec![dummy_track(1, "A"), dummy_track(2, "B")];
    let result = coord.start(None, track, queue, PlayMode::Sequential);

    assert!(result.ok, "start must succeed");
    assert_eq!(
        *calls.lock().unwrap(),
        vec!["stop".to_string(), "play_file:/music/A.mp3".to_string()],
        "#51: stop old playback before dispatching the new track"
    );
    assert_eq!(coord.current_track().map(|t| t.id), Some(Some(1)));
    assert!(coord.can_play_next(), "queue [A, B] positioned at A has a next");
    assert!(!coord.can_play_previous(), "queue positioned at the head has no previous");
}

// ─── CO-02: start URL track ───────────────────────────────────────

#[test]
fn co02_start_url_dispatches_play_url() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let track = url_track(1, "Stream", "https://cdn.example.com/x.mp3");
    let result = coord.start(None, track.clone(), vec![track], PlayMode::Sequential);

    assert!(result.ok);
    let calls = calls.lock().unwrap();
    assert_eq!(calls[0], "stop");
    assert_eq!(calls[1], "play_url:https://cdn.example.com/x.mp3");
    assert!(!calls.iter().any(|c| c.starts_with("play_file:")), "URL track must not hit play_file");
}

// ─── CO-03: no playable location ──────────────────────────────────

#[test]
fn co03_start_without_playable_location_is_classified_error() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let track = unplayable_track(1, "Dead");
    let result = coord.start(None, track.clone(), vec![track], PlayMode::Sequential);

    assert!(!result.ok);
    assert_eq!(result.error_kind, Some(CoordinatorErrorKind::NoPlayableLocation));
    assert!(
        calls.lock().unwrap().is_empty(),
        "#78/#81 guard: nothing must reach the player"
    );
    assert!(coord.current_track().is_none(), "no playing state without a location");
    assert!(!coord.can_play_next(), "no queue established");
}

// ─── CO-04: empty strings count as missing ────────────────────────

#[test]
fn co04_empty_path_or_url_counts_as_missing() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let mut local = dummy_track(1, "A");
    local.file_path = Some(String::new());
    let result = coord.start(None, local, vec![], PlayMode::Sequential);
    assert!(!result.ok, "empty file path must be rejected");

    let mut stream = url_track(2, "B", "https://example.com/b.mp3");
    stream.source_url = Some(String::new());
    let result = coord.start(None, stream, vec![], PlayMode::Sequential);
    assert!(!result.ok, "empty source URL must be rejected");
    assert!(calls.lock().unwrap().is_empty());
}

// ─── CO-05: recordPlay reaches the database ───────────────────────

#[test]
fn co05_start_records_play_in_library() {
    let (player, _, _, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let lib = Library::open(&db).unwrap();
    let saved = lib.add_track(&dummy_track(1, "A")).unwrap();
    assert_eq!(saved.play_count, 0);

    let result = coord.start(Some(&lib), saved.clone(), vec![saved], PlayMode::Sequential);
    assert!(result.ok);

    let tracks = lib.get_all_tracks().unwrap();
    assert_eq!(tracks[0].play_count, 1, "recordPlay(id) must reach the DB");
}

// ─── CO-06/07: queue build, positioning, and mode ─────────────────

#[test]
fn co06_start_positions_queue_at_track_and_obeys_mode() {
    let (player, _, _, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    // Positioned at the last track, Sequential has no next.
    let last = dummy_track(3, "C");
    let queue = vec![dummy_track(1, "A"), dummy_track(2, "B"), last.clone()];
    let result = coord.start(None, last, queue, PlayMode::Sequential);
    assert!(result.ok);
    assert!(!coord.can_play_next(), "sequential queue at the last track has no next");
    assert!(coord.can_play_previous());

    // Same position, ListLoop mode: next always exists.
    let last = dummy_track(3, "C");
    let queue = vec![dummy_track(1, "A"), dummy_track(2, "B"), last.clone()];
    let result = coord.start(None, last, queue, PlayMode::ListLoop);
    assert!(result.ok);
    assert!(coord.can_play_next(), "loop mode always has a next");
}

// ─── CO-08: immediate engine failure ──────────────────────────────

#[test]
fn co08_start_reports_immediate_engine_failure() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    player.fail_play_file.store(true, Ordering::SeqCst);
    let mut coord = make_coordinator(player);

    let track = dummy_track(1, "Missing");
    let result = coord.start(None, track, vec![], PlayMode::Sequential);

    assert!(!result.ok);
    assert_eq!(result.error_kind, Some(CoordinatorErrorKind::PlaybackFailed));
    assert_eq!(calls.lock().unwrap()[0], "stop", "stop must still run before the doomed play");
}

// ─── CO-09: next advances and stops first ─────────────────────────

#[test]
fn co09_next_advances_and_stops_before_play() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    let b = dummy_track(2, "B");
    let c = dummy_track(3, "C");
    let queue = vec![a.clone(), b.clone(), c.clone()];
    assert!(coord.start(None, a, queue, PlayMode::Sequential).ok);

    let result = coord.next(None);
    assert!(result.ok);
    assert_eq!(result.current_track.map(|t| t.title), Some("B".to_string()));
    assert_eq!(coord.current_track().map(|t| t.id), Some(Some(2)));
    let calls = calls.lock().unwrap();
    assert_eq!(calls[calls.len() - 2], "stop", "#51: stop before the next dispatch");
    assert_eq!(calls[calls.len() - 1], "play_file:/music/B.mp3");
}

// ─── CO-10: next skips unplayable tracks (bounded) ────────────────

#[test]
fn co10_next_skips_unplayable_tracks() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    let dead = unplayable_track(2, "Dead");
    let c = dummy_track(3, "C");
    let queue = vec![a.clone(), dead, c.clone()];
    assert!(coord.start(None, a, queue, PlayMode::Sequential).ok);

    let result = coord.next(None);
    assert!(result.ok);
    assert_eq!(
        result.current_track.map(|t| t.title),
        Some("C".to_string()),
        "#78: skip the unplayable track and land on the next playable one"
    );
    let calls = calls.lock().unwrap();
    assert!(calls.iter().any(|c| c == "play_file:/music/C.mp3"));
}

// ─── CO-11: next with all remaining unplayable → no-op ────────────

#[test]
fn co11_next_all_unplayable_gives_up_without_touching_state() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    let dead = unplayable_track(2, "Dead");
    let queue = vec![a.clone(), dead];
    assert!(coord.start(None, a, queue, PlayMode::Sequential).ok);
    calls.lock().unwrap().clear();

    let result = coord.next(None);
    assert!(result.ok, "giving up is not a failure");
    assert_eq!(
        result.current_track.map(|t| t.id),
        Some(Some(1)),
        "the current track keeps playing"
    );
    assert_eq!(coord.current_track().map(|t| t.id), Some(Some(1)));
    assert!(calls.lock().unwrap().is_empty(), "no engine calls when giving up");
}

// ─── CO-12: next at the end of the queue → no-op ──────────────────

#[test]
fn co12_next_exhausted_queue_is_no_op() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    let queue = vec![a.clone()];
    assert!(coord.start(None, a, queue, PlayMode::Sequential).ok);
    calls.lock().unwrap().clear();

    let result = coord.next(None);
    assert!(result.ok);
    assert_eq!(coord.current_track().map(|t| t.id), Some(Some(1)));
    assert!(calls.lock().unwrap().is_empty());
}

// ─── CO-13: previous walks backwards ──────────────────────────────

#[test]
fn co13_previous_walks_backwards_and_stops_at_head() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    let b = dummy_track(2, "B");
    let c = dummy_track(3, "C");
    let queue = vec![a.clone(), b.clone(), c.clone()];
    assert!(coord.start(None, c.clone(), queue, PlayMode::Sequential).ok);
    calls.lock().unwrap().clear();

    let result = coord.previous(None);
    assert!(result.ok);
    assert_eq!(result.current_track.map(|t| t.title), Some("B".to_string()));

    let result = coord.previous(None);
    assert!(result.ok);
    assert_eq!(result.current_track.map(|t| t.title), Some("A".to_string()));

    // Sequential at the head: `previous` returns the current track again, so
    // it is re-dispatched (the queue's own semantics — the old UI behaved the
    // same way); the current track id does not change.
    let result = coord.previous(None);
    assert!(result.ok);
    assert_eq!(coord.current_track().map(|t| t.id), Some(Some(1)));
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 6, "two real dispatches (B, A) plus the head replay (A)");
    assert_eq!(calls[calls.len() - 1], "play_file:/music/A.mp3");
}

// ─── CO-14: transport on a fresh coordinator → no-op ──────────────

#[test]
fn co14_next_without_queue_is_no_op() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let result = coord.next(None);
    assert!(result.ok);
    assert!(result.current_track.is_none());
    assert!(calls.lock().unwrap().is_empty());
}

// ─── CO-15: sync_queue after refresh ──────────────────────────────

#[test]
fn co15_sync_queue_replaces_and_jumps_back_to_current() {
    let (player, _, _, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    let b = dummy_track(2, "B");
    let c = dummy_track(3, "C");
    let queue = vec![a.clone(), b.clone(), c.clone()];
    assert!(coord.start(None, b.clone(), queue, PlayMode::Sequential).ok);

    // Refresh adds D at the end; the queue must keep the current track in
    // place (#69).
    let d = dummy_track(4, "D");
    coord.sync_queue(vec![a.clone(), b.clone(), c.clone(), d.clone()]);
    assert!(coord.can_play_next(), "B still has C and D ahead");
    assert!(coord.can_play_previous(), "B still has A behind");

    // Refresh deletes the current track: replace resets to the new head
    // (jumpTo cannot find the id anymore).
    coord.sync_queue(vec![c.clone(), d]);
    assert_eq!(coord.current_track().map(|t| t.id), Some(Some(2)), "UI-facing current track unchanged");
    assert!(!coord.can_play_previous(), "queue head is the first new track");
}

// ─── CO-16: stop clears transport state ───────────────────────────

#[test]
fn co16_stop_clears_current_and_queue() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    let b = dummy_track(2, "B");
    let queue = vec![a.clone(), b.clone()];
    assert!(coord.start(None, a, queue, PlayMode::Sequential).ok);

    coord.stop();
    assert!(coord.current_track().is_none());
    assert!(!coord.can_play_next());
    assert!(!coord.can_play_previous());
    let calls = calls.lock().unwrap();
    assert_eq!(calls[calls.len() - 1], "stop", "engine stop must be the last call");
}

// ─── CO-17: set_play_mode affects the queue ───────────────────────

#[test]
fn co17_set_play_mode_follows_into_queue() {
    let (player, _, _, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    let b = dummy_track(2, "B");
    let queue = vec![a.clone(), b.clone()];
    assert!(coord.start(None, b.clone(), queue, PlayMode::Sequential).ok);
    assert!(!coord.can_play_next(), "sequential at the last track");

    coord.set_play_mode(PlayMode::ListLoop);
    assert!(coord.can_play_next(), "list loop always has a next");
    assert_eq!(coord.play_mode(), PlayMode::ListLoop);
}

// ─── FFI round-trip (guard path — no engine needed) ───────────────

#[test]
fn ffi_coordinator_start_rejects_unplayable_track_with_structured_result() {
    use rhythm_core::ffi::*;

    let coord = rhythm_coordinator_create();
    assert!(!coord.is_null());

    let track_json = CString::new(serde_json::to_string(&unplayable_track(1, "Dead")).unwrap()).unwrap();
    let queue_json = CString::new("[]").unwrap();
    let result = unsafe {
        rhythm_coordinator_start(
            coord,
            std::ptr::null_mut(),
            track_json.as_ptr(),
            queue_json.as_ptr(),
            0,
        )
    };
    assert!(!result.is_null());
    let json = unsafe { std::ffi::CStr::from_ptr(result) }
        .to_str()
        .unwrap()
        .to_string();
    rhythm_free_string(result);

    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error_kind"], "no_playable_location");
    assert!(value["current_track"].is_null() || value.get("current_track").is_none());

    unsafe { rhythm_coordinator_destroy(coord) };
}

#[test]
fn ffi_coordinator_null_handle_and_fresh_state() {
    use rhythm_core::ffi::*;

    // Null handle → structured error, not a crash.
    let track_json = CString::new("{}").unwrap();
    let queue_json = CString::new("[]").unwrap();
    let result = unsafe {
        rhythm_coordinator_start(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            track_json.as_ptr(),
            queue_json.as_ptr(),
            0,
        )
    };
    let json = unsafe { std::ffi::CStr::from_ptr(result) }.to_str().unwrap().to_string();
    rhythm_free_string(result);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["ok"], false);
    assert_eq!(value["error_kind"], "invalid_input");

    // Fresh coordinator: no current track, no queue, mode defaults.
    let coord = rhythm_coordinator_create();
    assert_eq!(unsafe { rhythm_coordinator_has_next(coord) }, 0);
    assert_eq!(unsafe { rhythm_coordinator_has_previous(coord) }, 0);
    assert!(unsafe { rhythm_coordinator_current_track(coord) }.is_null());
    assert_eq!(unsafe { rhythm_coordinator_get_play_mode(coord) }, 0);
    assert_eq!(unsafe { rhythm_coordinator_get_state(coord) }, 0, "fresh → Stopped");
    assert!(unsafe { rhythm_coordinator_error(coord) }.is_null());
    unsafe { rhythm_coordinator_destroy(coord) };
}

#[test]
fn ffi_coordinator_sync_queue_and_mode_roundtrip() {
    use rhythm_core::ffi::*;

    let coord = rhythm_coordinator_create();
    // Invalid JSON must be a safe no-op.
    unsafe {
        let bad_json = CString::new("not json").unwrap();
        rhythm_coordinator_sync_queue(coord, bad_json.as_ptr());
        rhythm_coordinator_set_play_mode(coord, 3);
    }
    assert_eq!(unsafe { rhythm_coordinator_get_play_mode(coord) }, 3);
    unsafe { rhythm_coordinator_destroy(coord) };
}
// ─── CO-21: toggle pause / resume ─────────────────────────────────

#[test]
fn co21_toggle_pauses_while_playing_or_buffering() {
    let (player, calls, state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    assert!(coord.start(None, a.clone(), vec![a], PlayMode::Sequential).ok);
    calls.lock().unwrap().clear();

    let result = coord.toggle_play_pause(None);
    assert!(result.ok);
    assert!(!result.playback_active, "pause must report playback inactive");
    assert_eq!(calls.lock().unwrap()[0], "pause");

    // #111: pause during Buffering must stick.
    coord.player().stop();
    calls.lock().unwrap().clear();
    *state.lock().unwrap() = PlayerState::Buffering;
    let result = coord.toggle_play_pause(None);
    assert!(!result.playback_active);
    assert_eq!(calls.lock().unwrap()[0], "pause");
}

#[test]
fn co22_toggle_resumes_only_when_paused() {
    let (player, calls, state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let a = dummy_track(1, "A");
    assert!(coord.start(None, a.clone(), vec![a], PlayMode::Sequential).ok);
    calls.lock().unwrap().clear();

    // Paused → resume.
    *state.lock().unwrap() = PlayerState::Paused;
    let result = coord.toggle_play_pause(None);
    assert!(result.ok);
    assert!(result.playback_active, "resume must report playback active");
    assert_eq!(calls.lock().unwrap()[0], "resume");

    // Error → no-op (#111: only Paused may resume).
    calls.lock().unwrap().clear();
    *state.lock().unwrap() = PlayerState::Error("boom".into());
    let result = coord.toggle_play_pause(None);
    assert!(result.ok);
    assert!(!result.playback_active);
    assert!(calls.lock().unwrap().is_empty(), "no resume outside Paused");

    // Finished → no-op, playback inactive (the UI stops claiming playback).
    calls.lock().unwrap().clear();
    *state.lock().unwrap() = PlayerState::Finished;
    let result = coord.toggle_play_pause(None);
    assert!(result.ok);
    assert!(!result.playback_active);
    assert!(calls.lock().unwrap().is_empty());
}

// ─── CO-23: toggle idle-start ─────────────────────────────────────

#[test]
fn co23_toggle_idle_starts_first_playable_library_track() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let playable = dummy_track(1, "A");
    let dead = unplayable_track(2, "Dead");
    let also_playable = dummy_track(3, "C");
    // Library mirror is fed by sync_queue (the UI refreshes on open).
    coord.sync_queue(vec![playable.clone(), dead, also_playable.clone()]);

    let result = coord.toggle_play_pause(None);
    assert!(result.ok);
    assert!(result.playback_active);
    assert_eq!(result.current_track.map(|t| t.title), Some("A".to_string()));
    assert_eq!(coord.current_track().map(|t| t.id), Some(Some(1)));
    assert!(coord.can_play_next(), "queue = library mirror, positioned at A");

    let calls = calls.lock().unwrap();
    assert_eq!(calls[0], "stop", "#51: stop before the idle start dispatch");
    assert_eq!(calls[1], "play_file:/music/A.mp3");
}

#[test]
fn co24_toggle_empty_library_is_no_op() {
    let (player, calls, _state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let result = coord.toggle_play_pause(None);
    assert!(result.ok);
    assert!(!result.playback_active);
    assert!(result.current_track.is_none());
    assert!(calls.lock().unwrap().is_empty());
}

// ─── CO-25: availability exports ──────────────────────────────────

#[test]
fn co25_availability_matrix() {
    let (player, _, state, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    // Fresh: nothing to toggle, nothing to stop.
    assert!(!coord.can_toggle_playback());
    assert!(!coord.can_stop());

    // Library mirror only → toggle can start, still nothing to stop.
    coord.sync_queue(vec![dummy_track(1, "A")]);
    assert!(coord.can_toggle_playback());
    assert!(!coord.can_stop());

    // Started → both available.
    let a = dummy_track(1, "A");
    assert!(coord.start(None, a.clone(), vec![a], PlayMode::Sequential).ok);
    assert!(coord.can_toggle_playback());
    assert!(coord.can_stop());

    // Paused → still stoppable.
    *state.lock().unwrap() = PlayerState::Paused;
    assert!(coord.can_stop());

    // Stopped → nothing to stop.
    coord.stop();
    assert!(!coord.can_stop());
    assert!(coord.can_toggle_playback(), "library mirror still present");

    // Stopped with an empty library → nothing at all.
    let mut empty = coord;
    empty.sync_queue(vec![]);
    assert!(!empty.can_toggle_playback());
}

// ─── CO-26..31: event channel (ticket #172) ───────────────────────

/// Subscribes a recording event sink to the coordinator.
fn subscribe(coord: &PlaybackCoordinator) -> Arc<Mutex<Vec<CoordinatorEvent>>> {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    coord.set_event_callback(Arc::new(move |event| sink.lock().unwrap().push(event)));
    events
}

#[test]
fn co26_finished_auto_advances_and_emits_track_changed() {
    let (player, calls, _, bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);
    let events = subscribe(&coord);

    let a = dummy_track(1, "A");
    let b = dummy_track(2, "B");
    assert!(coord.start(None, a.clone(), vec![a.clone(), b.clone()], PlayMode::Sequential).ok);
    calls.lock().unwrap().clear();

    // Engine reports the track ended naturally (the event goes through the
    // engine state callback), then the dispatcher calls handle_finished.
    bus.fire_state(PlayerState::Finished);
    coord.handle_finished(None);

    assert_eq!(
        coord.current_track().map(|t| t.id),
        Some(Some(2)),
        "core-driven auto-advance (#165 decision)"
    );
    let calls = calls.lock().unwrap();
    assert_eq!(calls[calls.len() - 1], "play_file:/music/B.mp3");

    let events = events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(e, CoordinatorEvent::Finished)),
        "Finished event must be published"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CoordinatorEvent::TrackChanged { track } if track.id == Some(2))),
        "TrackChanged must follow the auto-advance"
    );
}

#[test]
fn co27_finished_without_next_leaves_playback_ended() {
    let (player, calls, _, bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);
    let events = subscribe(&coord);

    let a = dummy_track(1, "A");
    assert!(coord.start(None, a.clone(), vec![a], PlayMode::Sequential).ok);
    calls.lock().unwrap().clear();

    let before = events.lock().unwrap().len();
    bus.fire_state(PlayerState::Finished);
    coord.handle_finished(None);

    assert_eq!(coord.current_track().map(|t| t.id), Some(Some(1)));
    assert!(calls.lock().unwrap().is_empty(), "no engine calls at queue end");
    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(e, CoordinatorEvent::Finished)));
    assert!(
        !events[before..]
            .iter()
            .any(|e| matches!(e, CoordinatorEvent::TrackChanged { .. })),
        "no track change when the queue is exhausted"
    );
}

#[test]
fn co28_error_state_publishes_error_event() {
    let (player, _, _, bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);
    let events = subscribe(&coord);

    let a = dummy_track(1, "A");
    assert!(coord.start(None, a.clone(), vec![a], PlayMode::Sequential).ok);

    // The engine's state callback delivers the failure.
    bus.fire_state(PlayerState::Error("GET x failed: HTTP 403".into()));

    let events = events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CoordinatorEvent::Error { message, .. } if message == "GET x failed: HTTP 403")),
        "the failure message must be published as an Error event"
    );
}

#[test]
fn co29_progress_events_are_forwarded() {
    let (player, _, _, bus) = FakePlayer::new();
    let coord = make_coordinator(player);
    let events = subscribe(&coord);

    bus.fire_progress(12.5, 180.0);

    let events = events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(
            e,
            CoordinatorEvent::Progress { position, duration } if *position == 12.5 && *duration == 180.0
        )),
        "progress must be forwarded as an event"
    );
}

#[test]
fn co30_state_events_are_forwarded() {
    let (player, _, _, bus) = FakePlayer::new();
    let coord = make_coordinator(player);
    let events = subscribe(&coord);

    bus.fire_state(PlayerState::Playing);
    bus.fire_state(PlayerState::Paused);

    let events = events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(e, CoordinatorEvent::State { state: PlayerState::Playing })));
    assert!(events.iter().any(|e| matches!(e, CoordinatorEvent::State { state: PlayerState::Paused })));
}

#[test]
fn co31_start_and_transport_emit_track_changed() {
    let (player, _, _, _bus) = FakePlayer::new();
    let mut coord = make_coordinator(player);
    let events = subscribe(&coord);

    let a = dummy_track(1, "A");
    let b = dummy_track(2, "B");
    let c = dummy_track(3, "C");
    assert!(coord.start(None, a.clone(), vec![a, b.clone(), c], PlayMode::Sequential).ok);
    assert!(coord.next(None).ok);

    let events = events.lock().unwrap();
    let changed: Vec<i64> = events
        .iter()
        .filter_map(|e| match e {
            CoordinatorEvent::TrackChanged { track } => track.id,
            _ => None,
        })
        .collect();
    assert_eq!(changed, vec![1, 2], "start and next each publish a track change");
}
