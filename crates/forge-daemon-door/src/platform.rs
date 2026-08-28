//! Platform path resolvers — the SoT root only.
//!
//! SCOPE CUT from v2's `F:\NewRepo\crates\forge-daemon\src\platform.rs` (317
//! lines, 17 functions): only [`sot_root`] is ported. `timeline_recorder.rs`'s
//! `global()`/`after_commit()` are its only real callers in this port's
//! closure; the other 16 functions (`forge_repos`, `declarations_path`,
//! `roadmap_path`, `capability_index_path`, `census_path`, ...) resolve v2's
//! own repo-map/ROADMAP.json/capability-index tooling layout, which has no
//! v3 equivalent — porting them would be scaffolding against paths that
//! don't exist here. Windows-only, matching v2 (no WSL in this repo).

use std::path::PathBuf;

/// The SoT root — the folder that holds `.forge/`, `crates/`. The durable
/// time-machine tape (`.forge/timeline.chain`) is written relative to this.
///
/// Resolution: `FORGE_FLOOR` env override, else hardcoded `F:\v3` — this
/// workspace's own live SoT (v2's fallback was `F:\NewRepo`; v3's is `F:\v3`,
/// per `e_drive_is_tape`: E:\v3 is the backup tape, F:\v3 is the write surface).
pub fn sot_root() -> PathBuf {
    if let Ok(p) = std::env::var("FORGE_FLOOR") {
        return PathBuf::from(p);
    }
    PathBuf::from(r"F:\v3")
}

/// Serializes every test in this crate that sets/reads the `FORGE_FLOOR`
/// env var — it's process-global, and `cargo test`'s default parallelism
/// otherwise lets two such tests race each other's value (confirmed live:
/// `singleton::tests` and `door::tests` both redirect it independently).
#[cfg(test)]
pub(crate) fn forge_floor_test_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sot_root_is_the_live_v3_tree() {
        // FORGE_FLOOR unset in tests → hardcoded fallback.
        if std::env::var_os("FORGE_FLOOR").is_none() {
            assert_eq!(sot_root(), PathBuf::from(r"F:\v3"));
        }
    }
}
