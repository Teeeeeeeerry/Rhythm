//! MD-01–19：Metadata 行为清单（manifest: docs/testing/behavior/metadata.md）。
//!
//! 零接缝：夹具在测试内生成——lofty 写 ID3v2 标签到代码生成的 WAV，
//! 损坏文件以截断字节构造。

mod common;

use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use rhythm_core::metadata::{
    extract_artwork, extract_track_info, is_supported_audio, scan_directory, SUPPORTED_EXTENSIONS,
};
use rhythm_core::{RhythmError, SourceType};
use std::path::{Path, PathBuf};

fn set_full_tags(tag: &mut lofty::tag::Tag) {
    tag.set_title(String::from("My Title"));
    tag.set_artist(String::from("My Artist"));
    tag.set_album(String::from("My Album"));
    tag.set_track(7);
    tag.set_disk(2);
    tag.set_genre(String::from("Rock"));
    tag.set_year(2021);
}

fn write_tagged(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    common::write_tagged_wav(&path, 1.0, set_full_tags);
    path
}

// ─── MD-01 完整标签 ─────────────────────────────────────────────────

#[test]
fn md01_extract_track_info_full_tags() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_tagged(dir.path(), "full.wav");

    let info = extract_track_info(&path).unwrap();
    assert_eq!(info.title, "My Title");
    assert_eq!(info.artist.as_deref(), Some("My Artist"));
    assert_eq!(info.album.as_deref(), Some("My Album"));
    assert_eq!(info.track_number, Some(7));
    assert_eq!(info.disc_number, Some(2));
    assert_eq!(info.genre.as_deref(), Some("Rock"));
    assert_eq!(info.year, Some(2021));
    assert!((info.duration - 1.0).abs() < 0.05, "duration must be extracted");
    // #96: format is the container/encoding (WAV), not the tag type (id3v2).
    assert_eq!(info.format.as_deref(), Some("wav"));
    assert_eq!(info.bitrate, Some(1411));
    assert_eq!(info.sample_rate, Some(44100));
    assert_eq!(info.channels, Some(2));
    assert_eq!(info.source_type, SourceType::Local);
    assert_eq!(info.file_path.as_deref(), Some(path.to_str().unwrap()));
}

// ─── MD-02/03 回退 ──────────────────────────────────────────────────

#[test]
fn md02_extract_track_info_title_falls_back_to_stem() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fallback.wav");
    common::write_wav(&path, 1.0); // no tags

    let info = extract_track_info(&path).unwrap();
    assert_eq!(info.title, "fallback", "untagged file falls back to the file stem");
}

/// The manifest's 0.0-duration / extension-format fallbacks sit behind
/// `raw.duration.unwrap_or(0.0)` / `raw.format.or_else(...)` in
/// `extract_track_info`, but both `extract_with_lofty` (duration and format
/// always `Some`) and `extract_with_symphonia` (format always `Some`) keep
/// those branches unreachable from the public API — defensive dead code
/// today, not a defect. This test locks the observable behavior around it:
/// the symphonia path (see MD-04) still yields a real duration and a codec
/// format, and untagged lofty files report the real container format.
#[test]
fn md03_duration_and_format_fallbacks_not_observable() {
    let dir = tempfile::tempdir().unwrap();

    // Untagged WAV: lofty path — duration extracted (not 0.0), format = container.
    let untagged = dir.path().join("plain.wav");
    common::write_wav(&untagged, 2.0);
    let info = extract_track_info(&untagged).unwrap();
    assert!((info.duration - 2.0).abs() < 0.05);
    assert_eq!(info.format.as_deref(), Some("wav"));

    // No-extension WAV: symphonia path — duration extracted, format = codec.
    let mystery = dir.path().join("mystery");
    common::write_wav(&mystery, 1.0);
    let info = extract_track_info(&mystery).unwrap();
    assert!((info.duration - 1.0).abs() < 0.05);
    assert!(info.format.is_some());
}

// ─── MD-04 lofty→symphonia 回退 ─────────────────────────────────────

#[test]
fn md04_extract_track_info_falls_back_to_symphonia() {
    let dir = tempfile::tempdir().unwrap();
    // No extension: lofty cannot pick a parser by extension, symphonia
    // probes the container by magic bytes and extracts the duration.
    let path = dir.path().join("mystery");
    common::write_wav(&path, 1.0);

    let info = extract_track_info(&path).unwrap();
    assert_eq!(info.title, "mystery", "no tags — title falls back to stem");
    assert!((info.duration - 1.0).abs() < 0.05, "symphonia must extract the duration");
    assert_eq!(info.sample_rate, Some(44100));
    assert_eq!(info.channels, Some(2));
}

