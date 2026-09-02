//! PL-01–14：Playlist（M3U8）行为清单（manifest: docs/testing/behavior/playlist-m3u8.md）。
//!
//! 零接缝：纯文件 IO + tempfile；入库段用真实 SQLite 临时库。历史回归：
//! #34（M3U8 导出静默失败）、#136/#173（解析结果从未入库，同一缺陷在两平台各修一次）。

mod common;

use rhythm_core::library::Library;
use rhythm_core::playlist::{
    export_m3u8, import_entries_into_library, import_m3u8, import_m3u8_into_library, M3u8Entry,
};
use rhythm_core::{SourceType, TrackInfo};

fn track(
    title: &str,
    artist: Option<&str>,
    source_type: SourceType,
    file_path: Option<&str>,
    source_url: Option<&str>,
    duration: f64,
) -> TrackInfo {
    let mut t = match source_type {
        SourceType::Local => {
            common::test_local_track(file_path.unwrap_or_default(), title, artist, duration)
        }
        _ => common::test_url_track(source_url.unwrap_or_default(), title, artist, source_type, duration),
    };
    // The common builder defaults a local album; M3U8 tracks don't carry one.
    t.album = None;
    t
}

fn local(title: &str, artist: Option<&str>, path: &str) -> TrackInfo {
    track(title, artist, SourceType::Local, Some(path), None, 180.0)
}

fn url(title: &str, artist: Option<&str>, url: &str) -> TrackInfo {
    track(title, artist, SourceType::DirectUrl, None, Some(url), 240.0)
}

// ─── PL-01/02/03 export 格式 ────────────────────────────────────────

#[test]
fn pl01_export_m3u8_format_and_locations() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.m3u8");

    let tracks = vec![
        local("Local Title", Some("Local Artist"), "/music/a.mp3"),
        url("URL Title", Some("URL Artist"), "https://example.com/b.mp3"),
    ];
    export_m3u8(&out, &tracks).unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines[0], "#EXTM3U");
    assert_eq!(lines[1], "#EXTINF:180,Local Artist - Local Title");
    assert_eq!(lines[2], "/music/a.mp3", "local → file_path");
    assert_eq!(lines[3], "#EXTINF:240,URL Artist - URL Title");
    assert_eq!(lines[4], "https://example.com/b.mp3", "URL → source_url");
}

#[test]
fn pl03_export_m3u8_missing_artist_falls_back_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.m3u8");

    export_m3u8(&out, &[local("Bare Title", None, "/music/b.mp3")]).unwrap();
    let content = std::fs::read_to_string(&out).unwrap();
    assert!(content.contains("#EXTINF:180,Unknown Artist - Bare Title"));
}

#[test]
fn pl10_export_m3u8_empty_tracks_header_only() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.m3u8");

    export_m3u8(&out, &[]).unwrap();
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "#EXTM3U\n");
}

#[test]
fn pl13_export_m3u8_truncates_duration_to_integer_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.m3u8");

    let mut fractional = local("F", Some("A"), "/f.mp3");
    fractional.duration = 180.9;
    let mut negative = local("N", Some("B"), "/n.mp3");
    negative.duration = -5.0;
    export_m3u8(&out, &[fractional, negative]).unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    assert!(content.contains("#EXTINF:180,A - F"), "fractional seconds truncate");
    assert!(content.contains("#EXTINF:-5,B - N"), "negative durations must not panic");
}

// ─── PL-09 导出失败（#34）───────────────────────────────────────────

#[test]
fn pl09_export_m3u8_unwritable_path_errors() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("no/such/dir/out.m3u8");

    assert!(export_m3u8(&out, &[local("T", None, "/t.mp3")]).is_err());
}

// ─── PL-04/05 import 标准解析 ───────────────────────────────────────

#[test]
fn pl04_import_m3u8_standard_parsing_preserves_order() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.m3u8");
    std::fs::write(&file, "#EXTM3U\n#EXTINF:180,Artist A - Title A\n/music/a.mp3\n#EXTINF:240,Artist B - Title B\nhttps://x.com/b.mp3\n").unwrap();

    let entries = import_m3u8(&file).unwrap();
    assert_eq!(
        entries,
        vec![
            M3u8Entry { title: "Title A".to_string(), artist: Some("Artist A".to_string()), location: "/music/a.mp3".to_string() },
            M3u8Entry { title: "Title B".to_string(), artist: Some("Artist B".to_string()), location: "https://x.com/b.mp3".to_string() },
        ]
    );
}

