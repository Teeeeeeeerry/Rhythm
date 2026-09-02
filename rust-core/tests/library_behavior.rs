//! LB-01–26：Library 行为清单（manifest: docs/testing/behavior/library.md）。
//!
//! 零接缝：真实 SQLite（tempfile）+ 代码生成 WAV 夹具。历史回归：
//! #40/#57（URL 去重）、#54（Mutex 死锁）、#55（后台导入）、#56（URL 持久化）、
//! #66/#67（艺人/专辑分组）。

mod common;

use lofty::prelude::*;
use rhythm_core::library::{ImportOutcome, Library};
use rhythm_core::{SourceType, TrackInfo};
use std::path::Path;

// ─── Fixtures ───────────────────────────────────────────────────────

fn local_track(path: &str, title: &str) -> TrackInfo {
    common::test_local_track(path, title, Some("Test Artist"), 200.0)
}

fn url_track(source_url: &str, title: &str, source_type: SourceType) -> TrackInfo {
    common::test_url_track(source_url, title, Some("URL Artist"), source_type, 240.0)
}

fn open_lib(dir: &Path) -> Library {
    Library::open(&dir.join("test.db")).unwrap()
}

// ─── LB-01 open 建库 ────────────────────────────────────────────────

#[test]
fn lb01_open_creates_parent_dirs_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("nested/deep/test.db");
    assert!(!db.parent().unwrap().exists());

    let lib = Library::open(&db).unwrap();
    drop(lib);

    assert!(db.exists(), "open must create the parent directories");
    Library::open(&db).unwrap(); // schema exists — must not error
}

// ─── LB-02/03/04 add_track 与去重 ───────────────────────────────────

#[test]
fn lb02_add_track_new_returns_db_id_with_fields() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let saved = lib
        .add_track(&local_track("/music/a.mp3", "Song A"))
        .unwrap();
    let id = saved.id.expect("new track must get a database id");
    assert_eq!(saved.title, "Song A");
    assert_eq!(saved.artist.as_deref(), Some("Test Artist"));
    assert_eq!(saved.play_count, 0);

    let fetched = lib.get_track_by_id(id).unwrap();
    assert_eq!(fetched.id, saved.id);
    assert_eq!(fetched.title, saved.title);
    assert_eq!(fetched.artist, saved.artist);
    assert_eq!(lib.get_all_tracks().unwrap().len(), 1);
}

#[test]
fn lb03_add_track_same_file_path_updates_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let first = lib
        .add_track(&local_track("/music/same.mp3", "Old Title"))
        .unwrap();
    let second = lib
        .add_track(&local_track("/music/same.mp3", "New Title"))
        .unwrap();

    assert_eq!(second.id, first.id, "same file_path must update, not insert");
    assert_eq!(second.title, "New Title");
    assert_eq!(lib.get_all_tracks().unwrap().len(), 1);
}

#[test]
fn lb04_add_track_same_url_updates_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let url = "https://youtube.com/watch?v=same";
    let first = lib
        .add_track(&url_track(url, "Old", SourceType::YouTube))
        .unwrap();
    let second = lib
        .add_track(&url_track(url, "New", SourceType::YouTube))
        .unwrap();

    assert_eq!(second.id, first.id, "same source_url must update, not insert (#40/#57)");
    assert_eq!(second.title, "New");
    assert_eq!(lib.get_all_tracks().unwrap().len(), 1);
}

#[test]
fn lb04_url_dedup_partial_index_rejects_direct_insert() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.db");
    let url = "https://youtube.com/watch?v=direct";

    {
        let lib = Library::open(&db).unwrap();
        lib.add_track(&url_track(url, "App Layer", SourceType::YouTube))
            .unwrap();
    }

    // Bypass the application-level dedup: a raw INSERT of the same URL must
    // be rejected by the partial unique index (#40).
    let conn = rusqlite::Connection::open(&db).unwrap();
    let err = conn
        .execute(
            "INSERT INTO tracks (source_type, source_url, title)
             VALUES ('youtube', ?1, 'Direct Insert')",
            [url],
        )
        .unwrap_err();
    assert!(
        format!("{err}").contains("UNIQUE") || format!("{err}").contains("constraint"),
        "raw duplicate URL insert must hit the unique index, got: {err}"
    );
}

