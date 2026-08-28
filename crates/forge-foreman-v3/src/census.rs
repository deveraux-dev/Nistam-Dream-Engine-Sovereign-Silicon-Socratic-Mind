//! The census file the loop consumes: `.forge/census.tsv`.
//!
//! One row per crate, tab-separated, append-friendly, and readable by the same
//! eye that reads the tape. M3 owns filling it for all 127 v2 crates; M2 owns
//! the consuming contract, so the format lives here. Dispositions are ARCH000
//! calls — rows are written as proposals and the file header says so.
//!
//! Columns, in order:
//!
//! ```text
//! crate <TAB> v2_path <TAB> disposition <TAB> status <TAB> note
//! ```
//!
//! `disposition` ∈ adopt | port | rewrite | condemn. `status` ∈ pending |
//! green | queued | condemned. The loop takes the FIRST row whose disposition
//! is workable (not condemn) and whose status is `pending` — file order is the
//! DAG order M3 computes, so the foreman does not re-derive it.

use std::path::{Path, PathBuf};

/// What ARCH000 (or a proposing session) ruled for a crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Take the v2 crate as-is through the customs gate.
    Adopt,
    /// Syntax-level port — the sidecar's grind work.
    Port,
    /// Re-founded from a brief; the v2 source is reference only.
    Rewrite,
    /// Left behind, with its reason. Never worked, row never deleted.
    Condemn,
    /// Undecided, not rejected — awaiting an ARCH000 ruling before the loop
    /// may even attempt it (distinct from `Condemn`: hold can still become
    /// Adopt/Port/Rewrite once ruled; condemn never does). Added 2026-08-15:
    /// the census gained this word (row `forge-render`, "ARCH000 ruling
    /// wanted") before this parser knew it, which halted `foreman run`
    /// loudly — the correct failure, not a bug to route around quietly.
    Hold,
}

impl Disposition {
    /// Parse the census column. Unknown words are an error — a typo must not
    /// silently condemn a crate.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "adopt" => Ok(Self::Adopt),
            "port" => Ok(Self::Port),
            "rewrite" => Ok(Self::Rewrite),
            "condemn" => Ok(Self::Condemn),
            "hold" => Ok(Self::Hold),
            other => Err(format!("unknown disposition {other:?}")),
        }
    }

    /// The census column spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Adopt => "adopt",
            Self::Port => "port",
            Self::Rewrite => "rewrite",
            Self::Condemn => "condemn",
            Self::Hold => "hold",
        }
    }
}

/// Where a row is in its travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Not yet attempted — the loop may take it.
    Pending,
    /// Gate green, on the tape. Terminal for the loop.
    Green,
    /// Red after the retry budget; its brief sits in `.forge/brief-queue/`.
    Queued,
    /// Disposition is condemn; the loop never takes it.
    Condemned,
    /// Gate green for what this row itself claims, but real, separate,
    /// untracked work remains (its note names it) — terminal for the loop
    /// exactly like `Green`, never picked, never silently reinterpreted as
    /// `Pending`. Added 2026-08-15: the census gained this word (row
    /// `forge-vix`, "landed... NOT ported: ...") before this parser knew it,
    /// which halted `foreman run` loudly rather than guess — the correct
    /// failure per this crate's own law, not a bug to route around quietly.
    Partial,
    /// Assets/data landed at a staging location pending a second ARCH000
    /// ruling on their permanent home — terminal for the loop like `Green`/
    /// `Partial`, never picked, never reinterpreted as `Pending`. Added
    /// 2026-08-15: same drift class as `Partial` (row `nde-sieve-corpus`,
    /// "OPEN (ruling 2 of 2)... still unruled").
    Staged,
    /// Real source confirmed to exist on disk, but its test/green status was
    /// NOT measured by whatever pass set this — terminal for the loop like
    /// `Green`/`Partial`/`Staged`, never picked (a weld attempt is the wrong
    /// tool here; this needs a verify pass, not a repair cycle). Added
    /// 2026-08-15: this census file's own header comment (line 35) already
    /// defined this word before the parser knew it.
    LandedUnverified,
}

impl Status {
    /// Parse the census column, refusing unknown words.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "pending" => Ok(Self::Pending),
            "green" => Ok(Self::Green),
            "queued" => Ok(Self::Queued),
            "condemned" => Ok(Self::Condemned),
            "partial" => Ok(Self::Partial),
            "staged" => Ok(Self::Staged),
            "landed-unverified" => Ok(Self::LandedUnverified),
            other => Err(format!("unknown status {other:?}")),
        }
    }

    /// The census column spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Green => "green",
            Self::Queued => "queued",
            Self::Condemned => "condemned",
            Self::Partial => "partial",
            Self::Staged => "staged",
            Self::LandedUnverified => "landed-unverified",
        }
    }
}

/// One census row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The v3 crate name the work lands under (`crates/<name>/`).
    pub name: String,
    /// The v2 source directory, read-only reference.
    pub v2_path: PathBuf,
    /// ARCH000's (or the proposal's) ruling.
    pub disposition: Disposition,
    /// Where the row is in its travel.
    pub status: Status,
    /// Free text: the reason, or the proposal marker.
    pub note: String,
}

/// The census: rows in file order, which is DAG order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Census {
    /// Every row, condemned ones included — no row is ever deleted.
    pub rows: Vec<Row>,
}

/// Where the census lives under a root.
pub fn census_path(root: &Path) -> PathBuf {
    root.join(".forge").join("census.tsv")
}

