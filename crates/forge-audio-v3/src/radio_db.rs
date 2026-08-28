//! SQLite-backed radio library — search, scan metadata, play history.
//!
//! Port of `dreadpirateradio/forge-audio::radio_db` (quarry 2026-06-25).
//! Gated behind `--features radio-db` (rusqlite bundled; no system dep).
//!
//! Tables: tracks, play_history, ghost_words, ghost_stems, hotcues,
//! beatgrids, smart_crates, mb_cache. Auto-migrates older DBs on open.

use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Metadata for a single track in the library.
///
/// `bpm_mbpm`/`duration_ms`/`energy_pmy` are integer fixed-point (2026-08-19,
/// Sean: "keeps forge-audio fully aligned with the no-float law, prevents
/// floating-point CPU drift across targets") — the DB/persistence boundary is
/// integer-exact; the live DSP/mixer domain downstream (`Deck`, `mixer.rs`)
/// stays float-native by its own established convention (audio processing),
/// converting at the one crossing point in `music_deck.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryTrack {
    pub id: i64,
    pub path: String,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub genre: Option<String>,
    /// Milli-BPM: `128_500` = 128.5 BPM.
    pub bpm_mbpm: Option<i32>,
    /// Track length in whole milliseconds.
    pub duration_ms: Option<i32>,
    /// Normalized energy, permyriad (`0..=10_000` == `0.0..=1.0`).
    pub energy_pmy: Option<i32>,
    pub key: Option<String>,
}

/// Sort column for a track list, as a TYPE rather than a loose string.
///
/// Lives here because [`RadioDb::safe_sort_col`] is the whitelist it lowers to —
/// a caller that holds a `SortCol` cannot name a column the query rejects.
/// Folded down from `technothesia::library` (2026-08-03), which re-exports it;
/// the four variants and their indices are unchanged so `from_idx` still means
/// what the terminal's column headers mean.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortCol { #[default] Title = 0, Artist = 1, Genre = 2, Duration = 3 }

impl SortCol {
    /// Column index as clicked on a header row.
    pub fn from_idx(i: usize) -> Self {
        match i { 1 => Self::Artist, 2 => Self::Genre, 3 => Self::Duration, _ => Self::Title }
    }
    /// The `tracks` column this sorts on — always inside `safe_sort_col`'s whitelist.
    pub fn as_sql(&self) -> &'static str {
        match self { Self::Title => "title", Self::Artist => "artist",
                     Self::Genre => "genre", Self::Duration => "duration_secs" }
    }
    /// Header label for the column.
    pub fn label(&self) -> &'static str {
        match self { Self::Title => "TITLE", Self::Artist => "ARTIST",
                     Self::Genre => "GENRE", Self::Duration => "TIME" }
    }
}

/// SQLite-backed radio library for search, picks, and play history.
pub struct RadioDb {
    conn: Connection,
}

impl RadioDb {
    /// Open (or create) the database at `path` and ensure tables exist.
    pub fn open(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        Self::init_tables(&conn)?;
        Ok(Self { conn })
    }

    /// Build from an existing connection (useful for in-memory test DBs).
    pub fn from_connection(conn: Connection) -> SqlResult<Self> {
        Self::init_tables(&conn)?;
        Ok(Self { conn })
    }

    /// Pick a random track from the library.
    pub fn get_random_track(&self) -> SqlResult<Option<LibraryTrack>> {
        self.conn.query_row(
            "SELECT id, path, artist, title, genre, bpm, duration_secs, energy, musical_key
             FROM tracks ORDER BY RANDOM() LIMIT 1",
            [],
            |row| Ok(Some(LibraryTrack {
                id: row.get(0)?, path: row.get(1)?, artist: row.get(2)?,
                title: row.get(3)?, genre: row.get(4)?,
                bpm_mbpm: Self::tolerant_int(row, 5, 1000.0)?,
                duration_ms: Self::tolerant_int(row, 6, 1000.0)?,
                energy_pmy: Self::tolerant_int(row, 7, 10_000.0)?,
                key: row.get(8)?,
            }))
        ).optional().map(|opt| opt.flatten())
    }