// ─── LB-05 排序 ─────────────────────────────────────────────────────

#[test]
fn lb05_get_all_tracks_sorted_by_title_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    lib.add_track(&local_track("/music/b.mp3", "banana")).unwrap();
    lib.add_track(&local_track("/music/a.mp3", "Apple")).unwrap();
    lib.add_track(&local_track("/music/c.mp3", "cherry")).unwrap();

    let titles: Vec<String> = lib
        .get_all_tracks()
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, vec!["Apple", "banana", "cherry"]);
}

// ─── LB-07 remove_track ─────────────────────────────────────────────

#[test]
fn lb07_remove_track_deletes_row_fts_and_playlist_rows() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let t1 = lib.add_track(&local_track("/music/t1.mp3", "Unique Tune")).unwrap();
    let t2 = lib.add_track(&local_track("/music/t2.mp3", "Other Tune")).unwrap();
    let pid = lib.create_playlist("P", None).unwrap();
    lib.add_to_playlist(pid, t1.id.unwrap()).unwrap();
    lib.add_to_playlist(pid, t2.id.unwrap()).unwrap();

    lib.remove_track(t1.id.unwrap()).unwrap();

    assert_eq!(lib.get_all_tracks().unwrap().len(), 1);
    assert!(
        !lib.search("Unique")
            .unwrap()
            .iter()
            .any(|t| t.id == t1.id),
        "FTS index must drop the deleted row"
    );
    let playlist = lib.get_playlist(pid).unwrap();
    assert_eq!(playlist.tracks.len(), 1, "playlist_tracks must cascade");
    assert_eq!(playlist.tracks[0].id, t2.id);
}

/// #98：不存在的 id 必须报错（0 行受影响 → NotFound），而非静默成功。
#[test]
fn lb07b_remove_track_missing_id_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    assert!(lib.remove_track(999).is_err(), "missing id must error (#98)");
    assert_eq!(lib.get_all_tracks().unwrap().len(), 0);
}

// ─── LB-08 record_play ──────────────────────────────────────────────

#[test]
fn lb08_record_play_updates_last_played_and_count() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let t = lib.add_track(&local_track("/music/t.mp3", "T")).unwrap();
    let id = t.id.unwrap();
    assert_eq!(t.last_played, None);

    lib.record_play(id).unwrap();
    let after1 = lib.get_track_by_id(id).unwrap();
    assert!(after1.last_played.is_some(), "last_played must be set");
    assert_eq!(after1.play_count, 1);

    lib.record_play(id).unwrap();
    let after2 = lib.get_track_by_id(id).unwrap();
    assert_eq!(after2.play_count, 2);
}

// ─── LB-09 verify_local_files ───────────────────────────────────────

#[test]
fn lb09_verify_local_files_marks_missing_only() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    // One real file, one missing.
    let real = dir.path().join("real.mp3");
    std::fs::write(&real, b"x").unwrap();
    let present = lib.add_track(&local_track(real.to_str().unwrap(), "Present")).unwrap();
    let missing = lib
        .add_track(&local_track("/nonexistent/ghost.mp3", "Ghost"))
        .unwrap();

    let unavailable = lib.verify_local_files().unwrap();
    assert_eq!(unavailable, vec![missing.id.unwrap()]);
    assert!(!lib.get_track_by_id(missing.id.unwrap()).unwrap().is_available);
    assert!(lib.get_track_by_id(present.id.unwrap()).unwrap().is_available);
}

// ─── LB-10 播放列表 CRUD ────────────────────────────────────────────

#[test]
fn lb10_playlist_crud_create_rename_delete() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let id = lib.create_playlist("Original", Some("desc")).unwrap();
    let created = lib.get_playlist(id).unwrap();
    assert_eq!(created.name, "Original");
    assert_eq!(created.description.as_deref(), Some("desc"));
    assert!(created.date_created.is_some());
    assert!(created.date_modified.is_some());

    lib.delete_playlist(id).unwrap();
    assert!(lib.get_all_playlists().unwrap().is_empty());
    assert!(lib.get_playlist(id).is_err());
}

