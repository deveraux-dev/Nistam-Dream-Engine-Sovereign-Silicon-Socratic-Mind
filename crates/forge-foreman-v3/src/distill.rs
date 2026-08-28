//! The distill and route verbs for the flywheel (W5).
//!
//! `distill` harvests the weld journal into a baked binary-quantized router.
//! `route` queries the router to find the best specialist for a prompt.

use forge_foreman_v3::directives;
use forge_foreman_v3::flywheel::{self, WeldPair, Verdict};
use forge_ml_bqrouter::{BqRouter, TrainingPair, embed_prompt, specialist_of, specialist_of_text};
use std::path::{Path, PathBuf};

/// Convert a verdict to an outcome permyriad score.
fn verdict_to_outcome(v: Verdict) -> i32 {
    match v {
        Verdict::Green => 10_000,
        Verdict::GateRed => 3_000,
        Verdict::ParseRefused => 2_000,
        Verdict::EngineRefused => 5_000,
    }
}

/// Build training pairs from journal rows.
fn pairs_from_rows(rows: &[WeldPair]) -> Vec<TrainingPair> {
    let mut pairs = Vec::new();
    for row in rows {
        // Skip resolution rows (attempt == 0) and empty prompts
        if row.attempt == 0 || row.prompt.is_empty() {
            continue;
        }
        // Only include rows where specialist_of succeeds
        if let Some(sid) = specialist_of(&row.crate_name) {
            let outcome = verdict_to_outcome(row.verdict);
            let query_i8 = embed_prompt(&row.prompt).to_vec();
            pairs.push(TrainingPair { specialist_id: sid as u8, outcome_permyriad: outcome, query_i8 });
        }
    }
    pairs
}

/// Gold-corpus outcome for harvested instruction pairs — curated corpus rows
/// are unconditionally positive training signal (v2 precedent: distill_pairs.rs
/// ingested mined gold at MASTER_GRADE_TIER).
const GOLD_OUTCOME: i32 = 10_000;

/// Harvest one JSONL text into training pairs. Accepts `instruction` (NDE
/// corpus) or `query` (mined pairs) as the routed text; classifies via the
/// Hermetic-7 text floor. Returns `(routed, unrouted, bad)` row counts.
fn harvest_jsonl(text: &str, pairs: &mut Vec<TrainingPair>) -> (usize, usize, usize) {
    let (mut routed, mut unrouted, mut bad) = (0usize, 0usize, 0usize);
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                bad += 1;
                continue;
            }
        };
        let q = v
            .get("instruction")
            .or_else(|| v.get("query"))
            .and_then(|x| x.as_str());
        match q {
            None => bad += 1,
            Some(q) => match specialist_of_text(q) {
                None => unrouted += 1,
                Some(sid) => {
                    routed += 1;
                    pairs.push(TrainingPair {
                        specialist_id: sid,
                        outcome_permyriad: GOLD_OUTCOME,
                        query_i8: embed_prompt(q).to_vec(),
                    });
                }
            },
        }
    }
    (routed, unrouted, bad)
}

/// `distill --pairs <file|dir>`: harvest instruction/query JSONL corpora
/// (referenced in place — quarry data is read, never copied) into the baked
/// router. Text-floor classification ONLY: journal rows (crate-name taxonomy)
/// are deliberately excluded so two id spaces never mix in one `.bqr` — see
/// `specialist_of_text`'s taxonomy note.
fn run_pairs(root: &PathBuf, src: &Path) -> Result<(), String> {
    let d = directives::load(root)?;
    // Flat, bounded listing — a file is itself, a dir is its *.jsonl children
    // (one level, no recursion: forbidden_ops.unbound_io).
    let files: Vec<PathBuf> = if src.is_dir() {
        let mut fs: Vec<PathBuf> = std::fs::read_dir(src)
            .map_err(|e| format!("distill --pairs: cannot list {}: {e}", src.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "jsonl").unwrap_or(false))
            .collect();
        fs.sort();
        fs
    } else {
        vec![src.to_path_buf()]
    };
    if files.is_empty() {
        return Err(format!("distill --pairs: no .jsonl under {}", src.display()));
    }

    let mut pairs = Vec::new();
    let (mut routed, mut unrouted, mut bad) = (0usize, 0usize, 0usize);
    for f in &files {
        let text = std::fs::read_to_string(f)
            .map_err(|e| format!("distill --pairs: cannot read {}: {e}", f.display()))?;
        let (r, u, b) = harvest_jsonl(&text, &mut pairs);
        routed += r;
        unrouted += u;
        bad += b;
    }

    let mut r = BqRouter::new(512);
    r.train_from_pairs(&pairs);
    let bqr = PathBuf::from(&d.bqr_path);
    let bqr = if bqr.is_absolute() { bqr } else { root.join(bqr) };
    if let Some(parent) = bqr.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("distill: cannot create {}: {e}", parent.display()))?;
    }
    r.save(&bqr).map_err(|e| e.to_string())?;
    eprintln!(
        "[distill --pairs] files={} routed={routed} unrouted={unrouted} bad={bad} active={} per_expert={:?} -> {}",
        files.len(),
        r.active_count(),
        r.per_expert_counts(),
        bqr.display()
    );
    Ok(())
}