#[test]
fn pl05_import_m3u8_skips_header_and_blank_lines() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.m3u8");
    std::fs::write(&file, "#EXTM3U\n\n\n#EXTINF:180,A - T\n/x.mp3\n\n").unwrap();

    let entries = import_m3u8(&file).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "T");
}

#[test]
fn pl06_import_m3u8_extinf_without_separator() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.m3u8");
    std::fs::write(&file, "#EXTM3U\n#EXTINF:180,Some Title\n/x.mp3\n").unwrap();

    let entries = import_m3u8(&file).unwrap();
    assert_eq!(entries[0].title, "Some Title", "whole text after comma is the title");
    assert_eq!(entries[0].artist, None, "no ` - ` separator → no artist");
}

#[test]
fn pl07_import_m3u8_location_without_extinf_falls_back_to_stem() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.m3u8");
    std::fs::write(&file, "#EXTM3U\n/music/song.mp3\nhttps://x.com/a.mp3\n").unwrap();

    let entries = import_m3u8(&file).unwrap();
    assert_eq!(entries[0].title, "song", "title falls back to the file stem");
    assert_eq!(entries[0].artist, None);
    assert_eq!(entries[1].title, "a");
}

#[test]
fn pl08_import_m3u8_skips_other_comment_lines() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.m3u8");
    std::fs::write(&file, "#EXTM3U\n#PLAYLIST:My List\n#EXTGRP:Group\n#EXTINF:180,A - T\n/x.mp3\n").unwrap();

    let entries = import_m3u8(&file).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "T");
}

// ─── PL-11/14 错误路径 ──────────────────────────────────────────────

#[test]
fn pl11_import_m3u8_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    assert!(import_m3u8(&dir.path().join("nope.m3u8")).is_err());
}

#[test]
fn pl14_import_m3u8_invalid_utf8_errors() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.m3u8");
    std::fs::write(&file, b"#EXTM3U\n#EXTINF:180,A - T\n\xFF\xFE garbage\n").unwrap();

    assert!(import_m3u8(&file).is_err());
}

// ─── PL-12 往返保真 ─────────────────────────────────────────────────

#[test]
fn pl12_export_then_import_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("rt.m3u8");

    let tracks = vec![
        local("Local One", Some("Artist One"), "/music/one.mp3"),
        url("URL Two", Some("Artist Two"), "https://example.com/two.mp3"),
        local("No Artist", None, "/music/three.mp3"),
    ];
    export_m3u8(&out, &tracks).unwrap();

    let entries = import_m3u8(&out).unwrap();
    let expected: Vec<M3u8Entry> = vec![
        M3u8Entry { title: "Local One".to_string(), artist: Some("Artist One".to_string()), location: "/music/one.mp3".to_string() },
        M3u8Entry { title: "URL Two".to_string(), artist: Some("Artist Two".to_string()), location: "https://example.com/two.mp3".to_string() },
        M3u8Entry { title: "No Artist".to_string(), artist: Some("Unknown Artist".to_string()), location: "/music/three.mp3".to_string() },
    ];
    assert_eq!(entries, expected, "title/artist/location must survive a full roundtrip");
}


// ─── PL-17–23 解析并入库（#233） ────────────────────────────────────

fn open_lib(dir: &std::path::Path) -> Library {
    Library::open(&dir.join("test.db")).unwrap()
}

/// Write a playlist file whose entries are `(extinf, location)` pairs.
fn write_playlist(path: &std::path::Path, lines: &[(&str, &str)]) {
    let mut body = String::from("#EXTM3U\n");
    for (extinf, location) in lines {
        if !extinf.is_empty() {
            body.push_str(&format!("#EXTINF:{extinf}\n"));
        }
        body.push_str(location);
        body.push('\n');
    }
    std::fs::write(path, body).unwrap();
}

