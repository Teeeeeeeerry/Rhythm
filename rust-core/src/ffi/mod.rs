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

use crate::coordinator::{
    CoordinatorErrorKind, CoordinatorEvent, CoordinatorResult, PlaybackCoordinator,
};
use crate::library::Library;
use crate::message::{self, MessageLanguage, MessagePlatform};
use crate::metadata;
use crate::playlist;
use crate::queue::PlayMode;
use crate::resolver::{self, ResolveErrorKind};
use crate::{HttpErrorKind, PlayerState, TrackInfo};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::sync::{Arc, Mutex};

// ─── Opaque Handle Types ───────────────────────────────────────────

/// Opaque handle to a Library instance.
pub struct RhythmLibrary(Library);

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

/// Serialize a library import outcome for the seam (#239).
fn import_outcome_to_c_string(outcome: &crate::library::ImportOutcome) -> *mut c_char {
    str_to_c_string(&serde_json::to_string(outcome).unwrap_or_default())
}

/// Import every audio file under a directory (#239).
///
/// Returns the named outcome as snake_case JSON
/// (`{"imported":N,"unsupported":N,"failed":N}`), or null for a null handle.
/// Caller must free the string with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_import_directory(
    ptr: *mut RhythmLibrary,
    dir: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    let path = unsafe { c_str_to_str(dir) };
    import_outcome_to_c_string(&lib.import_directory(Path::new(path)))
}

/// Import a single audio file (#239). Same result shape as the directory
/// path; an unsupported format and a read failure keep their own counts.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_import_single_file(
    ptr: *mut RhythmLibrary,
    file_path: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    let path = unsafe { c_str_to_str(file_path) };
    import_outcome_to_c_string(&lib.import_single_file(Path::new(path)))
}

/// Import a mixed batch of directories and files (#239). `paths_json` is a
/// JSON array of path strings; the "partial success" aggregation happens in
/// the core, so both UI layers render the same counts. A malformed array
/// imports nothing and reports all zeroes.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_library_import_paths(
    ptr: *mut RhythmLibrary,
    paths_json: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    let json = unsafe { c_str_to_str(paths_json) };
    let paths: Vec<std::path::PathBuf> = match serde_json::from_str::<Vec<String>>(json) {
        Ok(list) => list.into_iter().map(std::path::PathBuf::from).collect(),
        Err(e) => {
            log::error!("Import paths payload is not a JSON string array: {e}");
            return import_outcome_to_c_string(&crate::library::ImportOutcome::default());
        }
    };
    import_outcome_to_c_string(&lib.import_paths(&paths))
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

// ─── URL Resolver FFI ─────────────────────────────────────────────

/// Structured result of a resolve call (#176): success payload + classified
/// error in a single return — the old "null, then query `rhythm_last_error`"
/// two-step protocol is gone.
///
/// Success: `{"ok":true,"resolved":{...ResolvedUrl...}}`
/// Failure: `{"ok":false,"error_kind":"invalid_url|yt_dlp_missing|timeout|network|unavailable|no_audio_stream|yt_dlp_outdated|internal","error_message":"..."}`
///
/// Free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_resolve_url(url: *const c_char) -> *mut c_char {
    let u = unsafe { c_str_to_str(url) };
    let json = match resolver::resolve_url(u) {
        Ok(resolved) => serde_json::json!({ "ok": true, "resolved": resolved }),
        Err(failure) => serde_json::json!({
            "ok": false,
            "error_kind": failure.kind,
            "error_message": failure.message,
        }),
    };
    str_to_c_string(&json.to_string())
}

/// Classify a URL. Returns a structured result JSON (#181, no global error
/// slot): `{"ok":true,"source_type":"youtube"}` or
/// `{"ok":false,"error_kind":"...","error_message":"..."}`.
/// Free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_classify_url(url: *const c_char) -> *mut c_char {
    let u = unsafe { c_str_to_str(url) };
    let json = match resolver::classify_url(u) {
        Ok(source_type) => serde_json::json!({ "ok": true, "source_type": source_type }),
        Err(failure) => serde_json::json!({
            "ok": false,
            "error_kind": failure.kind,
            "error_message": failure.message,
        }),
    };
    str_to_c_string(&json.to_string())
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

/// Install or update Rhythm's own yt-dlp copy now. Returns a structured
/// result JSON (#181, no global error slot):
/// `{"ok":true,"path":"..."}` or
/// `{"ok":false,"error_kind":"...","error_message":"..."}`.
/// Blocks for the duration of the download. Free with `rhythm_free_string`.
#[no_mangle]
pub extern "C" fn rhythm_install_ytdlp() -> *mut c_char {
    let json = match resolver::install::update_now() {
        Ok(path) => serde_json::json!({ "ok": true, "path": path.display().to_string() }),
        Err(failure) => serde_json::json!({
            "ok": false,
            "error_kind": failure.kind,
            "error_message": failure.message,
        }),
    };
    str_to_c_string(&json.to_string())
}

