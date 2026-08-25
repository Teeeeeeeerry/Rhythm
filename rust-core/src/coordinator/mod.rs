//! Playback coordinator — the single orchestration layer above the audio
//! engine and the play queue (parent issue #165, ticket #170).
//!
//! Before this module, the start/transport orchestration (stop old playback,
//! dispatch by source type, record plays, build and position the queue,
//! bounded skip of unplayable tracks) lived twice: once in the macOS
//! `AppState`, once in the Windows `AppState` — two near-line-for-line
//! mirrors that had already drifted apart (#78 vs #81, #137, #136). The
//! coordinator owns the rules once; both UI layers become thin adapters that
//! render the coordinator's state.
//!
//! The coordinator owns the audio engine (`PlayerSurface`), the play queue,
//! the current track, and the play mode. Tests inject a fake player surface
//! (docs/testing/behavior/coordinator.md) so orchestration can be exercised
//! without an audio device.

use crate::audio::AudioEngine;
use crate::library::Library;
use crate::queue::{PlayMode, PlayQueue};
use crate::{
    HttpErrorKind, PlayerState, ProgressCallback, RhythmResult, SourceType, StateCallback, TrackInfo,
};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The playback surface the coordinator orchestrates.
///
/// Production wiring uses `AudioEngine`; tests inject a fake that records the
/// exact call sequence (e.g. `stop` before `play_file` — #51) without
/// touching the audio engine. This is the Rust-side counterpart of the macOS
/// `RhythmPlayerProtocol` seam.
pub trait PlayerSurface {
    fn play_file(&self, path: &Path) -> RhythmResult<()>;
    fn play_url(&self, url: &str) -> RhythmResult<()>;
    fn pause(&self);
    fn resume(&self);
    fn stop(&self);
    fn seek(&self, seconds: f64) -> RhythmResult<()>;
    fn set_volume(&self, volume: f32);
    fn volume(&self) -> f32;
    fn state(&self) -> PlayerState;
    fn position(&self) -> f64;
    fn duration(&self) -> f64;
    /// The last playback failure message, when the state is `Error`.
    fn error_message(&self) -> Option<String>;
    /// Classification of the last playback failure when it was HTTP (#120).
    fn error_kind(&self) -> Option<HttpErrorKind>;
    /// Register a callback for engine state transitions (the coordinator
    /// forwards these as events — ticket #172).
    fn on_state_change(&self, callback: StateCallback);
    /// Register a callback for playback progress (forwarded as events).
    fn on_progress(&self, callback: ProgressCallback);
}

impl PlayerSurface for AudioEngine {
    fn play_file(&self, path: &Path) -> RhythmResult<()> {
        AudioEngine::play_file(self, path)
    }
    fn play_url(&self, url: &str) -> RhythmResult<()> {
        AudioEngine::play_url(self, url)
    }
    fn pause(&self) {
        AudioEngine::pause(self);
    }
    fn resume(&self) {
        AudioEngine::resume(self);
    }
    fn stop(&self) {
        AudioEngine::stop(self);
    }
    fn seek(&self, seconds: f64) -> RhythmResult<()> {
        AudioEngine::seek(self, seconds)
    }
    fn set_volume(&self, volume: f32) {
        AudioEngine::set_volume(self, volume);
    }
    fn volume(&self) -> f32 {
        AudioEngine::volume(self)
    }
    fn state(&self) -> PlayerState {
        AudioEngine::state(self)
    }
    fn position(&self) -> f64 {
        AudioEngine::position(self)
    }
    fn duration(&self) -> f64 {
        AudioEngine::duration(self)
    }
    fn error_message(&self) -> Option<String> {
        match AudioEngine::state(self) {
            PlayerState::Error(message) => Some(message),
            _ => None,
        }
    }
    fn error_kind(&self) -> Option<HttpErrorKind> {
        AudioEngine::last_error_kind(self)
    }
    fn on_state_change(&self, callback: StateCallback) {
        AudioEngine::on_state_change(self, callback);
    }
    fn on_progress(&self, callback: ProgressCallback) {
        AudioEngine::on_progress(self, callback);
    }
}

