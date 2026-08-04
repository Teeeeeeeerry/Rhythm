//! FFI layer — C-ABI compatible exports for macOS (Swift) and Windows (C++) UI layers.
//!
//! All functions use opaque pointers to pass Rust objects across FFI boundaries.
//! The UI layer creates/destroys objects via these functions and never accesses
//! Rust internals directly.

use crate::audio::AudioEngine;
use crate::library::Library;
use crate::metadata;
use crate::playlist;
use crate::queue::{PlayMode, PlayQueue};
use crate::resolver;
use crate::{PlayerState, TrackInfo};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

// ─── Opaque Handle Types ───────────────────────────────────────────

/// Opaque handle to a Library instance.
pub struct RhythmLibrary(Library);

/// Opaque handle to an AudioEngine instance.
pub struct RhythmPlayer(AudioEngine);

/// Opaque handle to a PlayQueue instance.
pub struct RhythmQueue(Mutex<PlayQueue>);

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
pub extern "C" fn rhythm_library_open(db_path: *const c_char) -> *mut RhythmLibrary {
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
pub extern "C" fn rhythm_library_close(ptr: *mut RhythmLibrary) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Scan a directory and import all audio files into the library.
/// Returns the number of tracks imported, or -1 on error.
#[no_mangle]
pub extern "C" fn rhythm_library_import(ptr: *mut RhythmLibrary, dir: *const c_char) -> i32 {
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

/// Get all tracks as a JSON string. Caller must free with `rhythm_free_string`.
#[no_mangle]
pub extern "C" fn rhythm_library_get_all_tracks(ptr: *mut RhythmLibrary) -> *mut c_char {
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
pub extern "C" fn rhythm_library_search(
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
pub extern "C" fn rhythm_library_add_track(
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
pub extern "C" fn rhythm_library_remove_track(ptr: *mut RhythmLibrary, track_id: i64) -> i32 {
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
pub extern "C" fn rhythm_library_verify_files(ptr: *mut RhythmLibrary) -> *mut c_char {
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
pub extern "C" fn rhythm_library_get_playlists(ptr: *mut RhythmLibrary) -> *mut c_char {
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
pub extern "C" fn rhythm_library_create_playlist(
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
    match lib.create_playlist(n, d) {
        Ok(id) => id,
        Err(_) => -1,
    }
}

/// Add a track to a playlist. Returns 0 on success.
#[no_mangle]
pub extern "C" fn rhythm_library_playlist_add(
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
pub extern "C" fn rhythm_library_playlist_remove(
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
pub extern "C" fn rhythm_library_delete_playlist(ptr: *mut RhythmLibrary, playlist_id: i64) -> i32 {
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
pub extern "C" fn rhythm_library_record_play(ptr: *mut RhythmLibrary, track_id: i64) -> i32 {
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
pub extern "C" fn rhythm_metadata_extract(path: *const c_char) -> *mut c_char {
    let p = unsafe { c_str_to_str(path) };
    match metadata::extract_track_info(Path::new(p)) {
        Ok(track) => str_to_c_string(&track_to_json(&track)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Scan a directory for audio files. Returns JSON array of TrackInfo.
#[no_mangle]
pub extern "C" fn rhythm_metadata_scan(dir: *const c_char) -> *mut c_char {
    let p = unsafe { c_str_to_str(dir) };
    match metadata::scan_directory(Path::new(p)) {
        Ok(tracks) => str_to_c_string(&serde_json::to_string(&tracks).unwrap_or_default()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Extract cover art from a local audio file and save to cache directory.
/// Returns the file path of the saved artwork, or null if none found.
#[no_mangle]
pub extern "C" fn rhythm_metadata_extract_artwork(
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
pub extern "C" fn rhythm_player_destroy(ptr: *mut RhythmPlayer) {
    if !ptr.is_null() {
        let player = unsafe { Box::from_raw(ptr) };
        player.0.stop();
    }
}

/// Play a local file by path. Returns 0 on success.
#[no_mangle]
pub extern "C" fn rhythm_player_play_file(
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
pub extern "C" fn rhythm_player_play_url(
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
pub extern "C" fn rhythm_player_pause(ptr: *mut RhythmPlayer) {
    if let Some(player) = unsafe { ptr.as_ref() } {
        player.0.pause();
    }
}

/// Resume playback.
#[no_mangle]
pub extern "C" fn rhythm_player_resume(ptr: *mut RhythmPlayer) {
    if let Some(player) = unsafe { ptr.as_ref() } {
        player.0.resume();
    }
}

/// Stop playback.
#[no_mangle]
pub extern "C" fn rhythm_player_stop(ptr: *mut RhythmPlayer) {
    if let Some(player) = unsafe { ptr.as_ref() } {
        player.0.stop();
    }
}

/// Set volume (0.0 - 1.0).
#[no_mangle]
pub extern "C" fn rhythm_player_set_volume(ptr: *mut RhythmPlayer, volume: f32) {
    if let Some(player) = unsafe { ptr.as_ref() } {
        player.0.set_volume(volume);
    }
}

/// Get current volume.
#[no_mangle]
pub extern "C" fn rhythm_player_get_volume(ptr: *mut RhythmPlayer) -> f32 {
    unsafe { ptr.as_ref().map(|p| p.0.volume()).unwrap_or(0.0) }
}

/// Get current playback position in seconds.
#[no_mangle]
pub extern "C" fn rhythm_player_get_position(ptr: *mut RhythmPlayer) -> f64 {
    unsafe { ptr.as_ref().map(|p| p.0.position()).unwrap_or(0.0) }
}

/// Get media duration in seconds.
#[no_mangle]
pub extern "C" fn rhythm_player_get_duration(ptr: *mut RhythmPlayer) -> f64 {
    unsafe { ptr.as_ref().map(|p| p.0.duration()).unwrap_or(0.0) }
}

/// Get player state: 0=Stopped, 1=Playing, 2=Paused, 3=Buffering, 4=Error, 5=Finished
#[no_mangle]
pub extern "C" fn rhythm_player_get_state(ptr: *mut RhythmPlayer) -> i32 {
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
pub extern "C" fn rhythm_player_error(ptr: *mut RhythmPlayer) -> *mut c_char {
    unsafe {
        match ptr.as_ref().map(|p| p.0.state()) {
            Some(PlayerState::Error(message)) => str_to_c_string(&message),
            _ => std::ptr::null_mut(),
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
pub extern "C" fn rhythm_resolve_url(url: *const c_char) -> *mut c_char {
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
pub extern "C" fn rhythm_classify_url(url: *const c_char) -> *mut c_char {
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

// ─── Play Queue FFI ───────────────────────────────────────────────

/// Create a new play queue from a JSON array of tracks.
/// Returns opaque handle, or null on error.
#[no_mangle]
pub extern "C" fn rhythm_queue_create(tracks_json: *const c_char) -> *mut RhythmQueue {
    let json = unsafe { c_str_to_str(tracks_json) };
    let tracks: Vec<TrackInfo> = match serde_json::from_str(json) {
        Ok(t) => t,
        Err(_) => return std::ptr::null_mut(),
    };
    Box::into_raw(Box::new(RhythmQueue(Mutex::new(PlayQueue::new(tracks)))))
}

/// Destroy a queue handle.
#[no_mangle]
pub extern "C" fn rhythm_queue_destroy(ptr: *mut RhythmQueue) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr); }
    }
}

/// Get the current track as JSON. Caller must free with `rhythm_free_string`.
#[no_mangle]
pub extern "C" fn rhythm_queue_current(ptr: *mut RhythmQueue) -> *mut c_char {
    if ptr.is_null() { return std::ptr::null_mut(); }
    let q = unsafe { &(*ptr).0 };
    let guard = q.lock().unwrap();
    match guard.current() {
        Some(t) => str_to_c_string(&serde_json::to_string(t).unwrap_or_default()),
        None => std::ptr::null_mut(),
    }
}

/// Advance to the next track and return it as JSON. Returns null if exhausted.
#[no_mangle]
pub extern "C" fn rhythm_queue_next(ptr: *mut RhythmQueue) -> *mut c_char {
    if ptr.is_null() { return std::ptr::null_mut(); }
    let q = unsafe { &(*ptr).0 };
    let mut guard = q.lock().unwrap();
    match guard.next() {
        Some(t) => str_to_c_string(&serde_json::to_string(t).unwrap_or_default()),
        None => std::ptr::null_mut(),
    }
}

/// Move to the previous track and return it as JSON.
#[no_mangle]
pub extern "C" fn rhythm_queue_previous(ptr: *mut RhythmQueue) -> *mut c_char {
    if ptr.is_null() { return std::ptr::null_mut(); }
    let q = unsafe { &(*ptr).0 };
    let mut guard = q.lock().unwrap();
    match guard.previous() {
        Some(t) => str_to_c_string(&serde_json::to_string(t).unwrap_or_default()),
        None => std::ptr::null_mut(),
    }
}

/// Set the play mode: 0=Sequential, 1=Shuffle, 2=SingleLoop, 3=ListLoop
#[no_mangle]
pub extern "C" fn rhythm_queue_set_mode(ptr: *mut RhythmQueue, mode: i32) {
    if ptr.is_null() { return; }
    let q = unsafe { &(*ptr).0 };
    let mut guard = q.lock().unwrap();
    guard.set_mode(PlayMode::from_i32(mode));
}

/// Jump to a specific track by ID. Returns 0 on success, -1 if not found.
#[no_mangle]
pub extern "C" fn rhythm_queue_jump_to(ptr: *mut RhythmQueue, track_id: i64) -> i32 {
    if ptr.is_null() { return -1; }
    let q = unsafe { &(*ptr).0 };
    let mut guard = q.lock().unwrap();
    if guard.jump_to(track_id) { 0 } else { -1 }
}

/// Replace the queue contents with a new track list.
#[no_mangle]
pub extern "C" fn rhythm_queue_replace(ptr: *mut RhythmQueue, tracks_json: *const c_char) {
    if ptr.is_null() { return; }
    let json = unsafe { c_str_to_str(tracks_json) };
    let tracks: Vec<TrackInfo> = match serde_json::from_str(json) {
        Ok(t) => t,
        Err(_) => return,
    };
    let q = unsafe { &(*ptr).0 };
    let mut guard = q.lock().unwrap();
    guard.replace(tracks);
}

/// Whether the queue has a next track. Returns 1 for true, 0 for false.
#[no_mangle]
pub extern "C" fn rhythm_queue_has_next(ptr: *mut RhythmQueue) -> i32 {
    if ptr.is_null() { return 0; }
    let q = unsafe { &(*ptr).0 };
    q.lock().unwrap().has_next() as i32
}

/// Whether the queue has a previous track.
#[no_mangle]
pub extern "C" fn rhythm_queue_has_previous(ptr: *mut RhythmQueue) -> i32 {
    if ptr.is_null() { return 0; }
    let q = unsafe { &(*ptr).0 };
    q.lock().unwrap().has_previous() as i32
}

// ─── M3U8 Import/Export FFI ───────────────────────────────────────

/// Export tracks to an M3U8 file. Returns 0 on success.
#[no_mangle]
pub extern "C" fn rhythm_export_m3u8(
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
pub extern "C" fn rhythm_import_m3u8(path: *const c_char) -> *mut c_char {
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