/// The `distill` verb: harvest the weld journal into a baked router, or with
/// `--pairs <file|dir>` harvest an instruction/query JSONL corpus instead.
pub fn run(root: &PathBuf, args: &[String]) -> Result<(), String> {
    if let Some(at) = args.iter().position(|a| a == "--pairs") {
        let src = args.get(at + 1).ok_or("distill --pairs takes a file or directory of .jsonl")?;
        return run_pairs(root, Path::new(src));
    }
    let d = directives::load(root)?;
    let journal = flywheel::journal_path(root, &d);
    let rows = flywheel::load(&journal)?;

    // Build training pairs
    let pairs = pairs_from_rows(&rows);

    // Train and save the router
    let mut r = BqRouter::new(512);
    r.train_from_pairs(&pairs);
    let bqr = std::path::PathBuf::from(&d.bqr_path);
    if let Some(parent) = bqr.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("distill: cannot create {}: {e}", parent.display()))?;
    }
    r.save(&bqr).map_err(|e| e.to_string())?;

    // Report to stderr
    eprintln!("[distill] pairs={} active={} per_expert={:?}", pairs.len(), r.active_count(), r.per_expert_counts());
    Ok(())
}

/// The `route` verb: query the router to find the best specialist for a prompt.
pub fn route(root: &PathBuf, args: &[String]) -> Result<(), String> {
    let d = directives::load(root)?;

    // Extract the prompt from args after the "route" verb
    let route_idx = args.iter().position(|a| a == "route")
        .ok_or("route verb not found in args")?;
    let prompt_args = &args[route_idx + 1..];
    let prompt = prompt_args.join(" ");
    if prompt.is_empty() {
        return Err("route: give a prompt".to_string());
    }

    // Load the router and route the query
    let r = BqRouter::load(&PathBuf::from(&d.bqr_path), 512).map_err(|e| e.to_string())?;
    match r.route(&embed_prompt(&prompt)) {
        Some((sid, margin)) => eprintln!("[route] specialist={sid} margin={margin}"),
        None => eprintln!("[route] no active centroids yet"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_from_rows_skips_resolution_rows() {
        let s = flywheel::session_of("test");
        let rows = vec![
            flywheel::attempt_row(s, 1, "forge-book", "fix this", "code", Verdict::GateRed, "error"),
            flywheel::resolution(s, "forge-book", "green code"),
        ];
        let pairs = pairs_from_rows(&rows);
        // Only the attempt row should produce a pair
        assert_eq!(pairs.len(), 1, "resolution rows are skipped");
        assert_eq!(pairs[0].specialist_id, 6); // forge-book should map to id 6
    }

    #[test]
    fn pairs_from_rows_filters_by_specialist() {
        let s = flywheel::session_of("test");
        let rows = vec![
            flywheel::attempt_row(s, 1, "forge-book", "fix vcs", "code", Verdict::GateRed, "error"),
            flywheel::attempt_row(s, 2, "unknown-crate", "fix unknown", "code", Verdict::GateRed, "error"),
        ];
        let pairs = pairs_from_rows(&rows);
        // Only the forge-book row should produce a pair
        assert_eq!(pairs.len(), 1, "rows for unknown crates are skipped");
    }

    #[test]
    fn verdict_to_outcome_maps_all_verdicts() {
        let verdicts = vec![
            (Verdict::Green, 10_000),
            (Verdict::GateRed, 3_000),
            (Verdict::ParseRefused, 2_000),
            (Verdict::EngineRefused, 5_000),
        ];
        for (v, expected) in verdicts {
            let outcome = verdict_to_outcome(v);
            assert_eq!(outcome, expected, "verdict {:?} maps to {expected}", v);
        }
    }

    #[test]
    fn harvest_jsonl_routes_skips_and_counts() {
        let text = concat!(
            r#"{"instruction": "crossfade the audio stems at matched bpm", "response": "x"}"#, "\n",
            r#"{"query": "the render pass binds a wgpu shader", "kit": "y"}"#, "\n",
            r#"{"instruction": "quantum knitting", "response": "z"}"#, "\n",
            "not json at all\n",
            r#"{"response": "no routed text field"}"#, "\n",
        );
        let mut pairs = Vec::new();
        let (routed, unrouted, bad) = harvest_jsonl(text, &mut pairs);
        assert_eq!((routed, unrouted, bad), (2, 1, 2));
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].specialist_id, 0); // audio
        assert_eq!(pairs[1].specialist_id, 1); // render
        assert!(pairs.iter().all(|p| p.outcome_permyriad == GOLD_OUTCOME));
    }

    #[test]
    fn all_specialist_ids_in_pairs_are_valid() {
        let s = flywheel::session_of("test");
        let rows = vec![
            flywheel::attempt_row(s, 1, "forge-book", "fix", "code", Verdict::Green, ""),
            flywheel::attempt_row(s, 2, "forge-tui", "fix", "code", Verdict::GateRed, ""),
            flywheel::attempt_row(s, 3, "forge-render", "fix", "code", Verdict::ParseRefused, ""),
        ];
        let pairs = pairs_from_rows(&rows);
        assert!(!pairs.is_empty(), "specialist crates produce pairs");
        for p in &pairs {
            assert!(p.specialist_id < 7, "all specialist_id < 7");
        }
    }
}
