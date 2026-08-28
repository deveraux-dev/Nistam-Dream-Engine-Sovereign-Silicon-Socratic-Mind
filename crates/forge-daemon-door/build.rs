//! Embeds a `BrutalHash` of this daemon's GATED SOURCES — its own `src/*.rs`
//! plus `forge-foreman-v3/src/*.rs`, whose hook logic it links and executes —
//! so `staleness.rs` can tell at run time whether the running daemon still
//! matches the tree. Deterministic and order-sensitive: dirs in `GATED_DIRS`
//! order, files sorted within each, `BrutalHash::combine`. `staleness.rs`
//! MUST walk the same order or the gate reds forever.

use forge_vcs_v3::hash::BrutalHashExt;
use forge_vcs_v3::spine::BrutalHash;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo"));
    let crates_dir = manifest_dir.parent().expect("crate lives under crates/");

    // Own sources first, then the dependency whose code this binary runs. A
    // stale daemon carrying old `forge_foreman_v3::hook` logic reported itself
    // GREEN before this: its hash covered only the crate that had not changed.
    let gated = [manifest_dir.join("src"), crates_dir.join("forge-foreman-v3").join("src")];

    let mut hashes = Vec::new();
    for dir in &gated {
        // Watch the DIRECTORY as well as each file: per-file watches only cover
        // files that existed at the LAST build, so ADDING a src/*.rs moves the
        // live hash while no watched path does, and build.rs never re-runs
        // (the scar forge-foreman-v3/build.rs:17-21 already records).
        println!("cargo:rerun-if-changed={}", dir.display());

        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("gated dir {}: {e}", dir.display()))
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "rs"))
            .collect();
        files.sort();

        for f in &files {
            println!("cargo:rerun-if-changed={}", f.display());
            let bytes = std::fs::read(f).unwrap_or_else(|e| panic!("read {}: {e}", f.display()));
            hashes.push(BrutalHash::of(&bytes));
        }
    }
    let combined = BrutalHash::combine(&hashes);

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    let dest = PathBuf::from(out_dir).join("built_src_hash.rs");
    std::fs::write(
        &dest,
        format!("/// `BrutalHash::combine` over this daemon's gated sources, computed by `build.rs` at compile time.\npub const BUILT_SRC_HASH: u64 = {};\n", combined.0),
    )
        .unwrap_or_else(|e| panic!("write {}: {e}", dest.display()));
}