/// Why a coordinator operation failed, classified — the structured-result
/// contract shape shared with the FFI deepening group (#166 group, #176).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinatorErrorKind {
    /// The track has no playable location (no file path / no source URL, or
    /// empty strings). The #78/#81 guard: nothing can be handed to the
    /// player, so the playing state must not be entered.
    NoPlayableLocation,
    /// The engine rejected the play request immediately (e.g. file not found).
    PlaybackFailed,
    /// The request was malformed (bad JSON) or the handle was null.
    InvalidInput,
}

/// Structured result of a coordinator call: success payload + classified
/// error in a single return (no global error slot).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoordinatorResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<CoordinatorErrorKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// The current track after the operation (when one is set). Lets the UI
    /// follow transport moves without a second query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_track: Option<TrackInfo>,
    /// Whether playback is active (engine Playing/Buffering) after the
    /// operation — what the UI should render for `isPlaying`.
    pub playback_active: bool,
}

impl CoordinatorResult {
    pub fn ok() -> Self {
        Self {
            ok: true,
            error_kind: None,
            error_message: None,
            current_track: None,
            playback_active: false,
        }
    }

    pub fn ok_with_track(track: TrackInfo) -> Self {
        Self {
            ok: true,
            error_kind: None,
            error_message: None,
            current_track: Some(track),
            playback_active: true,
        }
    }

    /// Like `ok_with_track`, but with an explicit playback-active flag (the
    /// toggle operation reports the engine state after the op).
    pub fn ok_with_track_active(track: TrackInfo, active: bool) -> Self {
        Self {
            ok: true,
            error_kind: None,
            error_message: None,
            current_track: Some(track),
            playback_active: active,
        }
    }

    /// Successful no-track result with an explicit playback-active flag.
    pub fn ok_active(active: bool) -> Self {
        Self {
            ok: true,
            error_kind: None,
            error_message: None,
            current_track: None,
            playback_active: active,
        }
    }

    pub fn error(kind: CoordinatorErrorKind, message: String) -> Self {
        Self {
            ok: false,
            error_kind: Some(kind),
            error_message: Some(message),
            current_track: None,
            playback_active: false,
        }
    }
}

/// Events the coordinator publishes to the UI (ticket #172): the UI
/// subscribes instead of polling the engine. Serialized as
/// `{"type":"...", ...}` (snake_case keys and values).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoordinatorEvent {
    /// The current track ended naturally. The coordinator auto-advances when
    /// the queue has a next track (a `TrackChanged` event follows); when the
    /// queue is exhausted playback just ends.
    Finished,
    /// Playback failed. `kind` is the #120 classification ("expired" /
    /// "cdn_rejected" / "other") when the failure was HTTP; the FFI layer
    /// fills it from the engine's last failure classification.
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        kind: Option<HttpErrorKind>,
        message: String,
    },
    /// Playback progress (position / duration in seconds).
    Progress { position: f64, duration: f64 },
    /// Engine state transition (named, not numeric).
    State { state: PlayerState },
    /// The current track changed (start / transport move / auto-advance).
    /// Boxed so the enum stays small (the track is the largest payload).
    TrackChanged { track: Box<TrackInfo> },
}

/// The UI's event subscription. `Arc` so wiring closures can clone it out of
/// the holder before invoking (the engine callback can re-enter while the
/// coordinator mutates — e.g. auto-advance).
pub type CoordinatorEventCallback = Arc<dyn Fn(CoordinatorEvent) + Send + Sync>;

fn event_from_player_state(state: &PlayerState) -> CoordinatorEvent {
    match state {
        PlayerState::Finished => CoordinatorEvent::Finished,
        PlayerState::Error(message) => CoordinatorEvent::Error {
            kind: None,
            message: message.clone(),
        },
        other => CoordinatorEvent::State { state: other.clone() },
    }
}

/// The player-reachable location of a track: local tracks need a file path,
/// streamed tracks a URL. Empty strings count as missing — they pass a
/// nil-only check and reach the player as a doomed play call (#78).
enum PlayableLocation {
    File(String),
    Url(String),
}