    /// Full-text-ish search across artist, title, genre with optional filters.
    pub fn search(
        &self,
        query: &str,
        genre: Option<&str>,
        bpm_min: Option<i32>,
        bpm_max: Option<i32>,
        limit: usize,
        key: Option<&str>,
        order_by: Option<&str>,
        order_dir: Option<&str>,
    ) -> SqlResult<Vec<LibraryTrack>> {
        let mut sql = String::from(
            "SELECT id, path, artist, title, genre, bpm, duration_secs, NULL AS energy, musical_key
             FROM tracks WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if !query.is_empty() {
            sql.push_str(" AND (artist LIKE ?1 OR title LIKE ?1 OR genre LIKE ?1)");
            param_values.push(Box::new(format!("%{query}%")));
        }
        let mut idx = param_values.len() + 1;

        if let Some(g) = genre {
            sql.push_str(&format!(" AND genre = ?{idx}"));
            param_values.push(Box::new(g.to_string())); idx += 1;
        }
        if let Some(lo) = bpm_min {
            sql.push_str(&format!(" AND bpm >= ?{idx}"));
            param_values.push(Box::new(lo)); idx += 1;
        }
        if let Some(hi) = bpm_max {
            sql.push_str(&format!(" AND bpm <= ?{idx}"));
            param_values.push(Box::new(hi)); idx += 1;
        }
        if let Some(k) = key {
            sql.push_str(&format!(" AND musical_key = ?{idx}"));
            param_values.push(Box::new(k.to_string())); idx += 1;
        }

        let col = Self::safe_sort_col(order_by);
        let dir = if order_dir == Some("desc") { "DESC" } else { "ASC" };
        sql.push_str(&format!(" ORDER BY {col} {dir} NULLS LAST LIMIT ?{idx}"));
        param_values.push(Box::new(limit as i64));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_ref.as_slice(), Self::row_to_track)?;
        rows.collect()
    }

    /// Paginated search with offset.
    pub fn search_paged(
        &self,
        query: &str,
        genre: Option<&str>,
        bpm_min: Option<i32>,
        bpm_max: Option<i32>,
        key: Option<&str>,
        order_by: Option<&str>,
        order_dir: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> SqlResult<Vec<LibraryTrack>> {
        let mut sql = String::from(
            "SELECT id, path, artist, title, genre, bpm, duration_secs, NULL AS energy, musical_key
             FROM tracks WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if !query.is_empty() {
            sql.push_str(" AND (artist LIKE ?1 OR title LIKE ?1 OR genre LIKE ?1)");
            param_values.push(Box::new(format!("%{query}%")));
        }
        let mut idx = param_values.len() + 1;

        if let Some(g) = genre {
            sql.push_str(&format!(" AND genre = ?{idx}"));
            param_values.push(Box::new(g.to_string())); idx += 1;
        }
        if let Some(lo) = bpm_min {
            sql.push_str(&format!(" AND bpm >= ?{idx}"));
            param_values.push(Box::new(lo)); idx += 1;
        }
        if let Some(hi) = bpm_max {
            sql.push_str(&format!(" AND bpm <= ?{idx}"));
            param_values.push(Box::new(hi)); idx += 1;
        }
        if let Some(k) = key {
            sql.push_str(&format!(" AND musical_key = ?{idx}"));
            param_values.push(Box::new(k.to_string())); idx += 1;
        }

        let col = Self::safe_sort_col(order_by);
        let dir = if order_dir == Some("desc") { "DESC" } else { "ASC" };
        sql.push_str(&format!(
            " ORDER BY {col} {dir} NULLS LAST LIMIT ?{idx} OFFSET ?{}",
            idx + 1
        ));
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_ref.as_slice(), Self::row_to_track)?;
        rows.collect()
    }

    /// Count tracks matching search criteria (for pagination display).
    pub fn search_count(
        &self,
        query: &str,
        genre: Option<&str>,
        bpm_min: Option<i32>,
        bpm_max: Option<i32>,
        key: Option<&str>,
    ) -> SqlResult<usize> {
        let mut sql = String::from("SELECT COUNT(*) FROM tracks WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if !query.is_empty() {
            sql.push_str(" AND (artist LIKE ?1 OR title LIKE ?1 OR genre LIKE ?1)");
            param_values.push(Box::new(format!("%{query}%")));
        }
        let mut idx = param_values.len() + 1;

        if let Some(g) = genre { sql.push_str(&format!(" AND genre = ?{idx}")); param_values.push(Box::new(g.to_string())); idx += 1; }
        if let Some(lo) = bpm_min { sql.push_str(&format!(" AND bpm >= ?{idx}")); param_values.push(Box::new(lo)); idx += 1; }
        if let Some(hi) = bpm_max { sql.push_str(&format!(" AND bpm <= ?{idx}")); param_values.push(Box::new(hi)); idx += 1; }
        if let Some(k) = key { sql.push_str(&format!(" AND musical_key = ?{idx}")); param_values.push(Box::new(k.to_string())); let _ = idx; }

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|b| b.as_ref()).collect();
        let count: i64 = self.conn.query_row(&sql, params_ref.as_slice(), |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Return the set of track IDs (from `ids`) that have at least one stem entry.
    pub fn tracks_with_stems(&self, ids: &[i64]) -> SqlResult<HashSet<i64>> {
        if ids.is_empty() { return Ok(HashSet::new()); }
        // Build parameterized IN clause
        let placeholders: String = ids.iter().enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT DISTINCT track_id FROM ghost_stems WHERE track_id IN ({placeholders})"
        );
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_ref.as_slice(), |row| row.get::<_, i64>(0))?;
        rows.collect::<SqlResult<HashSet<_>>>()
    }

