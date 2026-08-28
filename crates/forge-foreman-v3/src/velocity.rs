//! L21 diff-floor circuit breaker — stateful `PreToolUse` gate.
//!
//! `-1/0/+1` as a real state machine, not a metaphor this time: `check` reads
//! `.forge/yield_state.json`'s `locked` bool and is the `PreToolUse` hook body
//! (exit 0 = allow, exit 1 = deny); `yield_now` is model-issued (sets `locked = true`,
//! only callable while unlocked); `unlock` is `UserPromptSubmit`-issued ONLY — nothing
//! else in this module can clear the lock, so once locked, no further `Edit`/`Write`
//! tool call (this module's `check`, plus any call routed through it) can succeed
//! until a genuine new user prompt fires the hook that calls `unlock`.
//!
//! **Aperture, stated plainly (C09):** scoped to `Edit`/`Write` only — the tools the
//! `PreToolUse` hook matcher names in `.claude/settings.json`. Read-only tools and
//! non-mutating PowerShell calls are NOT gated; a model can still gather context and
//! explain itself while locked, it just cannot mutate files. Locking does not stop
//! `PowerShell` generally — that is a deliberately smaller v1 scope than the original
//! handoff spec's full mutation surface, named as a gap, not hidden.
//!
//! State lives relative to CWD (`.` — hooks run with CWD = project root, same
//! convention `drift.rs`'s `beat_drift` already uses), never an absolute literal.

use std::path::{Path, PathBuf};

// No `serde` derive in this crate by design (Cargo.toml header: "no serde" —
// `serde_json` is taken bare, `Value` read/written manually, same convention
// `drift.rs`'s `beat_drift` already uses for settings.json).

fn state_path(root: &str) -> PathBuf {
    PathBuf::from(format!("{root}/.forge/yield_state.json"))
}

/// Absent or malformed state reads as unlocked — a missing file must never
/// accidentally deadlock the harness (fail open on read, fail closed only on an
/// explicit `locked: true` row).
fn read_locked(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("locked").and_then(serde_json::Value::as_bool))
        .unwrap_or(false)
}

fn write_locked(path: &Path, locked: bool) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let body = serde_json::json!({ "locked": locked });
    std::fs::write(path, serde_json::to_string_pretty(&body).expect("json! output is always serializable"))
}

/// `foreman velocity check` — the `PreToolUse` hook body. Returns the process exit
/// code the caller should use: `0` allow, `1` deny. Never panics — a read failure
/// already resolves to unlocked inside `read_locked`.
pub fn check(root: &str) -> i32 {
    if read_locked(&state_path(root)) {
        eprintln!(
            "L21 VIOLATION: a diff floor was stated (YIELD) and not yet cleared by a new \
             user turn. Blocking this Edit/Write until the next prompt arrives."
        );
        1
    } else {
        0
    }
}

/// `foreman velocity yield` — model-issued. Refuses if already locked (no
/// double-lock, and no way to use this to "renew" a lock the model itself set).
pub fn yield_now(root: &str) -> Result<(), String> {
    let path = state_path(root);
    if read_locked(&path) {
        return Err("already locked — call unlock (new user turn) before yielding again".into());
    }
    write_locked(&path, true).map_err(|e| e.to_string())?;
    eprintln!("[velocity] LOCKED — further Edit/Write blocked until the next user prompt.");
    Ok(())
}

/// `foreman velocity unlock` — `UserPromptSubmit`-issued ONLY. This is the one
/// function in this module a model-issued command must never be the sole caller of;
/// it exists to be wired to the hook that only fires on real user input.
pub fn unlock(root: &str) -> Result<(), String> {
    write_locked(&state_path(root), false).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> (tempfile_dir::TempDir, String) {
        let dir = tempfile_dir::TempDir::new();
        let root = dir.path_str();
        (dir, root)
    }

    /// Minimal scratch-dir helper — no external tempfile dep in this crate, so a
    /// tiny self-cleaning wrapper over `std::env::temp_dir()` + a unique suffix.
    mod tempfile_dir {
        use std::path::{Path, PathBuf};

        pub struct TempDir(PathBuf);

        impl TempDir {
            pub fn new() -> Self {
                let mut n = 0u64;
                loop {
                    let p = std::env::temp_dir().join(format!("velocity_test_{n}"));
                    if std::fs::create_dir(&p).is_ok() {
                        return Self(p);
                    }
                    n += 1;
                }
            }
            pub fn path_str(&self) -> String {
                self.0.to_string_lossy().into_owned()
            }
            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    /// No state file yet: fail OPEN (allow), never deadlock a fresh checkout.
    #[test]
    fn missing_state_file_checks_as_unlocked() {
        let (_dir, root) = tmp_root();
        assert_eq!(check(&root), 0);
    }

    /// The actual gate: yield locks it, and a subsequent check denies.
    #[test]
    fn yield_then_check_denies() {
        let (_dir, root) = tmp_root();
        yield_now(&root).expect("first yield must succeed");
        assert_eq!(check(&root), 1, "locked state must deny");
    }

    /// The actual release: only unlock clears it, and check allows again after.
    #[test]
    fn unlock_after_yield_restores_allow() {
        let (_dir, root) = tmp_root();
        yield_now(&root).expect("yield");
        assert_eq!(check(&root), 1);
        unlock(&root).expect("unlock");
        assert_eq!(check(&root), 0, "unlocked state must allow");
    }

    /// The closed loophole: yield cannot be called twice to "renew" — a second
    /// yield while locked is refused, so the model cannot re-lock its way around
    /// an unlock it doesn't control.
    #[test]
    fn double_yield_without_unlock_is_refused() {
        let (_dir, root) = tmp_root();
        yield_now(&root).expect("first yield");
        assert!(yield_now(&root).is_err(), "second yield while locked must be refused");
    }

    #[test]
    fn state_file_lands_under_dot_forge() {
        let (dir, root) = tmp_root();
        yield_now(&root).expect("yield");
        assert!(dir.path().join(".forge/yield_state.json").exists());
    }
}