fn playable_location(track: &TrackInfo) -> Option<PlayableLocation> {
    match track.source_type {
        SourceType::Local => {
            let path = track.file_path.as_deref().unwrap_or("");
            if path.is_empty() {
                None
            } else {
                Some(PlayableLocation::File(path.to_string()))
            }
        }
        _ => {
            let url = track.source_url.as_deref().unwrap_or("");
            if url.is_empty() {
                None
            } else {
                Some(PlayableLocation::Url(url.to_string()))
            }
        }
    }
}

/// Owns the playback state machine: the engine, the queue, the current
/// track, the play mode, and a mirror of the library list (used by the
/// toggle's idle-start rule).
pub struct PlaybackCoordinator {
    player: Box<dyn PlayerSurface>,
    queue: Option<PlayQueue>,
    current_track: Option<TrackInfo>,
    play_mode: PlayMode,
    /// Last library snapshot (`sync_queue` mirror). The toggle's idle-start
    /// rule picks the first playable track from here when nothing is
    /// playing (#78 candidate selection lives in the coordinator).
    library_tracks: Vec<TrackInfo>,
    /// The UI's event subscription (ticket #172). `Arc` so the engine
    /// wiring closures can hold a clone without a self-reference cycle.
    event_callback: Arc<Mutex<Option<CoordinatorEventCallback>>>,
}

impl PlaybackCoordinator {
    /// Production constructor: real audio engine.
    pub fn new() -> Self {
        Self::with_player(Box::new(AudioEngine::new()))
    }

    /// Test seam: construct a coordinator over an injected player surface
    /// (e.g. a call-recording fake) instead of the real audio engine.
    pub fn with_player(player: Box<dyn PlayerSurface>) -> Self {
        let coordinator = PlaybackCoordinator {
            player,
            queue: None,
            current_track: None,
            play_mode: PlayMode::Sequential,
            library_tracks: Vec::new(),
            event_callback: Arc::new(Mutex::new(None)),
        };
        coordinator.wire_engine_events();
        coordinator
    }

    /// Subscribe to coordinator events (ticket #172).
    pub fn set_event_callback(&self, callback: CoordinatorEventCallback) {
        *self.event_callback.lock().unwrap() = Some(callback);
    }

    /// Wire the engine's state/progress callbacks to the coordinator's event
    /// channel. Called once at construction.
    fn wire_engine_events(&self) {
        let holder = self.event_callback.clone();
        self.player.on_state_change(Box::new(move |state| {
            let event = event_from_player_state(&state);
            emit_event(&holder, event);
        }));
        let holder = self.event_callback.clone();
        self.player.on_progress(Box::new(move |position, duration| {
            emit_event(
                &holder,
                CoordinatorEvent::Progress {
                    position,
                    duration,
                },
            );
        }));
    }

    /// Handle a natural track end: auto-advance to the next playable track
    /// when the queue has one (core-driven — parent issue #165 decision),
    /// otherwise leave playback ended. Invoked by the FFI event dispatcher
    /// when a `Finished` event fires.
    pub fn handle_finished(&mut self, library: Option<&Library>) -> CoordinatorResult {
        if self.queue.as_ref().is_some_and(|q| q.has_next()) {
            self.play_adjacent(library, Direction::Next)
        } else {
            self.result_for_current()
        }
    }

    fn emit(&self, event: CoordinatorEvent) {
        emit_event(&self.event_callback, event);
    }

    /// The underlying player surface (volume/seek/state pass-throughs).
    pub fn player(&self) -> &dyn PlayerSurface {
        self.player.as_ref()
    }