    /// Total number of tracks in the library.
    pub fn track_count(&self) -> SqlResult<usize> {
        let n: i64 = self.conn.query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// True if `path` is already indexed (fast path, no file I/O).
    pub fn track_exists_by_path(&self, path: &str) -> SqlResult<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE path = ?1",
            rusqlite::params![path],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// True if a track with the same content hash is already indexed (dedup).
    pub fn track_exists_by_hash(&self, hash: &str) -> SqlResult<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tracks WHERE sha256 = ?1",
            rusqlite::params![hash],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Insert a fully analysed track record.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_track_full(
        &mut self,
        path: &str,
        size_bytes: i64,
        sha256: &str,
        artist: Option<&str>,
        album: Option<&str>,
        title: Option<&str>,
        genre: Option<&str>,
        year: Option<i32>,
        duration_ms: i32,
        sample_rate: i32,
        channels: i32,
        format: &str,
        bpm_mbpm: Option<i32>,
        musical_key: Option<&str>,
        detected_genre: Option<&str>,
        waveform_peaks: Option<&[u8]>,
    ) -> SqlResult<i64> {
        let effective_genre = detected_genre.or(genre);
        self.conn.execute(
            "INSERT OR IGNORE INTO tracks
             (path, size_bytes, sha256, artist, album, title, genre, year,
              duration_secs, sample_rate, channels, format, bpm, musical_key, waveform_peaks)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            rusqlite::params![
                path, size_bytes, sha256, artist, album, title, effective_genre, year,
                duration_ms, sample_rate, channels, format, bpm_mbpm, musical_key, waveform_peaks,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Resolve the integer track id for `path`, or `None` if not indexed.
    pub fn get_track_id_by_path(&self, path: &str) -> SqlResult<Option<i64>> {
        self.conn.query_row(
            "SELECT id FROM tracks WHERE path = ?1",
            rusqlite::params![path],
            |r| r.get(0),
        ).optional()
    }

    /// Load all hotcues for `track_id`. Returns a fixed 8-slot vec where absent slots are `None`.
    pub fn load_hotcues(&self, track_id: i64) -> SqlResult<Vec<Option<(f64, Option<String>, Option<String>)>>> {
        let mut stmt = self.conn.prepare(
            "SELECT slot, position_frac, label, color FROM hotcues WHERE track_id = ?1 ORDER BY slot"
        )?;
        let rows = stmt.query_map(rusqlite::params![track_id], |r| {
            Ok((r.get::<_, i64>(0)? as usize, r.get::<_, f64>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?))
        })?;
        let mut cues: Vec<Option<(f64, Option<String>, Option<String>)>> = vec![None; 8];
        for row in rows {
            let (slot, frac, label, color) = row?;
            if slot < 8 { cues[slot] = Some((frac, label, color)); }
        }
        Ok(cues)
    }

    /// Persist a hotcue (upsert by track_id + slot).
    pub fn save_hotcue(&self, track_id: i64, slot: usize, position_frac: f64, label: Option<&str>, color: Option<&str>) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO hotcues (track_id, slot, position_frac, label, color) VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(track_id, slot) DO UPDATE SET position_frac=excluded.position_frac, label=excluded.label, color=excluded.color",
            rusqlite::params![track_id, slot as i64, position_frac, label, color],
        )?;
        Ok(())
    }

    /// Remove a hotcue by track and slot index.
    pub fn delete_hotcue(&self, track_id: i64, slot: usize) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM hotcues WHERE track_id = ?1 AND slot = ?2",
            rusqlite::params![track_id, slot as i64],
        )?;
        Ok(())
    }