// ─── LB-11/12 播放列表曲目操作 ──────────────────────────────────────

#[test]
fn lb11_add_to_playlist_appends_and_dedups() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let t1 = lib.add_track(&local_track("/music/t1.mp3", "One")).unwrap();
    let t2 = lib.add_track(&local_track("/music/t2.mp3", "Two")).unwrap();
    let pid = lib.create_playlist("P", None).unwrap();

    lib.add_to_playlist(pid, t1.id.unwrap()).unwrap();
    lib.add_to_playlist(pid, t2.id.unwrap()).unwrap();
    lib.add_to_playlist(pid, t1.id.unwrap()).unwrap(); // duplicate: ignored

    let tracks = lib.get_playlist(pid).unwrap().tracks;
    let titles: Vec<&str> = tracks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, vec!["One", "Two"], "append in order, no duplicate rows");
}

#[test]
fn lb12_remove_and_reorder_playlist_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let t1 = lib.add_track(&local_track("/music/t1.mp3", "One")).unwrap();
    let t2 = lib.add_track(&local_track("/music/t2.mp3", "Two")).unwrap();
    let t3 = lib.add_track(&local_track("/music/t3.mp3", "Three")).unwrap();
    let pid = lib.create_playlist("P", None).unwrap();
    lib.add_to_playlist(pid, t1.id.unwrap()).unwrap();
    lib.add_to_playlist(pid, t2.id.unwrap()).unwrap();
    lib.add_to_playlist(pid, t3.id.unwrap()).unwrap();

    lib.remove_from_playlist(pid, t2.id.unwrap()).unwrap();
    // Move t1 to a free position at the end (conflicting positions are
    // covered by the red test below, rhythm#95).
    lib.reorder_playlist_track(pid, t1.id.unwrap(), 5).unwrap();

    let playlist = lib.get_playlist(pid).unwrap();
    let titles: Vec<&str> = playlist.tracks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, vec!["Three", "One"]);
}

/// 期望：reorder 到已占用 position 时顺序仍与操作一致（拖拽到中间位置）。
/// rhythm#95 修复后：目标行插入新位置，其余行依次移位，position 无重复。
#[test]
fn lb12_reorder_to_occupied_position_keeps_order() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let t1 = lib.add_track(&local_track("/music/t1.mp3", "One")).unwrap();
    let t2 = lib.add_track(&local_track("/music/t2.mp3", "Two")).unwrap();
    let t3 = lib.add_track(&local_track("/music/t3.mp3", "Three")).unwrap();
    let pid = lib.create_playlist("P", None).unwrap();
    lib.add_to_playlist(pid, t1.id.unwrap()).unwrap();
    lib.add_to_playlist(pid, t2.id.unwrap()).unwrap();
    lib.add_to_playlist(pid, t3.id.unwrap()).unwrap();

    lib.remove_from_playlist(pid, t2.id.unwrap()).unwrap();
    lib.reorder_playlist_track(pid, t3.id.unwrap(), 0).unwrap();

    let playlist = lib.get_playlist(pid).unwrap();
    let titles: Vec<&str> = playlist.tracks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, vec!["Three", "One"], "reordered track must come first");
}

#[test]
fn lb13_get_all_playlists_returns_full_metadata_in_position_order() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let t1 = lib.add_track(&local_track("/music/t1.mp3", "One")).unwrap();
    let t2 = lib.add_track(&local_track("/music/t2.mp3", "Two")).unwrap();
    let pid = lib.create_playlist("My List", Some("about")).unwrap();
    lib.add_to_playlist(pid, t2.id.unwrap()).unwrap();
    lib.add_to_playlist(pid, t1.id.unwrap()).unwrap();

    let playlists = lib.get_all_playlists().unwrap();
    assert_eq!(playlists.len(), 1);
    let p = &playlists[0];
    assert_eq!(p.name, "My List");
    assert_eq!(p.description.as_deref(), Some("about"));
    assert!(p.date_created.is_some());
    assert!(p.date_modified.is_some());
    let titles: Vec<&str> = p.tracks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, vec!["Two", "One"], "tracks ordered by position");
}

