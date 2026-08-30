//! `edl` — the per-frame production manifest (JSONL), and its bijection with
//! [`ReelClock`] columns. Ported 2026-08-26 from v2's
//! `F:\NewRepo\crates\forge-gui\src\reel\edl.rs` (210 LOC, std only).

use std::fs::File;
use std::io::{BufRead, BufReader, Result, Write};
use std::path::Path;

use forge_engine_v3::{EngineTick8, REGISTER_PURGATORIO, RUN_STATE_REPLAY};

use crate::clock::ReelClock;

/// One frame's row in the manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdlRow {
    /// Frame ordinal within the reel.
    pub frame: usize,
    /// The rendered file this row points at.
    pub file: String,
    /// The 120Hz carrier frame this row was stamped on — the tape join.
    pub tick: u64,
    /// Scene ordinal.
    pub scene: usize,
    /// The line the frame carries.
    pub truth: String,
    /// Palette token name.
    pub palette: String,
    /// Camera move token.
    pub cam: String,
    /// Frame carries a flash.
    pub flash: bool,
    /// Frame carries a scar.
    pub scar: bool,
    /// MIDI note for the voice, if any.
    pub voice_note: Option<u8>,
    /// Audio offset in milliseconds.
    pub wav_offset_ms: u64,
}

/// The manifest's closing receipt.
///
/// v2 hardcoded `"size":"360x260"` and `"wav":"voice.wav"` into the writer —
/// its own `dual_rail` dimensions baked into a manifest that claims to describe
/// any reel. Carried as data here so the receipt cannot lie about a reel it did
/// not render.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdlReceipt {
    /// Audio file the offsets index into.
    pub wav: String,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
}

impl EdlReceipt {
    /// The v2 shape, for a reel that really is 360x260 over `voice.wav`.
    pub fn dual_rail() -> Self {
        Self { wav: "voice.wav".to_string(), width: 360, height: 260 }
    }
}

/// Manifest refusals — typed, never a bare string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdlError {
    /// A row's `tick` exceeds the carrier's `u32` frame counter.
    TickOutOfRange {
        /// The offending row's frame ordinal.
        frame: usize,
        /// The tick that would not encode.
        tick: u64,
    },
    /// `EngineTick8::encode` refused the frame outright.
    TickUnencodable {
        /// The carrier frame that failed to encode.
        frame: u32,
    },
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// One row as a JSONL line. Field order is fixed — the manifest is diffed.
pub fn row_json(r: &EdlRow) -> String {
    let voice_note = r.voice_note.map(|n| n.to_string()).unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"frame\":{},\"file\":\"{}\",\"tick\":{},\"scene\":{},\"truth\":\"{}\",\"palette\":\"{}\",\"cam\":\"{}\",\"flash\":{},\"scar\":{},\"voice_note\":{},\"wav_offset_ms\":{}}}",
        r.frame,
        escape_json(&r.file),
        r.tick,
        r.scene,
        escape_json(&r.truth),
        escape_json(&r.palette),
        escape_json(&r.cam),
        r.flash,
        r.scar,
        voice_note,
        r.wav_offset_ms,
    )
}

/// The per-column audio stride, read off the first two rows rather than passed:
/// the row stream is the only source of truth for it.
fn derive_col_ms(rows: &[EdlRow]) -> u64 {
    if rows.len() >= 2 {
        rows[1].wav_offset_ms.saturating_sub(rows[0].wav_offset_ms)
    } else {
        0
    }
}

/// Write the manifest: one line per row, then the receipt line.
pub fn write_edl(path: &Path, rows: &[EdlRow], receipt: &EdlReceipt) -> Result<usize> {
    let mut f = File::create(path)?;
    for r in rows {
        writeln!(f, "{}", row_json(r))?;
    }
    writeln!(
        f,
        "{{\"receipt\":{{\"frames\":{},\"col_ms\":{},\"wav\":\"{}\",\"size\":\"{}x{}\"}}}}",
        rows.len(),
        derive_col_ms(rows),
        escape_json(&receipt.wav),
        receipt.width,
        receipt.height,
    )?;
    Ok(rows.len())
}

/// Count the manifest's rows, skipping blanks and the receipt.
pub fn read_edl_count(path: &Path) -> Result<usize> {
    let f = File::open(path)?;
    let mut n = 0usize;
    for line in BufReader::new(f).lines() {
        let line = line?;
        let t = line.trim();
        if t.is_empty() || t.starts_with("{\"receipt\"") {
            continue;
        }
        n += 1;
    }
    Ok(n)
}