    /// Tolerant numeric read (2026-08-19, real bug against real data: 3,147
    /// tracks in a live `radio.db` were written under the pre-conversion
    /// schema, `bpm`/`duration_secs`/`energy` stored as SQLite REAL — a
    /// plain `row.get::<_, i32>` errors on those rows, `INSERT OR IGNORE`
    /// never got a chance to normalize them since they already exist).
    /// Reads whichever storage class is actually there: an `Integer` column
    /// is already the new convention, taken as-is; a `Real` column is the
    /// legacy raw unit (bpm, seconds, or a 0.0..1.0 fraction), scaled up and
    /// rounded into the same convention on the fly. The file itself is never
    /// rewritten — this is a read-time normalization, not a migration.
    fn tolerant_int(row: &rusqlite::Row<'_>, idx: usize, legacy_scale: f64) -> rusqlite::Result<Option<i32>> {
        use rusqlite::types::ValueRef;
        match row.get_ref(idx)? {
            ValueRef::Null => Ok(None),
            ValueRef::Integer(i) => Ok(Some(i as i32)),
            ValueRef::Real(f) => Ok(Some((f * legacy_scale).round() as i32)),
            other => Err(rusqlite::Error::InvalidColumnType(idx, format!("{other:?}"), rusqlite::types::Type::Integer)),
        }
    }

    fn row_to_track(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryTrack> {
        Ok(LibraryTrack {
            id: row.get(0)?, path: row.get(1)?, artist: row.get(2)?,
            title: row.get(3)?, genre: row.get(4)?,
            bpm_mbpm: Self::tolerant_int(row, 5, 1000.0)?,
            duration_ms: Self::tolerant_int(row, 6, 1000.0)?,
            energy_pmy: Self::tolerant_int(row, 7, 10_000.0)?,
            key: row.get(8)?,
        })
    }

    fn safe_sort_col(col: Option<&str>) -> &'static str {
        match col.unwrap_or("title") {
            "artist" => "artist", "title" => "title", "bpm" => "bpm",
            "key_tag" | "musical_key" => "musical_key", "genre" => "genre",
            "duration_secs" => "duration_secs", _ => "title",
        }
    }

