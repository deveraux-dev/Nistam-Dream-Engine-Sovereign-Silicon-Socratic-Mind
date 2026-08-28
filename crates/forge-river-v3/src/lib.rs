//! `.forge/river.idx` reader/writer — the ONE home (L05) for `RiverEntry`/
//! `RiverIndex`. Extracted verbatim from `xtask/src/river.rs` (2026-08-19)
//! so anything in the workspace can depend on it, not just the `xtask`
//! binary — `forge-daemon-door` is the first real caller (native
//! `river_set_head`/`river_set_aperture` wire verbs instead of every caller
//! shelling out to `cargo xtask river ...`).
//!
//! Signal Law: every authored line is ≤60 bytes except `Raw` rows (another
//! writer's property, round-tripped verbatim, never sized).

#![deny(missing_docs)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Maximum bytes per authored line (Signal Law).
pub const MAX_LINE_BYTES: usize = 60;

/// One river row, v0.1 subset (`Spill` intentionally not ported yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiverEntry {
    /// The current goal/target pointer.
    Head(String),
    /// The current next-step pointer.
    Aperture(String),
    /// A named tool/verb entry.
    Tool {
        /// Tool name.
        name: String,
        /// One-line purpose.
        purpose: String,
    },
    /// A crate/domain status entry.
    Map {
        /// Crate name.
        krate: String,
        /// Domain label.
        domain: String,
        /// Status label.
        status: String,
        /// Anchor citation.
        anchor: String,
    },
    /// Anything outside this schema (5D coord rows, hand law rows, BUILD) —
    /// round-trips verbatim, never dropped.
    Raw(String),
}

impl RiverEntry {
    /// Render this row as its on-disk line (no trailing newline).
    pub fn to_line(&self) -> String {
        match self {
            Self::Head(s) => format!("HEAD\t{s}"),
            Self::Aperture(s) => format!("APERTURE\t{s}"),
            Self::Tool { name, purpose } => format!("TOOL\t{name}\t{purpose}"),
            Self::Map { krate, domain, status, anchor } => format!("MAP\t{krate}\t{domain}\t{status}\t{anchor}"),
            Self::Raw(s) => s.clone(),
        }
    }

    /// Parse one on-disk line back into a row. Never fails — anything
    /// outside the known schema becomes [`RiverEntry::Raw`].
    pub fn parse(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.iter().any(|f| is_coord_field(f)) {
            return Some(Self::Raw(line.to_string()));
        }
        match parts.first().copied()? {
            "HEAD" => Some(Self::Head(parts.get(1).unwrap_or(&"").to_string())),
            "APERTURE" => Some(Self::Aperture(parts.get(1).unwrap_or(&"").to_string())),
            "TOOL" => Some(Self::Tool { name: parts.get(1).unwrap_or(&"").to_string(), purpose: parts.get(2).unwrap_or(&"").to_string() }),
            "MAP" => Some(Self::Map {
                krate: parts.get(1).unwrap_or(&"").to_string(),
                domain: parts.get(2).unwrap_or(&"").to_string(),
                status: parts.get(3).unwrap_or(&"").to_string(),
                anchor: parts.get(4).unwrap_or(&"").to_string(),
            }),
            _ => Some(Self::Raw(line.to_string())),
        }
    }
}

