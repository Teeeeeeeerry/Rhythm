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

/// A track JSON with a real database id.
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


// ─── FF-12/13 resolve 与 last_error（#21）───────────────────────────

#[test]
fn ff12_resolve_returns_structured_result() {
    let _guard = RESOLVER_LOCK.lock().unwrap();

    // Failure → one structured result carries kind + message (#176); the
    // global error slot is not used by this call any more.
    let failed = unsafe { rhythm_resolve_url(c("not a url").as_ptr()) };
    assert!(!failed.is_null(), "resolve must never return null now");
    unsafe {
        let json = take(failed);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error_kind"], "invalid_url");
        assert!(!v["error_message"].as_str().unwrap().is_empty());
    }

    // Success → structured payload.
    let resolved = unsafe { rhythm_resolve_url(c("https://example.com/ffi-tone.mp3").as_ptr()) };
    unsafe {
        let json = take(resolved);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["resolved"]["stream_url"], "https://example.com/ffi-tone.mp3");
        assert_eq!(v["resolved"]["source_type"], "direct_url");
    }
}

#[test]
fn ff13_classify_url() {
    let _guard = RESOLVER_LOCK.lock().unwrap();

    // #181: classify returns a structured result.
    unsafe {
        for (url, expected) in [
            ("https://youtube.com/watch?v=abc", "youtube"),
            ("https://www.bilibili.com/video/BV1xx411c7mD", "bilibili"),
            ("https://example.com/song.mp3", "direct_url"),
        ] {
            let json = take(rhythm_classify_url(c(url).as_ptr()));
            let v: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(v["ok"], true);
            assert_eq!(v["source_type"], expected);
        }
    }

    // #181: classify returns a structured result — no global error slot.
    let bad = unsafe { rhythm_classify_url(c("plain text").as_ptr()) };
    unsafe {
        let json = take(bad);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
        assert_eq!(v["error_kind"], "invalid_url");
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

/// FF-23: the parse-and-store export returns the named outcome as JSON
/// (#234) — the UI decodes counts, never re-derives them.
#[test]
fn ff23_m3u8_import_into_library_returns_named_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let playlist = dir.path().join("list.m3u8");
    std::fs::write(
        &playlist,
        "#EXTM3U\n#EXTINF:180,A - One\n/music/one.mp3\n#EXTINF:0,B - Two\nhttps://example.com/two.mp3\n",
    )
    .unwrap();

    let lib = open_lib(dir.path());
    let json = unsafe {
        take(rhythm_import_m3u8_into_library(
            lib,
            c(playlist.to_str().unwrap()).as_ptr(),
        ))
    };
    let outcome: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(outcome["imported"], 2, "snake_case named counts, not a magic number");
    assert_eq!(outcome["failed"], 0);

    // The tracks really are in the library, both source types mapped.
    let tracks = unsafe { take(rhythm_library_get_all_tracks(lib)) };
    assert!(tracks.contains("\"local\"") && tracks.contains("\"direct_url\""));

    // Unreadable playlist and a null handle both report null.
    assert!(unsafe {
        rhythm_import_m3u8_into_library(lib, c("/nonexistent/nope.m3u8").as_ptr())
    }
    .is_null());
    assert!(unsafe {
        rhythm_import_m3u8_into_library(std::ptr::null_mut(), c(playlist.to_str().unwrap()).as_ptr())
    }
    .is_null());
    unsafe { rhythm_library_close(lib) };
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

// ─── FF-20 错误码函数空指针安全默认 ─────────────────────────────────

#[test]
fn ff20_null_pointer_safe_defaults() {
    // resolve() touches LAST_RESOLVE_ERROR; serialize with the resolver tests.
    let _guard = RESOLVER_LOCK.lock().unwrap();
    let null_lib: *mut RhythmLibrary = std::ptr::null_mut();

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

    assert!(unsafe { rhythm_metadata_extract(std::ptr::null_mut()) }.is_null());
    assert!(unsafe { rhythm_metadata_scan(std::ptr::null_mut()) }.is_null());
    assert!(unsafe { rhythm_metadata_extract_artwork(std::ptr::null_mut(), std::ptr::null_mut()) }.is_null());
    // #176: resolve never returns null — a null input is a classified error
    // in the structured result.
    unsafe {
        let json = take(rhythm_resolve_url(std::ptr::null_mut()));
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
    }
    // #181: classify never returns null — a structured result always comes
    // back (null input is a classified failure).
    unsafe {
        let json = take(rhythm_classify_url(std::ptr::null_mut()));
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["ok"], false);
    }
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
