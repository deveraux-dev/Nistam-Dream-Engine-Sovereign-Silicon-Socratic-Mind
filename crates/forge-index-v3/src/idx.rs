//! Build/query surface for `.forge/outland-v3crates.idx` — wraps `embed5` +
//! `walker::walk_bounded_skipping` (both already ported) with no new math.
//! Per-file granularity: one `embed5` vector per `.rs` file under `crates/`.

use std::path::{Path, PathBuf};

use crate::walker::walk_bounded_skipping;
use crate::{dist_sq_family_dominant, embed5};

/// One indexed file's path and its ranked distance to a query.
#[derive(Debug, Clone)]
pub struct RankedHit {
    /// Path relative to the workspace root.
    pub path: PathBuf,
    /// Family-dominant distance to the query (`dist_sq_family_dominant`).
    pub dist: i128,
}

/// Walk `root/crates` (bounded, skip-dirs applied) and write one
/// `path\tx\ty\tz\ttheta\tw` row per `.rs` file to `out_path`. Returns the
/// row count written.
pub fn build_index(root: &Path, out_path: &Path, max_files: usize, max_seconds: u64) -> std::io::Result<usize> {
    let crates_dir = root.join("crates");
    let report = walk_bounded_skipping(&crates_dir, max_files, max_seconds, &[]);
    let mut body = String::new();
    let mut n = 0usize;
    for entry in &report.entries {
        if entry.is_dir {
            continue;
        }
        if entry.path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&entry.path) else {
            continue;
        };
        let v = embed5(&content);
        let rel = entry.path.strip_prefix(root).unwrap_or(&entry.path);
        body.push_str(&format!("{}\t{}\t{}\t{}\t{}\t{}\n", rel.display(), v[0], v[1], v[2], v[3], v[4]));
        n += 1;
    }
    std::fs::write(out_path, body)?;
    Ok(n)
}

/// Embed `query`, rank every row in `idx_path` by `dist_sq_family_dominant`,
/// return the closest `top`. An unreadable/missing index yields an empty
/// list (absence is visible, not an error — matches `walker`'s posture).
pub fn query_index(idx_path: &Path, query: &str, top: usize) -> Vec<RankedHit> {
    let Ok(body) = std::fs::read_to_string(idx_path) else {
        return Vec::new();
    };
    let qv = embed5(query);
    let mut hits: Vec<RankedHit> = body
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let path = parts.next()?;
            let v: Vec<i64> = parts.filter_map(|p| p.parse().ok()).collect();
            if v.len() != 5 {
                return None;
            }
            let vec = [v[0], v[1], v[2], v[3], v[4]];
            Some(RankedHit { path: PathBuf::from(path), dist: dist_sq_family_dominant(qv, vec) })
        })
        .collect();
    hits.sort_by_key(|h| h.dist);
    hits.truncate(top);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        std::env::temp_dir().join(format!(
            "outland-idx-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ))
    }

    #[test]
    fn build_then_query_finds_the_matching_family() {
        let root = scratch();
        std::fs::create_dir_all(root.join("crates").join("fake-crate").join("src")).unwrap();
        std::fs::write(
            root.join("crates").join("fake-crate").join("src").join("mutex.rs"),
            "pub fn lock_mutex() { thread::spawn(|| {}); }",
        )
        .unwrap();
        std::fs::write(
            root.join("crates").join("fake-crate").join("src").join("shader.rs"),
            "pub fn draw_pixel(texture: u32) { let _ = texture; }",
        )
        .unwrap();

        let out = root.join("out.idx");
        let n = build_index(&root, &out, 1000, 5).unwrap();
        assert_eq!(n, 2, "both .rs files indexed");

        let hits = query_index(&out, "mutex thread lock", 5);
        assert!(!hits.is_empty());
        assert!(hits[0].path.to_string_lossy().contains("mutex.rs"), "closest hit should be the concurrency file");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_index_queries_to_empty_not_an_error() {
        let hits = query_index(Path::new("F:\\this_index_does_not_exist_xyz.idx"), "anything", 5);
        assert!(hits.is_empty());
    }
}
