//! `foreman witness <scenario>` — the CLI wire for `forge-witness-v3`.
//!
//! Same shape as [`crate::gate`]: a thin verb that runs a real check and
//! returns `Err` (loud HALT, non-zero exit) on any failure. Owns argument
//! parsing only — every actual capture/inject/diff decision lives in
//! `forge-witness-v3`, one home (L05).

use std::path::PathBuf;

/// `foreman witness <scenario>|--all [--tolerance N] [--bless]` — build shell,
/// launch it, run the scenario(s), diff against stored baselines.
pub fn verb(root: &PathBuf, args: &[String]) -> Result<(), String> {
    let all = args.iter().any(|a| a == "--all");
    let bless = args.iter().any(|a| a == "--bless");
    let tolerance: u8 = args
        .iter()
        .position(|a| a == "--tolerance")
        .and_then(|i| args.get(i + 1))
        .map(|v| v.parse::<u8>().map_err(|e| format!("--tolerance: {e}")))
        .transpose()?
        .unwrap_or(forge_witness_v3::DEFAULT_TOLERANCE);
    // A bare positional scenario name: skip every flag AND the value that
    // immediately follows a value-taking flag (`--root <dir>`, `--tolerance
    // <n>`) — a string-equality guess against the root path is fragile
    // across path-formatting differences, this position-based skip is not.
    let mut name: Option<String> = None;
    let mut skip_next = false;
    for a in &args[1..] {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a == "--root" || a == "--tolerance" {
            skip_next = true;
            continue;
        }
        if a.starts_with("--") {
            continue;
        }
        name = Some(a.clone());
        break;
    }

    let names: Vec<String> = if all {
        forge_witness_v3::all_scenarios().iter().map(|s| s.name().to_string()).collect()
    } else {
        vec![name.ok_or("witness: need a scenario name or --all")?]
    };

    let mut failures = Vec::new();
    for n in &names {
        match forge_witness_v3::run_named(root, n, tolerance, bless) {
            Ok(receipt) => println!("WITNESS  {receipt}"),
            Err(e) => {
                eprintln!("WITNESS RED  {n}: {e}");
                failures.push(n.clone());
            }
        }
    }
    if !failures.is_empty() {
        return Err(format!("witness: {} scenario(s) failed: {}", failures.len(), failures.join(", ")));
    }
    Ok(())
}
