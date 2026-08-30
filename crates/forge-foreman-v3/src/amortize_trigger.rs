//! Amortize fire-and-forget trigger: spawn subprocess, return immediately.
//!
//! Runs `cargo xtask pkm amortize` as a detached background process on entry/exit.
//! Zero latency penalty: spawn returns before subprocess completes. No waiting.

use std::path::Path;
use std::process::Command;

/// Trigger amortize session capture: spawn background process, fire and forget.
/// Returns immediately (does not wait for subprocess to complete).
/// Amortize runs offline, capturing agent work into the corpus.
pub fn trigger_amortize(root: &Path, session_summary: &Path) -> Result<(), String> {
    let summary_path = session_summary.to_string_lossy().to_string();
    let root_str = root.to_string_lossy().to_string();

    // Spawn detached subprocess: foreman does not wait for it.
    // The subprocess inherits stdout/stderr but is not joined.
    let _child = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args(&[
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &format!(
                    "cd '{}' && cargo xtask pkm amortize '{}' 2>$null",
                    root_str, summary_path
                ),
            ])
            .spawn()
            .map_err(|e| format!("amortize spawn failed: {e}"))?
    } else {
        Command::new("sh")
            .args(&[
                "-c",
                &format!(
                    "cd '{}' && cargo xtask pkm amortize '{}' 2>/dev/null",
                    root_str, summary_path
                ),
            ])
            .spawn()
            .map_err(|e| format!("amortize spawn failed: {e}"))?
    };

    // Intentionally drop child: subprocess runs detached, we do not wait.
    // The subprocess inherits pipes and runs to completion independently.
    drop(_child);
    Ok(())
}

/// Trigger TTL zeroization sweep and compaction: spawn background process.
/// Runs `cargo xtask pkm archive` to decay, prune, and compact the corpus.
pub fn trigger_ttl_sweep(root: &Path) -> Result<(), String> {
    let root_str = root.to_string_lossy().to_string();

    let _child = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args(&[
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &format!("cd '{}' && cargo xtask pkm archive 2>$null", root_str),
            ])
            .spawn()
            .map_err(|e| format!("ttl sweep spawn failed: {e}"))?
    } else {
        Command::new("sh")
            .args(&[
                "-c",
                &format!("cd '{}' && cargo xtask pkm archive 2>/dev/null", root_str),
            ])
            .spawn()
            .map_err(|e| format!("ttl sweep spawn failed: {e}"))?
    };

    drop(_child);
    Ok(())
}