    /// Start playback of `track` with `queue_tracks` as the queue: stop old
    /// playback (#51), dispatch by source type, record the play, establish
    /// the queue and position it at the current track.
    ///
    /// The #78/#81 guard lives here: a track without a playable location
    /// returns a classified failure and nothing changes.
    pub fn start(
        &mut self,
        library: Option<&Library>,
        track: TrackInfo,
        queue_tracks: Vec<TrackInfo>,
        mode: PlayMode,
    ) -> CoordinatorResult {
        let Some(location) = playable_location(&track) else {
            return CoordinatorResult::error(
                CoordinatorErrorKind::NoPlayableLocation,
                "track has no playable location".to_string(),
            );
        };

        // #51: stop old playback before starting the new one.
        self.player.stop();
        let dispatch = match &location {
            PlayableLocation::File(path) => self.player.play_file(Path::new(path)),
            PlayableLocation::Url(url) => self.player.play_url(url),
        };
        if let Err(e) = dispatch {
            return CoordinatorResult::error(
                CoordinatorErrorKind::PlaybackFailed,
                e.to_string(),
            );
        }

        if let Some(lib) = library {
            if let Some(id) = track.id {
                if id >= 0 {
                    let _ = lib.record_play(id);
                }
            }
        }

        // Establish the queue and position it at the current track.
        let mut queue = PlayQueue::new(queue_tracks);
        queue.set_mode(mode);
        if let Some(id) = track.id {
            if id >= 0 {
                queue.jump_to(id);
            }
        }
        self.queue = Some(queue);
        self.play_mode = mode;
        self.current_track = Some(track.clone());
        self.emit(CoordinatorEvent::TrackChanged { track: Box::new(track.clone()) });
        CoordinatorResult::ok_with_track(track)
    }

    /// Play the next playable track, skipping unplayable ones. The skip is
    /// bounded by the queue length so an all-dead queue cannot loop forever
    /// (#78). No-op (current track unchanged) when there is no queue, no
    /// next track, or every remaining track is unplayable.
    pub fn next(&mut self, library: Option<&Library>) -> CoordinatorResult {
        self.play_adjacent(library, Direction::Next)
    }

    /// Play the previous playable track, skipping unplayable ones (bounded,
    /// walking backwards — #78).
    pub fn previous(&mut self, library: Option<&Library>) -> CoordinatorResult {
        self.play_adjacent(library, Direction::Previous)
    }

    /// Sync the queue after a library refresh (#69): replace the contents,
    /// jump back to the current track by its database id, and mirror the
    /// library list (the toggle's idle-start source).
    pub fn sync_queue(&mut self, tracks: Vec<TrackInfo>) {
        self.library_tracks = tracks.clone();
        let Some(queue) = self.queue.as_mut() else { return };
        queue.replace(tracks);
        if let Some(track) = &self.current_track {
            if let Some(id) = track.id {
                if id >= 0 {
                    queue.jump_to(id);
                }
            }
        }
    }

    /// Stop playback and clear the transport state (current track + queue).
    pub fn stop(&mut self) {
        self.player.stop();
        self.current_track = None;
        self.queue = None;
    }

    pub fn play_mode(&self) -> PlayMode {
        self.play_mode
    }

    /// Switch the play mode; the queue (when present) follows.
    pub fn set_play_mode(&mut self, mode: PlayMode) {
        self.play_mode = mode;
        if let Some(queue) = self.queue.as_mut() {
            queue.set_mode(mode);
        }
    }

    pub fn current_track(&self) -> Option<&TrackInfo> {
        self.current_track.as_ref()
    }

    /// Whether the queue has a next track (transport availability).
    pub fn can_play_next(&self) -> bool {
        self.queue.as_ref().is_some_and(|q| q.has_next())
    }

    /// Whether the queue has a previous track (transport availability).
    pub fn can_play_previous(&self) -> bool {
        self.queue.as_ref().is_some_and(|q| q.has_previous())
    }

    /// Whether the toggle has something to act on: a current track (pause /
    /// resume) or a library to start from (idle start).
    pub fn can_toggle_playback(&self) -> bool {
        self.current_track.is_some() || !self.library_tracks.is_empty()
    }

    /// Whether playback can be stopped: the engine is playing, buffering, or
    /// paused.
    pub fn can_stop(&self) -> bool {
        matches!(
            self.player.state(),
            PlayerState::Playing | PlayerState::Buffering | PlayerState::Paused
        )
    }

