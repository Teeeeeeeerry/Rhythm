//! FFI layer — C-ABI compatible exports for macOS (Swift) and Windows (C++) UI layers.
//!
//! All functions use opaque pointers to pass Rust objects across FFI boundaries.
//! The UI layer creates/destroys objects via these functions and never accesses
//! Rust internals directly.
//!
//! # Safety (#143)
//!
//! Every export taking raw pointers is declared `unsafe extern "C"`: callers
//! must pass pointers that are valid for the handle's lifetime and, for
//! `*const c_char`, NUL-terminated. Null pointers are handled by each export
//! (or its helpers): error/no-op for handles, empty input for strings. The
//! keyword is invisible to C/C++/Swift callers but states the contract
//! honestly.

use crate::audio::AudioEngine;
use crate::coordinator::{
    CoordinatorErrorKind, CoordinatorEvent, CoordinatorResult, PlaybackCoordinator,
};
use crate::library::Library;
use crate::metadata;
use crate::playlist;
use crate::queue::PlayMode;
use crate::resolver;
use crate::{PlayerState, TrackInfo};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

// ─── Opaque Handle Types ───────────────────────────────────────────

/// Opaque handle to a Library instance.
pub struct RhythmLibrary(Library);

/// Opaque handle to an AudioEngine instance.
pub struct RhythmPlayer(AudioEngine);

/// Opaque handle to a playback coordinator (owns the engine, queue, current
/// track, and play mode — parent issue #165, ticket #170).
///
/// `library` is the library handle registered by the UI
/// (`rhythm_coordinator_set_library`); the event dispatcher uses it to
/// record plays on auto-advance (ticket #172).
pub struct RhythmCoordinator {
    inner: Mutex<PlaybackCoordinator>,
    library: Mutex<*mut RhythmLibrary>,
}

// ─── String/FFI Helpers ────────────────────────────────────────────

unsafe fn c_str_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

fn str_to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

fn free_c_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Raw pointer made explicitly `Send + Sync` for the event callback: it
/// fires on the playback thread while the coordinator handle itself is owned
/// by the UI thread, which guarantees the handle outlives the callback.
struct SendPtr<T>(*mut T);
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

impl<T> SendPtr<T> {
    fn get(&self) -> *mut T {
        self.0
    }
}

fn track_to_json(track: &TrackInfo) -> String {
    serde_json::to_string(track).unwrap_or_default()
}

fn json_to_track(json: &str) -> Option<TrackInfo> {
    serde_json::from_str(json).ok()
}

// ─── Library FFI ───────────────────────────────────────────────────

/// Create a new library, opening or creating the database at `db_path`.
/// Returns an opaque pointer to the Library, or null on error.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_open(db_path: *const c_char) -> *mut RhythmLibrary {
    let path = unsafe { c_str_to_str(db_path) };
    match Library::open(Path::new(path)) {
        Ok(lib) => Box::into_raw(Box::new(RhythmLibrary(lib))),
        Err(e) => {
            log::error!("Failed to open library: {e}");
            std::ptr::null_mut()
        }
    }
}

/// Destroy a library handle and free resources.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_close(ptr: *mut RhythmLibrary) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Scan a directory and import all audio files into the library.
/// Returns the number of tracks imported, or -1 on error.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_import(ptr: *mut RhythmLibrary, dir: *const c_char) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let lib = unsafe { &(*ptr).0 };
    let path = unsafe { c_str_to_str(dir) };
    match lib.import_from_directory(Path::new(path)) {
        Ok(count) => count as i32,
        Err(e) => {
            log::error!("Import failed: {e}");
            -1
        }
    }
}

/// Import a single audio file into the library.
/// Returns 1 on success, 0 if the file is not a supported audio format,
/// or -1 on error.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_import_file(
    ptr: *mut RhythmLibrary,
    file_path: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let lib = unsafe { &(*ptr).0 };
    let path = unsafe { c_str_to_str(file_path) };
    match lib.import_file(Path::new(path)) {
        Ok(count) => count,
        // Unsupported format → 0 per the documented contract (#79).
        Err(crate::RhythmError::UnsupportedFormat(msg)) => {
            log::warn!("Import file skipped: {msg}");
            0
        }
        Err(e) => {
            log::error!("Import file failed: {e}");
            -1
        }
    }
}