// ─── LB-14 search（FTS）─────────────────────────────────────────────

#[test]
fn lb14_search_hits_title_artist_album_genre() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    // FTS5 tokenizes on word boundaries: "love" must be its own token in
    // each field (a prefix like "Lovely" would not match).
    let mut by_title = local_track("/music/1.mp3", "My Love");
    by_title.artist = Some("Singer".into());
    by_title.album = Some("Plain".into());
    by_title.genre = Some("Pop".into());
    lib.add_track(&by_title).unwrap();
    let mut by_artist = local_track("/music/2.mp3", "Instrumental");
    by_artist.artist = Some("Love Band".into());
    lib.add_track(&by_artist).unwrap();
    let mut by_album = local_track("/music/3.mp3", "Track 03");
    by_album.album = Some("Love Album".into());
    lib.add_track(&by_album).unwrap();
    let mut by_genre = local_track("/music/4.mp3", "Track 04");
    by_genre.genre = Some("Love Genre".into());
    lib.add_track(&by_genre).unwrap();

    assert_eq!(lib.search("love").unwrap().len(), 4, "title/artist/album/genre must all match");
}

#[test]
fn lb14_search_ranks_relevance() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    lib.add_track(&local_track("/music/1.mp3", "love")).unwrap();
    lib.add_track(&local_track("/music/2.mp3", "love song")).unwrap();

    let titles: Vec<String> = lib
        .search("love")
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, vec!["love", "love song"], "exact match ranks first");
}

#[test]
fn lb14_search_limits_to_100() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    for i in 0..105 {
        lib.add_track(&local_track(&format!("/music/{i}.mp3"), &format!("common {i}")))
            .unwrap();
    }
    assert_eq!(lib.search("common").unwrap().len(), 100);
}

#[test]
fn lb14_search_sanitizes_special_chars() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    lib.add_track(&local_track("/music/1.mp3", "love")).unwrap();
    let hits = lib.search("love*\"()").unwrap();
    assert_eq!(hits.len(), 1, "`*`/`\"`/`(`/`)` must be stripped, not error");
}

// ─── LB-15/16/23 目录与文件导入 ─────────────────────────────────────

#[test]
fn lb15_import_from_directory_recursive_with_partial_failure() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let music = dir.path().join("music");
    std::fs::create_dir_all(music.join("sub")).unwrap();
    common::write_tagged_wav(&music.join("alpha.wav"), 0.5, |tag| {
        tag.set_title(String::from("Alpha"));
    });
    common::write_tagged_wav(&music.join("sub/beta.wav"), 0.5, |tag| {
        tag.set_title(String::from("Beta"));
    });
    std::fs::write(music.join("bad.wav"), b"RIFF").unwrap(); // truncated: fails
    std::fs::write(music.join("note.txt"), b"not audio").unwrap();

    let scanned = lib.import_from_directory(&music).unwrap();
    assert_eq!(scanned, 2, "returns the number of scanned tracks");
    assert_eq!(lib.get_all_tracks().unwrap().len(), 2);
    assert!(
        lib.search("bad").unwrap().is_empty(),
        "failed file must not be imported"
    );
}

#[test]
fn lb16_import_file_supported_returns_1_with_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let wav = dir.path().join("tagged.wav");
    common::write_tagged_wav(&wav, 1.0, |tag| {
        tag.set_title(String::from("Tagged Title"));
        tag.set_artist(String::from("Tagged Artist"));
    });

    assert_eq!(lib.import_file(&wav).unwrap(), 1);
    let tracks = lib.get_all_tracks().unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "Tagged Title");
    assert_eq!(tracks[0].artist.as_deref(), Some("Tagged Artist"));
    assert_eq!(tracks[0].source_type, SourceType::Local);
}

