//! Hook wiring drift detection.
//! Ported from forge-daemon/harness_config.rs (lines 150-250).
//!
//! Audit expected wiring (from settings.json) vs actual hooks declared.
//! MISSING = compiled but not wired on disk (the gate is dead).
//! EXTRA = wired on disk with no compiled row (an unowned second truth).

use std::collections::BTreeSet;
use std::path::Path;

/// Default hooks wiring table (AUTHORED, W10b closure; event renamed 2026-08-12;
/// REWIRED 2026-08-16 — Sean: "the faster we can get rid of all ambient ps1
/// scripts the better"). The five `.ps1` rows (foreman-gate-snapshot,
/// foreman-witness-snapshot, foreman-gate, foreman-witness, foreman-gate-report)
/// plus the two non-foreman scripts (phase0-gate.ps1, root-check-gate.ps1) are
/// replaced by the compiled `foreman hook <event>` verb (`hook.rs`, same crate):
/// - SessionEnd → foreman hook session-end: measured workspace-gate beat
/// - UserPromptSubmit → foreman drift: audit hook configuration each prompt
/// - PreToolUse/Write|Edit → foreman hook pre-edit: L25 phase-zero gate (armed loops only)
/// - PreToolUse/PowerShell → foreman hook pre-shell: L18 source-write bypass block
/// - PostToolUse/Write|Edit → foreman hook post-edit: record touched crate/witness scope
/// - Stop → foreman hook stop: gate the turn's touched work ONCE (no auto-revert)
///
/// The per-edit build+revert died on purpose: path-keyed snapshots with no
/// session identity destroyed three crates and zeroed 64KB of aspire.rs under
/// concurrent sessions (foreman-gate.ps1's own receipts). Turn-granular
/// verification at Stop replaces it; RED is reported, never enforced by
/// mutating files the hook does not own.
///
/// "SessionEnd", not "SessionStop": the harness schema has no SessionStop event
/// (settings.json validation receipt, 2026-08-12).
///
/// FALLBACK ONLY (Sean 2026-08-19: "hardcoded list baked into the binary" —
/// editing wiring doctrine used to require a recompile+redeploy of
/// .forge/bin/foreman.exe before drift would see it). `load_expected_hooks`
/// below reads `.forge/hook-manifest.tsv` at runtime instead; this const is
/// the safety net when that file is absent/empty/unparseable, so a missing
/// manifest fails loud (every row MISSING → FAIL) rather than silently PASSing
/// on zero expectations.
pub const DEFAULT_EXPECTED_HOOKS: &[(&str, &str, &str)] = &[
    ("SessionEnd", "", "door-hook session-end"),
    ("UserPromptSubmit", "", "door-hook drift"),
    ("PreToolUse", "Write|Edit", "door-hook pre-edit"),
    ("PreToolUse", "PowerShell", "door-hook pre-shell"),
    ("PostToolUse", "Write|Edit", "door-hook post-edit"),
    ("Stop", "", "door-hook stop"),
];

/// Load the expected-hooks table from `<root>/.forge/hook-manifest.tsv`
/// (tab-separated `event\tmatcher\tverb`, `#`-prefixed comment lines and
/// blank lines skipped) — data, not code, per the config-parity law: edit
/// the manifest and `foreman drift` picks it up next run, no rebuild. Falls
/// back to `DEFAULT_EXPECTED_HOOKS` if the manifest is missing or every line
/// in it is empty/malformed, so a broken manifest is loud, not a false PASS.
pub fn load_expected_hooks(root: &Path) -> Vec<(String, String, String)> {
    let manifest_path = root.join(".forge/hook-manifest.tsv");
    let mut rows = Vec::new();
    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split('\t');
            let event = parts.next().unwrap_or("").trim().to_string();
            let matcher = parts.next().unwrap_or("").trim().to_string();
            let verb = parts.next().unwrap_or("").trim().to_string();
            if !event.is_empty() && !verb.is_empty() {
                rows.push((event, matcher, verb));
            }
        }
    }
    if rows.is_empty() {
        DEFAULT_EXPECTED_HOOKS
            .iter()
            .map(|(e, m, v)| (e.to_string(), m.to_string(), v.to_string()))
            .collect()
    } else {
        rows
    }
}

/// A verb is wired when every one of its whitespace tokens appears in the
/// disk command. Token containment, not string equality: a real hook row
/// carries an exe path and flags around the verb ("F:/v3/.forge/bin/
/// foreman.exe hook pre-edit --root F:/v3"). Aperture: "foreman" also matches
/// inside "foreman.exe" — intended. The event-name token ("pre-edit" vs
/// "pre-grep" vs "post-edit") is what separates sibling hook rows; none is a
/// substring of another, so rows cannot satisfy each other.
fn verb_wired(command: &str, verb: &str) -> bool {
    verb.split_whitespace().all(|token| command.contains(token))
}