impl Census {
    /// Parse census text. Lines starting `#` and blank lines are commentary;
    /// every other line must carry exactly five tab-separated columns.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut rows = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() != 5 {
                return Err(format!("census line {}: {} column(s), want 5", i + 1, cols.len()));
            }
            rows.push(Row {
                name: cols[0].to_string(),
                v2_path: PathBuf::from(cols[1]),
                disposition: Disposition::parse(cols[2])
                    .map_err(|e| format!("census line {}: {e}", i + 1))?,
                status: Status::parse(cols[3]).map_err(|e| format!("census line {}: {e}", i + 1))?,
                note: cols[4].to_string(),
            });
        }
        Ok(Self { rows })
    }

    /// Read `<root>/.forge/census.tsv`. A missing census is an error — the
    /// loop has nothing to take and must say so, not spin.
    pub fn load(root: &Path) -> Result<Self, String> {
        let p = census_path(root);
        let text =
            std::fs::read_to_string(&p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
        Self::parse(&text)
    }

    /// The next row the loop may take: first in file order that is workable
    /// (not condemn) and `pending`.
    pub fn next_actionable(&self) -> Option<&Row> {
        self.rows
            .iter()
            .find(|r| {
                r.disposition != Disposition::Condemn
                    && r.disposition != Disposition::Hold
                    && r.status == Status::Pending
            })
    }

    /// Flip one row's status by crate name. Errors if the name has no row —
    /// flipping a phantom row would report progress nothing made.
    pub fn set_status(&mut self, name: &str, status: Status) -> Result<(), String> {
        let row = self
            .rows
            .iter_mut()
            .find(|r| r.name == name)
            .ok_or_else(|| format!("census has no row named {name:?}"))?;
        row.status = status;
        Ok(())
    }

    /// Serialize back to census text, header comment included.
    pub fn encode(&self) -> String {
        let mut out = String::from(
            "# V3 CENSUS — one row per crate, tab-separated. File order IS the work order.\n\
             # crate\tv2_path\tdisposition\tstatus\tnote\n\
             # Dispositions are ARCH000 calls; rows marked `proposed:` in the note are\n\
             # session proposals awaiting a ruling (MIGRATION §M3).\n",
        );
        for r in &self.rows {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                r.name,
                r.v2_path.display(),
                r.disposition.as_str(),
                r.status.as_str(),
                r.note
            ));
        }
        out
    }

    /// Write the census back to `<root>/.forge/census.tsv`.
    pub fn store(&self, root: &Path) -> Result<(), String> {
        let p = census_path(root);
        std::fs::write(&p, self.encode()).map_err(|e| format!("cannot write {}: {e}", p.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# header\n\
        forge-a-v3\tF:/NewRepo/crates/a\tport\tpending\tproposed: tiny\n\
        forge-b-v3\tF:/NewRepo/crates/b\tcondemn\tcondemned\tfloat physics\n\
        forge-c-v3\tF:/NewRepo/crates/c\tadopt\tpending\tproposed\n";

    #[test]
    fn a_census_round_trips_through_its_own_encoder() {
        let c = Census::parse(SAMPLE).unwrap();
        assert_eq!(c.rows.len(), 3);
        let again = Census::parse(&c.encode()).unwrap();
        assert_eq!(c, again, "encode -> parse is identity over the rows (L07)");
    }

    #[test]
    fn next_actionable_skips_condemned_and_respects_file_order() {
        let mut c = Census::parse(SAMPLE).unwrap();
        assert_eq!(c.next_actionable().unwrap().name, "forge-a-v3");
        c.set_status("forge-a-v3", Status::Green).unwrap();
        assert_eq!(c.next_actionable().unwrap().name, "forge-c-v3", "condemn row b is skipped");
        c.set_status("forge-c-v3", Status::Queued).unwrap();
        assert!(c.next_actionable().is_none(), "queued rows are not re-taken");
    }

    /// Real census drift this session (2026-08-15): `forge-vix`'s row used
    /// `partial` status and `forge-render`'s row used `hold` disposition
    /// before this parser knew either word — `foreman run` refused loudly
    /// rather than guess (the correct failure). Both parse now; `hold` must
    /// ALSO be excluded from the picker (like `condemn`) even when its
    /// status column still says `pending`, since a hold row is explicitly
    /// "awaiting a ruling," not "the loop may take it."
    #[test]
    fn partial_status_parses_and_hold_disposition_is_never_picked_even_when_pending() {
        const REAL: &str = "# header\n\
            forge-vix\tF:/NewRepo/crates/forge-vix\tadopt\tpartial\tlanded slice, more owed\n\
            forge-render\tF:/NewRepo/crates/forge-render\thold\tpending\tARCH000 ruling wanted\n\
            forge-z-v3\tF:/NewRepo/crates/z\tport\tpending\tthe only real pick\n";
        let c = Census::parse(REAL).unwrap();
        assert_eq!(c.rows[0].status, Status::Partial);
        assert_eq!(c.rows[1].disposition, Disposition::Hold);
        assert_eq!(
            c.next_actionable().unwrap().name,
            "forge-z-v3",
            "hold row must be skipped despite pending status; partial row must be skipped despite file order"
        );
    }

    #[test]
    fn a_typo_disposition_is_refused_not_condemned() {
        let bad = "x\tp\tadpot\tpending\tnote\n";
        assert!(Census::parse(bad).is_err());
        assert!(Census::parse("x\tp\tadopt\tpending\n").is_err(), "4 columns is not a row");
        let mut c = Census::parse(SAMPLE).unwrap();
        assert!(c.set_status("phantom", Status::Green).is_err());
    }
}