#[test]
fn lb16_import_file_extracts_artwork() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let wav = dir.path().join("art.wav");
    let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3, 4];
    common::write_tagged_wav(&wav, 0.5, |tag| {
        tag.push_picture(lofty::picture::Picture::new_unchecked(
            lofty::picture::PictureType::CoverFront,
            Some(lofty::picture::MimeType::Png),
            None,
            png,
        ));
    });

    lib.import_file(&wav).unwrap();
    let tracks = lib.get_all_tracks().unwrap();
    let art = tracks[0].artwork_path.as_ref().expect("embedded art must be extracted");
    assert!(Path::new(art).exists(), "artwork file must exist: {art}");
}

#[test]
fn lb23_import_from_directory_twice_no_duplicate_rows() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let music = dir.path().join("music");
    std::fs::create_dir_all(&music).unwrap();
    common::write_wav(&music.join("only.wav"), 0.5);

    lib.import_from_directory(&music).unwrap();
    lib.import_from_directory(&music).unwrap();

    assert_eq!(lib.get_all_tracks().unwrap().len(), 1, "file_path dedup");
}

// ─── LB-17/18 去重边界 ──────────────────────────────────────────────

#[test]
fn lb17_url_track_without_source_url_skips_dedup() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let mut a = url_track("", "No URL A", SourceType::YouTube);
    a.source_url = None;
    let mut b = a.clone();
    b.title = "No URL B".into();

    lib.add_track(&a).unwrap();
    lib.add_track(&b).unwrap();
    assert_eq!(lib.get_all_tracks().unwrap().len(), 2);
}

#[test]
fn lb18_file_path_dedup_takes_priority_over_url() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let mut first = local_track("/music/both.mp3", "V1");
    first.source_url = Some("https://example.com/one".into());
    let mut second = local_track("/music/both.mp3", "V2");
    second.source_url = Some("https://example.com/two".into());

    let a = lib.add_track(&first).unwrap();
    let b = lib.add_track(&second).unwrap();
    assert_eq!(a.id, b.id, "file_path dedup wins");
    assert_eq!(lib.get_all_tracks().unwrap().len(), 1);
}

// ─── LB-19/20 边界 ──────────────────────────────────────────────────

#[test]
fn lb19_search_blank_query_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());
    lib.add_track(&local_track("/music/t.mp3", "T")).unwrap();

    // Current behavior: FTS5 MATCH '' is a syntax error surfaced as Err.
    // The manifest requires "no panic, result may be empty" — an Err
    // satisfies the spirit (no crash); locked as-is.
    assert!(lib.search("").is_err());
    assert!(lib.search("   ").is_err());
}

#[test]
fn lb20_import_file_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let missing = dir.path().join("missing.mp3");
    let err = lib.import_file(&missing).unwrap_err();
    assert!(
        matches!(
            err,
            rhythm_core::RhythmError::FileNotFound(_)
                | rhythm_core::RhythmError::UnsupportedFormat(_)
        ),
        "Unsupported or FileNotFound is acceptable, got: {err:?}"
    );
}

// ─── LB-21/22 边界 ──────────────────────────────────────────────────

#[test]
fn lb21_mark_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let t = lib.add_track(&local_track("/music/t.mp3", "T")).unwrap();
    let id = t.id.unwrap();

    lib.mark_unavailable(id).unwrap();
    assert!(!lib.get_track_by_id(id).unwrap().is_available);
}

#[test]
fn lb22_add_to_playlist_foreign_key_errors() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    // Missing playlist → foreign-key error, not silence.
    assert!(lib.add_to_playlist(999, 1).is_err());

    let pid = lib.create_playlist("P", None).unwrap();
    // Missing track → foreign-key error.
    assert!(lib.add_to_playlist(pid, 4242).is_err());
}

// ─── LB-24/25/26 错误路径 ───────────────────────────────────────────

#[test]
fn lb24_open_directory_path_errors() {
    let dir = tempfile::tempdir().unwrap();
    // A directory is not a valid SQLite path. The manifest's "unwritable/
    // corrupted" variants are locked to this proxy: permission-based
    // variants are not reliable in CI (root ignores chmod), and a
    // corrupted DB is covered by SQLite's own error surfacing.
    assert!(Library::open(dir.path()).is_err());
}

#[test]
fn lb25_record_play_missing_id_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());
    lib.add_track(&local_track("/music/t.mp3", "T")).unwrap();

    lib.record_play(999).unwrap();
    assert_eq!(lib.get_all_tracks().unwrap()[0].play_count, 0);
}