// ─── Message Spec FFI (#227) ───────────────────────────────────────
//
// 分类到文案键的分派在核心（`crate::message`）：这些导出返回一条消息
// 规格 JSON `{"segments":[{"segment":"key","key":"...","params":{...}}|
// {"segment":"literal","text":"..."}]}`，双端适配层只按键取模板、按参数
// 填占位符，再顺序拼接。

/// 核心 #120 分类值到枚举；空串与未知值都表示「不是 HTTP 失败」。
fn http_error_kind_from_code(code: &str) -> Option<HttpErrorKind> {
    match code {
        "expired" => Some(HttpErrorKind::Expired),
        "cdn_rejected" => Some(HttpErrorKind::CdnRejected),
        "other" => Some(HttpErrorKind::Other),
        _ => None,
    }
}

/// 播放失败的文案规格。`kind` 是核心的 #120 分类值（`expired` /
/// `cdn_rejected` / `other`，非 HTTP 失败传空串），`detail` 是引擎原文，
/// `language` 是适配层解析出的语言标识（`zh` 开头为中文，其余按英文）。
/// Free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_message_playback_failure(
    kind: *const c_char,
    detail: *const c_char,
    language: *const c_char,
) -> *mut c_char {
    let kind = http_error_kind_from_code(unsafe { c_str_to_str(kind) });
    let detail = unsafe { c_str_to_str(detail) };
    let language = MessageLanguage::from_code(unsafe { c_str_to_str(language) });
    let spec = message::playback_failure(kind, detail, language);
    str_to_c_string(&serde_json::to_string(&spec).unwrap_or_default())
}

/// 核心解析错误分类值到枚举；空串与未知值走回退分支（返回引擎原文）。
fn resolve_error_kind_from_code(code: &str) -> Option<ResolveErrorKind> {
    match code {
        "invalid_url" => Some(ResolveErrorKind::InvalidUrl),
        "yt_dlp_missing" => Some(ResolveErrorKind::YtDlpMissing),
        "timeout" => Some(ResolveErrorKind::Timeout),
        "network" => Some(ResolveErrorKind::Network),
        "unavailable" => Some(ResolveErrorKind::Unavailable),
        "no_audio_stream" => Some(ResolveErrorKind::NoAudioStream),
        "yt_dlp_outdated" => Some(ResolveErrorKind::YtDlpOutdated),
        "internal" => Some(ResolveErrorKind::Internal),
        _ => None,
    }
}

/// 解析失败的文案规格（#229）。`kind` 是核心的 `ResolveErrorKind` 值，
/// `detail` 是引擎原文，`language` 是适配层解析出的语言标识。平台差异
/// （yt-dlp 安装命令的 brew 与 winget 之分）由构建目标决定，调用方无从
/// 传错。Free with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_message_resolve_failure(
    kind: *const c_char,
    detail: *const c_char,
    language: *const c_char,
) -> *mut c_char {
    let kind = resolve_error_kind_from_code(unsafe { c_str_to_str(kind) });
    let detail = unsafe { c_str_to_str(detail) };
    let language = MessageLanguage::from_code(unsafe { c_str_to_str(language) });
    let spec = message::resolve_failure(kind, detail, language, MessagePlatform::current());
    str_to_c_string(&serde_json::to_string(&spec).unwrap_or_default())
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

/// Parse an M3U8 file and import every entry into the library (#234).
///
/// Returns the named import outcome as snake_case JSON
/// (`{"imported":N,"failed":M}`), or null when the playlist cannot be read.
/// The storage rules (location type, title fallback, success test) live in
/// the core (#233) — callers only render the counts. Caller must free the
/// string with `rhythm_free_string`.
#[no_mangle]
///
/// # Safety
/// See the module-level `# Safety` contract.
pub unsafe extern "C" fn rhythm_import_m3u8_into_library(
    ptr: *mut RhythmLibrary,
    path: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let lib = unsafe { &(*ptr).0 };
    let p = unsafe { c_str_to_str(path) };
    match playlist::import_m3u8_into_library(Path::new(p), lib) {
        Ok(outcome) => str_to_c_string(&serde_json::to_string(&outcome).unwrap_or_default()),
        Err(e) => {
            log::error!("M3U8 import failed: {e}");
            std::ptr::null_mut()
        }
    }
}

// ─── Memory Management ────────────────────────────────────────────

/// Free a string returned by any rhythm_* function.
#[no_mangle]
pub extern "C" fn rhythm_free_string(ptr: *mut c_char) {
    free_c_string(ptr);
}