    fn init_tables(conn: &Connection) -> SqlResult<()> {
        // SQL column names (`duration_secs`, `bpm`, `energy`) are unchanged from the
        // donor for migration continuity; their unit is now milliseconds/milli-BPM/
        // permyriad respectively (2026-08-19 integer conversion) — the Rust-side
        // `LibraryTrack` field names carry the real unit (`duration_ms`/`bpm_mbpm`/
        // `energy_pmy`), the SQL identifiers do not.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS play_history (
                id INTEGER PRIMARY KEY,
                track_id INTEGER,
                played_at TEXT NOT NULL DEFAULT (datetime('now')),
                source TEXT DEFAULT 'autopilot'
            );
            CREATE INDEX IF NOT EXISTS idx_ph_time ON play_history(played_at);
            CREATE INDEX IF NOT EXISTS idx_ph_track ON play_history(track_id);
            CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                size_bytes INTEGER NOT NULL DEFAULT 0,
                sha256 TEXT, artist TEXT, album TEXT, title TEXT, genre TEXT,
                year INTEGER, duration_secs INTEGER, sample_rate INTEGER, channels INTEGER,
                format TEXT NOT NULL DEFAULT 'unknown', bpm INTEGER, waveform_peaks BLOB,
                energy INTEGER, musical_key TEXT, vocal_energy BLOB,
                scanned_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_tracks_bpm ON tracks(bpm);
            CREATE INDEX IF NOT EXISTS idx_tracks_genre ON tracks(genre);
            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE TABLE IF NOT EXISTS ghost_stems (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id INTEGER NOT NULL,
                stem_type TEXT NOT NULL,
                file_path TEXT, energy BLOB,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (track_id) REFERENCES tracks(id),
                UNIQUE(track_id, stem_type)
            );
            CREATE INDEX IF NOT EXISTS idx_gs_track ON ghost_stems(track_id);
            CREATE TABLE IF NOT EXISTS hotcues (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id INTEGER NOT NULL,
                slot INTEGER NOT NULL,
                position_frac REAL NOT NULL,
                label TEXT,
                color TEXT,
                FOREIGN KEY (track_id) REFERENCES tracks(id),
                UNIQUE(track_id, slot)
            );
            CREATE INDEX IF NOT EXISTS idx_hc_track ON hotcues(track_id);",
        )?;
        // Auto-migrate: add columns absent in older DBs (errors silently ignored)
        for sql in &[
            "ALTER TABLE tracks ADD COLUMN vocal_energy BLOB",
            "ALTER TABLE tracks ADD COLUMN musical_key TEXT",
            "ALTER TABLE tracks ADD COLUMN audio_hash TEXT",
        ] {
            let _ = conn.execute(sql, []);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory() -> RadioDb {
        RadioDb::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }

    /// The point of the type: every variant lowers to a column the query keeps.
    /// A variant that fell through `safe_sort_col` would silently sort by title.
    #[test]
    fn sort_col_survives_the_whitelist() {
        for c in [SortCol::Title, SortCol::Artist, SortCol::Genre, SortCol::Duration] {
            assert_eq!(RadioDb::safe_sort_col(Some(c.as_sql())), c.as_sql());
        }
    }

    #[test]
    fn sort_col_from_idx_matches_header_order() {
        assert_eq!(SortCol::from_idx(0), SortCol::Title);
        assert_eq!(SortCol::from_idx(3), SortCol::Duration);
        assert_eq!(SortCol::from_idx(99), SortCol::Title);
    }

    #[test]
    fn empty_search_returns_empty() {
        let db = in_memory();
        assert_eq!(db.search("", None, None, None, 100, None, None, None).unwrap().len(), 0);
    }

    #[test]
    fn search_count_empty_db() {
        let db = in_memory();
        assert_eq!(db.search_count("", None, None, None, None).unwrap(), 0);
    }

    #[test]
    fn tracks_with_stems_empty_ids() {
        let db = in_memory();
        assert!(db.tracks_with_stems(&[]).unwrap().is_empty());
    }

    #[test]
    fn get_random_track_empty_db() {
        let db = in_memory();
        assert!(db.get_random_track().unwrap().is_none());
    }

    /// The point of the 2026-08-19 integer conversion: bpm/duration/energy
    /// round-trip through SQLite as EXACT integers, never a float that could
    /// drift a bit differently on a different CPU/build.
    #[test]
    fn bpm_duration_energy_round_trip_as_exact_integers() {
        let mut db = in_memory();
        db.insert_track_full(
            "/roads.mp3", 4_200_000, "deadbeef",
            Some("Portishead"), Some("Dummy"), Some("Roads"), Some("Trip-Hop"),
            Some(1994), 248_500, 44_100, 2, "mp3",
            Some(90_500), Some("8A"), Some("Trip-Hop"), None,
        ).unwrap();

        let track = db.get_random_track().unwrap().unwrap();
        assert_eq!(track.bpm_mbpm, Some(90_500), "90.5 BPM exactly, not 90.49999...");
        assert_eq!(track.duration_ms, Some(248_500), "4:08.5 exactly, in whole ms");
        // energy was never written (insert_track_full has no energy param —
        // it lands via a separate analysis pass elsewhere); NULL round-trips
        // as None, not 0 or a fabricated default.
        assert_eq!(track.energy_pmy, None);

        let found = db.search("Roads", None, Some(90_000), Some(91_000), 10, None, None, None).unwrap();
        assert_eq!(found.len(), 1, "90_500 mbpm must fall inside a 90_000..91_000 range filter");
        assert_eq!(found[0].bpm_mbpm, Some(90_500));

        let missed = db.search("Roads", None, Some(100_000), Some(110_000), 10, None, None, None).unwrap();
        assert!(missed.is_empty(), "90_500 mbpm must NOT fall inside a 100_000..110_000 range filter");
    }
}