#[test]
fn lb26_import_from_directory_non_dir_is_invalid_input() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());
    let file = dir.path().join("f.txt");
    std::fs::write(&file, b"x").unwrap();

    let err = lib.import_from_directory(&file).unwrap_err();
    assert!(matches!(err, rhythm_core::RhythmError::InvalidInput(_)));
}


// ─── LB-27–33 导入结果分类（#238） ──────────────────────────────────

fn outcome(imported: i32, unsupported: i32, failed: i32) -> ImportOutcome {
    ImportOutcome {
        imported,
        unsupported,
        failed,
    }
}

#[test]
fn lb27_import_directory_all_succeed() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    std::fs::create_dir_all(&music).unwrap();
    common::write_wav(&music.join("a.wav"), 0.5);
    common::write_wav(&music.join("b.wav"), 0.5);
    let lib = open_lib(dir.path());

    assert_eq!(lib.import_directory(&music), outcome(2, 0, 0));
    assert_eq!(lib.get_all_tracks().unwrap().len(), 2);
}

#[test]
fn lb28_import_directory_empty_reports_zeroes() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let lib = open_lib(dir.path());

    assert_eq!(lib.import_directory(&empty), outcome(0, 0, 0));
    assert!(lib.get_all_tracks().unwrap().is_empty());
}

#[test]
fn lb29_import_directory_missing_path_is_one_failure() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    assert_eq!(lib.import_directory(&dir.path().join("nope")), outcome(0, 0, 1));
    // A file is not a directory either — the scan cannot run.
    let file = dir.path().join("f.txt");
    std::fs::write(&file, b"x").unwrap();
    assert_eq!(lib.import_directory(&file), outcome(0, 0, 1));
}

#[test]
fn lb30_import_single_file_separates_unsupported_from_failed() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("tone.wav");
    common::write_wav(&wav, 0.5);
    let txt = dir.path().join("note.txt");
    std::fs::write(&txt, b"not audio").unwrap();
    let lib = open_lib(dir.path());

    assert_eq!(lib.import_single_file(&wav), outcome(1, 0, 0));
    assert_eq!(
        lib.import_single_file(&txt),
        outcome(0, 1, 0),
        "unsupported format is not a read failure"
    );
    assert_eq!(
        lib.import_single_file(&dir.path().join("missing.wav")),
        outcome(0, 0, 1),
        "a supported extension that cannot be read is a failure"
    );
    assert_eq!(lib.get_all_tracks().unwrap().len(), 1);
}

#[test]
fn lb31_import_paths_aggregates_partial_success() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    std::fs::create_dir_all(&music).unwrap();
    common::write_wav(&music.join("a.wav"), 0.5);
    common::write_wav(&music.join("b.wav"), 0.5);
    let lone = dir.path().join("lone.wav");
    common::write_wav(&lone, 0.5);
    let lib = open_lib(dir.path());

    let total = lib.import_paths(&[music, lone, dir.path().join("missing.wav")]);

    assert_eq!(total, outcome(3, 0, 1), "the batch aggregation lives in the core");
    assert_eq!(lib.get_all_tracks().unwrap().len(), 3);
}

#[test]
fn lb32_import_paths_all_fail() {
    let dir = tempfile::tempdir().unwrap();
    let lib = open_lib(dir.path());

    let total = lib.import_paths(&[
        dir.path().join("missing-one.wav"),
        dir.path().join("missing-two.wav"),
    ]);

    assert_eq!(total, outcome(0, 0, 2));
    assert!(lib.get_all_tracks().unwrap().is_empty());
}

#[test]
fn lb33_import_paths_mixes_success_and_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("tone.wav");
    common::write_wav(&wav, 0.5);
    let txt = dir.path().join("note.txt");
    std::fs::write(&txt, b"not audio").unwrap();
    let lib = open_lib(dir.path());

    let total = lib.import_paths(&[wav, txt]);

    assert_eq!(
        total,
        outcome(1, 1, 0),
        "unsupported keeps its own count next to a success"
    );
}