// ─── MD-05 文件不存在 ───────────────────────────────────────────────

#[test]
fn md05_extract_track_info_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let err = extract_track_info(&dir.path().join("nope.mp3")).unwrap_err();
    assert!(matches!(err, RhythmError::FileNotFound(_)));
}

// ─── MD-06/17 扩展名支持 ────────────────────────────────────────────

#[test]
fn md06_is_supported_audio_all_extensions_case_insensitive() {
    for ext in SUPPORTED_EXTENSIONS {
        assert!(is_supported_audio(&PathBuf::from(format!("a.{ext}"))));
        assert!(
            is_supported_audio(&PathBuf::from(format!("a.{}", ext.to_uppercase()))),
            "{ext} must be case-insensitive"
        );
    }
    assert!(!is_supported_audio(&PathBuf::from("a.txt")));
    assert!(!is_supported_audio(&PathBuf::from("noextension")));
}

#[test]
fn md17_uppercase_extension_recognized() {
    assert!(is_supported_audio(&PathBuf::from("song.MP3")));
    assert!(is_supported_audio(&PathBuf::from("song.WaV")));
}

// ─── MD-08/09/10 artwork ────────────────────────────────────────────

/// Returns a WAV with an embedded picture of the given MIME type and data.
fn write_wav_with_picture(dir: &Path, name: &str, mime: MimeType, data: Vec<u8>) -> PathBuf {
    let path = dir.join(name);
    common::write_tagged_wav(&path, 0.5, move |tag| {
        tag.push_picture(Picture::new_unchecked(
            PictureType::CoverFront,
            Some(mime),
            None,
            data,
        ));
    });
    path
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[test]
fn md08_extract_artwork_writes_cache_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let path = write_wav_with_picture(dir.path(), "art.wav", MimeType::Jpeg, vec![1, 2, 3, 4]);

    let art = extract_artwork(&path, &cache).unwrap().expect("embedded art must be found");
    let art_path = Path::new(&art);
    let file_name = art_path.file_name().unwrap().to_str().unwrap();
    let (stem, _ext) = file_name.split_once('.').expect("name.ext");
    assert!(is_hex64(stem), "filename = blake3 hex digest, got {stem}");
    assert!(art_path.exists());

    // Idempotent: an existing cache file must not be rewritten.
    std::fs::write(art_path, b"stale").unwrap();
    let again = extract_artwork(&path, &cache).unwrap().unwrap();
    assert_eq!(again, art, "same data must map to the same filename");
    assert_eq!(std::fs::read(art_path).unwrap(), b"stale", "existing file untouched");

    // Different picture data → different filename.
    let other = write_wav_with_picture(dir.path(), "other.wav", MimeType::Jpeg, vec![9, 9, 9]);
    let art2 = extract_artwork(&other, &cache).unwrap().unwrap();
    assert_ne!(art2, art);
}

#[test]
fn md08_extract_artwork_no_picture_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.wav");
    common::write_wav(&path, 0.5);

    assert!(extract_artwork(&path, &dir.path().join("cache")).unwrap().is_none());
}

#[test]
fn md09_extract_artwork_skips_oversized() {
    let dir = tempfile::tempdir().unwrap();
    // MAX_ARTWORK_SIZE is 1 MiB — one byte over must be skipped.
    let big = vec![0u8; 1_048_576 + 1];
    let path = write_wav_with_picture(dir.path(), "big.wav", MimeType::Png, big);

    let result = extract_artwork(&path, &dir.path().join("cache")).unwrap();
    assert!(result.is_none(), "artwork over 1 MB must be skipped");
}

#[test]
fn md10_extract_artwork_mime_jpeg_and_other_map_to_jpg() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");

    let jpeg = write_wav_with_picture(dir.path(), "j.wav", MimeType::Jpeg, vec![1]);
    let art = extract_artwork(&jpeg, &cache).unwrap().unwrap();
    assert!(art.ends_with(".jpg"), "jpeg must map to .jpg, got {art}");

    let gif = write_wav_with_picture(dir.path(), "g.wav", MimeType::Gif, vec![2]);
    let art = extract_artwork(&gif, &cache).unwrap().unwrap();
    assert!(art.ends_with(".jpg"), "non-jpeg/png must fall back to .jpg, got {art}");
}