fn entry(title: &str, artist: Option<&str>, location: &str) -> M3u8Entry {
    M3u8Entry {
        title: title.to_string(),
        artist: artist.map(str::to_string),
        location: location.to_string(),
    }
}

#[test]
fn pl17_import_into_library_all_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());
    let file = dir.path().join("all.m3u8");
    write_playlist(
        &file,
        &[
            ("180,Artist One - One", "/music/one.mp3"),
            ("200,Artist Two - Two", "/music/two.mp3"),
        ],
    );

    let outcome = import_m3u8_into_library(&file, &lib).unwrap();

    assert_eq!(outcome.imported, 2);
    assert_eq!(outcome.failed, 0);
    let titles: Vec<String> = lib
        .get_all_tracks()
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, vec!["One".to_string(), "Two".to_string()]);
}

#[test]
fn pl18_import_into_library_empty_location_counts_as_failed() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let outcome = import_entries_into_library(
        &[
            entry("Good", None, "/music/good.mp3"),
            entry("No Location", None, ""),
        ],
        &lib,
    );

    assert_eq!(outcome.imported, 1);
    assert_eq!(outcome.failed, 1, "an entry with no location cannot be stored");
    assert_eq!(lib.get_all_tracks().unwrap().len(), 1);
}

#[test]
fn pl19_import_into_library_maps_mixed_sources() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());
    let file = dir.path().join("mixed.m3u8");
    write_playlist(
        &file,
        &[
            ("180,Local Artist - Local Song", "/music/local.mp3"),
            ("0,Remote Artist - Remote Song", "https://example.com/remote.mp3"),
            ("0,Plain Artist - Plain Song", "http://example.com/plain.mp3"),
        ],
    );

    let outcome = import_m3u8_into_library(&file, &lib).unwrap();
    assert_eq!((outcome.imported, outcome.failed), (3, 0));

    let tracks = lib.get_all_tracks().unwrap();
    let local = tracks.iter().find(|t| t.title == "Local Song").unwrap();
    assert_eq!(local.source_type, SourceType::Local);
    assert_eq!(local.file_path.as_deref(), Some("/music/local.mp3"));
    assert_eq!(local.source_url, None);

    for title in ["Remote Song", "Plain Song"] {
        let remote = tracks.iter().find(|t| t.title == title).unwrap();
        assert_eq!(remote.source_type, SourceType::DirectUrl, "http(s) → direct_url");
        assert!(remote.source_url.is_some());
        assert_eq!(remote.file_path, None);
    }
}

#[test]
fn pl20_import_into_library_missing_title_falls_back() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let outcome = import_entries_into_library(&[entry("", None, "/music/untitled.mp3")], &lib);

    assert_eq!((outcome.imported, outcome.failed), (1, 0));
    assert_eq!(lib.get_all_tracks().unwrap()[0].title, "Unknown");
}

#[test]
fn pl21_import_into_library_empty_playlist() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());
    let file = dir.path().join("empty.m3u8");
    std::fs::write(&file, "#EXTM3U\n").unwrap();

    let outcome = import_m3u8_into_library(&file, &lib).unwrap();

    assert_eq!((outcome.imported, outcome.failed), (0, 0));
    assert!(lib.get_all_tracks().unwrap().is_empty());
}

#[test]
fn pl22_import_into_library_all_fail() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let outcome = import_entries_into_library(
        &[entry("A", None, ""), entry("B", None, "")],
        &lib,
    );

    assert_eq!((outcome.imported, outcome.failed), (0, 2));
    assert!(lib.get_all_tracks().unwrap().is_empty());
}

#[test]
fn pl23_import_into_library_partial_failure_counts_both() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let outcome = import_entries_into_library(
        &[
            entry("Good One", None, "/music/one.mp3"),
            entry("Broken", None, ""),
            entry("Good Two", None, "https://example.com/two.mp3"),
        ],
        &lib,
    );

    assert_eq!((outcome.imported, outcome.failed), (2, 1));
    assert_eq!(lib.get_all_tracks().unwrap().len(), 2);
}

#[test]
fn pl24_import_into_library_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    assert!(import_m3u8_into_library(&dir.path().join("nope.m3u8"), &lib).is_err());
}
