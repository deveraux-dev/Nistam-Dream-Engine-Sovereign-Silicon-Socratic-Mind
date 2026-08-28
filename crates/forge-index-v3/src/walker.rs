//! Zero-`unsafe` substitute for `outland::mft`'s raw MFT-volume FFI walk.
//!
//! `outland/src/mft.rs` reads NTFS file records directly via `windows_sys`
//! (`CreateFileW`/`DeviceIoControl`, `unsafe` FFI, requires admin rights) to beat a
//! measured `read_dir` floor of "~18us/entry... 285ms for 15,987 entries"
//! (`outland/src/mft.rs:4`, `outland/src/lib.rs:826`). This workspace denies
//! `unsafe_code` outright (`Cargo.toml:117`), so that FFI path is not ported.
//!
//! What IS ported: the ~18us/entry floor itself is `read_dir`'s, not the FFI's — a
//! plain walk already lands comfortably sub-millisecond per entry. This is an
//! explicit-stack, bounded iterative walk (never an unbounded recursive function
//! call) per CLAUDE.md's `unbound_io` forbid, with the same `max_files`/
//! `max_seconds` budget shape `outland::lib.rs::walk_index` already used.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// One entry found by [`walk_bounded`].
#[derive(Debug, Clone)]
pub struct WalkEntry {
    /// The entry's full path.
    pub path: PathBuf,
    /// Whether this entry is a directory (and was therefore also queued for descent).
    pub is_dir: bool,
}

/// Why [`walk_bounded`] stopped before exhausting the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkStop {
    /// The tree was fully walked; nothing was cut short.
    Exhausted,
    /// `max_files` was reached.
    FileCap,
    /// `max_seconds` elapsed.
    TimeCap,
}

/// Result of a bounded walk: what was found, and why it stopped.
#[derive(Debug, Clone)]
pub struct WalkReport {
    /// Every entry found, root-relative traversal order (not sorted).
    pub entries: Vec<WalkEntry>,
    /// Why the walk ended.
    pub stop: WalkStop,
}

/// Explicit-stack, bounded directory walk — no recursive function call, no
/// `unsafe`, no external dependency. Descends into subdirectories via a `Vec`
/// work-stack, refusing to exceed `max_files` entries or `max_seconds` wall time.
/// A missing `root` yields an empty, `Exhausted` report — absence is visible, not
/// an error (matches `idx.rs`'s posture for a cold index).
pub fn walk_bounded(root: &Path, max_files: usize, max_seconds: u64) -> WalkReport {
    let start = Instant::now();
    let budget = Duration::from_secs(max_seconds);
    let mut entries = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    if !root.exists() {
        return WalkReport { entries, stop: WalkStop::Exhausted };
    }

    while let Some(dir) = stack.pop() {
        if entries.len() >= max_files {
            return WalkReport { entries, stop: WalkStop::FileCap };
        }
        if start.elapsed() >= budget {
            return WalkReport { entries, stop: WalkStop::TimeCap };
        }
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for item in read {
            let Ok(item) = item else { continue };
            let path = item.path();
            let is_dir = item.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                stack.push(path.clone());
            }
            entries.push(WalkEntry { path, is_dir });
            if entries.len() >= max_files {
                return WalkReport { entries, stop: WalkStop::FileCap };
            }
        }
    }
    WalkReport { entries, stop: WalkStop::Exhausted }
}

/// The conservative, machine-owned/OS-owned directory names a whole-drive scan
/// refuses to descend into. Exact-name match only (no glob/regex — forbidden_ops).
pub const DEFAULT_SKIP_DIRS: &[&str] = &[
    "System Volume Information",
    "$Recycle.Bin",
    "$WinREAgent",
    ".git",
    "target",
    "node_modules",
    ".wrangler",
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "AppData",
];

pub(crate) fn skips_dir(name: &str, extra: &[String]) -> bool {
    DEFAULT_SKIP_DIRS.contains(&name) || extra.iter().any(|e| e == name)
}

/// Same contract as [`walk_bounded`], but a directory whose name matches
/// [`DEFAULT_SKIP_DIRS`] (or `extra_skip`) is recorded as an entry and never
/// pushed onto the descent stack — the budget is spent on real content, not
/// re-walking `Windows`/`AppData`/`target` on every whole-drive scan.
pub fn walk_bounded_skipping(root: &Path, max_files: usize, max_seconds: u64, extra_skip: &[String]) -> WalkReport {
    let start = Instant::now();
    let budget = Duration::from_secs(max_seconds);
    let mut entries = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    if !root.exists() {
        return WalkReport { entries, stop: WalkStop::Exhausted };
    }

    while let Some(dir) = stack.pop() {
        if entries.len() >= max_files {
            return WalkReport { entries, stop: WalkStop::FileCap };
        }
        if start.elapsed() >= budget {
            return WalkReport { entries, stop: WalkStop::TimeCap };
        }
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for item in read {
            let Ok(item) = item else { continue };
            let path = item.path();
            let is_dir = item.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                let name = item.file_name().to_string_lossy().into_owned();
                if !skips_dir(&name, extra_skip) {
                    stack.push(path.clone());
                }
            }
            entries.push(WalkEntry { path, is_dir });
            if entries.len() >= max_files {
                return WalkReport { entries, stop: WalkStop::FileCap };
            }
        }
    }
    WalkReport { entries, stop: WalkStop::Exhausted }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_root_yields_an_empty_exhausted_report_not_an_error() {
        let report = walk_bounded(Path::new("F:\\this_path_does_not_exist_xyz123"), 100, 5);
        assert!(report.entries.is_empty());
        assert_eq!(report.stop, WalkStop::Exhausted);
    }

    #[test]
    fn this_crates_own_src_dir_walks_and_finds_its_own_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let report = walk_bounded(&root, 1000, 5);
        assert_eq!(report.stop, WalkStop::Exhausted);
        let names: Vec<String> = report
            .entries
            .iter()
            .filter_map(|e| e.path.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert!(names.contains(&"lib.rs".to_string()));
        assert!(names.contains(&"walker.rs".to_string()));
    }

    #[test]
    fn a_tiny_file_cap_stops_early_and_says_so() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let report = walk_bounded(&root, 1, 5);
        assert_eq!(report.stop, WalkStop::FileCap);
        assert_eq!(report.entries.len(), 1);
    }

    #[test]
    fn a_skipped_dir_is_recorded_but_never_descended_into() {
        // this crate's own src dir has no `target`/`.git` child to skip, so build a
        // throwaway tree instead: root/target/deep_file.txt + root/real.txt.
        let root = std::env::temp_dir().join(format!("walker-skip-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join("target").join("deep_file.txt"), b"x").unwrap();
        std::fs::write(root.join("real.txt"), b"x").unwrap();

        let report = walk_bounded_skipping(&root, 1000, 5, &[]);
        assert_eq!(report.stop, WalkStop::Exhausted);
        let names: Vec<String> =
            report.entries.iter().filter_map(|e| e.path.file_name().map(|n| n.to_string_lossy().into_owned())).collect();
        assert!(names.contains(&"target".to_string()), "the skipped dir itself is still recorded");
        assert!(names.contains(&"real.txt".to_string()));
        assert!(!names.contains(&"deep_file.txt".to_string()), "never descended into `target`");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn default_skip_dirs_covers_the_conservative_os_noise_list() {
        for name in ["Windows", "Program Files", "AppData", "target", "node_modules", ".git"] {
            assert!(DEFAULT_SKIP_DIRS.contains(&name), "{name} must be in the conservative skip list");
        }
    }
}