/// A row's carrier tick in REPLAY state — the scrub-lane stamp.
pub fn row_tick(r: &EdlRow) -> std::result::Result<EngineTick8, EdlError> {
    let frame =
        u32::try_from(r.tick).map_err(|_| EdlError::TickOutOfRange { frame: r.frame, tick: r.tick })?;
    EngineTick8::encode(frame, RUN_STATE_REPLAY, REGISTER_PURGATORIO)
        .ok_or(EdlError::TickUnencodable { frame })
}

/// Which dwell column a row falls in. The clock owns the arithmetic (L05) —
/// this only joins a row to it.
pub fn column_of(r: &EdlRow, clock: &ReelClock) -> std::result::Result<u32, EdlError> {
    Ok(clock.column_at(row_tick(r)?))
}

/// The FIRST row in `rows` that falls in `column` — the scrub answer.
///
/// The join runs the other way from [`column_of`], so a scrub and a stamp are
/// inverse: scrubbing to a row's own column finds that row back (L07).
pub fn scrub_to<'a>(
    rows: &'a [EdlRow],
    column: u32,
    clock: &ReelClock,
) -> Option<&'a EdlRow> {
    rows.iter().find(|r| column_of(r, clock).is_ok_and(|c| c == column))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_path(tag: &str) -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("edl_v3_{}_{}_{}.jsonl", std::process::id(), n, tag))
    }

    fn sample_row(frame: usize) -> EdlRow {
        EdlRow {
            frame,
            file: format!("{frame:04}.bmp"),
            tick: frame as u64 * 120,
            scene: frame / 8,
            truth: "the frame holds".to_string(),
            palette: "steel-blue".to_string(),
            cam: "hold".to_string(),
            flash: false,
            scar: false,
            voice_note: Some(60),
            wav_offset_ms: frame as u64 * 200,
        }
    }

    #[test]
    fn row_json_is_deterministic() {
        let r = sample_row(3);
        assert_eq!(row_json(&r), row_json(&r));
    }

    #[test]
    fn row_json_field_order_is_stable() {
        let j = row_json(&sample_row(0));
        assert!(j.starts_with("{\"frame\":0,\"file\":\"0000.bmp\",\"tick\":0,\"scene\":0,\"truth\":"));
        assert!(j.ends_with("\"wav_offset_ms\":0}"));
    }

    #[test]
    fn row_json_escapes_quotes_backslash_newline() {
        let mut r = sample_row(1);
        r.truth = "he said \"stop\\go\"\nnext line".to_string();
        let j = row_json(&r);
        assert!(j.contains("\\\"stop\\\\go\\\""));
        assert!(j.contains("\\n"));
        assert!(!j.contains("\"stop\\go\""), "an unescaped quote+backslash must not survive");
    }

    /// v2 typed `palette`/`cam` as `&'static str` and never escaped them. They
    /// are `String` here, so they take the same escaping every other field does.
    #[test]
    fn palette_and_cam_are_escaped_too() {
        let mut r = sample_row(1);
        r.palette = "a\"b".to_string();
        r.cam = "c\\d".to_string();
        let j = row_json(&r);
        assert!(j.contains("\"palette\":\"a\\\"b\""), "{j}");
        assert!(j.contains("\"cam\":\"c\\\\d\""), "{j}");
    }

    #[test]
    fn row_json_null_voice_note_when_absent() {
        let mut r = sample_row(2);
        r.voice_note = None;
        assert!(row_json(&r).contains("\"voice_note\":null"));
    }

    #[test]
    fn write_then_read_round_trips_row_count() {
        let path = tmp_path("roundtrip");
        let rows: Vec<EdlRow> = (0..5).map(sample_row).collect();
        assert_eq!(write_edl(&path, &rows, &EdlReceipt::dual_rail()).expect("write"), 5);
        assert_eq!(read_edl_count(&path).expect("count"), 5);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn receipt_line_is_last_and_carries_the_real_size() {
        let path = tmp_path("receipt");
        let rows: Vec<EdlRow> = (0..3).map(sample_row).collect();
        let receipt = EdlReceipt { wav: "take2.wav".to_string(), width: 1280, height: 720 };
        write_edl(&path, &rows, &receipt).expect("write");
        let text = std::fs::read_to_string(&path).expect("read back");
        let last = text.lines().last().expect("a line");
        assert!(last.starts_with("{\"receipt\":"), "{last}");
        assert!(last.contains("\"frames\":3"));
        // The v2 writer would have printed 360x260/voice.wav for this reel.
        assert!(last.contains("\"wav\":\"take2.wav\""), "{last}");
        assert!(last.contains("\"size\":\"1280x720\""), "{last}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn col_ms_derives_from_wav_offset_delta() {
        let path = tmp_path("colms");
        let rows: Vec<EdlRow> = (0..4).map(sample_row).collect();
        write_edl(&path, &rows, &EdlReceipt::dual_rail()).expect("write");
        let text = std::fs::read_to_string(&path).expect("read back");
        assert!(text.lines().last().unwrap().contains("\"col_ms\":200"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn single_row_col_ms_falls_back_to_zero() {
        let path = tmp_path("singlerow");
        write_edl(&path, &[sample_row(0)], &EdlReceipt::dual_rail()).expect("write");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.lines().last().unwrap().contains("\"col_ms\":0"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_rows_still_writes_a_receipt() {
        let path = tmp_path("empty");
        assert_eq!(write_edl(&path, &[], &EdlReceipt::dual_rail()).expect("write"), 0);
        assert_eq!(read_edl_count(&path).expect("count"), 0);
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_edl_count_on_missing_file_errors() {
        assert!(read_edl_count(&tmp_path("missing")).is_err());
    }

    #[test]
    fn write_edl_overwrites_a_stale_file() {
        let path = tmp_path("overwrite");
        let r = &EdlReceipt::dual_rail();
        write_edl(&path, &(0..6).map(sample_row).collect::<Vec<_>>(), r).expect("first");
        write_edl(&path, &(0..2).map(sample_row).collect::<Vec<_>>(), r).expect("second");
        assert_eq!(read_edl_count(&path).expect("count"), 2, "a rewrite replaces, never appends");
        let _ = std::fs::remove_file(&path);
    }

    // ── The scrub join: this is what v2's EDL could not do ──────────────────

    /// A row stamps into a column, and scrubbing that column finds the row back.
    #[test]
    fn a_row_and_its_column_are_inverse() {
        let clock = ReelClock::kept();
        let rows: Vec<EdlRow> = (0..8).map(sample_row).collect();
        for r in &rows {
            let col = column_of(r, &clock).expect("stamps");
            let found = scrub_to(&rows, col, &clock).expect("scrubs back");
            assert_eq!(column_of(found, &clock).unwrap(), col, "f(finv(c)) == c");
        }
    }

    /// KEPT_MS is 60 carrier frames per column, and the sample rows step 120
    /// frames apart — so every row lands two columns on from the last.
    #[test]
    fn the_clock_owns_the_column_width_not_this_module() {
        let clock = ReelClock::kept();
        assert_eq!(clock.frames_per_column(), 60);
        assert_eq!(column_of(&sample_row(0), &clock).unwrap(), 0);
        assert_eq!(column_of(&sample_row(1), &clock).unwrap(), 2);
        assert_eq!(column_of(&sample_row(3), &clock).unwrap(), 6);
    }

    /// A faster dwell re-columns the SAME rows — the manifest is not re-stamped,
    /// the clock is the only thing that moved.
    #[test]
    fn a_tighter_dwell_moves_the_columns_without_touching_the_rows() {
        let r = sample_row(1);
        assert_eq!(column_of(&r, &ReelClock::kept()).unwrap(), 2);
        assert_eq!(column_of(&r, &ReelClock::new(100)).unwrap(), 10);
    }

    #[test]
    fn a_column_with_no_row_scrubs_to_nothing() {
        let clock = ReelClock::kept();
        let rows: Vec<EdlRow> = (0..3).map(sample_row).collect();
        assert!(scrub_to(&rows, 1, &clock).is_none(), "odd columns hold no sample row");
        assert!(scrub_to(&rows, 9_999, &clock).is_none());
    }

    /// A tick past the 120Hz carrier's `u32` counter refuses by NAME rather than
    /// wrapping into some other column.
    #[test]
    fn an_out_of_range_tick_is_a_typed_refusal() {
        let mut r = sample_row(0);
        r.tick = u64::from(u32::MAX) + 1;
        assert_eq!(
            column_of(&r, &ReelClock::kept()),
            Err(EdlError::TickOutOfRange { frame: 0, tick: u64::from(u32::MAX) + 1 })
        );
    }
}