/// The `#<27 base64>` 5D coord wire form — shape-checked, never sigil-only.
fn is_coord_field(field: &str) -> bool {
    field.strip_prefix('#').is_some_and(|enc| enc.len() == 27 && enc.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
}

/// The `.forge/river.idx` file, read/written whole (small file, bounded rows).
pub struct RiverIndex {
    /// Absolute path to `river.idx`.
    pub path: PathBuf,
}

impl RiverIndex {
    /// Point at `<forge_root>/river.idx` (does not touch disk).
    pub fn new(forge_root: &Path) -> Self {
        Self { path: forge_root.join("river.idx") }
    }

    /// Read every row, skipping blank lines. Missing file reads as empty.
    pub fn read_all(&self) -> Vec<RiverEntry> {
        let content = fs::read_to_string(&self.path).unwrap_or_default();
        content.lines().filter(|l| !l.is_empty()).filter_map(RiverEntry::parse).collect()
    }

    /// Replace the whole file. Enforces the 60B signal law per line; `Raw`
    /// rows are grandfathered (other writers' property, never sized).
    pub fn write_all(&self, rows: &[RiverEntry]) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("mkdir: {e}"))?;
        }
        let mut buf = Vec::new();
        for row in rows {
            let line = row.to_line();
            if !matches!(row, RiverEntry::Raw(_)) && line.len() > MAX_LINE_BYTES {
                return Err(format!("river SIGNAL-LAW REJECT: {} B > {MAX_LINE_BYTES}B: {line}", line.len()));
            }
            buf.extend_from_slice(line.as_bytes());
            buf.push(b'\n');
        }
        fs::write(&self.path, &buf).map_err(|e| format!("write river.idx: {e}"))
    }

    /// Append one row, 60B-gated (fresh authoring always hard-rejects — no
    /// settle-to-grain in v0.1, so an oversize append is simply refused).
    pub fn append(&self, row: &RiverEntry) -> Result<(), String> {
        let line = row.to_line();
        if line.len() > MAX_LINE_BYTES {
            return Err(format!("river SIGNAL-LAW REJECT: {} B > {MAX_LINE_BYTES}B: {line}", line.len()));
        }
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&self.path).map_err(|e| format!("open river.idx: {e}"))?;
        writeln!(file, "{line}").map_err(|e| format!("append river.idx: {e}"))
    }

    /// Replace the HEAD row (never duplicates).
    pub fn set_head(&self, goal: &str) -> Result<(), String> {
        let fresh = RiverEntry::Head(goal.to_string());
        if fresh.to_line().len() > MAX_LINE_BYTES {
            return Err(format!("river SIGNAL-LAW REJECT: HEAD too long ({} B)", fresh.to_line().len()));
        }
        let mut rows = self.read_all();
        rows.retain(|r| !matches!(r, RiverEntry::Head(_)));
        rows.insert(0, fresh);
        self.write_all(&rows)
    }

    /// Pararity resolution (`n = 2m + k`, `PARARITY.md:88,118`) for a
    /// collision of `HEAD` rows written outside `set_head` (e.g. raw appends
    /// from another writer). `n=3` is the theorem's own stated case: the
    /// non-trivial involution gives `(k,m) = (1,1)` — one fixed point (the
    /// canonical HEAD, taken as the most recent / last in file order) and one
    /// 2-orbit pairing the remaining rows as `(previous, historical)`. Not a
    /// heuristic pick — the same math this repo already uses for its
    /// Composite/Prime and `anomaly_fold.rs` involutions.
    pub fn canonical_head(heads: &[String]) -> Option<(String, Vec<(String, String)>)> {
        let (last, rest) = heads.split_last()?;
        let mut orbits = Vec::new();
        let mut pairs = rest.chunks_exact(2);
        for chunk in &mut pairs {
            orbits.push((chunk[0].clone(), chunk[1].clone()));
        }
        // An odd remainder outside n=3's stated case still needs a home: it
        // pairs with itself, a degenerate fixed point within the historical
        // set rather than a silently dropped row.
        if let [odd] = pairs.remainder() {
            orbits.push((odd.clone(), odd.clone()));
        }
        Some((last.clone(), orbits))
    }

    /// Replace the APERTURE row, keeping it positioned right after HEAD.
    pub fn set_aperture(&self, aperture: &str) -> Result<(), String> {
        let fresh = RiverEntry::Aperture(aperture.to_string());
        if fresh.to_line().len() > MAX_LINE_BYTES {
            return Err(format!("river SIGNAL-LAW REJECT: APERTURE too long ({} B)", fresh.to_line().len()));
        }
        let mut rows = self.read_all();
        rows.retain(|r| !matches!(r, RiverEntry::Aperture(_)));
        // After HEAD if one exists (even when nothing follows it — `position`
        // alone can't tell "no non-HEAD row" apart from "no HEAD row", so
        // `unwrap_or(0)` on its own wrongly inserts before a lone HEAD).
        let pos = if rows.iter().any(|r| matches!(r, RiverEntry::Head(_))) {
            rows.iter().position(|r| !matches!(r, RiverEntry::Head(_))).unwrap_or(rows.len())
        } else {
            0
        };
        rows.insert(pos, fresh);
        self.write_all(&rows)
    }

    /// Current file size in bytes (0 if absent).
    pub fn size_bytes(&self) -> u64 {
        fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        std::env::temp_dir().join(format!("river-lib-test-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))
    }

    /// PARARITY.md:118's stated n=3 case: the non-trivial involution gives
    /// (k,m)=(1,1) — one fixed point, one 2-orbit. Three colliding HEAD rows
    /// must resolve to exactly one canonical + exactly one historical pair,
    /// with every original row accounted for (none dropped).
    #[test]
    fn canonical_head_resolves_n3_pararity_case() {
        let heads = vec!["oldest".to_string(), "middle".to_string(), "newest".to_string()];
        let (canonical, orbits) = RiverIndex::canonical_head(&heads).expect("n=3 must resolve");
        assert_eq!(canonical, "newest", "the fixed point is the most recent HEAD");
        assert_eq!(orbits, vec![("oldest".to_string(), "middle".to_string())], "exactly one 2-orbit for the remaining two rows");
    }

    #[test]
    fn canonical_head_single_row_has_no_orbits() {
        let heads = vec!["only".to_string()];
        let (canonical, orbits) = RiverIndex::canonical_head(&heads).expect("n=1 must resolve");
        assert_eq!(canonical, "only");
        assert!(orbits.is_empty(), "a single HEAD is the fixed point with nothing left to pair");
    }

    #[test]
    fn canonical_head_empty_returns_none() {
        assert_eq!(RiverIndex::canonical_head(&[]), None);
    }

    #[test]
    fn write_and_read_round_trips() {
        let dir = scratch();
        let river = RiverIndex::new(&dir);
        let rows = vec![RiverEntry::Head("thin-door".into()), RiverEntry::Aperture("xtask".into()), RiverEntry::Tool { name: "ping".into(), purpose: "liveness".into() }];
        river.write_all(&rows).unwrap();
        assert_eq!(river.read_all(), rows);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn raw_rows_round_trip_verbatim() {
        let dir = scratch();
        let river = RiverIndex::new(&dir);
        let foreign = "BAN\tNO-VCS/Git hard (Sean 2026-07-05) - this line is well over sixty bytes on purpose";
        fs::create_dir_all(&dir).unwrap();
        fs::write(&river.path, format!("HEAD\tepoch\n{foreign}\nAPERTURE\txtask\n")).unwrap();
        river.set_aperture("river-wiring").unwrap();
        let after = fs::read_to_string(&river.path).unwrap();
        assert!(after.contains(foreign), "foreign/oversize law row must survive byte-identical");
        assert!(after.contains("APERTURE\triver-wiring"), "dial moved");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_head_replaces_not_duplicates() {
        let dir = scratch();
        let river = RiverIndex::new(&dir);
        river.set_head("goal-a").unwrap();
        river.set_head("goal-b").unwrap();
        let rows = river.read_all();
        let heads: Vec<_> = rows.iter().filter(|r| matches!(r, RiverEntry::Head(_))).collect();
        assert_eq!(heads.len(), 1, "only one HEAD row");
        assert_eq!(heads[0], &RiverEntry::Head("goal-b".into()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_aperture_moves_the_dial_behind_head() {
        let dir = scratch();
        let river = RiverIndex::new(&dir);
        river.write_all(&[RiverEntry::Head("epoch".into()), RiverEntry::Aperture("old".into())]).unwrap();
        river.set_aperture("new".into()).unwrap();
        let rows = river.read_all();
        assert!(matches!(rows[0], RiverEntry::Head(_)), "HEAD stays first");
        assert_eq!(rows[1], RiverEntry::Aperture("new".into()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_oversize_fresh_append() {
        let dir = scratch();
        let river = RiverIndex::new(&dir);
        let too_long = RiverEntry::Head("x".repeat(56));
        assert_eq!(too_long.to_line().len(), 61);
        let err = river.append(&too_long);
        assert!(err.is_err(), "append must reject >60B lines");
        assert!(err.unwrap_err().contains("SIGNAL-LAW REJECT"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_coord_row_round_trips_through_the_writer_untouched() {
        let dir = scratch();
        let river = RiverIndex::new(&dir);
        let coord = format!("#{}", "A".repeat(27));
        let migrated = format!("MAP\t{coord}\t@d34db33f");
        let rows = vec![RiverEntry::Head("the head".into()), RiverEntry::parse(&migrated).unwrap()];
        assert!(matches!(rows[1], RiverEntry::Raw(_)), "geometry is never this writer's schema");
        river.write_all(&rows).unwrap();
        let back = river.read_all();
        assert_eq!(back[1].to_line(), migrated, "byte-exact through write_all + read_all");
        let _ = fs::remove_dir_all(&dir);
    }
}
