//! The reel script as a MOD/XM pattern table — rows of columns, not Rust
//! literals. A new reel is a new table: no recompile, no Rust for the author.
//! Lowers to the `(micros, FrameType)` cuts `cutlist` already consumes.

use crate::droplaw::FrameType;

/// The tracker "no change" cell. A row that leaves a column empty INHERITS the
/// running value, exactly as a MOD pattern does — that is what makes a table
/// readable at a glance instead of a wall of repeats.
const HOLD: &str = "..";

/// Column count, in the header's order: note, palette, cam, flash, scar, truth.
const COLUMNS: usize = 6;

/// One row of the pattern — a single frame's worth of every column.
///
/// `None` means the row held the previous value; the resolved running state is
/// what [`resolve`] produces. Keeping "held" distinct from "set to the same
/// thing" is the difference between reading a table and guessing at one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptColumn {
    /// Row index as authored, 0-based.
    pub row: u32,
    /// Tracker note cell, e.g. `C-4`. Carried verbatim — this module does not
    /// pretend to know the instrument.
    pub note: Option<String>,
    /// Palette slot.
    pub palette: Option<u8>,
    /// Camera move, e.g. `PAN`, `PUSH`, `HOLD`.
    pub cam: Option<String>,
    /// Flash on this row.
    pub flash: bool,
    /// Scar index struck on this row.
    pub scar: Option<u8>,
    /// The line of truth spoken on this row.
    pub truth: Option<String>,
}

/// A parse failure that names its own row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternError {
    /// 1-based source line.
    pub line: u32,
    /// What went wrong.
    pub message: String,
}

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pattern line {}: {}", self.line, self.message)
    }
}

fn cell(raw: &str) -> Option<&str> {
    let t = raw.trim();
    if t.is_empty() || t.chars().all(|c| c == '.') {
        return None;
    }
    Some(t)
}

fn parse_u8(raw: &str, line: u32, what: &str) -> Result<u8, PatternError> {
    raw.parse::<u8>()
        .map_err(|_| PatternError { line, message: format!("{what} {raw:?} is not 0..=255") })
}

/// Parse a pattern table. `#` starts a comment, blank lines are skipped, and
/// every other line is `row | note | pal | cam | flash | scar | truth`.
///
/// Split on `|` and trimmed — no regex (CLAUDE.md forbidden_ops). The truth
/// column is last so it may contain anything except a pipe.
pub fn parse_pattern(src: &str) -> Result<Vec<ScriptColumn>, PatternError> {
    let mut out: Vec<ScriptColumn> = Vec::new();

    for (idx, raw_line) in src.lines().enumerate() {
        let line = (idx + 1) as u32;
        let text = raw_line.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = text.split('|').collect();
        if parts.len() != COLUMNS + 1 {
            return Err(PatternError {
                line,
                message: format!(
                    "expected {} columns (row | note | pal | cam | flash | scar | truth), found {}",
                    COLUMNS + 1,
                    parts.len()
                ),
            });
        }

        let row_raw = parts[0].trim();
        let row = row_raw.parse::<u32>().map_err(|_| PatternError {
            line,
            message: format!("row index {row_raw:?} is not a number"),
        })?;
        if let Some(prev) = out.last() {
            if row <= prev.row {
                return Err(PatternError {
                    line,
                    message: format!("row {row} does not advance past {}", prev.row),
                });
            }
        }

        let flash_cell = parts[4].trim();
        let flash = match flash_cell {
            c if c.is_empty() || c.chars().all(|ch| ch == '.') => false,
            "X" | "x" => true,
            other => {
                return Err(PatternError {
                    line,
                    message: format!("flash cell {other:?} must be X or {HOLD}"),
                })
            }
        };

        out.push(ScriptColumn {
            row,
            note: cell(parts[1]).map(str::to_string),
            palette: cell(parts[2]).map(|c| parse_u8(c, line, "palette")).transpose()?,
            cam: cell(parts[3]).map(str::to_string),
            flash,
            scar: cell(parts[5]).map(|c| parse_u8(c, line, "scar")).transpose()?,
            truth: cell(parts[6]).map(str::to_string),
        });
    }

    Ok(out)
}

