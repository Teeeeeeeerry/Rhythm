//! Behavior tests for the playback coordinator
//! (docs/testing/behavior/coordinator.md).
//!
//! The coordinator is the seam where the macOS/Windows orchestration rules
//! (stop old playback, dispatch by source type, record plays, queue build +
//! positioning, bounded skip of unplayable tracks) converge into one place
//! (parent issue #165). Tests drive the coordinator interface with a
//! call-recording fake player surface — no audio device required.

use rhythm_core::coordinator::{CoordinatorErrorKind, PlaybackCoordinator, PlayerSurface};
use rhythm_core::library::Library;
use rhythm_core::queue::PlayMode;
use rhythm_core::{HttpErrorKind, PlayerState, RhythmResult, SourceType, TrackInfo};
use std::path::Path;
use std::ffi::CString;
use std::sync::{Arc, Mutex};

// ─── Fake player surface ──────────────────────────────────────────

/// Call-recording fake: records every engine call so tests can assert the
/// exact orchestration sequence (e.g. `stop` before `play_file` — #51).
struct FakePlayer {
    calls: Arc<Mutex<Vec<String>>>,
    state: Mutex<PlayerState>,
    fail_play_file: bool,
    fail_play_url: bool,
    error_kind: Mutex<Option<HttpErrorKind>>,
}

impl FakePlayer {
    fn new() -> (FakePlayer, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let player = FakePlayer {
            calls: calls.clone(),
            state: Mutex::new(PlayerState::Stopped),
            fail_play_file: false,
            fail_play_url: false,
            error_kind: Mutex::new(None),
        };
        (player, calls)
    }

    fn record(&self, call: &str) {
        self.calls.lock().unwrap().push(call.to_string());
    }
}

impl PlayerSurface for FakePlayer {
    fn play_file(&self, path: &Path) -> RhythmResult<()> {
        self.record(&format!("play_file:{}", path.display()));
        if self.fail_play_file {
            return Err(rhythm_core::RhythmError::FileNotFound(path.display().to_string()));
        }
        *self.state.lock().unwrap() = PlayerState::Playing;
        Ok(())
    }

    fn play_url(&self, url: &str) -> RhythmResult<()> {
        self.record(&format!("play_url:{url}"));
        if self.fail_play_url {
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
}

// ─── Track fixtures ───────────────────────────────────────────────

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
    let (player, calls) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
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
    let (player, _) = FakePlayer::new();
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
    let (player, _) = FakePlayer::new();
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
    let (mut player, calls) = FakePlayer::new();
    player.fail_play_file = true;
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
    let (player, calls) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
    let mut coord = make_coordinator(player);

    let result = coord.next(None);
    assert!(result.ok);
    assert!(result.current_track.is_none());
    assert!(calls.lock().unwrap().is_empty());
}

// ─── CO-15: sync_queue after refresh ──────────────────────────────

#[test]
fn co15_sync_queue_replaces_and_jumps_back_to_current() {
    let (player, _) = FakePlayer::new();
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
    let (player, calls) = FakePlayer::new();
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
    let (player, _) = FakePlayer::new();
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