    /// Toggle play/pause, with the full transport semantics the UI layers
    /// used to implement twice (ticket #171):
    ///
    /// - engine Playing/Buffering → pause (#111: pause during Buffering must
    ///   stick),
    /// - engine Paused → resume (only Paused may resume — #111),
    /// - engine Finished → no-op (the UI stops claiming playback),
    /// - engine Stopped with a current track → no-op (nothing to resume),
    /// - engine Stopped with no current track → start the first playable
    ///   library track (idle start; candidate selection lives here — #78).
    pub fn toggle_play_pause(&mut self, library: Option<&Library>) -> CoordinatorResult {
        match self.player.state() {
            PlayerState::Playing | PlayerState::Buffering => {
                self.player.pause();
                self.result_for_current_active(false)
            }
            PlayerState::Paused => {
                self.player.resume();
                self.result_for_current_active(true)
            }
            _ => {
                if self.current_track.is_none() {
                    // Idle start: first playable library track (#78).
                    let candidate = self
                        .library_tracks
                        .iter()
                        .find(|t| playable_location(t).is_some());
                    if let Some(track) = candidate {
                        let queue = self.library_tracks.clone();
                        return self.start(library, track.clone(), queue, self.play_mode);
                    }
                }
                // Finished / Stopped with a current track: nothing to do.
                self.result_for_current_active(false)
            }
        }
    }
}

enum Direction {
    Next,
    Previous,
}

impl PlaybackCoordinator {
    /// Shared next/previous walk: advance/retreat through the queue up to the
    /// queue length, skipping tracks with no playable location (#78). The
    /// first playable candidate is dispatched (stop old first — #51), the
    /// play recorded, and the coordinator's current track updated.
    fn play_adjacent(&mut self, library: Option<&Library>, direction: Direction) -> CoordinatorResult {
        // Bound the skip loop: in loop modes `advance()` never returns None,
        // so without a bound an all-dead queue would spin forever (#78).
        let bound = match &self.queue {
            Some(q) => q.tracks.len(),
            None => return self.result_for_current(),
        };
        for _ in 0..bound {
            // Take the queue borrow only for the cursor move, releasing it
            // before the engine dispatch / event emission below.
            let candidate = {
                let queue = self.queue.as_mut().expect("queue exists");
                match direction {
                    Direction::Next => queue.advance().cloned(),
                    Direction::Previous => queue.previous().cloned(),
                }
            };
            let Some(candidate) = candidate else {
                return self.result_for_current();
            };
            let Some(location) = playable_location(&candidate) else { continue };

            self.player.stop();
            let dispatch = match &location {
                PlayableLocation::File(path) => self.player.play_file(Path::new(path)),
                PlayableLocation::Url(url) => self.player.play_url(url),
            };
            if let Err(e) = dispatch {
                return CoordinatorResult::error(
                    CoordinatorErrorKind::PlaybackFailed,
                    e.to_string(),
                );
            }
            if let Some(lib) = library {
                if let Some(id) = candidate.id {
                    if id >= 0 {
                        let _ = lib.record_play(id);
                    }
                }
            }
            self.current_track = Some(candidate.clone());
            self.emit(CoordinatorEvent::TrackChanged { track: Box::new(candidate.clone()) });
            return CoordinatorResult::ok_with_track(candidate.clone());
        }
        // Every remaining track is unplayable — give up; the current track
        // keeps playing.
        self.result_for_current()
    }

    fn result_for_current(&self) -> CoordinatorResult {
        match &self.current_track {
            Some(track) => CoordinatorResult::ok_with_track(track.clone()),
            None => CoordinatorResult::ok(),
        }
    }

    fn result_for_current_active(&self, active: bool) -> CoordinatorResult {
        match &self.current_track {
            Some(track) => CoordinatorResult::ok_with_track_active(track.clone(), active),
            None => CoordinatorResult::ok_active(active),
        }
    }
}

impl Default for PlaybackCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

fn emit_event(holder: &Arc<Mutex<Option<CoordinatorEventCallback>>>, event: CoordinatorEvent) {
    if let Some(callback) = holder.lock().unwrap().as_ref() {
        // Clone the Arc and drop the lock before invoking: the callback can
        // re-enter the coordinator (auto-advance emits further events), and
        // std Mutex is not reentrant.
        let callback = callback.clone();
        callback(event);
    }
}
