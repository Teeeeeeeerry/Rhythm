//! FFI layer — C-ABI compatible exports for macOS (Swift) and Windows (C++) UI layers.
//!
//! All functions use opaque pointers to pass Rust objects across FFI boundaries.
//! The UI layer creates/destroys objects via these functions and never accesses
//! Rust internals directly.

use crate::audio::AudioEngine;
use crate::library::Library;
use crate::metadata;
use crate::playlist;
use crate::resolver;
use crate::{PlayerState, TrackInfo};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

// ─── Opaque Handle Types ───────────────────────────────────────────

/// Opaque handle to a Library instance.
pub struct RhythmLibrary(Library);

/// Opaque handle to an AudioEngine instance.
pub struct RhythmPlayer(AudioEngine);

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

/// Get player state: 0=Stopped, 1=Playing, 2=Paused, 3=Buffering, 4=Error
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
            })
            .unwrap_or(-1)
    }
}

// ─── URL Resolver FFI ─────────────────────────────────────────────

/// Resolve a URL to a playable stream. Returns JSON ResolvedUrl or null.
#[no_mangle]
pub extern "C" fn rhythm_resolve_url(url: *const c_char) -> *mut c_char {
    let u = unsafe { c_str_to_str(url) };
    match resolver::resolve_url(u) {
        Ok(resolved) => str_to_c_string(&serde_json::to_string(&resolved).unwrap_or_default()),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Classify a URL. Returns the source type string ("youtube", "bilibili", "direct_url", "local").
#[no_mangle]
pub extern "C" fn rhythm_classify_url(url: *const c_char) -> *mut c_char {
    let u = unsafe { c_str_to_str(url) };
    match resolver::classify_url(u) {
        Ok(source_type) => str_to_c_string(&source_type.to_string()),
        Err(_) => std::ptr::null_mut(),
    }
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