/// Get all tracks as a JSON string. Caller must free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_get_all_tracks(ptr: *mut RhythmLibrary) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    match lib.get_all_tracks() {
        Ok(tracks) => str_to_c_string(&serde_json::to_string(&tracks).unwrap_or_default()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Search the library. Returns JSON array of matching tracks.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_search(
    ptr: *mut RhythmLibrary,
    query: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    let q = unsafe { c_str_to_str(query) };
    match lib.search(q) {
        Ok(results) => str_to_c_string(&serde_json::to_string(&results).unwrap_or_default()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Add a track from JSON. Returns the track with its database ID, or null.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_add_track(
    ptr: *mut RhythmLibrary,
    track_json: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    let json = unsafe { c_str_to_str(track_json) };
    if let Some(track) = json_to_track(json) {
        match lib.add_track(&track) {
            Ok(saved) => str_to_c_string(&track_to_json(&saved)),
            Err(_) => std::ptr::null_mut(),
        }
    } else {
        std::ptr::null_mut()
    }
}

/// Remove a track by ID. Returns 0 on success, -1 on error.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_remove_track(ptr: *mut RhythmLibrary, track_id: i64) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let lib = unsafe { &(*ptr).0 };
    match lib.remove_track(track_id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Verify all local files exist. Returns JSON array of unavailable track IDs.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_verify_files(ptr: *mut RhythmLibrary) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    match lib.verify_local_files() {
        Ok(ids) => str_to_c_string(&serde_json::to_string(&ids).unwrap_or_default()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get all playlists as JSON.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_get_playlists(ptr: *mut RhythmLibrary) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    match lib.get_all_playlists() {
        Ok(playlists) => str_to_c_string(&serde_json::to_string(&playlists).unwrap_or_default()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create a new playlist. Returns the playlist ID, or -1 on error.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_create_playlist(
    ptr: *mut RhythmLibrary,
    name: *const c_char,
    description: *const c_char,
) -> i64 {
    if ptr.is_null() {
        return -1;
    }
    let lib = unsafe { &(*ptr).0 };
    let n = unsafe { c_str_to_str(name) };
    let d = unsafe {
        let s = c_str_to_str(description);
        if s.is_empty() { None } else { Some(s) }
    };
    lib.create_playlist(n, d).unwrap_or(-1)
}

/// Add a track to a playlist. Returns 0 on success.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_playlist_add(
    ptr: *mut RhythmLibrary,
    playlist_id: i64,
    track_id: i64,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let lib = unsafe { &(*ptr).0 };
    match lib.add_to_playlist(playlist_id, track_id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Remove a track from a playlist. Returns 0 on success.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_playlist_remove(
    ptr: *mut RhythmLibrary,
    playlist_id: i64,
    track_id: i64,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let lib = unsafe { &(*ptr).0 };
    match lib.remove_from_playlist(playlist_id, track_id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Delete a playlist entirely. Returns 0 on success.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_delete_playlist(ptr: *mut RhythmLibrary, playlist_id: i64) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let lib = unsafe { &(*ptr).0 };
    match lib.delete_playlist(playlist_id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Mark a track as played (update last_played and play_count).
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_record_play(ptr: *mut RhythmLibrary, track_id: i64) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let lib = unsafe { &(*ptr).0 };
    match lib.record_play(track_id) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// ─── Metadata FFI ──────────────────────────────────────────────────

/// Extract metadata from a local audio file. Returns JSON TrackInfo, or null.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_metadata_extract(path: *const c_char) -> *mut c_char {
    let p = unsafe { c_str_to_str(path) };
    match metadata::extract_track_info(Path::new(p)) {
        Ok(track) => str_to_c_string(&track_to_json(&track)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Scan a directory for audio files. Returns JSON array of TrackInfo.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_metadata_scan(dir: *const c_char) -> *mut c_char {
    let p = unsafe { c_str_to_str(dir) };
    match metadata::scan_directory(Path::new(p)) {
        Ok(tracks) => str_to_c_string(&serde_json::to_string(&tracks).unwrap_or_default()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Extract cover art from a local audio file and save to cache directory.
/// Returns the file path of the saved artwork, or null if none found.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_metadata_extract_artwork(
    file_path: *const c_char,
    cache_dir: *const c_char,
) -> *mut c_char {
    let fp = unsafe { c_str_to_str(file_path) };
    let cd = unsafe { c_str_to_str(cache_dir) };
    match metadata::extract_artwork(Path::new(fp), Path::new(cd)) {
        Ok(Some(path)) => str_to_c_string(&path),
        Ok(None) => std::ptr::null_mut(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ─── Audio Player FFI ─────────────────────────────────────────────

/// Create a new audio player. Returns opaque handle.
#[no_mangle]
pub extern "C" fn rhythm_player_create() -> *mut RhythmPlayer {
    let engine = AudioEngine::new();
    Box::into_raw(Box::new(RhythmPlayer(engine)))
}

/// Destroy a player handle.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_destroy(ptr: *mut RhythmPlayer) {
    if !ptr.is_null() {
        let player = unsafe { Box::from_raw(ptr) };
        player.0.stop();
    }
}

/// Play a local file by path. Returns 0 on success.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_play_file(
    ptr: *mut RhythmPlayer,
    path: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let player = unsafe { &(*ptr).0 };
    let p = unsafe { c_str_to_str(path) };
    match player.play_file(Path::new(p)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Play a URL stream. Returns 0 on success.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_play_url(
    ptr: *mut RhythmPlayer,
    url: *const c_char,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let player = unsafe { &(*ptr).0 };
    let u = unsafe { c_str_to_str(url) };
    match player.play_url(u) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Pause playback.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_pause(ptr: *mut RhythmPlayer) {
    if let Some(player) = unsafe { ptr.as_ref() } {
        player.0.pause();
    }
}

/// Resume playback.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_resume(ptr: *mut RhythmPlayer) {
    if let Some(player) = unsafe { ptr.as_ref() } {
        player.0.resume();
    }
}

/// Stop playback.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_stop(ptr: *mut RhythmPlayer) {
    if let Some(player) = unsafe { ptr.as_ref() } {
        player.0.stop();
    }
}

/// Set volume (0.0 - 1.0).
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_set_volume(ptr: *mut RhythmPlayer, volume: f32) {
    if let Some(player) = unsafe { ptr.as_ref() } {
        player.0.set_volume(volume);
    }
}

/// Get current volume.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_get_volume(ptr: *mut RhythmPlayer) -> f32 {
    unsafe { ptr.as_ref().map(|p| p.0.volume()).unwrap_or(0.0) }
}

/// Seek to a position in seconds. Returns 0 on success, -1 on error
/// (null pointer, negative position, or position out of range).
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_seek(ptr: *mut RhythmPlayer, seconds: f64) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    let player = unsafe { &(*ptr).0 };
    match player.seek(seconds) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get current playback position in seconds.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_get_position(ptr: *mut RhythmPlayer) -> f64 {
    unsafe { ptr.as_ref().map(|p| p.0.position()).unwrap_or(0.0) }
}

/// Get media duration in seconds.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_get_duration(ptr: *mut RhythmPlayer) -> f64 {
    unsafe { ptr.as_ref().map(|p| p.0.duration()).unwrap_or(0.0) }
}

/// Get player state: 0=Stopped, 1=Playing, 2=Paused, 3=Buffering, 4=Error, 5=Finished
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_get_state(ptr: *mut RhythmPlayer) -> i32 {
    unsafe {
        ptr.as_ref()
            .map(|p| match p.0.state() {
                PlayerState::Stopped => 0,
                PlayerState::Playing => 1,
                PlayerState::Paused => 2,
                PlayerState::Buffering => 3,
                PlayerState::Error(_) => 4,
                PlayerState::Finished => 5,
            })
            .unwrap_or(-1)
    }
}

/// Why playback failed, when the state is Error (4); null otherwise.
///
/// Without this the UI can only show a stopped player sitting at 0:00 with no
/// explanation — which is exactly how a 403 from a CDN used to look. Free
/// with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_error(ptr: *mut RhythmPlayer) -> *mut c_char {
    unsafe {
        match ptr.as_ref().map(|p| p.0.state()) {
            Some(PlayerState::Error(message)) => str_to_c_string(&message),
            _ => std::ptr::null_mut(),
        }
    }
}

/// Classification of the last playback failure, when it was HTTP: "expired" |
/// "cdn_rejected" | "other"; null otherwise (#120).
///
/// The error *message* is the same raw network text as before; this lets the
/// UI swap its headline between "link expired, re-paste it" (which is only
/// true for `expired`) and "the CDN rejected your network" (`cdn_rejected`).
/// Free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_player_error_kind(ptr: *mut RhythmPlayer) -> *mut c_char {
    use crate::HttpErrorKind;
    unsafe {
        match ptr.as_ref().and_then(|p| p.0.last_error_kind()) {
            Some(HttpErrorKind::Expired) => str_to_c_string("expired"),
            Some(HttpErrorKind::CdnRejected) => str_to_c_string("cdn_rejected"),
            Some(HttpErrorKind::Other) => str_to_c_string("other"),
            None => std::ptr::null_mut(),
        }
    }
}

// ─── URL Resolver FFI ─────────────────────────────────────────────

/// The most recent resolver failure, as JSON.
///
/// A null return from `rhythm_resolve_url` used to be the only signal the UI
/// got, which reduced every distinct failure — yt-dlp missing, timeout,
/// private video — to a single generic "resolution failed" alert (#21).
static LAST_RESOLVE_ERROR: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

fn set_last_resolve_error(failure: &resolver::ResolveFailure) {
    let json = serde_json::to_string(failure).unwrap_or_else(|_| {
        format!(
            "{{\"kind\":\"internal\",\"message\":{}}}",
            serde_json::Value::String(failure.message.clone())
        )
    });
    *LAST_RESOLVE_ERROR.lock().unwrap() = Some(json);
}

fn clear_last_resolve_error() {
    *LAST_RESOLVE_ERROR.lock().unwrap() = None;
}

/// Resolve a URL to a playable stream. Returns JSON ResolvedUrl, or null on
/// failure — call `rhythm_last_error` for the reason.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_resolve_url(url: *const c_char) -> *mut c_char {
    let u = unsafe { c_str_to_str(url) };
    match resolver::resolve_url(u) {
        Ok(resolved) => {
            clear_last_resolve_error();
            str_to_c_string(&serde_json::to_string(&resolved).unwrap_or_default())
        }
        Err(failure) => {
            set_last_resolve_error(&failure);
            std::ptr::null_mut()
        }
    }
}

/// Classify a URL. Returns the source type string ("youtube", "bilibili", "direct_url", "local").
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_classify_url(url: *const c_char) -> *mut c_char {
    let u = unsafe { c_str_to_str(url) };
    match resolver::classify_url(u) {
        Ok(source_type) => str_to_c_string(&source_type.to_string()),
        Err(failure) => {
            set_last_resolve_error(&failure);
            std::ptr::null_mut()
        }
    }
}

/// The last resolver failure as JSON `{"kind": "...", "message": "..."}`, or
/// null if the last resolution succeeded. Free with `rhythm_free_string`.
#[no_mangle]
pub extern "C" fn rhythm_last_error() -> *mut c_char {
    match LAST_RESOLVE_ERROR.lock().unwrap().as_deref() {
        Some(json) => str_to_c_string(json),
        None => std::ptr::null_mut(),
    }
}

/// Resolver environment as JSON: yt-dlp path and version, the
/// `RHYTHM_YTDLP_PATH` override, the inherited PATH, and the log file
/// location. Intended for bug reports. Free with `rhythm_free_string`.
#[no_mangle]
pub extern "C" fn rhythm_resolver_diagnostics() -> *mut c_char {
    str_to_c_string(&resolver::diagnostics().to_string())
}

/// Progress of yt-dlp provisioning as JSON, e.g.
/// `{"phase":"downloading","received":1048576,"total":41943040}`.
///
/// Phases: idle, checking, downloading, verifying, updating, ready, failed.
/// The UI polls this while a resolution is in flight so a first-run download
/// doesn't look like a hang. Free with `rhythm_free_string`.
#[no_mangle]
pub extern "C" fn rhythm_resolver_status() -> *mut c_char {
    let json = serde_json::to_string(&resolver::install::status()).unwrap_or_default();
    str_to_c_string(&json)
}

/// Install or update Rhythm's own yt-dlp copy now. Returns the binary path,
/// or null on failure — call `rhythm_last_error` for the reason. Blocks for
/// the duration of the download. Free with `rhythm_free_string`.
#[no_mangle]
pub extern "C" fn rhythm_install_ytdlp() -> *mut c_char {
    match resolver::install::update_now() {
        Ok(path) => {
            clear_last_resolve_error();
            str_to_c_string(&path.display().to_string())
        }
        Err(failure) => {
            set_last_resolve_error(&failure);
            std::ptr::null_mut()
        }
    }
}

// ─── Playback Coordinator FFI ─────────────────────────────────────
//
// The coordinator owns the orchestration rules (stop old playback, dispatch
// by source type, record plays, queue build + positioning, bounded skip of
// unplayable tracks) that used to live twice in the UI layers (parent issue
// #165). Every call returns a structured result JSON — success payload +
// classified error in a single return (#170).

fn coordinator_result_to_c_string(result: &CoordinatorResult) -> *mut c_char {
    str_to_c_string(&serde_json::to_string(result).unwrap_or_default())
}

/// Create a new playback coordinator. Returns opaque handle.
#[no_mangle]
pub extern "C" fn rhythm_coordinator_create() -> *mut RhythmCoordinator {
    Box::into_raw(Box::new(RhythmCoordinator {
        inner: Mutex::new(PlaybackCoordinator::new()),
        library: Mutex::new(std::ptr::null_mut()),
    }))
}

/// Destroy a coordinator handle.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_destroy(ptr: *mut RhythmCoordinator) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr); }
    }
}

/// Register the library handle the coordinator uses for play recording
/// (transport moves and auto-advance). The UI calls this whenever its
/// library handle changes.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_set_library(
    ptr: *mut RhythmCoordinator,
    library: *mut RhythmLibrary,
) {
    if ptr.is_null() { return; }
    *(*ptr).library.lock().unwrap() = library;
}

/// C callback receiving coordinator events (ticket #172): a JSON string
/// `{"type":"finished"|"error"|"progress"|"state"|"track_changed", ...}`
/// that must be freed with `rhythm_free_string`. Invoked from the playback
/// thread.
pub type RhythmEventCallback =
    extern "C" fn(userdata: *mut std::os::raw::c_void, event_json: *mut c_char);

/// Subscribe to coordinator events. On a `Finished` event the coordinator
/// auto-advances to the next playable track (core-driven auto-advance —
/// parent issue #165), using the registered library for play recording.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_set_event_callback(
    ptr: *mut RhythmCoordinator,
    callback: Option<RhythmEventCallback>,
    userdata: *mut std::os::raw::c_void,
) {
    if ptr.is_null() { return; }
    let raw: SendPtr<RhythmCoordinator> = SendPtr(ptr);
    let library: Arc<Mutex<SendPtr<RhythmLibrary>>> =
        Arc::new(Mutex::new(SendPtr(*(*ptr).library.lock().unwrap())));
    let userdata: SendPtr<std::os::raw::c_void> = SendPtr(userdata);
    let wrapped: crate::coordinator::CoordinatorEventCallback = Arc::new(move |event| {
        // Auto-advance: a natural track end moves to the next playable
        // track inside the coordinator (the `TrackChanged` event that
        // follows lets the UI re-render).
        if let CoordinatorEvent::Finished = &event {
            let lib = library.lock().unwrap().get();
            let mut coord = unsafe { &(*raw.0).inner }.lock().unwrap();
            let lib_ref = if lib.is_null() {
                None
            } else {
                Some(unsafe { &(*lib).0 })
            };
            let _ = coord.handle_finished(lib_ref);
        }
        // Enrich Error events with the engine's #120 classification.
        let event = match &event {
            CoordinatorEvent::Error { kind: None, message } => {
                let kind = unsafe { &(*raw.get()).inner }.lock().unwrap().player().error_kind();
                CoordinatorEvent::Error {
                    kind,
                    message: message.clone(),
                }
            }
            other => other.clone(),
        };
        if let Some(callback) = callback {
            let json = serde_json::to_string(&event).unwrap_or_default();
            callback(userdata.get(), str_to_c_string(&json));
        }
    });
    unsafe { &(*ptr).inner }.lock().unwrap().set_event_callback(wrapped);
}

/// Start playback of a track with a queue. Returns structured result JSON:
/// `{"ok":true,"current_track":{...}}` or
/// `{"ok":false,"error_kind":"...","error_message":"..."}`. Free with
/// `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_start(
    ptr: *mut RhythmCoordinator,
    library: *mut RhythmLibrary,
    track_json: *const c_char,
    queue_tracks_json: *const c_char,
    mode: i32,
) -> *mut c_char {
    if ptr.is_null() {
        return coordinator_result_to_c_string(&CoordinatorResult::error(
            CoordinatorErrorKind::InvalidInput,
            "null coordinator handle".to_string(),
        ));
    }
    let track: TrackInfo = match serde_json::from_str(unsafe { c_str_to_str(track_json) }) {
        Ok(t) => t,
        Err(_) => {
            return coordinator_result_to_c_string(&CoordinatorResult::error(
                CoordinatorErrorKind::InvalidInput,
                "malformed track JSON".to_string(),
            ));
        }
    };
    let queue_tracks: Vec<TrackInfo> = match serde_json::from_str(unsafe { c_str_to_str(queue_tracks_json) })
    {
        Ok(t) => t,
        Err(_) => {
            return coordinator_result_to_c_string(&CoordinatorResult::error(
                CoordinatorErrorKind::InvalidInput,
                "malformed queue tracks JSON".to_string(),
            ));
        }
    };
    let lib = if library.is_null() { None } else { Some(unsafe { &(*library).0 }) };
    let mut coord = unsafe { &(*ptr).inner }.lock().unwrap();
    let result = coord.start(lib, track, queue_tracks, PlayMode::from_i32(mode));
    coordinator_result_to_c_string(&result)
}

/// Advance to the next playable track (bounded skip of unplayable ones).
/// Returns structured result JSON like `rhythm_coordinator_start`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_next(
    ptr: *mut RhythmCoordinator,
    library: *mut RhythmLibrary,
) -> *mut c_char {
    if ptr.is_null() {
        return coordinator_result_to_c_string(&CoordinatorResult::error(
            CoordinatorErrorKind::InvalidInput,
            "null coordinator handle".to_string(),
        ));
    }
    let lib = if library.is_null() { None } else { Some(unsafe { &(*library).0 }) };
    let mut coord = unsafe { &(*ptr).inner }.lock().unwrap();
    let result = coord.next(lib);
    coordinator_result_to_c_string(&result)
}

/// Move to the previous playable track (bounded skip of unplayable ones).
/// Returns structured result JSON like `rhythm_coordinator_start`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_previous(
    ptr: *mut RhythmCoordinator,
    library: *mut RhythmLibrary,
) -> *mut c_char {
    if ptr.is_null() {
        return coordinator_result_to_c_string(&CoordinatorResult::error(
            CoordinatorErrorKind::InvalidInput,
            "null coordinator handle".to_string(),
        ));
    }
    let lib = if library.is_null() { None } else { Some(unsafe { &(*library).0 }) };
    let mut coord = unsafe { &(*ptr).inner }.lock().unwrap();
    let result = coord.previous(lib);
    coordinator_result_to_c_string(&result)
}

/// Toggle play/pause (pause while playing/buffering, resume only when
/// paused, idle-start the first playable library track). Returns structured
/// result JSON like `rhythm_coordinator_start`, including `playback_active`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_toggle_play_pause(
    ptr: *mut RhythmCoordinator,
    library: *mut RhythmLibrary,
) -> *mut c_char {
    if ptr.is_null() {
        return coordinator_result_to_c_string(&CoordinatorResult::error(
            CoordinatorErrorKind::InvalidInput,
            "null coordinator handle".to_string(),
        ));
    }
    let lib = if library.is_null() { None } else { Some(unsafe { &(*library).0 }) };
    let mut coord = unsafe { &(*ptr).inner }.lock().unwrap();
    let result = coord.toggle_play_pause(lib);
    coordinator_result_to_c_string(&result)
}

/// Whether the toggle has something to act on. Returns 1 for true, 0 for
/// false.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_can_toggle_playback(ptr: *mut RhythmCoordinator) -> i32 {
    unsafe { ptr.as_ref().map(|c| c.inner.lock().unwrap().can_toggle_playback() as i32).unwrap_or(0) }
}

/// Whether playback can be stopped (engine playing/buffering/paused).
/// Returns 1 for true, 0 for false.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_can_stop(ptr: *mut RhythmCoordinator) -> i32 {
    unsafe { ptr.as_ref().map(|c| c.inner.lock().unwrap().can_stop() as i32).unwrap_or(0) }
}

/// Sync the queue after a library refresh (#69): replace contents and jump
/// back to the current track by its database id.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_sync_queue(
    ptr: *mut RhythmCoordinator,
    tracks_json: *const c_char,
) {
    if ptr.is_null() { return; }
    let tracks: Vec<TrackInfo> = match serde_json::from_str(unsafe { c_str_to_str(tracks_json) }) {
        Ok(t) => t,
        Err(_) => return,
    };
    let mut coord = unsafe { &(*ptr).inner }.lock().unwrap();
    coord.sync_queue(tracks);
}

/// Stop playback and clear the transport state (current track + queue).
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_stop(ptr: *mut RhythmCoordinator) {
    if let Some(coord) = unsafe { ptr.as_ref() } {
        coord.inner.lock().unwrap().stop();
    }
}

/// Pause playback.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_pause(ptr: *mut RhythmCoordinator) {
    if let Some(coord) = unsafe { ptr.as_ref() } {
        coord.inner.lock().unwrap().player().pause();
    }
}

/// Resume playback (only when the engine is actually Paused).
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_resume(ptr: *mut RhythmCoordinator) {
    if let Some(coord) = unsafe { ptr.as_ref() } {
        coord.inner.lock().unwrap().player().resume();
    }
}

/// Seek to a position in seconds. Returns 0 on success, -1 on error.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_seek(ptr: *mut RhythmCoordinator, seconds: f64) -> i32 {
    if ptr.is_null() { return -1; }
    let coord = unsafe { &(*ptr).inner }.lock().unwrap();
    match coord.player().seek(seconds) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Set volume (0.0 - 1.0).
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_set_volume(ptr: *mut RhythmCoordinator, volume: f32) {
    if let Some(coord) = unsafe { ptr.as_ref() } {
        coord.inner.lock().unwrap().player().set_volume(volume);
    }
}

/// Get current volume.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_get_volume(ptr: *mut RhythmCoordinator) -> f32 {
    unsafe { ptr.as_ref().map(|c| c.inner.lock().unwrap().player().volume()).unwrap_or(0.0) }
}

/// Get current playback position in seconds.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_get_position(ptr: *mut RhythmCoordinator) -> f64 {
    unsafe { ptr.as_ref().map(|c| c.inner.lock().unwrap().player().position()).unwrap_or(0.0) }
}

