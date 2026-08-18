//! FF-01–21：FFI 层行为清单（manifest: docs/testing/behavior/ffi.md）。
//!
//! 零接缝：C ABI 直调 + 真实 Library/Queue/Player，tempfile 临时库。
//! 历史回归：#21（解析失败只有 null、无原因）、#79（import_file 返回值契约）。

mod common;

use lofty::prelude::*;
use rhythm_core::ffi::*;
use std::ffi::CString;
use std::sync::Mutex;

// `LAST_RESOLVE_ERROR` is a process-global; resolver tests run serialized so
// they don't observe each other's last_error.
static RESOLVER_LOCK: Mutex<()> = Mutex::new(());

/// Pack a Rust string for FFI (panics on interior NUL — test inputs don't have one).
fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Take ownership of an FFI-returned string (frees it, returns the content).
unsafe fn take(s: *mut std::os::raw::c_char) -> String {
    assert!(!s.is_null(), "expected a non-null string");
    CString::from_raw(s).to_string_lossy().into_owned()
}

fn open_lib(dir: &std::path::Path) -> *mut RhythmLibrary {
    let db = dir.join("test.db");
    let lib = unsafe { rhythm_library_open(c(db.to_str().unwrap()).as_ptr()) };
    assert!(!lib.is_null());
    lib
}

/// A track JSON with a real database id (for queue jump_to by id).
fn track_json(id: i64, title: &str, path: &str) -> String {
    let t = common::test_local_track(path, title, None, 200.0);
    let mut v = serde_json::to_value(&t).unwrap();
    v["id"] = serde_json::json!(id);
    serde_json::to_string(&v).unwrap()
}

// ─── FF-01/02 library open/close ────────────────────────────────────

#[test]
fn ff01_open_close_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());
    unsafe { rhythm_library_close(lib) };
    unsafe { rhythm_library_close(std::ptr::null_mut()) }; // must be safe
}

#[test]
fn ff02_open_failure_returns_null() {
    let dir = tempfile::tempdir().unwrap();
    let lib = unsafe { rhythm_library_open(c(dir.path().to_str().unwrap()).as_ptr()) };
    assert!(lib.is_null(), "a directory is not a valid database path");
}

// ─── FF-03/04 import ────────────────────────────────────────────────

#[test]
fn ff03_import_directory_counts_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    std::fs::create_dir_all(&music).unwrap();
    common::write_wav(&music.join("a.wav"), 0.5);
    common::write_wav(&music.join("b.wav"), 0.5);

    let lib = open_lib(dir.path());
    let count = unsafe { rhythm_library_import(lib, c(music.to_str().unwrap()).as_ptr()) };
    assert_eq!(count, 2);

    // Errors → -1: null pointer and non-directory path.
    assert_eq!(unsafe { rhythm_library_import(std::ptr::null_mut(), c("x").as_ptr()) }, -1);
    let file = music.join("a.wav");
    assert_eq!(unsafe { rhythm_library_import(lib, c(file.to_str().unwrap()).as_ptr()) }, -1);
    unsafe { rhythm_library_close(lib) };
}

/// 文档契约 "1 成功、0 不支持、-1 错误"（#79 修复后启用）。
#[test]
fn ff04_import_file_unsupported_returns_zero() {
    let dir = tempfile::tempdir().unwrap();
    let txt = dir.path().join("note.txt");
    std::fs::write(&txt, b"not audio").unwrap();

    let lib = open_lib(dir.path());
    let ret = unsafe { rhythm_library_import_file(lib, c(txt.to_str().unwrap()).as_ptr()) };
    assert_eq!(ret, 0, "unsupported format must return 0");
    unsafe { rhythm_library_close(lib) };
}

#[test]
fn ff04_import_file_success_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("tone.wav");
    common::write_wav(&wav, 0.5);
    let missing = dir.path().join("missing.mp3");

    let lib = open_lib(dir.path());
    assert_eq!(unsafe { rhythm_library_import_file(lib, c(wav.to_str().unwrap()).as_ptr()) }, 1);
    assert_eq!(unsafe { rhythm_library_import_file(lib, c(missing.to_str().unwrap()).as_ptr()) }, -1);
    assert_eq!(unsafe { rhythm_library_import_file(std::ptr::null_mut(), c("x").as_ptr()) }, -1);
    unsafe { rhythm_library_close(lib) };
}

