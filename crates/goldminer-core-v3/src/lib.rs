//! Goldminer — point-at-any-repo 5D ray search.
//!
//! Sellable SKU core, distinct from `.forge/river.idx` (13forge's own state
//! spine). This crate indexes the CALLER's target directory — a customer's
//! codebase, not ours. Same embedding/ray math as `forge_ml::nearest_neighbor`
//! (proven daemon-free, Gemma-free — `tools/goldminer/DAEMON-DOWN-FEASIBILITY-2026-07-12.md`),
//! generalized from "one line of river.idx" to "one line of any source file."
//!
//! Indexing is cheap deterministic hashing (no ML inference), so rebuilding
//! on every run is the default and resolves the one caveat the feasibility
//! report flagged against a sealed/frozen snapshot: this always reads the
//! target directory's CURRENT bytes, never a stale seal.

use forge_ml_bqrouter::nearest_neighbor::{
    closest_point_on_ray, embed_river_line, nearest, ray_between, squared_distance, CodeEntry,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Directories never descended into — build/vendor/vcs noise, not source.
const EXCLUDE_DIRS: &[&str] = &[
    "node_modules", ".git", "target", "venv", ".venv", "__pycache__", "dist", "build", ".next",
    "out",
];

/// Extensions indexed when the caller doesn't specify its own list.
pub const DEFAULT_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "rb", "php", "java", "c", "cpp", "h", "hpp", "cs",
    "html", "css", "md", "toml", "json", "yaml", "yml",
];

/// One indexed source line: where it came from + its text.
#[derive(Debug, Clone)]
pub struct IndexedLine {
    /// Absolute or relative path to the source file.
    pub file: PathBuf,
    /// 1-based line number in the source file.
    pub line_no: usize,
    /// Exact text content of the indexed line.
    pub text: String,
}

/// A built index over one target directory: parallel codebook + source map.
#[derive(Debug, Default)]
pub struct Index {
    codebook: Vec<CodeEntry>,
    lines: Vec<IndexedLine>,
}

impl Index {
    /// Number of indexed source lines.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the index contains zero lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The indexed source lines, read-only. `ray_query` bakes `embed_river_line` (FNV1a)
    /// into the codebook at build time, so a caller that wants a NEWER 5D lane — the
    /// semantic/distributional embedders that postdate this crate — must re-embed the
    /// text itself. Exposing the lines is what makes that possible without a second walk
    /// of the tree (`forge_studio::aim_corpus::ray_indices`, Sean 08-02: "the ray was made
    /// before a lot of our tech including trit"). The codebook stays private: handing out
    /// coordinates from one embedding invites mixing spaces, which is the one thing a ray
    /// cannot survive.
    pub fn lines(&self) -> &[IndexedLine] {
        &self.lines
    }
}

fn is_excluded(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        EXCLUDE_DIRS.iter().any(|ex| *ex == s)
    })
}

fn has_indexed_ext(path: &Path, extensions: &[&str]) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)),
        None => false,
    }
}

fn walk_files(root: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if is_excluded(root) {
        return Ok(());
    }
    // A denied/vanished subdir (ACL wall, locked junction) skips, never aborts:
    // one unreadable corner must not kill the index of a whole target tree.
    let Ok(entries) = fs::read_dir(root) else { return Ok(()) };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_files(&path, extensions, out)?;
        } else if has_indexed_ext(&path, extensions) {
            out.push(path);
        }
    }
    Ok(())
}

/// Build a fresh index over `root` — the CALLER's target directory. Cheap
/// deterministic per-line embedding; rebuild every run rather than cache.
pub fn build_index(root: &Path, extensions: &[&str]) -> std::io::Result<Index> {
    let mut files = Vec::new();
    walk_files(root, extensions, &mut files)?;
    files.sort();

    let mut codebook = Vec::new();
    let mut lines = Vec::new();
    let mut id: u32 = 0;
    for file in files {
        let Ok(bytes) = fs::read(&file) else { continue };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            codebook.push(CodeEntry { id, coords: embed_river_line(line) });
            lines.push(IndexedLine { file: file.clone(), line_no: i + 1, text: line.to_string() });
            id += 1;
        }
    }
    Ok(Index { codebook, lines })
}

/// One ranked search hit.
#[derive(Debug, Clone)]
pub struct Hit<'a> {
    /// Reference to the matched source line.
    pub line: &'a IndexedLine,
    /// Squared distance — perpendicular-to-ray for `ray_query`, direct for `search`. Lower = closer.
    pub score: i64,
}

/// Point search: rank every indexed line by distance to `query_text`'s own
/// embedding. Good for "find lines like this one."
pub fn search<'a>(index: &'a Index, query_text: &str, top_n: usize) -> Vec<Hit<'a>> {
    let q = embed_river_line(query_text);
    let mut hits: Vec<Hit> = index
        .codebook
        .iter()
        .zip(index.lines.iter())
        .map(|(entry, line)| Hit { line, score: squared_distance(&q, &entry.coords) })
        .collect();
    hits.sort_by_key(|h| h.score);
    hits.truncate(top_n);
    hits
}