/// Drift rows, sorted and deduped.
/// (event, matcher, verb) triples.
/// MISSING = an expected verb with no disk command containing it on that event.
/// EXTRA = a disk row that invokes foreman but matches no expected verb —
/// only foreman-owned wiring is policed; foreign hooks are not this audit's
/// to judge (stated aperture, C09).
pub fn detect_drift(
    disk_wiring: &[(String, String, String)],
    compiled_wiring: &[(String, String, String)],
) -> Vec<String> {
    let mut rows: BTreeSet<String> = BTreeSet::new();

    for (event, matcher, verb) in compiled_wiring {
        let wired = disk_wiring
            .iter()
            .any(|(de, _dm, dc)| de == event && verb_wired(dc, verb));
        if !wired {
            rows.insert(format!("MISSING {event}/{matcher} -> {verb}"));
        }
    }

    for (de, dm, dc) in disk_wiring {
        let ours = dc.contains("foreman");
        let expected = compiled_wiring
            .iter()
            .any(|(event, _m, verb)| event == de && verb_wired(dc, verb));
        if ours && !expected {
            rows.insert(format!("EXTRA {de}/{dm} -> {dc}"));
        }
    }

    rows.into_iter().collect()
}

/// PASS = zero drift rows.
pub fn verdict(drift_rows: usize) -> &'static str {
    if drift_rows == 0 {
        "PASS"
    } else {
        "FAIL"
    }
}

/// Real `ron` parse of `CLAUDE.md`'s fenced ```ron block. `None` when the
/// file is missing or has no fence (not this crate's concern); `Some(Err)`
/// carries the actual `ron` parser error text (line/col) on a broken file.
pub fn check_claude_ron(root: &Path) -> Option<Result<usize, String>> {
    let text = std::fs::read_to_string(root.join("CLAUDE.md")).ok()?;
    let start = text.find("```ron")? + "```ron".len();
    let end = text[start..].find("```")? + start;
    let body = &text[start..end];
    match ron::from_str::<ron::Value>(body) {
        Ok(ron::Value::Map(m)) => Some(Ok(m.len())),
        Ok(_) => Some(Ok(0)),
        Err(e) => Some(Err(format!("{e}"))),
    }
}

/// L25 arm-state lines: .loop-active age (STALE past 60min) and current.json
/// presence — file-stat only, proof_command is never run here. The `bool` is
/// true when BOTH are healthy, so a caller can stay silent on an ordinary turn.
fn arm_state(root: &Path) -> (String, bool) {
    let mut s = String::new();
    let mut ok = true;
    let flag = root.join(".claude").join("hooks").join(".loop-active");
    match std::fs::metadata(&flag).and_then(|m| m.modified()) {
        Ok(t) => {
            let mins = t.elapsed().map(|a| a.as_secs() / 60).unwrap_or(0);
            if mins > 60 {
                ok = false;
                s.push_str(&format!("ARM .loop-active:STALE({mins}min>60) L25 gate disarmed\n"));
            } else {
                s.push_str(&format!("ARM .loop-active:live({mins}min)\n"));
            }
        }
        Err(_) => {
            ok = false;
            s.push_str("ARM .loop-active:absent (L25 gate not armed)\n");
        }
    }
    let cur = root.join(".claude").join("hooks").join(".phase0").join("current.json");
    if cur.is_file() {
        s.push_str("ARM phase0:current.json present\n");
    } else {
        ok = false;
        s.push_str("ARM phase0:current.json MISSING\n");
    }
    (s, ok)
}

/// The drift report plus whether every gate was green. `green` exists so the
/// door can take `door_hook.rs`'s documented silent path on an ordinary turn
/// instead of spending four lines of every agent's context saying nothing.
/// The text is built identically either way — the record is never suppressed,
/// only its per-turn display.
pub struct DriftReport {
    /// The full multi-line report, exactly as `run_report` returns it.
    pub text: String,
    /// True when there is no drift, no staleness, the RON parsed, and L25 is armed.
    pub green: bool,
}

/// Full `foreman drift` report as one string: drift rows, staleness line (if
/// stale), then the verdict line last — the same order `beat_drift` in
/// `main.rs` printed, factored out so `door.rs`'s `hook_drift` verb (2026-08-21)
/// can produce the identical report without a `foreman.exe` subprocess.
pub fn run_report(root: &Path) -> String {
    run_report_full(root).text
}

