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

    let grouped = lib.get_tracks_by_artist_album().unwrap();
    assert_eq!(grouped.len(), 3, "should have 3 artist-album groups");
    assert_eq!(grouped[0].0, "Artist 1");
    assert_eq!(grouped[0].2.len(), 1);
    assert_eq!(grouped[0].2[0].title, "Song A");
    assert_eq!(grouped[1].0, "Artist 2");
    assert_eq!(grouped[1].2[0].title, "Song B");
    assert_eq!(grouped[2].0, "Artist 3");
    assert_eq!(grouped[2].2[0].title, "Song C");
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

    let grouped = lib.get_tracks_by_artist_album().unwrap();
    // URL tracks have no album, so they default to "Unknown Album"
    // Local tracks have their actual albums
    assert_eq!(grouped.len(), 3, "should have 3 distinct artist groups");

    // Verify each artist still has the correct track
    for (artist, _album, tracks) in &grouped {
        assert_eq!(tracks.len(), 1, "artist '{artist}' should have exactly 1 track");
        match artist.as_str() {
            "Artist A" => assert_eq!(tracks[0].title, "Local A"),
            "Artist B" => assert_eq!(tracks[0].title, "Local B"),
            "URL Artist X" => assert_eq!(tracks[0].title, "URL Song X"),
            _ => panic!("unexpected artist: {artist}"),
        }
    }
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

    let grouped = lib.get_tracks_by_artist_album().unwrap();
    // 4 artists: 1 local + 3 URL
    assert_eq!(grouped.len(), 4, "should have 4 artist groups, got {}", grouped.len());

    // Each artist should have exactly one track
    let mut titles: Vec<String> = grouped.iter().flat_map(|(_, _, tracks)| tracks.iter().map(|t| t.title.clone())).collect();
    titles.sort();
    assert_eq!(titles, vec!["Local Track", "URL Song 1", "URL Song 2", "URL Song 3"]);
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

#[test]
fn test_url_tracks_with_same_artist_grouped_together() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let lib = Library::open(&db_path).unwrap();

    // Two URL tracks from the same artist
    lib.add_track(&dummy_url_track(None, "Song Alpha", "Same Artist", "https://youtube.com/watch?v=aa")).unwrap();
    lib.add_track(&dummy_url_track(None, "Song Beta", "Same Artist", "https://youtube.com/watch?v=bb")).unwrap();

    let grouped = lib.get_tracks_by_artist_album().unwrap();
    // Same artist, both URL tracks (no album → "Unknown Album")
    assert_eq!(grouped.len(), 1, "same artist should be 1 group");
    assert_eq!(grouped[0].0, "Same Artist");
    assert_eq!(grouped[0].2.len(), 2, "should have 2 tracks in the group");
}

#[test]
fn test_add_track_preserves_artist_album_integrity() {
    // Comprehensive test that matches the bug report scenario:
    // 1. Have local tracks from multiple artists/albums
    // 2. Import URL tracks from multiple sources
    // 3. Verify get_tracks_by_artist_album returns correct grouping
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let lib = Library::open(&db_path).unwrap();

    // Pre-populate with local tracks — a realistic library
    lib.add_track(&dummy_local_track(None, "Bohemian Rhapsody", "Queen", "A Night at the Opera", "/music/queen_bohemian.mp3")).unwrap();
    lib.add_track(&dummy_local_track(None, "Love of My Life", "Queen", "A Night at the Opera", "/music/queen_love.mp3")).unwrap();
    lib.add_track(&dummy_local_track(None, "Imagine", "John Lennon", "Imagine", "/music/lennon_imagine.mp3")).unwrap();
    lib.add_track(&dummy_local_track(None, "Hotel California", "Eagles", "Hotel California", "/music/eagles_hotel.mp3")).unwrap();

    // Now simulate playing/importing URL tracks
    lib.add_track(&dummy_url_track(None, "Live Performance A", "Coldplay", "https://youtube.com/watch?v=cp001")).unwrap();
    lib.add_track(&dummy_url_track(None, "Live Performance B", "Coldplay", "https://youtube.com/watch?v=cp002")).unwrap();
    lib.add_track(&dummy_url_track(None, "Tutorial Video", "Random Creator", "https://bilibili.com/video/BV123")).unwrap();

    let all = lib.get_all_tracks().unwrap();
    assert_eq!(all.len(), 7, "should have 7 tracks total");

    let grouped = lib.get_tracks_by_artist_album().unwrap();
    // Expected groups:
    // - Coldplay / Unknown Album (2 tracks)
    // - Eagles / Hotel California (1 track)
    // - John Lennon / Imagine (1 track)
    // - Queen / A Night at the Opera (2 tracks)
    // - Random Creator / Unknown Album (1 track)
    assert_eq!(grouped.len(), 5, "should have 5 artist groups");

    // Verify each group has the correct tracks
    // Queen → 2 tracks
    let queen_group = grouped.iter().find(|(a, _, _)| a == "Queen").unwrap();
    assert_eq!(queen_group.2.len(), 2);
    let queen_titles: Vec<&str> = queen_group.2.iter().map(|t| t.title.as_str()).collect();
    assert!(queen_titles.contains(&"Bohemian Rhapsody"));
    assert!(queen_titles.contains(&"Love of My Life"));

    // Coldplay → 2 tracks
    let cp_group = grouped.iter().find(|(a, _, _)| a == "Coldplay").unwrap();
    assert_eq!(cp_group.2.len(), 2);
    let cp_titles: Vec<&str> = cp_group.2.iter().map(|t| t.title.as_str()).collect();
    assert!(cp_titles.contains(&"Live Performance A"));
    assert!(cp_titles.contains(&"Live Performance B"));

    // Eagles → 1 track
    let eagles_group = grouped.iter().find(|(a, _, _)| a == "Eagles").unwrap();
    assert_eq!(eagles_group.2.len(), 1);
    assert_eq!(eagles_group.2[0].title, "Hotel California");

    // John Lennon → 1 track
    let lennon_group = grouped.iter().find(|(a, _, _)| a == "John Lennon").unwrap();
    assert_eq!(lennon_group.2.len(), 1);
    assert_eq!(lennon_group.2[0].title, "Imagine");

    // Random Creator → 1 track
    let rc_group = grouped.iter().find(|(a, _, _)| a == "Random Creator").unwrap();
    assert_eq!(rc_group.2.len(), 1);
    assert_eq!(rc_group.2[0].title, "Tutorial Video");
}
