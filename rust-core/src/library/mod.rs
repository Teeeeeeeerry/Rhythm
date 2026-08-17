use crate::{RhythmError, RhythmResult, SourceType, TrackInfo};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

mod search;
// pub use search::*; // Reserved for future search enhancements

/// Library database backed by SQLite.
/// Manages the track catalog, playlists, and search index.
pub struct Library {
    conn: Mutex<Connection>,
}

impl Library {
    /// Open or create the library database at the given path.
    pub fn open(db_path: &Path) -> RhythmResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        // Enable WAL mode and performance optimizations
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA cache_size=-8000;     -- 8MB page cache
             PRAGMA mmap_size=67108864;    -- 64MB memory-mapped I/O
             PRAGMA synchronous=NORMAL;    -- Safe enough with WAL
             PRAGMA temp_store=MEMORY;     -- Use memory for temp tables
             PRAGMA busy_timeout=5000;     -- Wait up to 5s on lock"
        )?;

        let lib = Library {
            conn: Mutex::new(conn),
        };
        lib.initialize_schema()?;
        Ok(lib)
    }

    /// Create tables and indexes if they don't exist.
    fn initialize_schema(&self) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT UNIQUE,
                source_type TEXT NOT NULL DEFAULT 'local',
                source_url TEXT,
                title TEXT NOT NULL,
                artist TEXT,
                album TEXT,
                album_artist TEXT,
                track_number INTEGER,
                disc_number INTEGER,
                genre TEXT,
                year INTEGER,
                duration REAL NOT NULL DEFAULT 0,
                format TEXT,
                bitrate INTEGER,
                sample_rate INTEGER,
                channels INTEGER,
                file_size INTEGER,
                date_added TEXT DEFAULT (datetime('now')),
                last_played TEXT,
                play_count INTEGER DEFAULT 0,
                artwork_path TEXT,
                is_available INTEGER DEFAULT 1,
                checksum TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
            CREATE INDEX IF NOT EXISTS idx_tracks_source_type ON tracks(source_type);
            CREATE INDEX IF NOT EXISTS idx_tracks_file_path ON tracks(file_path);

            -- Partial unique index: prevent duplicate URL tracks at the
            -- database level, even if the application-level dedup in
            -- add_track() is bypassed (#40).
            CREATE UNIQUE INDEX IF NOT EXISTS idx_tracks_source_url_unique
                ON tracks(source_url)
                WHERE source_url IS NOT NULL AND source_type != 'local';

            CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT DEFAULT '',
                date_created TEXT DEFAULT (datetime('now')),
                date_modified TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS playlist_tracks (
                playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
                position INTEGER NOT NULL DEFAULT 0,
                date_added TEXT DEFAULT (datetime('now')),
                PRIMARY KEY (playlist_id, track_id)
            );

            CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist ON playlist_tracks(playlist_id, position);

            -- Full-text search
            CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
                title, artist, album, genre,
                content=tracks, content_rowid=id
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
                INSERT INTO tracks_fts(rowid, title, artist, album, genre)
                VALUES (new.id, new.title, new.artist, new.album, new.genre);
            END;

            CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks BEGIN
                INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, genre)
                VALUES ('delete', old.id, old.title, old.artist, old.album, old.genre);
            END;

            CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks BEGIN
                INSERT INTO tracks_fts(tracks_fts, rowid, title, artist, album, genre)
                VALUES ('delete', old.id, old.title, old.artist, old.album, old.genre);
                INSERT INTO tracks_fts(rowid, title, artist, album, genre)
                VALUES (new.id, new.title, new.artist, new.album, new.genre);
            END;
            ",
        )?;

        Ok(())
    }

    // ─── Track CRUD ───────────────────────────────────────────────

    /// Add a single track to the library.
    /// Returns the inserted track with its database id.
    pub fn add_track(&self, track: &TrackInfo) -> RhythmResult<TrackInfo> {
        let conn = self.conn.lock().unwrap();

        // Check for duplicate by file_path (local files)
        if let Some(ref fp) = track.file_path {
            let existing: Option<i64> = conn
                .query_row(
                    "SELECT id FROM tracks WHERE file_path = ?1",
                    params![fp],
                    |row| row.get(0),
                )
                .ok();

            if let Some(id) = existing {
                // Update instead of insert
                self.update_track_impl(&conn, id, track)?;
                return self.get_track_by_id_impl(&conn, id);
            }
        }

        // Check for duplicate by source_url (URL tracks — YouTube, Bilibili,
        // direct URL, etc.). Without this, resolving the same URL twice would
        // insert a new row every time (#40).
        if track.file_path.is_none() {
            if let Some(ref url) = track.source_url {
                let existing: Option<i64> = conn
                    .query_row(
                        "SELECT id FROM tracks WHERE source_url = ?1 AND source_type != 'local'",
                        params![url],
                        |row| row.get(0),
                    )
                    .ok();

                if let Some(id) = existing {
                    self.update_track_impl(&conn, id, track)?;
                    return self.get_track_by_id_impl(&conn, id);
                }
            }
        }

        conn.execute(
            "INSERT INTO tracks (file_path, source_type, source_url, title, artist, album,
             album_artist, track_number, disc_number, genre, year, duration, format,
             bitrate, sample_rate, channels, file_size, artwork_path, is_available)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                track.file_path,
                track.source_type.to_string(),
                track.source_url,
                track.title,
                track.artist,
                track.album,
                track.album_artist,
                track.track_number,
                track.disc_number,
                track.genre,
                track.year,
                track.duration,
                track.format,
                track.bitrate,
                track.sample_rate,
                track.channels,
                track.file_size,
                track.artwork_path,
                track.is_available as i32,
            ],
        )?;

        let id = conn.last_insert_rowid();
        self.get_track_by_id_impl(&conn, id)
    }

    /// Update an existing track.
    fn update_track_impl(&self, conn: &Connection, id: i64, track: &TrackInfo) -> RhythmResult<()> {
        conn.execute(
            "UPDATE tracks SET title=?1, artist=?2, album=?3, album_artist=?4,
             track_number=?5, disc_number=?6, genre=?7, year=?8, duration=?9,
             format=?10, bitrate=?11, sample_rate=?12, channels=?13, file_size=?14,
             artwork_path=?15, is_available=?16, source_url=?17, source_type=?18
             WHERE id=?19",
            params![
                track.title,
                track.artist,
                track.album,
                track.album_artist,
                track.track_number,
                track.disc_number,
                track.genre,
                track.year,
                track.duration,
                track.format,
                track.bitrate,
                track.sample_rate,
                track.channels,
                track.file_size,
                track.artwork_path,
                track.is_available as i32,
                track.source_url,
                track.source_type.to_string(),
                id,
            ],
        )?;
        Ok(())
    }

    /// Get a track by its database ID (lock-free, call with conn already held).
    fn get_track_by_id_impl(&self, conn: &Connection, id: i64) -> RhythmResult<TrackInfo> {
        let mut stmt = conn.prepare(
            "SELECT id, file_path, source_type, source_url, title, artist, album,
             album_artist, track_number, disc_number, genre, year, duration, format,
             bitrate, sample_rate, channels, file_size, date_added, last_played,
             play_count, artwork_path, is_available
             FROM tracks WHERE id = ?1",
        )?;

        stmt.query_row(params![id], |row| row_to_track(row)).map_err(|e| e.into())
    }

    /// Get a track by its database ID.
    pub fn get_track_by_id(&self, id: i64) -> RhythmResult<TrackInfo> {
        let conn = self.conn.lock().unwrap();
        self.get_track_by_id_impl(&conn, id)
    }

    /// Get all tracks in the library.
    pub fn get_all_tracks(&self) -> RhythmResult<Vec<TrackInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, source_type, source_url, title, artist, album,
             album_artist, track_number, disc_number, genre, year, duration, format,
             bitrate, sample_rate, channels, file_size, date_added, last_played,
             play_count, artwork_path, is_available
             FROM tracks ORDER BY title COLLATE NOCASE",
        )?;

        let tracks: Result<Vec<_>, _> = stmt
            .query_map([], |row| row_to_track(row))?
            .collect();

        Ok(tracks?)
    }

    /// Get tracks grouped by artist > album (for library browser view).
    pub fn get_tracks_by_artist_album(&self) -> RhythmResult<Vec<(String, String, Vec<TrackInfo>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, file_path, source_type, source_url, title, artist, album,
             album_artist, track_number, disc_number, genre, year, duration, format,
             bitrate, sample_rate, channels, file_size, date_added, last_played,
             play_count, artwork_path, is_available
             FROM tracks
             ORDER BY artist COLLATE NOCASE, album COLLATE NOCASE, disc_number, track_number",
        )?;

        let all_tracks: Vec<TrackInfo> = stmt
            .query_map([], |row| row_to_track(row))?
            .collect::<Result<Vec<_>, _>>()?;

        // Group by (artist, album)
        let mut grouped: Vec<(String, String, Vec<TrackInfo>)> = Vec::new();
        for track in all_tracks {
            let artist = track.artist.clone().unwrap_or_else(|| "Unknown Artist".to_string());
            let album = track.album.clone().unwrap_or_else(|| "Unknown Album".to_string());

            if let Some(last) = grouped.last_mut() {
                if last.0 == artist && last.1 == album {
                    last.2.push(track);
                    continue;
                }
            }
            grouped.push((artist, album, vec![track]));
        }

        Ok(grouped)
    }

    /// Delete a track from the library.
    ///
    /// Errors with [`RhythmError::NotFound`] when the id matches no row,
    /// so callers can distinguish "deleted" from "nothing to delete" (#98).
    pub fn remove_track(&self, id: i64) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(RhythmError::NotFound(format!("track id {id}")));
        }
        Ok(())
    }

    /// Mark a track as unavailable (greyed out).
    pub fn mark_unavailable(&self, id: i64) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tracks SET is_available = 0 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Mark a track as available.
    pub fn mark_available(&self, id: i64) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tracks SET is_available = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Update the last_played timestamp and play count.
    pub fn record_play(&self, id: i64) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tracks SET last_played = datetime('now'), play_count = play_count + 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Verify that all local tracks still exist on disk.
    /// Returns the IDs of tracks that are no longer available.
    pub fn verify_local_files(&self) -> RhythmResult<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, file_path FROM tracks WHERE source_type = 'local'")?;

        let mut unavailable: Vec<i64> = Vec::new();
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        for (id, file_path) in rows {
            if !Path::new(&file_path).exists() {
                conn.execute(
                    "UPDATE tracks SET is_available = 0 WHERE id = ?1",
                    params![id],
                )?;
                unavailable.push(id);
            }
        }

        Ok(unavailable)
    }

    // ─── Playlist Management ──────────────────────────────────────

    /// Create a new playlist.
    pub fn create_playlist(&self, name: &str, description: Option<&str>) -> RhythmResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO playlists (name, description) VALUES (?1, ?2)",
            params![name, description],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Delete a playlist by ID.
    pub fn delete_playlist(&self, playlist_id: i64) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM playlists WHERE id = ?1",
            params![playlist_id],
        )?;
        Ok(())
    }

    /// Rename a playlist.
    pub fn rename_playlist(&self, playlist_id: i64, new_name: &str) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE playlists SET name = ?1, date_modified = datetime('now') WHERE id = ?2",
            params![new_name, playlist_id],
        )?;
        Ok(())
    }

    /// Add a track to a playlist at a given position (or at the end if position is None).
    pub fn add_to_playlist(&self, playlist_id: i64, track_id: i64) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();

        // Get max position
        let max_pos: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) FROM playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
                |row| row.get(0),
            )?;

        conn.execute(
            "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position)
             VALUES (?1, ?2, ?3)",
            params![playlist_id, track_id, max_pos + 1],
        )?;

        conn.execute(
            "UPDATE playlists SET date_modified = datetime('now') WHERE id = ?1",
            params![playlist_id],
        )?;

        Ok(())
    }

    /// Remove a track from a playlist.
    pub fn remove_from_playlist(&self, playlist_id: i64, track_id: i64) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            params![playlist_id, track_id],
        )?;
        Ok(())
    }

    /// Reorder a track within a playlist.
    ///
    /// Moves the track to `new_position` and shifts the other rows so the
    /// playlist has no duplicate positions: the resulting `get_playlist`
    /// order is stable and matches the drag operation (#95).
    pub fn reorder_playlist_track(
        &self,
        playlist_id: i64,
        track_id: i64,
        new_position: i32,
    ) -> RhythmResult<()> {
        let conn = self.conn.lock().unwrap();

        // Current order, stable by position then rowid.
        let mut stmt = conn.prepare(
            "SELECT pt.track_id FROM playlist_tracks pt
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position, pt.rowid",
        )?;
        let mut ids: Vec<i64> = stmt
            .query_map(params![playlist_id], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        // Track not in this playlist: no-op.
        let Some(from) = ids.iter().position(|&id| id == track_id) else {
            return Ok(());
        };

        // Remove and re-insert at the clamped target index, shifting the rest.
        let target = (new_position.max(0) as usize).min(ids.len() - 1);
        let id = ids.remove(from);
        ids.insert(target, id);

        conn.execute_batch("BEGIN")?;
        let result = (|| {
            let mut update = conn.prepare(
                "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND track_id = ?3",
            )?;
            for (pos, tid) in ids.iter().enumerate() {
                update.execute(params![pos as i32, playlist_id, tid])?;
            }
            conn.execute(
                "UPDATE playlists SET date_modified = datetime('now') WHERE id = ?1",
                params![playlist_id],
            )?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(e) => {
                conn.execute_batch("ROLLBACK")?;
                Err(e)
            }
        }
    }

    /// Get all playlists.
    pub fn get_all_playlists(&self) -> RhythmResult<Vec<crate::Playlist>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, date_created, date_modified FROM playlists ORDER BY name COLLATE NOCASE",
        )?;

        let playlists: Vec<(i64, String, Option<String>, Option<String>, Option<String>)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        playlists
            .into_iter()
            .map(|(id, name, desc, dc, dm)| {
                let tracks = self.get_playlist_tracks(&conn, id)?;
                Ok(crate::Playlist {
                    id: Some(id),
                    name,
                    description: desc,
                    date_created: dc,
                    date_modified: dm,
                    tracks,
                })
            })
            .collect()
    }

    /// Get a single playlist by ID.
    pub fn get_playlist(&self, playlist_id: i64) -> RhythmResult<crate::Playlist> {
        let conn = self.conn.lock().unwrap();
        let (id, name, desc, dc, dm): (
            i64,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = conn.query_row(
            "SELECT id, name, description, date_created, date_modified FROM playlists WHERE id = ?1",
            params![playlist_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )?;

        let tracks = self.get_playlist_tracks(&conn, id)?;

        Ok(crate::Playlist {
            id: Some(id),
            name,
            description: desc,
            date_created: dc,
            date_modified: dm,
            tracks,
        })
    }

    fn get_playlist_tracks(&self, conn: &Connection, playlist_id: i64) -> RhythmResult<Vec<TrackInfo>> {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.file_path, t.source_type, t.source_url, t.title, t.artist, t.album,
             t.album_artist, t.track_number, t.disc_number, t.genre, t.year, t.duration, t.format,
             t.bitrate, t.sample_rate, t.channels, t.file_size, t.date_added, t.last_played,
             t.play_count, t.artwork_path, t.is_available
             FROM tracks t
             INNER JOIN playlist_tracks pt ON t.id = pt.track_id
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position",
        )?;

        let tracks: Result<Vec<_>, _> = stmt
            .query_map(params![playlist_id], |row| row_to_track(row))?
            .collect();

        Ok(tracks?)
    }

    // ─── Search ───────────────────────────────────────────────────

    /// Full-text search across title, artist, album, and genre.
    pub fn search(&self, query: &str) -> RhythmResult<Vec<TrackInfo>> {
        let conn = self.conn.lock().unwrap();

        // Sanitize: escape FTS5 special characters
        let safe_query = query.replace(['*', '"', '(', ')'], "");

        let mut stmt = conn.prepare(
            "SELECT t.id, t.file_path, t.source_type, t.source_url, t.title, t.artist, t.album,
             t.album_artist, t.track_number, t.disc_number, t.genre, t.year, t.duration, t.format,
             t.bitrate, t.sample_rate, t.channels, t.file_size, t.date_added, t.last_played,
             t.play_count, t.artwork_path, t.is_available
             FROM tracks t
             INNER JOIN tracks_fts fts ON t.id = fts.rowid
             WHERE tracks_fts MATCH ?1
             ORDER BY rank
             LIMIT 100",
        )?;

        let tracks: Result<Vec<_>, _> = stmt
            .query_map(params![safe_query], |row| row_to_track(row))?
            .collect();

        Ok(tracks?)
    }

    /// Batch-import tracks from a directory scan.
    pub fn import_from_directory(&self, dir: &Path) -> RhythmResult<usize> {
        use crate::metadata::{scan_directory, extract_artwork};

        // Determine cache directory relative to the database
        let artwork_cache = dir.parent().unwrap_or(Path::new(".")).join(".rhythm_artwork");

        let tracks = scan_directory(dir)?;
        let count = tracks.len();

        for track in &tracks {
            // Try to extract artwork for local files
            let mut t = track.clone();
            if t.source_type == SourceType::Local {
                if let Some(ref file_path) = t.file_path {
                    if let Ok(Some(art_path)) = extract_artwork(Path::new(file_path), &artwork_cache) {
                        t.artwork_path = Some(art_path);
                    }
                }
            }
            if let Err(e) = self.add_track(&t) {
                log::warn!("Failed to import {}: {e}", t.title);
            }
        }

        Ok(count)
    }

    /// Import a single audio file into the library.
    ///
    /// Uses `is_supported_audio` as the gate, extracts metadata & artwork,
    /// then delegates to `add_track`. Returns 1 on success, or an error
    /// if the file is not a supported audio format or cannot be read.
    pub fn import_file(&self, file_path: &Path) -> RhythmResult<i32> {
        use crate::metadata::{is_supported_audio, extract_track_info, extract_artwork};

        if !is_supported_audio(file_path) {
            return Err(crate::RhythmError::UnsupportedFormat(format!(
                "Unsupported audio format: {}",
                file_path.display()
            )));
        }

        let mut track = extract_track_info(file_path)?;

        // Artwork cache goes next to the file (unlike directory imports,
        // where .rhythm_artwork sits next to the imported directory).
        let artwork_cache = file_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(".rhythm_artwork");
        if let Ok(Some(art_path)) = extract_artwork(file_path, &artwork_cache) {
            track.artwork_path = Some(art_path);
        }

        self.add_track(&track)?;
        Ok(1)
    }
}

fn row_to_track(row: &rusqlite::Row<'_>) -> rusqlite::Result<TrackInfo> {
    let source_type_str: String = row.get(2)?;
    let source_type = SourceType::try_from(source_type_str.as_str())
        .unwrap_or(SourceType::Local);

    Ok(TrackInfo {
        id: row.get(0)?,
        file_path: row.get(1)?,
        source_type,
        source_url: row.get(3)?,
        title: row.get(4)?,
        artist: row.get(5)?,
        album: row.get(6)?,
        album_artist: row.get(7)?,
        track_number: row.get(8)?,
        disc_number: row.get(9)?,
        genre: row.get(10)?,
        year: row.get(11)?,
        duration: row.get(12)?,
        format: row.get(13)?,
        bitrate: row.get(14)?,
        sample_rate: row.get(15)?,
        channels: row.get(16)?,
        file_size: row.get(17)?,
        date_added: row.get(18)?,
        last_played: row.get(19)?,
        play_count: row.get(20).unwrap_or(0),
        artwork_path: row.get(21)?,
        is_available: row.get::<_, i32>(22)? != 0,
    })
}