/// Ray search: cast from `from_text`'s embedding toward `toward_text`'s
/// embedding, rank every indexed line by perpendicular distance to that ray.
///
/// Honest scope: `embed_river_line` is a SYNTACTIC hash embedding (avalanche
/// on exact text — see `forge_ml_bqrouter::nearest_neighbor` module doc), not semantic.
/// It does NOT infer meaning, and "shares a tag" is only a statistical lean
/// toward ranking closer, not a guarantee (the other 4 lanes are independent
/// hashes of the rest of the line, so a same-tag row can still land far from
/// the ray). The one thing that IS deterministic, by construction of
/// `closest_point_on_ray`: if `toward_text` matches an indexed line's text
/// exactly, that line's perpendicular distance is exactly 0 and it ranks
/// first — because a true ray (`t >= 0`) passes exactly through its own
/// endpoint at `t = 1`. Use this for "show me what's ranked around a known
/// example," not "infer what continues this trajectory of prose."
pub fn ray_query<'a>(index: &'a Index, from_text: &str, toward_text: &str, top_n: usize) -> Vec<Hit<'a>> {
    let ray = ray_between(&embed_river_line(from_text), &embed_river_line(toward_text));
    let mut hits: Vec<Hit> = index
        .codebook
        .iter()
        .zip(index.lines.iter())
        .map(|(entry, line)| {
            let (_, perp) = closest_point_on_ray(&ray, &entry.coords);
            Hit { line, score: perp }
        })
        .collect();
    hits.sort_by_key(|h| h.score);
    hits.truncate(top_n);
    hits
}

/// Single nearest hit to a raw 5D coordinate — thin wrapper for callers that
/// already have an embedding (e.g. re-querying a prior hit's coords).
pub fn nearest_hit<'a>(index: &'a Index, coords: &[i32; forge_ml_bqrouter::nearest_neighbor::EMBED_DIM]) -> Option<Hit<'a>> {
    let (id, score) = nearest(coords, &index.codebook)?;
    let line = index.lines.get(id as usize)?;
    Some(Hit { line, score })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &Path, name: &str, contents: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    #[test]
    fn indexes_the_targets_own_directory_not_ours() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "widget.rs", "fn make_widget() -> Widget {\n    Widget::new()\n}\n");
        write_file(tmp.path(), "vendor/lib.js", "console.log('vendored, should still index');\n");

        let idx = build_index(tmp.path(), DEFAULT_EXTENSIONS).unwrap();
        assert_eq!(idx.len(), 4, "4 non-empty lines across the 2 files");
        assert!(idx.lines.iter().all(|l| l.file.starts_with(tmp.path())),
            "every hit must trace back into the CALLER's directory, never our own repo");
    }

    #[test]
    fn excludes_noise_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "src/real.rs", "fn real() {}\n");
        write_file(tmp.path(), "node_modules/pkg/index.js", "module.exports = 1;\n");
        write_file(tmp.path(), "target/debug/build.rs", "// build artifact\n");

        let idx = build_index(tmp.path(), DEFAULT_EXTENSIONS).unwrap();
        assert_eq!(idx.len(), 1);
        assert!(idx.lines[0].file.ends_with("src/real.rs") || idx.lines[0].file.ends_with("src\\real.rs"));
    }

    #[test]
    fn search_ranks_the_closest_line_first() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "a.rs", "fn parse_widget_config() {}\n");
        write_file(tmp.path(), "b.rs", "fn totally_unrelated_thing() {}\n");

        let idx = build_index(tmp.path(), DEFAULT_EXTENSIONS).unwrap();
        let hits = search(&idx, "fn parse_widget_config() {}", 2);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].score, 0, "exact text match must be a zero-distance top hit");
        assert!(hits[0].line.file.ends_with("a.rs") || hits[0].line.file.ends_with("a.rs"));
    }

    #[test]
    fn ray_query_ranks_both_ray_endpoints_first_with_zero_distance() {
        // The one thing ray_query deterministically guarantees (see its doc):
        // a true ray (t >= 0) passes exactly through its own origin at t = 0
        // AND its own `to` endpoint at t = 1. So both `from_text`'s and
        // `toward_text`'s indexed lines tie at zero perpendicular distance
        // and must outrank everything else.
        let tmp = tempfile::tempdir().unwrap();
        write_file(
            tmp.path(),
            "chain.txt",
            "CHAIN\tstep_one\nCHAIN\tstep_two\nCHAIN\tstep_three\nOTHER\tunrelated_row\n",
        );

        let idx = build_index(tmp.path(), &["txt"]).unwrap();
        let hits = ray_query(&idx, "CHAIN\tstep_one", "CHAIN\tstep_two", 4);
        assert_eq!(hits.len(), 4);
        assert_eq!(hits[0].score, 0, "the ray's own origin must be a zero-distance hit");
        assert_eq!(hits[1].score, 0, "the ray's own toward-endpoint must be a zero-distance hit");
        assert!(hits[0].line.text.contains("step_one"), "stable sort keeps the lower-id (origin) entry first among the zero-score tie");
        assert!(hits[1].line.text.contains("step_two"));
        assert!(hits[2].score > 0 && hits[3].score > 0, "everything off the ray must rank strictly behind the tied endpoints");
    }

    #[test]
    fn rebuilds_fresh_every_call_no_staleness() {
        let tmp = tempfile::tempdir().unwrap();
        write_file(tmp.path(), "a.rs", "fn v1() {}\n");
        let idx1 = build_index(tmp.path(), DEFAULT_EXTENSIONS).unwrap();
        assert_eq!(idx1.len(), 1);

        write_file(tmp.path(), "b.rs", "fn v2() {}\n");
        let idx2 = build_index(tmp.path(), DEFAULT_EXTENSIONS).unwrap();
        assert_eq!(idx2.len(), 2, "a rebuild must see files written after the first index, unlike a sealed snapshot");
    }
}
