//! Embeds a `BrutalHash` of this crate's own `src/*.rs` files into the
//! compiled binary at build time — the staleness receipt `staleness.rs`
//! checks at runtime against a live re-hash of the same files (ADR:
//! crates/forge-vcs-v3/docs/ADR-forge-vcs-v3-not-git-and-the-binary-staleness-gap.md).
//! Deliberately shallow (`src/` only, no subdirs — this crate has none) and
//! deterministic: sorted file order, `BrutalHash::combine` (order-sensitive,
//! matches `staleness.rs`'s own hashing exactly).

use forge_vcs_v3::hash::BrutalHashExt;
use forge_vcs_v3::spine::BrutalHash;
use std::path::PathBuf;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let src_dir = PathBuf::from(&manifest_dir).join("src");

    // Watch the DIRECTORY, not just the files in it. The per-file watches below
    // only cover files that existed at the LAST build, so ADDING a new src/*.rs
    // changed the live hash while no watched path moved — build.rs never re-ran,
    // the embedded hash stayed stale, and the gate cried wolf about a binary
    // that was in fact current (2026-08-26: sidecar_launch.rs).
    println!("cargo:rerun-if-changed={}", src_dir.display());

    let mut files: Vec<PathBuf> = std::fs::read_dir(&src_dir)
        .expect("this crate's own src/ dir must exist")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();

    let mut hashes = Vec::with_capacity(files.len());
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let bytes = std::fs::read(f).unwrap_or_else(|e| panic!("read {}: {e}", f.display()));
        hashes.push(BrutalHash::of(&bytes));
    }
    let combined = BrutalHash::combine(&hashes);

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    let dest = PathBuf::from(out_dir).join("built_src_hash.rs");
    std::fs::write(
        &dest,
        format!("/// `BrutalHash::combine` over this crate's own `src/*.rs`, computed by `build.rs` at compile time.\npub const BUILT_SRC_HASH: u64 = {};\n", combined.0),
    )
        .unwrap_or_else(|e| panic!("write {}: {e}", dest.display()));
}