// ─── FF-05/06 JSON 契约 ─────────────────────────────────────────────

#[test]
fn ff05_string_memory_contract() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    // Empty library still returns a valid JSON string ("[]").
    let empty = unsafe { rhythm_library_get_all_tracks(lib) };
    assert!(!empty.is_null());
    unsafe {
        assert_eq!(take(empty), "[]");
    }

    // After adding a track the JSON carries it; caller frees via take().
    let t = track_json(-1, "FFI Track", "/music/f.mp3");
    let saved = unsafe { rhythm_library_add_track(lib, c(&t).as_ptr()) };
    assert!(!saved.is_null());
    unsafe {
        let saved_str = take(saved);
        assert!(saved_str.contains("\"id\":"));
        assert!(saved_str.contains("FFI Track"));
    }

    // Null pointer → null.
    assert!(unsafe { rhythm_library_get_all_tracks(std::ptr::null_mut()) }.is_null());
    unsafe { rhythm_library_close(lib) };
}

#[test]
fn ff06_add_track_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let json = track_json(-1, "Roundtrip", "/music/r.mp3");
    let saved = unsafe { rhythm_library_add_track(lib, c(&json).as_ptr()) };
    assert!(!saved.is_null());
    unsafe {
        let saved_str = take(saved);
        let v: serde_json::Value = serde_json::from_str(&saved_str).unwrap();
        assert!(v["id"].as_i64().unwrap() >= 1, "must carry the database id");
        assert_eq!(v["title"], "Roundtrip");
    }

    assert!(unsafe { rhythm_library_add_track(lib, c("not json{").as_ptr()) }.is_null());
    unsafe { rhythm_library_close(lib) };
}

// ─── FF-07/08/09/10 player ──────────────────────────────────────────

#[test]
fn ff07_player_create_destroy() {
    let player = rhythm_player_create();
    assert!(!player.is_null());
    unsafe { rhythm_player_destroy(player) };
    unsafe { rhythm_player_destroy(std::ptr::null_mut()) }; // must be safe
}

#[test]
fn ff08_state_code_mapping() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("tone.wav");
    common::write_wav(&wav, 3.0);

    let player = rhythm_player_create();
    assert_eq!(unsafe { rhythm_player_get_state(player) }, 0, "fresh → Stopped");
    unsafe { rhythm_player_play_file(player, c(wav.to_str().unwrap()).as_ptr()) };
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert_eq!(unsafe { rhythm_player_get_state(player) }, 1, "playing → 1");
    unsafe { rhythm_player_pause(player) };
    assert_eq!(unsafe { rhythm_player_get_state(player) }, 2, "paused → 2");
    unsafe { rhythm_player_stop(player) };
    assert_eq!(unsafe { rhythm_player_get_state(player) }, 0, "stopped → 0");
    assert_eq!(unsafe { rhythm_player_get_state(std::ptr::null_mut()) }, -1, "null → -1");
    unsafe { rhythm_player_destroy(player) };

    // Finished (5): play a short file to the end. Buffering (3) is not
    // reachable without a network stream (play_url) and stays unasserted —
    // noted in the manifest.
    let short = dir.path().join("short.wav");
    common::write_wav(&short, 0.3);
    let player2 = rhythm_player_create();
    unsafe { rhythm_player_play_file(player2, c(short.to_str().unwrap()).as_ptr()) };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
    while unsafe { rhythm_player_get_state(player2) } != 5 && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(unsafe { rhythm_player_get_state(player2) }, 5, "finished → 5");
    unsafe { rhythm_player_destroy(player2) };
}

#[test]
fn ff09_player_error_message() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.wav");
    std::fs::write(&bad, b"RIFF").unwrap();

    let player = rhythm_player_create();
    assert!(unsafe { rhythm_player_error(player) }.is_null(), "fresh player has no error");

    unsafe { rhythm_player_play_file(player, c(bad.to_str().unwrap()).as_ptr()) };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while unsafe { rhythm_player_get_state(player) } != 4 && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(unsafe { rhythm_player_get_state(player) }, 4, "corrupted file → Error state");
    unsafe {
        let msg = take(rhythm_player_error(player));
        assert!(!msg.is_empty(), "error message must be non-empty");
    }
    unsafe { rhythm_player_destroy(player) };
}