/// Fill every held cell with the running value, tracker-style. `flash` is NOT
/// carried — a flash is an event on its row, not a state the reel sits in.
pub fn resolve(rows: &[ScriptColumn]) -> Vec<ScriptColumn> {
    let mut running = ScriptColumn::default();
    rows.iter()
        .map(|r| {
            if r.note.is_some() {
                running.note = r.note.clone();
            }
            if r.palette.is_some() {
                running.palette = r.palette;
            }
            if r.cam.is_some() {
                running.cam = r.cam.clone();
            }
            if r.scar.is_some() {
                running.scar = r.scar;
            }
            if r.truth.is_some() {
                running.truth = r.truth.clone();
            }
            ScriptColumn { row: r.row, flash: r.flash, ..running.clone() }
        })
        .collect()
}

/// `[AUTHORED]` mapping from a row's shape to a Drop Law frame type. The Drop
/// Law owns what each type COSTS; this only says which one a row is.
///
/// A flash is the struck moment (Key); a spoken line is Dialogue; a camera
/// move with nothing said is Motion; a bare row is a Pillow. The first row is
/// an Establish whatever else it carries — a reel has to ground its geography
/// before it can cut.
pub fn frame_type_of(row: &ScriptColumn, is_first: bool) -> FrameType {
    if is_first {
        return FrameType::Establish;
    }
    if row.flash {
        return FrameType::Key;
    }
    if row.truth.is_some() {
        return FrameType::Dialogue;
    }
    if row.cam.is_some() {
        return FrameType::Motion;
    }
    FrameType::Pillow
}