/// [`run_report`] plus the green flag — see [`DriftReport`].
pub fn run_report_full(root: &Path) -> DriftReport {
    let settings_path = root.join(".claude/settings.json");
    let mut disk_wiring: Vec<(String, String, String)> = Vec::new();
    if let Ok(content) = std::fs::read_to_string(&settings_path) {
        if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(events) = settings.get("hooks").and_then(|h| h.as_object()) {
                for (event, entries) in events {
                    for entry in entries.as_array().into_iter().flatten() {
                        let matcher = entry
                            .get("matcher")
                            .and_then(|m| m.as_str())
                            .unwrap_or("")
                            .to_string();
                        for cmd in entry
                            .get("hooks")
                            .and_then(|h| h.as_array())
                            .into_iter()
                            .flatten()
                        {
                            if let Some(command) = cmd.get("command").and_then(|c| c.as_str()) {
                                disk_wiring.push((event.clone(), matcher.clone(), command.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    let expected = load_expected_hooks(root);
    let drift_rows = detect_drift(&disk_wiring, &expected);

    let mut out = String::new();
    for row in &drift_rows {
        out.push_str(row);
        out.push('\n');
    }
    let stale = crate::staleness::check(root);
    if let Some(msg) = &stale {
        out.push_str(msg);
        out.push('\n');
    }
    let ron_check = check_claude_ron(root);
    let ron_failed = matches!(ron_check, Some(Err(_)));
    match ron_check {
        Some(Ok(n)) => out.push_str(&format!("CLAUDE.md ron:PROVEN({n} keys)\n")),
        Some(Err(e)) => out.push_str(&format!("CLAUDE.md ron:FAIL({e})\n")),
        None => {}
    }
    let (arm, armed) = arm_state(root);
    out.push_str(&arm);
    // Staleness counts toward the verdict here (2026-08-28) so the door no
    // longer has to re-check it and splice PASS->FAIL by hand — that second
    // pass printed the same STALE BINARY line twice.
    let verdict_rows =
        drift_rows.len() + usize::from(ron_failed) + usize::from(stale.is_some());
    out.push_str(&format!("DRIFT verdict:{}", verdict(verdict_rows)));
    DriftReport {
        text: out,
        green: drift_rows.is_empty() && stale.is_none() && !ron_failed && armed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_drift_no_diff() {
        let compiled = vec![
            ("SessionStop".to_string(), "".to_string(), "foreman beat PASS --green 10 --red 0 --unwired 0".to_string()),
        ];
        let disk = compiled.clone();
        let drift = detect_drift(&disk, &compiled);
        assert_eq!(drift.len(), 0);
    }

    #[test]
    fn test_detect_drift_missing() {
        let compiled = vec![
            ("SessionStop".to_string(), "".to_string(), "foreman beat PASS --green 10 --red 0 --unwired 0".to_string()),
        ];
        let disk = vec![];
        let drift = detect_drift(&disk, &compiled);
        assert_eq!(drift.len(), 1);
        assert!(drift[0].starts_with("MISSING"));
    }

    #[test]
    fn test_detect_drift_extra() {
        let compiled = vec![];
        let disk = vec![
            ("SessionStop".to_string(), "".to_string(), "foreman some_old_verb".to_string()),
        ];
        let drift = detect_drift(&disk, &compiled);
        assert_eq!(drift.len(), 1);
        assert!(drift[0].starts_with("EXTRA"));
    }

    #[test]
    fn test_detect_drift_ignores_foreign_hooks() {
        // Aperture: non-foreman wiring is not this audit's to judge.
        let compiled = vec![];
        let disk = vec![
            ("SessionStop".to_string(), "".to_string(), "some_old_hook".to_string()),
        ];
        let drift = detect_drift(&disk, &compiled);
        assert_eq!(drift.len(), 0);
    }

    #[test]
    fn test_detect_drift_real_wiring_matches_by_tokens() {
        // A real hook row carries an exe path around the verb; token
        // containment must still count it as wired.
        let compiled: Vec<(String, String, String)> = DEFAULT_EXPECTED_HOOKS
            .iter()
            .map(|(e, m, v)| (e.to_string(), m.to_string(), v.to_string()))
            .collect();
        let disk = vec![(
            "UserPromptSubmit".to_string(),
            "".to_string(),
            "cargo run --quiet -p xtask -- door-hook drift".to_string(),
        )];
        let drift = detect_drift(&disk, &compiled);
        // SessionEnd, PreToolUse x2, PostToolUse, Stop still missing;
        // the drift row itself is wired.
        assert_eq!(drift.len(), 5);
        assert!(drift.iter().any(|r| r.starts_with("MISSING SessionEnd") && r.contains("door-hook session-end")));
        assert!(drift.iter().any(|r| r.starts_with("MISSING PreToolUse") && r.contains("door-hook pre-edit")));
        assert!(drift.iter().any(|r| r.starts_with("MISSING PreToolUse") && r.contains("door-hook pre-shell")));
        assert!(drift.iter().any(|r| r.starts_with("MISSING PostToolUse") && r.contains("door-hook post-edit")));
        assert!(drift.iter().any(|r| r.starts_with("MISSING Stop") && r.contains("door-hook stop")));
    }

    #[test]
    fn test_sibling_hook_rows_cannot_satisfy_each_other() {
        // A wired pre-edit row must not count as the pre-shell or post-edit row.
        let compiled: Vec<(String, String, String)> = DEFAULT_EXPECTED_HOOKS
            .iter()
            .map(|(e, m, v)| (e.to_string(), m.to_string(), v.to_string()))
            .collect();
        let disk = vec![(
            "PreToolUse".to_string(),
            "Write|Edit".to_string(),
            "cargo run --quiet -p xtask -- door-hook pre-edit --root F:/v3".to_string(),
        )];
        let drift = detect_drift(&disk, &compiled);
        // 5 of 6 expected rows still missing; the disk row is wired, not EXTRA.
        assert_eq!(drift.len(), 5);
        assert!(!drift.iter().any(|r| r.contains("door-hook pre-edit")));
        assert!(drift.iter().any(|r| r.contains("door-hook pre-shell")));
        assert!(drift.iter().any(|r| r.contains("door-hook post-edit")));
    }

    #[test]
    fn test_detect_drift_event_must_match() {
        // The right verb on the wrong event is still MISSING.
        let compiled: Vec<(String, String, String)> = DEFAULT_EXPECTED_HOOKS
            .iter()
            .map(|(e, m, v)| (e.to_string(), m.to_string(), v.to_string()))
            .collect();
        let disk = vec![(
            "SessionStop".to_string(),
            "".to_string(),
            "foreman.exe drift".to_string(),
        )];
        let drift = detect_drift(&disk, &compiled);
        // All 6 expected rows missing, plus the misplaced row is EXTRA.
        assert_eq!(drift.len(), 7);
        assert!(drift.iter().any(|r| r.starts_with("EXTRA SessionStop")));
    }

    #[test]
    fn test_verdict_pass() {
        assert_eq!(verdict(0), "PASS");
    }

    #[test]
    fn test_verdict_fail() {
        assert_eq!(verdict(1), "FAIL");
    }

    #[test]
    fn test_detect_drift_with_expected_hooks() {
        // Simulate the expected hooks from the compiled wiring table
        let compiled: Vec<(String, String, String)> = DEFAULT_EXPECTED_HOOKS
            .iter()
            .map(|(e, m, v)| (e.to_string(), m.to_string(), v.to_string()))
            .collect();

        // Simulate an empty disk wiring (no hooks section in settings.json)
        let disk: Vec<(String, String, String)> = Vec::new();

        let drift = detect_drift(&disk, &compiled);

        // Should report 6 MISSING rows (SessionEnd, UserPromptSubmit,
        // PreToolUse x2, PostToolUse, Stop)
        assert_eq!(drift.len(), 6);
        assert!(drift.iter().all(|r| r.starts_with("MISSING")));
        assert!(drift.iter().any(|r| r.contains("SessionEnd")));
        assert!(drift.iter().any(|r| r.contains("UserPromptSubmit")));
        assert!(drift.iter().any(|r| r.contains("PreToolUse")));
        assert!(drift.iter().any(|r| r.contains("PostToolUse")));
        assert!(drift.iter().any(|r| r.contains("MISSING Stop/")));
    }

    #[test]
    fn test_drift_verdict_with_missing_hooks() {
        // 2 missing hooks should be FAIL verdict
        assert_eq!(verdict(2), "FAIL");
    }

    #[test]
    fn test_manifest_matches_default_expected_hooks() {
        // The const must never drift from .forge/hook-manifest.tsv.
        // This test reads the real manifest at repo root and asserts it matches
        // the fallback constant, so a missing manifest fails loud with expected values.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manifest_hooks = load_expected_hooks(&root);
        let const_hooks: Vec<(String, String, String)> = DEFAULT_EXPECTED_HOOKS
            .iter()
            .map(|(e, m, v)| (e.to_string(), m.to_string(), v.to_string()))
            .collect();
        assert_eq!(
            manifest_hooks, const_hooks,
            "DEFAULT_EXPECTED_HOOKS is stale; update it to match .forge/hook-manifest.tsv"
        );
    }
}
