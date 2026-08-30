//! flag_gauge — flags that cannot stale. A flag is a PURE probe over live disk,
//! recomputed every read (chaos_gauge discipline); no stored word exists to rot.
//! Probe target unreadable = VOID and LOUD (arch_tablets law), never silently Clear.

use std::path::Path;

/// How a debt is gauged from disk. All probes re-read at eval time.
#[derive(Debug, Clone, Copy)]
pub enum Probe {
    /// Debt LIVE while `needle` appears in NO `.rs` file under `dir`
    /// (a consumer that should exist, doesn't). `exclude` names the one file
    /// (by file name) that may legitimately contain the needle — the definition
    /// site — so a definition can never satisfy its own consumer probe.
    NeedleAbsentDir {
        /// The directory to scan.
        dir: &'static str,
        /// The needle string expected to appear in some consumer file.
        needle: &'static str,
        /// The definition-site file name exempted from counting as its own consumer.
        exclude: &'static str,
    },
    /// Debt LIVE while `needle` still appears somewhere under `dir`
    /// (a stale marker that should be gone, isn't).
    NeedlePresentDir {
        /// The directory to scan.
        dir: &'static str,
        /// The stale needle string that should no longer appear.
        needle: &'static str,
    },
    /// Debt LIVE while `path` itself is missing.
    PathMissing {
        /// The path that must exist on disk.
        path: &'static str,
    },
}

/// One gauged flag: an id, the debt it names, and the probe that decides it.
#[derive(Debug, Clone, Copy)]
pub struct FlagSpec {
    /// Unique identifier for this flag.
    pub id: &'static str,
    /// Human-readable description of the debt this flag tracks.
    pub debt: &'static str,
    /// The probe that determines whether this flag is live, clear, or void.
    pub probe: Probe,
}

/// The three honest states. `Void` = the probe's own target is gone/unreadable —
/// the flag's AIM rotted, which must be louder than either verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagState {
    /// Flag is live (debt is present); contains a receipt or explanation.
    Live(String),
    /// Flag is cleared (debt is resolved); contains a receipt or explanation.
    Clear(String),
    /// Flag's probe target is unreadable or missing; contains the error message.
    Void(String),
}

impl FlagState {
    /// Get the state tag ("LIVE", "CLEAR", or "VOID").
    pub fn tag(&self) -> &'static str {
        match self {
            FlagState::Live(_) => "LIVE",
            FlagState::Clear(_) => "CLEAR",
            FlagState::Void(_) => "VOID",
        }
    }
    /// Get the receipt or explanation string for this state.
    pub fn receipt(&self) -> &str {
        match self {
            FlagState::Live(r) | FlagState::Clear(r) | FlagState::Void(r) => r,
        }
    }
}

fn scan_dir_bounded(
    dir: &Path,
    needle: &str,
    exclude: &str,
    hits: &mut Vec<String>,
    depth: usize,
) -> Result<(), String> {
    const MAX_DEPTH: usize = 10;
    if depth > MAX_DEPTH {
        return Ok(());
    }

    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();

                // Skip target directories
                if path.components().any(|c| c.as_os_str() == "target") {
                    continue;
                }

                if path.is_file() {
                    let is_rs = path.extension().map(|e| e == "rs").unwrap_or(false);
                    let excluded = !exclude.is_empty()
                        && path.file_name().map(|f| f == exclude).unwrap_or(false);

                    if is_rs && !excluded {
                        if let Ok(src) = std::fs::read_to_string(&path) {
                            if src.contains(needle) {
                                hits.push(path.display().to_string());
                            }
                        }
                    }
                } else if path.is_dir() {
                    // Recurse into subdirectories with bounded depth
                    let _ = scan_dir_bounded(&path, needle, exclude, hits, depth + 1);
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("failed to read dir: {}", e)),
    }
}

fn scan_dir(root: &Path, dir: &str, needle: &str, exclude: &str) -> Result<Vec<String>, String> {
    let base = root.join(dir);
    if !base.exists() {
        return Err(format!("probe dir missing: {dir}"));
    }
    let mut hits = Vec::new();
    scan_dir_bounded(&base, needle, exclude, &mut hits, 0)?;
    Ok(hits)
}

/// Recompute one flag from disk. No caching anywhere on any path.
pub fn eval(root: &Path, spec: &FlagSpec) -> FlagState {
    match spec.probe {
        Probe::NeedleAbsentDir { dir, needle, exclude } => match scan_dir(root, dir, needle, exclude) {
            Err(e) => FlagState::Void(e),
            Ok(hits) if hits.is_empty() => {
                FlagState::Live(format!("no consumer of '{needle}' under {dir}"))
            }
            Ok(hits) => FlagState::Clear(format!("consumer: {}", hits[0])),
        },
        Probe::NeedlePresentDir { dir, needle } => match scan_dir(root, dir, needle, "") {
            Err(e) => FlagState::Void(e),
            Ok(hits) if hits.is_empty() => {
                FlagState::Clear(format!("'{needle}' gone from {dir}"))
            }
            Ok(hits) => FlagState::Live(format!("still present: {}", hits[0])),
        },
        Probe::PathMissing { path } => {
            if root.join(path).exists() {
                FlagState::Clear(format!("{path} on disk"))
            } else {
                FlagState::Live(format!("{path} missing"))
            }
        }
    }
}