#[test]
fn ff10_seek_validation() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("tone.wav");
    common::write_wav(&wav, 0.5);

    assert_eq!(unsafe { rhythm_player_seek(std::ptr::null_mut(), 1.0) }, -1, "null → -1");

    let player = rhythm_player_create();
    assert_eq!(unsafe { rhythm_player_seek(player, -1.0) }, -1, "negative → -1");

    unsafe { rhythm_player_play_file(player, c(wav.to_str().unwrap()).as_ptr()) };
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert_eq!(unsafe { rhythm_player_seek(player, 99.0) }, -1, "out of range → -1");
    assert_eq!(unsafe { rhythm_player_seek(player, 0.1) }, 0, "in range → 0");
    unsafe { rhythm_player_destroy(player) };
}

// ─── FF-11 queue 全链路 ─────────────────────────────────────────────

#[test]
fn ff11_queue_full_roundtrip() {
    let t1 = track_json(1, "One", "/music/one.mp3");
    let t2 = track_json(2, "Two", "/music/two.mp3");
    let queue_json = format!("[{t1},{t2}]");

    let q = unsafe { rhythm_queue_create(c(&queue_json).as_ptr()) };
    assert!(!q.is_null());

    unsafe {
        let cur = take(rhythm_queue_current(q));
        assert!(cur.contains("\"One\""), "initial current = first track");
    }
    assert_eq!(unsafe { rhythm_queue_has_next(q) }, 1);
    assert_eq!(unsafe { rhythm_queue_has_previous(q) }, 0);
    unsafe {
        let nxt = take(rhythm_queue_next(q));
        assert!(nxt.contains("\"Two\""));
    }
    // previous() from the second track returns the first.
    unsafe {
        let prev = take(rhythm_queue_previous(q));
        assert!(prev.contains("\"One\""));
    }
    unsafe {
        let nxt = take(rhythm_queue_next(q));
        assert!(nxt.contains("\"Two\""));
    }
    assert_eq!(unsafe { rhythm_queue_has_next(q) }, 0, "sequential queue exhausts");
    assert!(unsafe { rhythm_queue_next(q) }.is_null(), "exhausted → null");
    // Exhaustion parks the cursor past the end (len); previous() steps back
    // onto the last track — locked as current queue semantics.
    unsafe {
        let prev = take(rhythm_queue_previous(q));
        assert!(prev.contains("\"Two\""));
    }
    assert_eq!(unsafe { rhythm_queue_jump_to(q, 1) }, 0);
    unsafe {
        let cur = take(rhythm_queue_current(q));
        assert!(cur.contains("\"One\""));
    }
    assert_eq!(unsafe { rhythm_queue_jump_to(q, 2) }, 0);
    unsafe {
        let cur = take(rhythm_queue_current(q));
        assert!(cur.contains("\"Two\""));
    }
    assert_eq!(unsafe { rhythm_queue_jump_to(q, 999) }, -1, "unknown id → -1");

    // SingleLoop: next() stays at the current track.
    unsafe { rhythm_queue_set_mode(q, 2) };
    unsafe {
        let nxt = take(rhythm_queue_next(q));
        assert!(nxt.contains("\"Two\""));
    }

    // replace with a new list resets to the new first track.
    unsafe { rhythm_queue_replace(q, c(&queue_json).as_ptr()) };
    unsafe {
        let cur = take(rhythm_queue_current(q));
        assert!(cur.contains("\"One\""));
    }

    unsafe { rhythm_queue_destroy(q) };
    unsafe { rhythm_queue_destroy(std::ptr::null_mut()) };
}

// ─── FF-12/13 resolve 与 last_error（#21）───────────────────────────

