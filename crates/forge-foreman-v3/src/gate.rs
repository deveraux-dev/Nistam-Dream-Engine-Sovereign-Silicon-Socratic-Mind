//! Running `pipeline.gate` and reading its verdict.
//!
//! The gate is the measurement (L02): the sidecar's draft is proven by
//! `cargo test`, never by reading it. The command string comes from the
//! directives verbatim with `{crate}` substituted; the foreman adds nothing
//! to it and never truncates its pipeline (SESSION-HANDOFF lesson: capture,
//! then filter).

use std::path::Path;
use std::process::Command;

/// One gate run's full result. `output` is stdout+stderr interleaved by
/// stream, captured whole — a red's evidence is the retry prompt's input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// Did the gate exit 0.
    pub green: bool,
    /// Combined stdout then stderr, uncut.
    pub output: String,
}

/// Substitute `{crate}` and run the gate in `cwd`. A gate that cannot spawn is
/// an error distinct from a red — a missing cargo must not read as a failing
/// draft.
pub fn run(gate_cmd: &str, krate: &str, cwd: &Path) -> Result<Verdict, String> {
    let cmd = gate_cmd.replace("{crate}", krate);
    let mut parts = cmd.split_whitespace();
    let program = parts.next().ok_or("pipeline.gate is empty")?;
    let args: Vec<&str> = parts.collect();

    let out = Command::new(program)
        .args(&args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("gate {cmd:?} failed to spawn in {}: {e}", cwd.display()))?;

    let mut output = String::from_utf8_lossy(&out.stdout).into_owned();
    output.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(Verdict { green: out.status.success(), output })
}

/// Run a gate command with an isolated target directory to avoid conflicts
/// with the running executable. Sets CARGO_TARGET_DIR env var.
fn run_isolated(gate_cmd: &str, cwd: &std::path::Path, target: &std::path::Path) -> Result<Verdict, String> {
    let mut parts = gate_cmd.split_whitespace();
    let program = parts.next().ok_or("gate cmd empty")?;
    let args: Vec<&str> = parts.collect();
    let out = Command::new(program)
        .args(&args)
        .current_dir(cwd)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .map_err(|e| format!("gate {gate_cmd:?} failed to spawn: {e}"))?;
    let mut output = String::from_utf8_lossy(&out.stdout).into_owned();
    output.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(Verdict { green: out.status.success(), output })
}

/// `foreman gate --crate <name> | --workspace --root <path>` — the hardened gate ladder,
/// code-enforced: build must succeed with ZERO warnings, then tests must be green. Any
/// failure returns Err (loud, HALT). Uses isolated target dir to avoid conflicts.
pub fn verb(root: &std::path::PathBuf, args: &[String]) -> Result<(), String> {
    let ws = args.iter().any(|a| a == "--workspace");
    let krate = args.iter().position(|a| a == "--crate").and_then(|i| args.get(i + 1)).cloned();
    if !ws && krate.is_none() {
        return Err("gate: need --crate <name> or --workspace".into());
    }
    let (build_cmd, test_cmd, label) = if ws {
        ("cargo build --workspace".to_string(), "cargo test --workspace".to_string(), "workspace".to_string())
    } else {
        let k = krate.unwrap();
        (format!("cargo build -p {k}"), format!("cargo test -p {k}"), k)
    };
    let target = root.join("target").join("gate").join(&label);
    let b = run_isolated(&build_cmd, root.as_path(), &target)?;
    if !b.green {
        return Err(format!("[gate] {label} BUILD FAILED\n{}", tail(&b.output)));
    }
    let warns: Vec<&str> = b.output.lines().filter(|l| l.contains("warning:")).collect();
    if !warns.is_empty() {
        return Err(format!("[gate] {label} has WARNINGS (zero-warnings law):\n{}", warns.join("\n")));
    }
    let t = run_isolated(&test_cmd, root.as_path(), &target)?;
    if !t.green {
        return Err(format!("[gate] {label} TESTS RED\n{}", tail(&t.output)));
    }
    eprintln!("[gate] {label} build=clean test=green");
    Ok(())
}

fn tail(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(20);
    lines[start..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate's verdict is the exit code, and both directions are observed
    /// through a real spawned process, not a stubbed bool.
    #[test]
    fn green_and_red_are_the_exit_code_not_a_feeling() {
        let cwd = std::env::temp_dir();
        let ok = run("cmd /c exit 0", "unused", &cwd).unwrap();
        assert!(ok.green);
        let red = run("cmd /c exit 1", "unused", &cwd).unwrap();
        assert!(!red.green);
    }

    #[test]
    fn the_crate_placeholder_is_substituted() {
        let cwd = std::env::temp_dir();
        let v = run("cmd /c echo gate-for-{crate}", "forge-x-v3", &cwd).unwrap();
        assert!(v.green);
        assert!(v.output.contains("gate-for-forge-x-v3"), "got: {}", v.output);
    }

    #[test]
    fn an_unspawnable_gate_is_an_error_not_a_red() {
        let cwd = std::env::temp_dir();
        assert!(run("no-such-program-exists-here", "x", &cwd).is_err());
    }
}