/// Get media duration in seconds.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_get_duration(ptr: *mut RhythmCoordinator) -> f64 {
    unsafe { ptr.as_ref().map(|c| c.inner.lock().unwrap().player().duration()).unwrap_or(0.0) }
}

/// Get player state: 0=Stopped, 1=Playing, 2=Paused, 3=Buffering, 4=Error, 5=Finished
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_get_state(ptr: *mut RhythmCoordinator) -> i32 {
    unsafe {
        ptr.as_ref()
            .map(|c| {
                let state = c.inner.lock().unwrap().player().state();
                match state {
                    PlayerState::Stopped => 0,
                    PlayerState::Playing => 1,
                    PlayerState::Paused => 2,
                    PlayerState::Buffering => 3,
                    PlayerState::Error(_) => 4,
                    PlayerState::Finished => 5,
                }
            })
            .unwrap_or(-1)
    }
}

/// Why playback failed, when the state is Error (4); null otherwise.
/// Free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_error(ptr: *mut RhythmCoordinator) -> *mut c_char {
    unsafe {
        match ptr.as_ref().and_then(|c| c.inner.lock().unwrap().player().error_message()) {
            Some(message) => str_to_c_string(&message),
            None => std::ptr::null_mut(),
        }
    }
}

/// Classification of the last playback failure, when it was HTTP:
/// "expired" | "cdn_rejected" | "other"; null otherwise (#120).
/// Free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_error_kind(ptr: *mut RhythmCoordinator) -> *mut c_char {
    use crate::HttpErrorKind;
    unsafe {
        match ptr.as_ref().and_then(|c| c.inner.lock().unwrap().player().error_kind()) {
            Some(HttpErrorKind::Expired) => str_to_c_string("expired"),
            Some(HttpErrorKind::CdnRejected) => str_to_c_string("cdn_rejected"),
            Some(HttpErrorKind::Other) => str_to_c_string("other"),
            None => std::ptr::null_mut(),
        }
    }
}

