//! Integration tests for Library — add_track, dedup, and artist-album grouping.
//!
//! Target bug: after importing URL tracks, every artist in the "By Artist/Album"
//! view shows only the most recently imported track.

use rhythm_core::library::Library;
use rhythm_core::{SourceType, TrackInfo};
fn dummy_local_track(id: Option<i64>, title: &str, artist: &str, album: &str, path: &str) -> TrackInfo {
    TrackInfo {
        id,
        file_path: Some(path.to_string()),
        source_type: SourceType::Local,
        source_url: None,
        title: title.to_string(),
        artist: Some(artist.to_string()),
        album: Some(album.to_string()),
        album_artist: None,
        track_number: Some(1),
        disc_number: Some(1),
        genre: None,
        year: Some(2024),
        duration: 200.0,
        format: Some("mp3".to_string()),
        bitrate: Some(320),
        sample_rate: Some(44100),
        channels: Some(2),
        file_size: Some(5_000_000),
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: None,
        is_available: true,
    }
}

fn dummy_url_track(id: Option<i64>, title: &str, artist: &str, source_url: &str) -> TrackInfo {
    TrackInfo {
        id,
        file_path: None,
        source_type: SourceType::YouTube,
        source_url: Some(source_url.to_string()),
        title: title.to_string(),
        artist: Some(artist.to_string()),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        genre: None,
        year: None,
        duration: 240.0,
        format: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_size: None,
        date_added: None,
        last_played: None,
        play_count: 0,
        artwork_path: None,
        is_available: true,
    }
}

#[test]
fn test_add_local_tracks_preserves_all_entries() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let lib = Library::open(&db_path).unwrap();

    // Add 3 local tracks with different artists
    lib.add_track(&dummy_local_track(None, "Song A", "Artist 1", "Album X", "/music/a.mp3")).unwrap();
    lib.add_track(&dummy_local_track(None, "Song B", "Artist 2", "Album Y", "/music/b.mp3")).unwrap();
    lib.add_track(&dummy_local_track(None, "Song C", "Artist 3", "Album Z", "/music/c.mp3")).unwrap();

    let all = lib.get_all_tracks().unwrap();
    assert_eq!(all.len(), 3, "should have 3 tracks");
    assert_eq!(all[0].title, "Song A");
    assert_eq!(all[1].title, "Song B");
    assert_eq!(all[2].title, "Song C");
}

#[test]
fn test_add_url_track_after_local_tracks_preserves_all() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let lib = Library::open(&db_path).unwrap();

    // Simulate existing library with 2 local tracks
    lib.add_track(&dummy_local_track(None, "Local A", "Artist A", "Album 1", "/music/a.mp3")).unwrap();
    lib.add_track(&dummy_local_track(None, "Local B", "Artist B", "Album 2", "/music/b.mp3")).unwrap();

    // Now simulate the "import URL" flow: add a URL track (like playResolved does)
    lib.add_track(&dummy_url_track(None, "URL Song X", "URL Artist X", "https://youtube.com/watch?v=abc123")).unwrap();

    let all = lib.get_all_tracks().unwrap();
    assert_eq!(all.len(), 3, "should have 3 tracks after URL import");
}

#[test]
fn test_multiple_url_imports_preserve_all_artists() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let lib = Library::open(&db_path).unwrap();

    // Start with one local track
    lib.add_track(&dummy_local_track(None, "Local Track", "Local Artist", "Local Album", "/music/local.mp3")).unwrap();

    // Simulate importing 3 different URL tracks (different URLs, different artists)
    lib.add_track(&dummy_url_track(None, "URL Song 1", "URL Artist 1", "https://youtube.com/watch?v=111")).unwrap();
    lib.add_track(&dummy_url_track(None, "URL Song 2", "URL Artist 2", "https://youtube.com/watch?v=222")).unwrap();
    lib.add_track(&dummy_url_track(None, "URL Song 3", "URL Artist 3", "https://youtube.com/watch?v=333")).unwrap();

    let all = lib.get_all_tracks().unwrap();
    assert_eq!(all.len(), 4, "should have 4 tracks total");
}

#[test]
fn test_reimport_same_url_updates_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let lib = Library::open(&db_path).unwrap();

    // First import of a URL
    let saved1 = lib.add_track(&dummy_url_track(None, "Original Title", "Original Artist", "https://youtube.com/watch?v=reuse")).unwrap();
    let id1 = saved1.id.unwrap();

    // Second import of the SAME URL (e.g., user pastes and plays it again)
    // This should UPDATE the existing row, not insert a new one
    let saved2 = lib.add_track(&dummy_url_track(None, "Updated Title", "Updated Artist", "https://youtube.com/watch?v=reuse")).unwrap();

    assert_eq!(saved2.id.unwrap(), id1, "re-import of same URL should update existing row, not create a new one");
    assert_eq!(saved2.title, "Updated Title");
    assert_eq!(saved2.artist.unwrap(), "Updated Artist");

    let all = lib.get_all_tracks().unwrap();
    assert_eq!(all.len(), 1, "should still be exactly 1 track after re-import of same URL");
}