/// 期望：PNG 内嵌图缓存为 `.png`。MIME 判定已改为大小写不敏感
/// （rhythm#94 修复），本测试转真断言。
#[test]
fn md10_extract_artwork_mime_png_maps_to_png() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let png = write_wav_with_picture(dir.path(), "p.wav", MimeType::Png, vec![3]);

    let art = extract_artwork(&png, &cache).unwrap().unwrap();
    assert!(art.ends_with(".png"), "png must map to .png, got {art}");
}

// ─── MD-11–15 scan_directory ────────────────────────────────────────

#[test]
fn md11_scan_directory_recursive_collects_supported() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("music");
    std::fs::create_dir_all(root.join("sub/deep")).unwrap();
    common::write_wav(&root.join("a.wav"), 0.5);
    common::write_wav(&root.join("sub/b.wav"), 0.5);
    common::write_wav(&root.join("sub/deep/c.wav"), 0.5);
    std::fs::write(root.join("note.txt"), b"x").unwrap();

    let tracks = scan_directory(&root).unwrap();
    let paths: Vec<&str> = tracks
        .iter()
        .map(|t| t.file_path.as_deref().unwrap())
        .collect();
    assert_eq!(tracks.len(), 3);
    assert!(paths.iter().any(|p| p.ends_with("a.wav")));
    assert!(paths.iter().any(|p| p.ends_with("b.wav")));
    assert!(paths.iter().any(|p| p.ends_with("c.wav")));
}

#[test]
fn md12_scan_directory_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("music");
    std::fs::create_dir_all(root.join(".hidden")).unwrap();
    common::write_wav(&root.join("visible.wav"), 0.5);
    common::write_wav(&root.join(".hidden/secret.wav"), 0.5);

    let tracks = scan_directory(&root).unwrap();
    assert_eq!(tracks.len(), 1, "dot-directories must be skipped");
    assert!(tracks[0].file_path.as_deref().unwrap().ends_with("visible.wav"));
}

#[test]
fn md13_scan_directory_non_dir_is_invalid_input() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.txt");
    std::fs::write(&file, b"x").unwrap();

    let err = scan_directory(&file).unwrap_err();
    assert!(matches!(err, RhythmError::InvalidInput(_)));
}

#[test]
fn md14_scan_directory_skips_bad_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("music");
    std::fs::create_dir_all(&root).unwrap();
    common::write_wav(&root.join("good.wav"), 0.5);
    std::fs::write(root.join("bad.wav"), b"RIFF").unwrap(); // truncated
    std::fs::write(root.join("empty.wav"), b"").unwrap(); // empty

    let tracks = scan_directory(&root).unwrap();
    assert_eq!(tracks.len(), 1, "corrupted files must be skipped, not fatal");
    assert!(tracks[0].file_path.as_deref().unwrap().ends_with("good.wav"));
}

#[test]
fn md15_scan_directory_empty_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("music");
    std::fs::create_dir_all(&root).unwrap();

    assert!(scan_directory(&root).unwrap().is_empty());
}

// ─── MD-16 无标签文件 ───────────────────────────────────────────────

#[test]
fn md16_untagged_file_extracts_with_fallbacks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("music");
    std::fs::create_dir_all(&root).unwrap();
    common::write_wav(&root.join("untitled.wav"), 1.0);

    let tracks = scan_directory(&root).unwrap();
    assert_eq!(tracks.len(), 1, "untagged files must not error");
    assert_eq!(tracks[0].title, "untitled", "title falls back to the stem");
}

// ─── MD-18/19 错误路径 ──────────────────────────────────────────────

#[test]
fn md18_corrupted_file_errors_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.wav");
    std::fs::write(&path, b"RIFF").unwrap(); // header only, no chunks

    let err = extract_track_info(&path).unwrap_err();
    assert!(
        matches!(err, RhythmError::Decode(_) | RhythmError::Metadata(_)),
        "corrupted file must yield Metadata/Decode, got: {err:?}"
    );
}

#[test]
fn md19_extract_artwork_unreadable_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let err = extract_artwork(&dir.path().join("nope.wav"), &dir.path().join("c")).unwrap_err();
    assert!(
        matches!(err, RhythmError::Metadata(_)),
        "unreadable file must error, got: {err:?}"
    );
}