/// Whether the queue has a next track. Returns 1 for true, 0 for false.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_has_next(ptr: *mut RhythmCoordinator) -> i32 {
    unsafe { ptr.as_ref().map(|c| c.inner.lock().unwrap().can_play_next() as i32).unwrap_or(0) }
}

/// Whether the queue has a previous track.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_has_previous(ptr: *mut RhythmCoordinator) -> i32 {
    unsafe { ptr.as_ref().map(|c| c.inner.lock().unwrap().can_play_previous() as i32).unwrap_or(0) }
}

/// Get the current track as JSON, or null when nothing is playing.
/// Free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_current_track(ptr: *mut RhythmCoordinator) -> *mut c_char {
    unsafe {
        match ptr.as_ref().and_then(|c| c.inner.lock().unwrap().current_track().cloned()) {
            Some(track) => str_to_c_string(&track_to_json(&track)),
            None => std::ptr::null_mut(),
        }
    }
}

/// Set the play mode: 0=Sequential, 1=Shuffle, 2=SingleLoop, 3=ListLoop.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_set_play_mode(ptr: *mut RhythmCoordinator, mode: i32) {
    if let Some(coord) = unsafe { ptr.as_ref() } {
        coord.inner.lock().unwrap().set_play_mode(PlayMode::from_i32(mode));
    }
}