#[test]
fn ff12_resolve_and_last_error() {
    let _guard = RESOLVER_LOCK.lock().unwrap();

    // Failure → null + last_error carries kind/message (#21).
    let failed = unsafe { rhythm_resolve_url(c("not a url").as_ptr()) };
    assert!(failed.is_null());
    unsafe {
        let err = take(rhythm_last_error());
        let v: serde_json::Value = serde_json::from_str(&err).unwrap();
        assert_eq!(v["kind"], "invalid_url");
        assert!(!v["message"].as_str().unwrap().is_empty());
    }

    // Success → JSON, last_error cleared.
    let resolved = unsafe { rhythm_resolve_url(c("https://example.com/ffi-tone.mp3").as_ptr()) };
    unsafe {
        let json = take(resolved);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["stream_url"], "https://example.com/ffi-tone.mp3");
        assert_eq!(v["source_type"], "direct_url");
    }
    assert!(rhythm_last_error().is_null(), "success must clear last_error");
}

#[test]
fn ff13_classify_url() {
    let _guard = RESOLVER_LOCK.lock().unwrap();

    unsafe {
        assert_eq!(take(rhythm_classify_url(c("https://youtube.com/watch?v=abc").as_ptr())), "youtube");
        assert_eq!(take(rhythm_classify_url(c("https://www.bilibili.com/video/BV1xx411c7mD").as_ptr())), "bilibili");
        assert_eq!(take(rhythm_classify_url(c("https://example.com/song.mp3").as_ptr())), "direct_url");
    }

    let bad = unsafe { rhythm_classify_url(c("plain text").as_ptr()) };
    assert!(bad.is_null(), "unclassifiable input → null");
    unsafe {
        let err = take(rhythm_last_error());
        assert!(err.contains("invalid_url"));
    }
}

// ─── FF-14 M3U8 FFI ─────────────────────────────────────────────────

#[test]
fn ff14_m3u8_export_import() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.m3u8");
    let t = track_json(1, "Export Me", "/music/e.mp3");
    let tracks = format!("[{t}]");

    assert_eq!(unsafe { rhythm_export_m3u8(c(out.to_str().unwrap()).as_ptr(), c(&tracks).as_ptr()) }, 0);
    assert!(out.exists());
    assert_eq!(unsafe { rhythm_export_m3u8(c(out.to_str().unwrap()).as_ptr(), c("bad{").as_ptr()) }, -1);

    let imported = unsafe { rhythm_import_m3u8(c(out.to_str().unwrap()).as_ptr()) };
    unsafe {
        let json = take(imported);
        assert!(json.contains("Export Me"));
    }
    assert!(unsafe { rhythm_import_m3u8(c("/nonexistent/nope.m3u8").as_ptr()) }.is_null());
}

// ─── FF-15/16 memory 与 metadata ────────────────────────────────────

#[test]
fn ff15_free_string_null_is_safe() {
    rhythm_free_string(std::ptr::null_mut());
}

#[test]
fn ff16_metadata_ffi() {
    let dir = tempfile::tempdir().unwrap();
    let tagged = dir.path().join("tagged.wav");
    common::write_tagged_wav(&tagged, 0.5, |tag| {
        tag.set_title(String::from("Meta Title"));
    });

    unsafe {
        let json = take(rhythm_metadata_extract(c(tagged.to_str().unwrap()).as_ptr()));
        assert!(json.contains("Meta Title"));
    }
    assert!(unsafe { rhythm_metadata_extract(c("/nonexistent/x.mp3").as_ptr()) }.is_null());

    let music = dir.path().join("music");
    std::fs::create_dir_all(&music).unwrap();
    common::write_wav(&music.join("a.wav"), 0.5);
    unsafe {
        let json = take(rhythm_metadata_scan(c(music.to_str().unwrap()).as_ptr()));
        assert!(json.contains("a.wav"));
    }
    assert!(unsafe { rhythm_metadata_scan(c(tagged.to_str().unwrap()).as_ptr()) }.is_null(), "file is not a dir");

    // Artwork: picture → path; plain → null.
    let art_wav = dir.path().join("art.wav");
    common::write_tagged_wav(&art_wav, 0.5, |tag| {
        tag.push_picture(lofty::picture::Picture::new_unchecked(
            lofty::picture::PictureType::CoverFront,
            Some(lofty::picture::MimeType::Jpeg),
            None,
            vec![1, 2, 3],
        ));
    });
    let cache = dir.path().join("cache");
    let art = unsafe { rhythm_metadata_extract_artwork(
        c(art_wav.to_str().unwrap()).as_ptr(),
        c(cache.to_str().unwrap()).as_ptr(),
    ) };
    assert!(!art.is_null());
    unsafe { let _ = take(art); }
    let plain = unsafe { rhythm_metadata_extract_artwork(
        c(tagged.to_str().unwrap()).as_ptr(),
        c(cache.to_str().unwrap()).as_ptr(),
    ) };
    assert!(plain.is_null(), "no embedded art → null");
}

