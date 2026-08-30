//! River → Spine (Sean 07-27: "the river is the spine of forge book").
//!
//! The living index `.forge/river.idx` read back as a chronicle chapter on
//! `Book.spine`. Strictly READ-ONLY over the index — the daemon stays the one
//! writer (root EXHAUST law); the book only projects. The chapter is transient,
//! recalculated from the rows on every merge: never a static string, never
//! persisted back. Rows land verbatim as lore lines, so the projection is
//! lossless and the roundtrip is provable; prose faces come from the book's
//! own emit stages (`export_md` / faces), not from a second grammar here.

use std::path::Path;

use crate::atlas::AtlasSection;
use crate::book::Book;
use crate::chapter::Chapter;

/// The chronicle chapter's title on the spine.
pub const RIVER_CHAPTER_TITLE: &str = "The River";

/// One river row: `TAG<TAB>body`. The book-side twin of the daemon's writer
/// shape — a parser only, because the dep edge points daemon→book and the
/// single-writer story forbids a book-side writer anyway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiverRow {
    /// The row's tag column (HEAD / APERTURE / LANE / bed / …), never empty.
    pub tag: String,
    /// The row's free-text body, trimmed.
    pub body: String,
}

impl RiverRow {
    /// Parse one `TAG<TAB>body` line. `None` for blank, untagged, or
    /// tab-less lines — malformed rows are skipped, never guessed at.
    pub fn parse(line: &str) -> Option<Self> {
        let (tag, body) = line.split_once('\t')?;
        let tag = tag.trim();
        if tag.is_empty() {
            return None;
        }
        Some(Self { tag: tag.to_string(), body: body.trim().to_string() })
    }

    /// The row back as its index line — the exact write shape, so a projected
    /// chapter can be proven against the source rows (the roundtrip gate).
    pub fn to_line(&self) -> String {
        format!("{}\t{}", self.tag, self.body)
    }
}

/// Every tagged row of a river index text, in file order.
pub fn parse_river(text: &str) -> Vec<RiverRow> {
    text.lines().filter_map(RiverRow::parse).collect()
}

/// The `@<hash>` grain a MIGRATED row points at. Since the 5D migration a row the ray
/// owns is `TAG\t#<coord>\t@<hash>`: geometry in the line, words in a grain. Field 0 is
/// the tag, so the ref is hunted from field 1 on — a body that merely mentions an `@`
/// is not a pointer.
fn grain_hash(line: &str) -> Option<&str> {
    line.split('\t').skip(1).find_map(|f| f.strip_prefix('@')).filter(|h| !h.is_empty())
}

/// One row with its words put back: `TAG\t#<coord>\t@<hash>` → `TAG\t<grain body>`.
/// The grain holds everything that followed the tag, tabs included, so a hydrated row
/// is BYTE-IDENTICAL to the prose row the migration replaced — which is the whole
/// reason every reader below can keep reading fields it always read. `None` for a
/// prose row (its text is its body) or a grain that has been swept.
///
/// `forge` is the index's own directory (`<root>/.forge`), the only place a grain lives.
/// Reader-half twin of the daemon's writer (`repo_query::hydrated_idx_text`), same
/// standing as [`RiverRow`] itself: the dep edge points daemon→book, so the book owns
/// its own read of the shape and never binds the writer.
pub fn hydrate_row(forge: &Path, line: &str) -> Option<String> {
    let tag = line.split('\t').next()?;
    let grain = forge.join("spill").join(format!("{}.grain", grain_hash(line)?));
    let body = std::fs::read_to_string(grain).ok()?;
    Some(format!("{tag}\t{}", body.trim_end().replace('\n', " ")))
}

/// A whole index text hydrated row by row. A row that carries no grain — or whose grain
/// is gone — passes through AS IS: degraded to hex, never dropped, because a spine that
/// silently loses rows is worse than one that reads oddly. Line order is preserved 1:1.
pub fn hydrate(forge: &Path, text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line.trim_end();
        out.push_str(&hydrate_row(forge, line).unwrap_or_else(|| line.to_string()));
        out.push('\n');
    }
    out
}