/// Get the play mode: 0=Sequential, 1=Shuffle, 2=SingleLoop, 3=ListLoop.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_coordinator_get_play_mode(ptr: *mut RhythmCoordinator) -> i32 {
    unsafe {
        ptr.as_ref()
            .map(|c| c.inner.lock().unwrap().play_mode().to_i32())
            .unwrap_or(0)
    }
}

// ─── M3U8 Import/Export FFI ───────────────────────────────────────

/// Export tracks to an M3U8 file. Returns 0 on success.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_export_m3u8(
    path: *const c_char,
    tracks_json: *const c_char,
) -> i32 {
    let p = unsafe { c_str_to_str(path) };
    let json = unsafe { c_str_to_str(tracks_json) };

    let tracks: Vec<TrackInfo> = match serde_json::from_str(json) {
        Ok(t) => t,
        Err(_) => return -1,
    };

    match playlist::export_m3u8(Path::new(p), &tracks) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Import an M3U8 file. Returns JSON array of entries.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_import_m3u8(path: *const c_char) -> *mut c_char {
    let p = unsafe { c_str_to_str(path) };
    match playlist::import_m3u8(Path::new(p)) {
        Ok(entries) => str_to_c_string(&serde_json::to_string(&entries).unwrap_or_default()),
        Err(_) => std::ptr::null_mut(),
    }
}

// ─── Memory Management ────────────────────────────────────────────

/// Free a string returned by any rhythm_* function.
#[no_mangle]
pub extern "C" fn rhythm_free_string(ptr: *mut c_char) {
    free_c_string(ptr);
}
