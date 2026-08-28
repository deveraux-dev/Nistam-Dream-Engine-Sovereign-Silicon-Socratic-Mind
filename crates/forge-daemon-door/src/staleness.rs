//! Binary/source staleness check — answers "does this compiled daemon
//! match the `forge-daemon-door` source that's on disk right now", so that
//! a stale daemon (compiled before recent edits to this crate or to
//! `forge-foreman-v3` which the daemon depends on) does not silently execute
//! old hook logic.
//!
//! `build.rs` embeds [`BUILT_SRC_HASH`] — a `BrutalHash::combine` over the
//! [`GATED_CRATES`] sources (this crate's own `src/*.rs` AND
//! `forge-foreman-v3`'s, whose hook logic this daemon executes), in that order,
//! sorted within each. [`check`] re-hashes
//! the SAME files, the SAME way, at run time, against the live tree, and
//! reports a mismatch — never silently. A missing/unreadable source tree
//! (e.g. running the binary somewhere without the repo checked out) is not
//! treated as "stale": there is nothing to compare against, so `check`
//! returns `None` rather than a false alarm.

use forge_vcs_v3::hash::BrutalHashExt;
use forge_vcs_v3::spine::BrutalHash;
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/built_src_hash.rs"));

/// The crates whose sources this binary's behaviour depends on, in the ORDER
/// `build.rs` hashes them. Own crate first, then the dependency whose hook
/// logic the daemon executes. Changing this order silently reds the gate.
const GATED_CRATES: [&str; 2] = ["forge-daemon-door", "forge-foreman-v3"];

/// Re-hash the gated sources under `root` exactly as `build.rs` hashed them at
/// compile time: `GATED_CRATES` order, files sorted within each crate,
/// `BrutalHash::combine`. `None` when the tree isn't there to hash — not a
/// staleness verdict, just nothing to compare.
fn live_source_hash(root: &Path) -> Option<u64> {
    let mut hashes = Vec::new();
    for krate in GATED_CRATES {
        let src_dir = root.join("crates").join(krate).join("src");
        let mut files: Vec<PathBuf> = std::fs::read_dir(&src_dir)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "rs"))
            .collect();
        if files.is_empty() {
            return None;
        }
        files.sort();
        for f in &files {
            let bytes = std::fs::read(f).ok()?;
            hashes.push(BrutalHash::of(&bytes));
        }
    }
    Some(BrutalHash::combine(&hashes).0)
}

/// The running executable's file name, for a message that names the binary
/// that is ACTUALLY stale. Hardcoding one name sent an operator chasing a
/// binary that was already current while the real stale one did the reporting
/// (2026-08-26).
pub(crate) fn exe_label() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "this binary".to_string())
}

/// `Some(message)` when the deployed binary's embedded source hash no
/// longer matches the live source tree under `root` — rebuild+restart is
/// named as the fix, since that's the only fix (never auto-rebuilds itself).
/// `None` covers both "genuinely fresh" and "nothing to compare against" —
/// callers that need to tell those apart should call `live_source_hash`
/// themselves; every current caller only needs "is there a problem to report".
pub fn check(root: &Path) -> Option<String> {
    let live = live_source_hash(root)?;
    if live == BUILT_SRC_HASH {
        None
    } else {
        let exe = exe_label();
        let gated = GATED_CRATES.map(|k| format!("crates/{k}/src/*.rs")).join(" + ");
        Some(format!(
            "STALE DAEMON: {exe} was built from source hash {BUILT_SRC_HASH:016x}, \
             but its gated sources ({gated}) now hash {live:016x} — rebuild + restart: \
             cargo xtask deploy daemon (or by hand: cargo build -p forge-daemon-door --bin \
             forgedaemon, then restart the daemon)."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_hash_matches_the_live_tree_this_binary_was_just_compiled_from() {
        // The strongest test this module can carry: build.rs and this test
        // both run against the SAME checked-out source, so `check` on the
        // real repo root must return None right after a fresh build — if
        // it doesn't, the two hashing paths have already diverged.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(check(&root), None, "a freshly built binary must not flag its own source as stale");
    }

    #[test]
    fn missing_source_tree_is_none_not_a_false_stale_alarm() {
        let nowhere = std::env::temp_dir().join("forge-daemon-door-staleness-test-nowhere");
        assert_eq!(check(&nowhere), None, "no source to compare against must never read as STALE");
    }

    /// Plant a fake tree with a `.rs` in every gated crate's `src/`.
    fn plant(temp_dir: &Path, crates: &[&str]) {
        for k in crates {
            let src = temp_dir.join("crates").join(k).join("src");
            let _ = std::fs::create_dir_all(&src);
            let _ = std::fs::write(src.join("test.rs"), b"// different source\n");
        }
    }

    #[test]
    fn stale_detector_rejects_mismatched_source() {
        let temp_dir = std::env::temp_dir().join("forge-daemon-door-staleness-mismatch-test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        plant(&temp_dir, &GATED_CRATES);

        let result = check(&temp_dir);
        assert!(
            result.is_some(),
            "mismatch between BUILT_SRC_HASH and live files must return Some(msg)"
        );
        let msg = result.unwrap();
        assert!(msg.contains("STALE DAEMON"), "message must reference daemon staleness");
        assert!(
            msg.contains("cargo build -p forge-daemon-door"),
            "message must include rebuild instructions"
        );
        assert!(
            msg.contains("cargo xtask deploy daemon"),
            "the verb-first remedy must be named"
        );
        assert!(
            msg.contains("forge-foreman-v3"),
            "the gated set must be named, so the operator knows a dependency edit counts"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    /// The whole point of this change: an edit to the DEPENDENCY moves the
    /// hash. Before it, only this crate's own sources counted and a daemon
    /// running stale `forge_foreman_v3::hook` logic reported itself GREEN.
    #[test]
    fn a_dependency_edit_moves_the_hash() {
        let temp_dir = std::env::temp_dir().join("forge-daemon-door-staleness-dep-edit");
        let _ = std::fs::remove_dir_all(&temp_dir);
        plant(&temp_dir, &GATED_CRATES);
        let before = live_source_hash(&temp_dir).expect("planted tree hashes");

        let dep_src = temp_dir.join("crates").join("forge-foreman-v3").join("src").join("test.rs");
        let _ = std::fs::write(&dep_src, b"// edited dependency\n");
        let after = live_source_hash(&temp_dir).expect("planted tree still hashes");

        assert_ne!(before, after, "editing forge-foreman-v3 must change the daemon's gated hash");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    /// A partial tree (own crate present, dependency absent) has nothing
    /// trustworthy to compare and must not read as STALE.
    #[test]
    fn a_missing_gated_crate_is_none_not_a_false_alarm() {
        let temp_dir = std::env::temp_dir().join("forge-daemon-door-staleness-partial");
        let _ = std::fs::remove_dir_all(&temp_dir);
        plant(&temp_dir, &["forge-daemon-door"]);
        assert_eq!(check(&temp_dir), None, "a half-present tree is not evidence of staleness");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