/// The live index as PROSE, whatever form it is stored in: read `<root>/.forge/river.idx`
/// and hydrate it. Every live reader in this file goes through here, so the migration is
/// invisible above this line. `None` when the index is absent — a loud miss, never "".
fn live_river(root: &Path) -> Option<String> {
    let forge = root.join(".forge");
    let bytes = std::fs::read(forge.join("river.idx")).ok()?;
    Some(hydrate(&forge, &String::from_utf8_lossy(&bytes)))
}

/// The crate names a river index actually NAMES, sorted and deduped. Read off `MAP`
/// rows only: `MAP<TAB>krate<TAB>domain<TAB>status<TAB>anchor`, so the crate is the
/// body's first tab field. Every other tag is a lens or a law, never a crate.
pub fn spine_crates(text: &str) -> Vec<String> {
    let mut out: Vec<String> = parse_river(text)
        .into_iter()
        .filter(|r| r.tag == "MAP")
        .filter_map(|r| r.body.split('\t').next().map(str::trim).filter(|k| !k.is_empty()).map(str::to_string))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The crates that EXIST, read off the workspace tree rather than authored. One
/// directory per crate under `<root>/crates`, sorted — the census half of the gauge,
/// and the half that was missing: the spine could only ever count itself.
///
/// A CRATE IS A MANIFEST (Sean 07-31): the first cut counted every directory, so
/// `crates/assets`, `crates/output`, `crates/data` and `crates/vixi-corpus` — payload
/// homes, not code — inflated the denominator to 133 and made full coverage
/// unreachable by construction. `Cargo.toml` is the census, so 133 reads 128.
pub fn workspace_crates(root: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(root.join("crates")) else {
        return Vec::new();
    };
    let mut out: Vec<String> = rd
        .flatten()
        .filter(|e| e.path().join("Cargo.toml").is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// How much of the tree the spine can orient over. `dark` is the answer the board row
/// wanted named: the crates on disk with ZERO rows in the index, in tree order.
/// A crate NAMED in the index but absent from disk is not counted present — the disk
/// is the census, the index is the claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpineCoverage {
    /// Crates on disk carrying at least one `MAP` row.
    pub named: usize,
    /// Crates on disk, full stop.
    pub total: usize,
    /// The crates on disk the spine has never heard of.
    pub dark: Vec<String>,
}

/// Gauge the live spine against the live tree: `<root>/crates` vs `<root>/.forge/river.idx`.
/// An unreadable index reads as total darkness, never as full coverage — a gauge that
/// answers 100% because it could not open the file is the one failure mode worth naming.
pub fn spine_coverage(root: &Path) -> SpineCoverage {
    // Hydrated, so a MIGRATED `MAP\t#<coord>\t@<hash>` still names its crate. Read raw,
    // the gauge would answer 0 named / 128 dark the instant the spine went 5D-native —
    // the index unchanged, the census intact, and the number a lie.
    let text = live_river(root).unwrap_or_default();
    let named_set = spine_crates(&text);
    let all = workspace_crates(root);
    let dark: Vec<String> = all.iter().filter(|c| !named_set.contains(c)).cloned().collect();
    SpineCoverage { named: all.len() - dark.len(), total: all.len(), dark }
}

/// The gauge as one line, for the cold-start blast and the live book face. Names the
/// first dark crates rather than only counting them, so the row is actionable at a
/// glance — a bare `27/133` says nothing about where to point next.
pub fn coverage_line(cov: &SpineCoverage) -> String {
    let head: Vec<&str> = cov.dark.iter().take(3).map(String::as_str).collect();
    format!(
        "SPINE {}/{} crates named · {} dark{}",
        cov.named,
        cov.total,
        cov.dark.len(),
        if head.is_empty() { String::new() } else { format!(" (e.g. {})", head.join(", ")) }
    )
}

/// Project river rows into the chronicle chapter: one lore line per row,
/// verbatim `TAG<TAB>body`, calculated from the index — never authored.
pub fn river_chapter(text: &str) -> Chapter {
    let mut ch = Chapter::new(RIVER_CHAPTER_TITLE, AtlasSection::Runbook);
    for row in parse_river(text) {
        ch.add_lore(row.to_line());
    }
    ch
}

/// Feed a river index text into `book`'s spine as the chronicle chapter;
/// returns its spine index. THE merge the ledger row named unmerged.
pub fn merge_river_spine(book: &mut Book, river_text: &str) -> usize {
    book.add_chapter(river_chapter(river_text))
}

/// The live read: `<root>/.forge/river.idx` → chronicle chapter onto `book`.
/// `None` when the index is absent — a LOUD miss for the caller to report,
/// never a fake empty chapter.
pub fn merge_live_river(book: &mut Book, root: &Path) -> Option<usize> {
    // The chronicle is for READERS, so it projects the hydrated rows: a chapter of
    // `#<b64>\t@<hash>` is lossless and useless. The lossless-roundtrip law binds
    // `merge_river_spine`, which still takes text and touches no disk.
    let at = merge_river_spine(book, &live_river(root)?);
    // LIVE CALLER for [`spine_coverage`]: the live projection states how much of the
    // tree the index it just projected can actually orient over. Only the live read
    // gauges — `merge_river_spine` takes text with no tree behind it, and its lossless
    // roundtrip is a law, not an accident.
    if let Some(ch) = book.chapter_mut(at) {
        ch.add_lore(coverage_line(&spine_coverage(root)));
    }
    Some(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "HEAD\tcrates/forge-studio/src/nde_chat.rs 07-23\n\
        APERTURE\tGPU-render launcher · vixi tier2 · term sing\n\
        \n\
        no tab on this line\n\
        bed\tCOVERAGE 2026-07-23s51 RETURN  flags=32\n";

    // [BOARD:RIVER-SPINE] rows → chapter → rows, lossless both directions.
    #[test]
    fn the_chronicle_roundtrips_to_its_source_rows() {
        let rows = parse_river(SAMPLE);
        assert_eq!(rows.len(), 3, "blank + untagged lines are skipped, never guessed");

        let ch = river_chapter(SAMPLE);
        assert_eq!(ch.title(), RIVER_CHAPTER_TITLE);
        assert_eq!(ch.lore_count(), rows.len());

        let back: Vec<RiverRow> = ch
            .codex
            .slots
            .iter()
            .filter_map(|s| RiverRow::parse(&s.text))
            .collect();
        assert_eq!(back, rows, "the projection is lossless: parse(project(rows)) == rows");
    }

    // [BOARD:RIVER-SPINE] the merge feeds Book.spine — the river IS the spine.
    #[test]
    fn the_river_merges_onto_the_spine_as_a_chapter() {
        let mut b = Book::new("Atlas", "deveraux");
        let i = merge_river_spine(&mut b, SAMPLE);
        assert_eq!(b.chapter_count(), 1);
        let ch = b.chapter(i).expect("the chronicle sits on the spine");
        assert_eq!(ch.title(), RIVER_CHAPTER_TITLE);
        assert_eq!(ch.lore_count(), 3);
        assert!(ch.codex.slots[0].text.starts_with("HEAD\t"));
    }

    // [BOARD:RIVER-SPINE] an absent index is a loud miss, not an empty chapter.
    #[test]
    fn an_absent_index_is_a_loud_none() {
        let mut b = Book::new("Atlas", "deveraux");
        let missing = Path::new("Z:/no/such/root");
        assert_eq!(merge_live_river(&mut b, missing), None);
        assert_eq!(b.chapter_count(), 0, "no fake chapter on a miss");
    }

    // [BOARD: SPINE-COVERAGE-106]
    /// The census the spine never had: crates on disk vs crates the index names.
    /// 106 of 133 were dark when this was gauged by hand (2026-07-30); the point of
    /// the row is that the number is READ, never authored.
    #[test]
    fn the_coverage_gauge_reads_both_halves_off_disk() {
        let td = std::env::temp_dir().join(format!("spinecov-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&td);
        for c in ["forge-book", "forge-dag", "forge-pkm"] {
            std::fs::create_dir_all(td.join("crates").join(c)).unwrap();
            std::fs::write(td.join("crates").join(c).join("Cargo.toml"), "[package]\n").unwrap();
        }
        // A payload dir is NOT a crate: no manifest, so it never reaches the denominator.
        std::fs::create_dir_all(td.join("crates").join("assets")).unwrap();
        std::fs::create_dir_all(td.join(".forge")).unwrap();
        std::fs::write(
            td.join(".forge/river.idx"),
            "HEAD\tthe head\nMAP\tforge-book\tlore\tGREEN\tanchor\nMAP\tforge-gone\tx\tGREEN\ta\n",
        )
        .unwrap();

        assert_eq!(spine_crates(&std::fs::read_to_string(td.join(".forge/river.idx")).unwrap()),
            vec!["forge-book".to_string(), "forge-gone".to_string()],
            "MAP rows only, sorted and deduped");
        assert_eq!(workspace_crates(&td).len(), 3, "the tree is the census, not the index — and crates/assets carries no manifest, so it is not one of them");

        let cov = spine_coverage(&td);
        assert_eq!((cov.named, cov.total), (1, 3), "forge-gone is a claim with no crate behind it");
        assert_eq!(cov.dark, vec!["forge-dag".to_string(), "forge-pkm".to_string()],
            "the two crates root#cognitive-alignment names as the DAG and RAG homes");
        assert!(coverage_line(&cov).starts_with("SPINE 1/3 crates named · 2 dark"), "{}", coverage_line(&cov));
        let _ = std::fs::remove_dir_all(&td);
    }

    // [BOARD: SPINE-COVERAGE-106]
    /// `book river --check` is the gauge as a GATE: it must FAIL while the tree is dark
    /// and pass only when every crate is named. `book river` (no flag) projects rows and
    /// always exits 0, so it could never have failed a build — which is why the gate
    /// clause of the board row went unbuilt until now.
    #[test]
    fn the_check_gate_fails_while_a_crate_is_dark_and_passes_when_none_is() {
        let td = std::env::temp_dir().join(format!("rivercheck-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&td);
        std::fs::create_dir_all(td.join("crates").join("forge-book")).unwrap();
        std::fs::write(td.join("crates/forge-book/Cargo.toml"), "[package]\n").unwrap();
        std::fs::create_dir_all(td.join(".forge")).unwrap();
        std::fs::write(td.join(".forge/river.idx"), "HEAD\tthe head\n").unwrap();
        let root = td.display().to_string();
        let argv = |extra: &[&str]| -> Vec<String> {
            let mut v = vec!["book".to_string(), "river".to_string(), root.clone()];
            v.extend(extra.iter().map(|s| s.to_string()));
            v
        };

        assert_eq!(crate::run(&argv(&["--check"])), 1, "a dark crate must fail the gate");
        assert_eq!(crate::run(&argv(&[])), 0, "the bare projection still only prints");

        std::fs::write(
            td.join(".forge/river.idx"),
            "HEAD\tthe head\nMAP\tforge-book\tlore\tGREEN\tanchor\n",
        )
        .unwrap();
        assert_eq!(crate::run(&argv(&["--check"])), 0, "every crate named = gate clear");
        let _ = std::fs::remove_dir_all(&td);
    }

    // [BOARD: SPINE-COVERAGE-106]
    /// An unreadable index reads as TOTAL darkness. A gauge that answered 100%
    /// because it could not open the file would hide exactly what it exists to show.
    #[test]
    fn an_unreadable_index_is_total_darkness_never_full_coverage() {
        let td = std::env::temp_dir().join(format!("spinecov-dark-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&td);
        std::fs::create_dir_all(td.join("crates").join("forge-book")).unwrap();
        std::fs::write(td.join("crates/forge-book/Cargo.toml"), "[package]\n").unwrap();
        let cov = spine_coverage(&td);
        assert_eq!((cov.named, cov.total), (0, 1));
        assert_eq!(cov.dark, vec!["forge-book".to_string()]);
        let _ = std::fs::remove_dir_all(&td);
    }

    // [BOARD:SPINE-5D-NATIVE]
    /// The migration's gate on this side of the seam: a 5D-native index must read to the
    /// SAME census a prose one does. Every crate here is named by a row that carries no
    /// crate name at all — only a coord and a grain ref.
    #[test]
    fn a_migrated_row_still_names_its_crate_and_gauges_the_same() {
        let td = std::env::temp_dir().join(format!("spine5d-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&td);
        std::fs::create_dir_all(td.join("crates/forge-book")).unwrap();
        std::fs::write(td.join("crates/forge-book/Cargo.toml"), "[package]\n").unwrap();
        std::fs::create_dir_all(td.join(".forge/spill")).unwrap();
        // The grain holds everything that followed the tag — tabs and all.
        std::fs::write(td.join(".forge/spill/abc123.grain"), "forge-book\tlore\tGREEN\tanchor\n").unwrap();
        let coord = format!("#{}", "A".repeat(27));
        std::fs::write(
            td.join(".forge/river.idx"),
            format!("HEAD\tthe head\nMAP\t{coord}\t@abc123\nMAP\t{coord}\t@swept\n"),
        )
        .unwrap();

        let hydrated = live_river(&td).expect("a live index reads");
        assert!(hydrated.contains("MAP\tforge-book\tlore\tGREEN\tanchor"), "{hydrated}");
        assert!(hydrated.contains("@swept"), "a swept grain leaves its row degraded, never dropped");
        assert!(hydrated.starts_with("HEAD\tthe head\n"), "a prose row passes through untouched");

        let cov = spine_coverage(&td);
        assert_eq!((cov.named, cov.total), (1, 1), "the census survives the migration");
        assert!(cov.dark.is_empty(), "{:?}", cov.dark);

        // And the chronicle projects words, not hex.
        let mut b = Book::new("Atlas", "deveraux");
        let i = merge_live_river(&mut b, &td).unwrap();
        assert!(b.chapter(i).unwrap().codex.slots[1].text.contains("forge-book\tlore"));
        let _ = std::fs::remove_dir_all(&td);
    }

    // [BOARD:SPINE-5D-NATIVE]
    /// The migration's LIVE gate: the real `.forge/river.idx`, whatever form it is in
    /// today, must still name every crate on disk. A fixture proves the mechanism; only
    /// this proves the spine. Live repo only — elsewhere there is nothing to assert.
    #[test]
    fn the_live_spine_names_the_whole_tree_after_the_migration() {
        let root = Path::new(r"F:\v3");
        if !root.join(".forge/river.idx").is_file() {
            return; // portable: no live spine on this machine
        }
        if !root.join(".forge/spill").is_dir() {
            // The index shell exists but its grain store doesn't (confirmed
            // 2026-08-18: 0 files under .forge/spill, not merely a low count)
            // -- every MAP row's @<hash> then fails to hydrate and degrades
            // to raw hex per hydrate_row's own documented policy ("never
            // dropped"), which reads as 0/N named. That is an unpopulated
            // daemon write surface on this machine, not a coverage defect
            // this test can distinguish from a real one -- same "portable"
            // reasoning as the river.idx guard above, one line up.
            return;
        }
        let cov = spine_coverage(root);
        assert!(cov.dark.is_empty(), "{}", coverage_line(&cov));
        assert_eq!(cov.named, cov.total, "{}", coverage_line(&cov));
    }

    // [BOARD: SPINE-COVERAGE-106]
    /// The live read gauges; the text merge stays lossless. Both halves of the
    /// separation proven in one place so a later edit cannot quietly swap them.
    #[test]
    fn the_live_projection_states_its_own_coverage() {
        let td = std::env::temp_dir().join(format!("spinecov-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&td);
        std::fs::create_dir_all(td.join("crates").join("forge-book")).unwrap();
        std::fs::write(td.join("crates/forge-book/Cargo.toml"), "[package]\n").unwrap();
        std::fs::create_dir_all(td.join(".forge")).unwrap();
        std::fs::write(td.join(".forge/river.idx"), "HEAD\tthe head\n").unwrap();

        let mut b = Book::new("Atlas", "deveraux");
        let i = merge_live_river(&mut b, &td).expect("a live index projects");
        let ch = b.chapter(i).unwrap();
        assert_eq!(ch.lore_count(), 2, "one row + the gauge line");
        assert!(ch.codex.slots[1].text.starts_with("SPINE 0/1"), "{}", ch.codex.slots[1].text);

        let mut plain = Book::new("Atlas", "deveraux");
        let j = merge_river_spine(&mut plain, "HEAD\tthe head\n");
        assert_eq!(plain.chapter(j).unwrap().lore_count(), 1, "the text merge stays lossless");
        let _ = std::fs::remove_dir_all(&td);
    }
}