// ─── FF-17/18/19 边界 ───────────────────────────────────────────────

#[test]
fn ff17_empty_path_inputs_do_not_crash() {
    // SQLite treats an empty filename as a temporary database, so open("")
    // succeeds — the manifest's core assertion is "no crash"; locked as-is.
    let lib = unsafe { rhythm_library_open(c("").as_ptr()) };
    if !lib.is_null() {
        unsafe { rhythm_library_close(lib) };
    }
    assert_eq!(unsafe { rhythm_library_import(std::ptr::null_mut(), c("").as_ptr()) }, -1);
}

#[test]
fn ff18_queue_create_invalid_json_is_null() {
    assert!(unsafe { rhythm_queue_create(c("not json").as_ptr()) }.is_null());
}

#[test]
fn ff19_queue_replace_invalid_json_keeps_queue() {
    let t1 = track_json(1, "Keep Me", "/music/k.mp3");
    let q = unsafe { rhythm_queue_create(c(&format!("[{t1}]")).as_ptr()) };

    unsafe { rhythm_queue_replace(q, c("bad json{").as_ptr()) };
    unsafe {
        let cur = take(rhythm_queue_current(q));
        assert!(cur.contains("Keep Me"), "invalid replace must not touch the queue");
    }
    unsafe { rhythm_queue_destroy(q) };
}

// ─── FF-20 错误码函数空指针安全默认 ─────────────────────────────────

#[test]
fn ff20_null_pointer_safe_defaults() {
    // resolve() touches LAST_RESOLVE_ERROR; serialize with the resolver tests.
    let _guard = RESOLVER_LOCK.lock().unwrap();
    let null_lib: *mut RhythmLibrary = std::ptr::null_mut();
    let null_player: *mut RhythmPlayer = std::ptr::null_mut();
    let null_queue: *mut RhythmQueue = std::ptr::null_mut();

    unsafe { rhythm_library_close(null_lib) };
    assert_eq!(unsafe { rhythm_library_import(null_lib, c("x").as_ptr()) }, -1);
    assert_eq!(unsafe { rhythm_library_import_file(null_lib, c("x").as_ptr()) }, -1);
    assert!(unsafe { rhythm_library_get_all_tracks(null_lib) }.is_null());
    assert!(unsafe { rhythm_library_search(null_lib, c("x").as_ptr()) }.is_null());
    assert!(unsafe { rhythm_library_add_track(null_lib, c("{}").as_ptr()) }.is_null());
    assert_eq!(unsafe { rhythm_library_remove_track(null_lib, 1) }, -1);
    assert!(unsafe { rhythm_library_verify_files(null_lib) }.is_null());
    assert!(unsafe { rhythm_library_get_playlists(null_lib) }.is_null());
    assert_eq!(unsafe { rhythm_library_create_playlist(null_lib, c("n").as_ptr(), c("d").as_ptr()) }, -1);
    assert_eq!(unsafe { rhythm_library_playlist_add(null_lib, 1, 2) }, -1);
    assert_eq!(unsafe { rhythm_library_playlist_remove(null_lib, 1, 2) }, -1);
    assert_eq!(unsafe { rhythm_library_delete_playlist(null_lib, 1) }, -1);
    assert_eq!(unsafe { rhythm_library_record_play(null_lib, 1) }, -1);

    unsafe { rhythm_player_destroy(null_player) };
    assert_eq!(unsafe { rhythm_player_play_file(null_player, c("x").as_ptr()) }, -1);
    assert_eq!(unsafe { rhythm_player_play_url(null_player, c("x").as_ptr()) }, -1);
    unsafe { rhythm_player_pause(null_player) };
    unsafe { rhythm_player_resume(null_player) };
    unsafe { rhythm_player_stop(null_player) };
    unsafe { rhythm_player_set_volume(null_player, 0.5) };
    assert_eq!(unsafe { rhythm_player_get_volume(null_player) }, 0.0);
    assert_eq!(unsafe { rhythm_player_seek(null_player, 1.0) }, -1);
    assert_eq!(unsafe { rhythm_player_get_position(null_player) }, 0.0);
    assert_eq!(unsafe { rhythm_player_get_duration(null_player) }, 0.0);
    assert_eq!(unsafe { rhythm_player_get_state(null_player) }, -1);
    assert!(unsafe { rhythm_player_error(null_player) }.is_null());

    unsafe { rhythm_queue_destroy(null_queue) };
    assert!(unsafe { rhythm_queue_current(null_queue) }.is_null());
    assert!(unsafe { rhythm_queue_next(null_queue) }.is_null());
    assert!(unsafe { rhythm_queue_previous(null_queue) }.is_null());
    unsafe { rhythm_queue_set_mode(null_queue, 1) };
    assert_eq!(unsafe { rhythm_queue_jump_to(null_queue, 1) }, -1);
    unsafe { rhythm_queue_replace(null_queue, c("[]").as_ptr()) };
    assert_eq!(unsafe { rhythm_queue_has_next(null_queue) }, 0);
    assert_eq!(unsafe { rhythm_queue_has_previous(null_queue) }, 0);

    assert!(unsafe { rhythm_metadata_extract(std::ptr::null_mut()) }.is_null());
    assert!(unsafe { rhythm_metadata_scan(std::ptr::null_mut()) }.is_null());
    assert!(unsafe { rhythm_metadata_extract_artwork(std::ptr::null_mut(), std::ptr::null_mut()) }.is_null());
    assert!(unsafe { rhythm_resolve_url(std::ptr::null_mut()) }.is_null());
    assert!(unsafe { rhythm_classify_url(std::ptr::null_mut()) }.is_null());
}

