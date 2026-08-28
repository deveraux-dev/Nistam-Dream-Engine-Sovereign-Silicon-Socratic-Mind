//! Title/artist resolver for track metadata.
//!
//! Used by deck-load call sites to derive visible title/artist from:
//! 1. Database values (tag-scanned at library scan time), when non-empty.
//! 2. Filename-based parsing, as fallback for tag-less tracks.
//!
//! Conventions handled by `parse_filename`:
//!   "Artist - Title.mp3"              -> (Title, Artist)
//!   "01 - Artist - Title.mp3"         -> (Title, Artist)
//!   "01. Artist - Title.mp3"          -> (Title, Artist)
//!   "01 Title.mp3"                    -> (Title, "")
//!   "Title.mp3"                       -> (Title, "")
//!   "Artist - Song With - Dashes.mp3" -> (Song With - Dashes, Artist)

use std::path::Path;

const SEPARATOR: &str = " - ";

/// Parse `(title, artist)` out of a file path by filename convention.
///
/// Strips extension, strips a leading track-number prefix if present,
/// then splits on " - ". First segment is artist, remainder is title.
/// Single-segment filenames return `(stem, "")`.
pub fn parse_filename(path: &Path) -> (String, String) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .trim();
    if stem.is_empty() {
        return (String::new(), String::new());
    }

    let body = strip_track_number_prefix(stem);

    if let Some(idx) = body.find(SEPARATOR) {
        let artist = body[..idx].trim().to_string();
        let title = body[idx + SEPARATOR.len()..].trim().to_string();
        if !artist.is_empty() && !title.is_empty() {
            return (title, artist);
        }
    }

    (body.to_string(), String::new())
}

/// Prefer DB metadata when populated; fall back to filename parse.
///
/// `db_title` / `db_artist` are treated as "populated" only when `Some`
/// and non-empty after trim — so `Some("")` is ignored and the filename
/// is consulted instead.
pub fn resolve_deck_metadata(
    path: &Path,
    db_title: Option<&str>,
    db_artist: Option<&str>,
) -> (String, String) {
    let db_title = db_title.map(str::trim).filter(|s| !s.is_empty());
    let db_artist = db_artist.map(str::trim).filter(|s| !s.is_empty());

    if let (Some(t), Some(a)) = (db_title, db_artist) {
        return (t.to_string(), a.to_string());
    }

    let (fn_title, fn_artist) = parse_filename(path);
    let title = db_title.map(str::to_string).unwrap_or(fn_title);
    let artist = db_artist.map(str::to_string).unwrap_or(fn_artist);
    (title, artist)
}

fn strip_track_number_prefix(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Track numbers are 1-3 digits. 4+ digits is almost always a year or date,
    // which belongs in the title (e.g. "2024 Remix", "1999 Live at Wembley").
    if i == 0 || i > 3 {
        return s;
    }
    // Require at least one separator character after the digits:
    // ". ", " - ", "- ", " ", "-", "."
    let rest = &s[i..];
    let trimmed = rest
        .trim_start_matches(|c: char| c == '.' || c == '-' || c.is_whitespace());
    if trimmed.len() == rest.len() {
        // Digits without separator — not a track-number prefix.
        return s;
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(s: &str) -> PathBuf { PathBuf::from(s) }

    #[test]
    fn artist_dash_title() {
        let (t, a) = parse_filename(&p("Aphex Twin - Xtal.flac"));
        assert_eq!(t, "Xtal");
        assert_eq!(a, "Aphex Twin");
    }

    #[test]
    fn track_number_dash_artist_dash_title() {
        let (t, a) = parse_filename(&p("03 - Boards of Canada - Roygbiv.mp3"));
        assert_eq!(t, "Roygbiv");
        assert_eq!(a, "Boards of Canada");
    }

    #[test]
    fn track_number_dot_artist_dash_title() {
        let (t, a) = parse_filename(&p("03. Boards of Canada - Roygbiv.mp3"));
        assert_eq!(t, "Roygbiv");
        assert_eq!(a, "Boards of Canada");
    }

    #[test]
    fn track_number_space_title_only() {
        let (t, a) = parse_filename(&p("07 Intro.wav"));
        assert_eq!(t, "Intro");
        assert_eq!(a, "");
    }

    #[test]
    fn title_only_no_artist() {
        let (t, a) = parse_filename(&p("Untitled.mp3"));
        assert_eq!(t, "Untitled");
        assert_eq!(a, "");
    }

    #[test]
    fn dashes_in_title_preserved() {
        let (t, a) = parse_filename(&p("Radiohead - Everything In Its Right Place - Remastered.mp3"));
        assert_eq!(t, "Everything In Its Right Place - Remastered");
        assert_eq!(a, "Radiohead");
    }

    #[test]
    fn leading_year_not_stripped() {
        // "2024 Remix" — "2024" is a token, not a track number (no separator after digits).
        let (t, a) = parse_filename(&p("2024 Remix.mp3"));
        assert_eq!(t, "2024 Remix");
        assert_eq!(a, "");
    }

    #[test]
    fn empty_path() {
        let (t, a) = parse_filename(&p(""));
        assert_eq!(t, "");
        assert_eq!(a, "");
    }

    #[test]
    fn whitespace_trimmed() {
        let (t, a) = parse_filename(&p("  Boards of Canada  -  Roygbiv  .mp3"));
        assert_eq!(t, "Roygbiv");
        assert_eq!(a, "Boards of Canada");
    }

    #[test]
    fn resolve_prefers_db_when_both_present() {
        let (t, a) = resolve_deck_metadata(
            &p("01 - X - Y.mp3"),
            Some("DB Title"),
            Some("DB Artist"),
        );
        assert_eq!(t, "DB Title");
        assert_eq!(a, "DB Artist");
    }

    #[test]
    fn resolve_falls_back_when_db_empty() {
        let (t, a) = resolve_deck_metadata(
            &p("Aphex Twin - Xtal.flac"),
            None,
            None,
        );
        assert_eq!(t, "Xtal");
        assert_eq!(a, "Aphex Twin");
    }

    #[test]
    fn resolve_treats_empty_string_as_missing() {
        let (t, a) = resolve_deck_metadata(
            &p("Aphex Twin - Xtal.flac"),
            Some(""),
            Some("   "),
        );
        assert_eq!(t, "Xtal");
        assert_eq!(a, "Aphex Twin");
    }

    #[test]
    fn resolve_mixes_db_and_filename() {
        // DB has artist but no title -> artist from DB, title from filename.
        let (t, a) = resolve_deck_metadata(
            &p("Some Artist - Actual Title.mp3"),
            None,
            Some("Curated Artist"),
        );
        assert_eq!(t, "Actual Title");
        assert_eq!(a, "Curated Artist");
    }
}
