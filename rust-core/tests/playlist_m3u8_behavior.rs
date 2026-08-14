//! PL-01–14：Playlist（M3U8）行为清单（manifest: docs/testing/behavior/playlist-m3u8.md）。
//!
//! 零接缝：纯文件 IO + tempfile。历史回归：#34（M3U8 导出静默失败）。

mod common;

use rhythm_core::playlist::{export_m3u8, import_m3u8};
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
            ("Title A".to_string(), Some("Artist A".to_string()), Some("/music/a.mp3".to_string())),
            ("Title B".to_string(), Some("Artist B".to_string()), Some("https://x.com/b.mp3".to_string())),
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
    assert_eq!(entries[0].0, "T");
}

#[test]
fn pl06_import_m3u8_extinf_without_separator() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.m3u8");
    std::fs::write(&file, "#EXTM3U\n#EXTINF:180,Some Title\n/x.mp3\n").unwrap();

    let entries = import_m3u8(&file).unwrap();
    assert_eq!(entries[0].0, "Some Title", "whole text after comma is the title");
    assert_eq!(entries[0].1, None, "no ` - ` separator → no artist");
}

#[test]
fn pl07_import_m3u8_location_without_extinf_falls_back_to_stem() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.m3u8");
    std::fs::write(&file, "#EXTM3U\n/music/song.mp3\nhttps://x.com/a.mp3\n").unwrap();

    let entries = import_m3u8(&file).unwrap();
    assert_eq!(entries[0].0, "song", "title falls back to the file stem");
    assert_eq!(entries[0].1, None);
    assert_eq!(entries[1].0, "a");
}

#[test]
fn pl08_import_m3u8_skips_other_comment_lines() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.m3u8");
    std::fs::write(&file, "#EXTM3U\n#PLAYLIST:My List\n#EXTGRP:Group\n#EXTINF:180,A - T\n/x.mp3\n").unwrap();

    let entries = import_m3u8(&file).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "T");
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
    assert_eq!(
        entries,
        vec![
            ("Local One".to_string(), Some("Artist One".to_string()), Some("/music/one.mp3".to_string())),
            ("URL Two".to_string(), Some("Artist Two".to_string()), Some("https://example.com/two.mp3".to_string())),
            ("No Artist".to_string(), Some("Unknown Artist".to_string()), Some("/music/three.mp3".to_string())),
        ],
        "title/artist/location must survive a full roundtrip"
    );
}