// ─── FF-21 大 JSON 往返 ─────────────────────────────────────────────

#[test]
fn ff21_large_json_roundtrip() {
    let tracks: Vec<serde_json::Value> = (0..300)
        .map(|i| {
            let t = common::test_local_track(&format!("/music/{i}.mp3"), &format!("Track {i}"), None, 200.0);
            let mut v = serde_json::to_value(&t).unwrap();
            v["id"] = serde_json::json!(i + 1);
            v
        })
        .collect();
    let json = serde_json::to_string(&tracks).unwrap();

    let q = unsafe { rhythm_queue_create(c(&json).as_ptr()) };
    assert!(!q.is_null());
    unsafe {
        let cur = take(rhythm_queue_current(q));
        assert!(cur.contains("\"Track 0\""));
        // Walk to the end — no truncation, no panic.
        let mut last = cur;
        loop {
            let nxt = rhythm_queue_next(q);
            if nxt.is_null() {
                break;
            }
            last = take(nxt);
        }
        assert!(last.contains("\"Track 299\""), "roundtrip must reach the last track");
    }
    unsafe { rhythm_queue_destroy(q) };
}

// ─── FF-22 remove_track 不存在的 id 返回 -1（#98） ──────────────────

#[test]
fn ff22_remove_track_missing_id_returns_minus_one() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    // 空库：任何 id 都不存在 → -1（Swift 侧判 false）。
    assert_eq!(unsafe { rhythm_library_remove_track(lib, 999) }, -1);

    // 加入一首真曲目后：删一次成功，再删同一 id → -1。
    let t = track_json(0, "Gone", "/music/g.mp3");
    let saved = unsafe { rhythm_library_add_track(lib, c(&t).as_ptr()) };
    assert!(!saved.is_null());
    unsafe {
        let v: serde_json::Value = serde_json::from_str(&take(saved)).unwrap();
        let id = v["id"].as_i64().unwrap();
        assert_eq!(rhythm_library_remove_track(lib, id), 0);
        assert_eq!(rhythm_library_remove_track(lib, id), -1, "second delete must report missing (#98)");
    }
    unsafe { rhythm_library_close(lib) };
}