/// The NEW flags — every prior prose flag re-authored as a live probe. When the
/// debt is paid the flag clears ITSELF on the next read; nothing to groom.
pub fn mmx3_flags() -> Vec<FlagSpec> {
    vec![
        // mill-live-caller: PRUNED — forge-pixel crate does not exist in v3
        // (verified: not in F:\v3\crates; mill_wrap not found in any v3 crate via grep)
        // eight-angles-host-bind: PRUNED — crates/forge-studio has no src/ dir in v3
        // (verified: forge-studio/ carries only ui/*.html, no Rust source at all)
        // mechanic-rail-sim-tick: PRUNED — crates/forge-game-systems does not exist in v3
        // (verified: not in F:\v3\crates)
        // ghostmoon-bridge-host: PRUNED — crates/forge-studio has no src/ dir in v3
        // (verified: forge-studio/ carries only ui/*.html, no Rust source at all;
        // the v2 producer repo_query.rs / consumer main.rs pairing has no v3 home yet)
    ]
}

/// Net-new watch (Sean 07-22: NET_NEW=sentinel-WATCHED, not a build-wall). Each
/// spec gauges a net-new primitive's wiring post-hoc — LIVE while unwired/missing,
/// self-clears when its consumer/module lands. Same probes, no new mechanism.
pub fn net_new_flags() -> Vec<FlagSpec> {
    vec![
        FlagSpec {
            id: "one-engine-encoded",
            debt: "root#one-engine names forge_book::one_engine (cargo xtask-harvested manifest) — module not built yet",
            probe: Probe::PathMissing { path: "crates/forge-book-v3/src/one_engine.rs" },
        },
    ]
}

/// Gauge every flag fresh. The whole system is this call — there is no store.
pub fn gauge_all(root: &Path) -> Vec<(FlagSpec, FlagState)> {
    mmx3_flags().into_iter().chain(net_new_flags())
        .map(|s| { let st = eval(root, &s); (s, st) }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join("flag_gauge_test").join(name);
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src")).unwrap();
        d
    }

    /// Falsifiable both poles: the SAME probe flips Live -> Clear when the
    /// consumer lands on disk. No stored state anywhere to go stale.
    #[test]
    fn probe_flips_when_disk_flips() {
        let root = fixture("flip");
        let spec = FlagSpec {
            id: "t",
            debt: "t",
            probe: Probe::NeedleAbsentDir { dir: "src", needle: "call_me()", exclude: "def.rs" },
        };
        std::fs::write(root.join("src/def.rs"), "pub fn call_me() {}").unwrap();
        assert!(matches!(eval(&root, &spec), FlagState::Live(_)), "definition alone is NOT a consumer");
        std::fs::write(root.join("src/user.rs"), "fn go() { call_me(); }").unwrap();
        assert!(matches!(eval(&root, &spec), FlagState::Clear(_)), "consumer clears the flag on next read");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dead_aim_goes_void_loud_never_silently_clear() {
        let root = fixture("void");
        let spec = FlagSpec {
            id: "t",
            debt: "t",
            probe: Probe::NeedleAbsentDir { dir: "no_such_dir", needle: "x", exclude: "" },
        };
        assert!(matches!(eval(&root, &spec), FlagState::Void(_)));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn present_probe_and_path_probe_gauge_both_poles() {
        let root = fixture("present");
        let stale = FlagSpec {
            id: "t",
            debt: "t",
            probe: Probe::NeedlePresentDir { dir: "src", needle: "OLD_MARK" },
        };
        std::fs::write(root.join("src/a.rs"), "// OLD_MARK").unwrap();
        assert!(matches!(eval(&root, &stale), FlagState::Live(_)));
        std::fs::write(root.join("src/a.rs"), "// clean").unwrap();
        assert!(matches!(eval(&root, &stale), FlagState::Clear(_)));
        let missing = FlagSpec { id: "t", debt: "t", probe: Probe::PathMissing { path: "src/a.rs" } };
        assert!(matches!(eval(&root, &missing), FlagState::Clear(_)));
        let gone = FlagSpec { id: "t", debt: "t", probe: Probe::PathMissing { path: "src/zz.rs" } };
        assert!(matches!(eval(&root, &gone), FlagState::Live(_)));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The REAL flags stay AIMED: none may be Void against the live repo. Their
    /// Live/Clear verdicts are deliberately NOT asserted — debt is information
    /// the gauge reports, never an expectation a test freezes.
    #[test]
    fn real_flags_are_aimed_at_living_targets() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        for (spec, state) in gauge_all(root) {
            assert!(
                !matches!(state, FlagState::Void(_)),
                "flag '{}' aim rotted: {}",
                spec.id,
                state.receipt()
            );
        }
    }
}
