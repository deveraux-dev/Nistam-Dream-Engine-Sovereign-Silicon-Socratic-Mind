//! Binary/source staleness check — answers "does this compiled `foreman.exe`
//! match the `forge-foreman-v3` source that's on disk right now", the
//! question tonight's two silent-drift bugs (the `pre_edit` bootstrap
//! deadlock, the `drift` false-FAIL) proved nothing else in the repo asks.
//!
//! `build.rs` embeds [`BUILT_SRC_HASH`] — a `BrutalHash::combine` over this
//! crate's own `src/*.rs` files, sorted, at compile time. [`check`] re-hashes
//! the SAME files, the SAME way, at run time, against the live tree, and
//! reports a mismatch — never silently. A missing/unreadable source tree
//! (e.g. running the binary somewhere without the repo checked out) is not
//! treated as "stale": there is nothing to compare against, so `check`
//! returns `None` rather than a false alarm.
//!
//! This crate is a LIBRARY as well as a bin: `foreman.exe`, `forgedaemon.exe`
//! and `xtask.exe` each bake their own copy of [`BUILT_SRC_HASH`]. Whichever
//! process calls [`check`] answers for ITSELF, so the message names
//! `current_exe()` — naming one binary unconditionally sent an operator to
//! redeploy an already-current `foreman.exe` while the stale daemon did the
//! reporting (2026-08-26).
//!
//! ADR: `crates/forge-vcs-v3/docs/ADR-forge-vcs-v3-not-git-and-the-binary-staleness-gap.md`.

use forge_vcs_v3::hash::BrutalHashExt;
use forge_vcs_v3::spine::BrutalHash;
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/built_src_hash.rs"));

/// Re-hash `crates/forge-foreman-v3/src/*.rs` under `root`, the same way
/// `build.rs` hashed them at compile time (sorted, `BrutalHash::combine`).
/// `None` when the source tree isn't there to hash — not a staleness
/// verdict, just nothing to compare.
fn live_source_hash(root: &Path) -> Option<u64> {
    let src_dir = root.join("crates").join("forge-foreman-v3").join("src");
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

    let mut hashes = Vec::with_capacity(files.len());
    for f in &files {
        let bytes = std::fs::read(f).ok()?;
        hashes.push(BrutalHash::of(&bytes));
    }
    Some(BrutalHash::combine(&hashes).0)
}

/// The rebuild+redeploy line for the binary that is actually stale. Naming the
/// right binary but printing another one's remedy is how an operator ends up
/// redeploying an already-current artifact in a loop.
fn remedy_for(exe: &str) -> &'static str {
    match exe {
        "forgedaemon.exe" | "forgedaemon" => {
            "cargo xtask deploy daemon (or by hand: cargo build -p forge-daemon-door --bin \
             forgedaemon, copy it into .forge/bin/forgedaemon.exe, then bounce it: xtask daemon \
             shutdown — the next hook respawns it detached)."
        }
        "xtask.exe" | "xtask" => "cargo xtask deploy xtask (or: cargo build -p xtask; it is read straight from target/debug).",
        _ => {
            "cargo xtask deploy foreman (or by hand: cargo build --release -p forge-foreman-v3 \
             --bin foreman, then copy target/release/foreman.exe into .forge/bin/foreman.exe)."
        }
    }
}

/// The running executable's file name. This crate is a LIBRARY as well as a
/// bin, so `check` may be running inside forgedaemon.exe or xtask.exe; naming
/// "foreman.exe" unconditionally sent an operator to redeploy a binary that was
/// already current while the real stale one did the reporting (2026-08-26).
fn exe_label() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "this binary".to_string())
}

/// `Some(message)` when the deployed binary's embedded source hash no
/// longer matches the live source tree under `root` — rebuild+redeploy is
/// named as the fix, since that's the only fix (never auto-rebuilds itself:
/// a hook process silently spawning `cargo build --release` on every turn
/// would be its own new failure mode). `None` covers both "genuinely
/// fresh" and "nothing to compare against" — callers that need to tell
/// those apart should call `live_source_hash` themselves; every current
/// caller (drift, stop) only needs "is there a problem to report".
pub fn check(root: &Path) -> Option<String> {
    let live = live_source_hash(root)?;
    if live == BUILT_SRC_HASH {
        None
    } else {
        let exe = exe_label();
        Some(format!(
            "STALE BINARY: {exe} was built from source hash {BUILT_SRC_HASH:016x}, \
             but crates/forge-foreman-v3/src/*.rs now hashes {live:016x} — rebuild + redeploy \
             {exe} specifically. This crate is linked by foreman.exe, forgedaemon.exe AND \
             xtask.exe, so each carries its own copy and redeploying one does not clear another. \
             {}",
            remedy_for(&exe)
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
        let nowhere = std::env::temp_dir().join("forge-foreman-staleness-test-nowhere");
        assert_eq!(check(&nowhere), None, "no source to compare against must never read as STALE");
    }
}