/// Lower a pattern to the `(micros, FrameType)` cuts `cutlist::analyze_cuts`
/// already takes — the row's "engine unchanged" clause, literally.
///
/// Row timing is the tracker's: every row is one tick of `row_micros`.
pub fn rows_to_cuts(rows: &[ScriptColumn], row_micros: i64) -> Vec<(i64, FrameType)> {
    let resolved = resolve(rows);
    resolved
        .iter()
        .enumerate()
        .map(|(i, r)| (r.row as i64 * row_micros, frame_type_of(r, i == 0)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "\
# reel: toll gate
# row | note | pal | cam  | fl | scar | truth
000 | C-4  | 01  | WIDE | .. | ..   | the gate stands shut
001 | ..   | ..  | ..   | .. | ..   | ..
002 | ..   | 02  | PUSH | .. | ..   | ..
003 | G-4  | ..  | ..   | X  | 07   | one toll for the dead
";

    #[test]
    fn a_table_parses_to_one_column_per_row() {
        let rows = parse_pattern(TABLE).expect("parses");
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].note.as_deref(), Some("C-4"));
        assert_eq!(rows[0].palette, Some(1));
        assert_eq!(rows[0].truth.as_deref(), Some("the gate stands shut"));
        assert!(rows[3].flash);
        assert_eq!(rows[3].scar, Some(7));
    }

    #[test]
    fn comments_and_blank_lines_are_not_rows() {
        let rows = parse_pattern("# just a header\n\n   \n").expect("parses");
        assert!(rows.is_empty());
    }

    /// The tracker law: an empty cell HOLDS, it does not clear.
    #[test]
    fn held_cells_inherit_the_running_value() {
        let rows = resolve(&parse_pattern(TABLE).expect("parses"));
        assert_eq!(rows[1].note.as_deref(), Some("C-4"), "row 1 holds row 0's note");
        assert_eq!(rows[1].cam.as_deref(), Some("WIDE"));
        assert_eq!(rows[2].palette, Some(2), "row 2 sets its own palette");
        assert_eq!(rows[2].note.as_deref(), Some("C-4"), "and still holds the note");
        assert_eq!(rows[3].cam.as_deref(), Some("PUSH"), "row 3 holds row 2's camera");
    }

    /// A flash is an event, not a state — it must not smear down the table.
    #[test]
    fn a_flash_does_not_carry_to_the_next_row() {
        let rows = resolve(&parse_pattern(TABLE).expect("parses"));
        assert!(rows[3].flash, "the struck row flashes");
        assert!(!rows[0].flash);
        assert!(!rows[1].flash, "and nothing before it does");
    }

    #[test]
    fn the_first_row_grounds_the_geography() {
        let rows = parse_pattern(TABLE).expect("parses");
        assert_eq!(frame_type_of(&rows[0], true), FrameType::Establish);
        assert_eq!(
            frame_type_of(&rows[0], false),
            FrameType::Dialogue,
            "the same row mid-reel is just its own shape"
        );
    }

    #[test]
    fn row_shape_picks_the_frame_type() {
        let flash = ScriptColumn { flash: true, ..Default::default() };
        let spoken = ScriptColumn { truth: Some("a line".into()), ..Default::default() };
        let moved = ScriptColumn { cam: Some("PAN".into()), ..Default::default() };
        let bare = ScriptColumn::default();
        assert_eq!(frame_type_of(&flash, false), FrameType::Key);
        assert_eq!(frame_type_of(&spoken, false), FrameType::Dialogue);
        assert_eq!(frame_type_of(&moved, false), FrameType::Motion);
        assert_eq!(frame_type_of(&bare, false), FrameType::Pillow);
    }

    /// "Engine unchanged": the table lowers straight into the cut list the
    /// landed Drop Law analyzer already eats.
    #[test]
    fn a_table_lowers_into_cuts_the_landed_analyzer_accepts() {
        let rows = parse_pattern(TABLE).expect("parses");
        let cuts = rows_to_cuts(&rows, 500_000);
        assert_eq!(cuts.len(), 4);
        assert_eq!(cuts[0], (0, FrameType::Establish));
        assert_eq!(cuts[3].0, 3 * 500_000, "row 3 lands on tick 3");
        assert_eq!(cuts[3].1, FrameType::Key);

        let analysis = crate::cutlist::analyze_cuts(&cuts, 30);
        assert!(!analysis.report.is_empty(), "the landed analyzer must accept the lowering");
    }

    #[test]
    fn a_short_line_names_its_own_row() {
        let err = parse_pattern("000 | C-4 | 01\n").expect_err("must refuse");
        assert_eq!(err.line, 1);
        assert!(err.message.contains("expected 7 columns"), "{err}");
    }

    #[test]
    fn a_non_numeric_row_index_refuses() {
        let err = parse_pattern("xx | .. | .. | .. | .. | .. | ..\n").expect_err("must refuse");
        assert!(err.message.contains("not a number"), "{err}");
    }

    /// Rows must advance — a table that jumps backwards is an authoring error,
    /// not a loop.
    #[test]
    fn rows_must_advance() {
        let src = "000 | .. | .. | .. | .. | .. | ..\n001 | .. | .. | .. | .. | .. | ..\n001 | .. | .. | .. | .. | .. | ..\n";
        let err = parse_pattern(src).expect_err("must refuse");
        assert_eq!(err.line, 3);
        assert!(err.message.contains("does not advance"), "{err}");
    }

    #[test]
    fn a_bad_flash_cell_refuses_instead_of_being_read_as_false() {
        let err = parse_pattern("000 | .. | .. | .. | yes | .. | ..\n").expect_err("must refuse");
        assert!(err.message.contains("must be X"), "{err}");
    }

    #[test]
    fn an_out_of_range_palette_names_the_column() {
        let err = parse_pattern("000 | .. | 999 | .. | .. | .. | ..\n").expect_err("must refuse");
        assert!(err.message.contains("palette"), "{err}");
    }
}
