//! The M2 loop: census item → sidecar draft → gate → tape.
//!
//! One call of [`run_once`] moves one census row to a terminal state:
//! `green` (promoted, stamped, on the tape) or `queued` (red past the retry
//! budget, brief in the queue). [`run_all`] repeats until the census has
//! nothing actionable. The loop is deterministic — every branch is a measured
//! verdict (a gate exit code, a parse refusal), never a judgment call.

use std::path::Path;

use forge_vcs_v3::spine::{Lane, ReceiptKind, SourceKind};
use forge_vcs_v3::{Stamp, VcsRoot};

use crate::census::{Census, Row, Status};
use crate::client::Sidecar;
use crate::directives::Directives;
use crate::gate;
use crate::land::{self, DraftFile};
use crate::queue::{self, Attempt};

/// How much v2 reference source a brief may carry. Past this the brief states
/// the truncation aloud rather than silently thinning the reference.
const V2_SOURCE_CAP: usize = 200_000;

/// How much of a red gate's output rides into the retry prompt.
const RETRY_OUTPUT_CAP: usize = 6_000;

/// The provenance stamp for gated sidecar source: it rode Speculative until
/// the gate spoke; it lands PriorAuthority with the gate's green as receipt.
const DRAFT_STAMP: Stamp = Stamp {
    lane: Lane::PriorAuthority,
    source_kind: SourceKind::LLMCandidate,
    receipt_kind: ReceiptKind::Compile,
};

/// The stamp for foreman-scaffolded build files (manifests, member edits):
/// machine-generated, not drafted — `AOTCompiled`, receipted by the same gate.
const SCAFFOLD_STAMP: Stamp = Stamp {
    lane: Lane::PriorAuthority,
    source_kind: SourceKind::AOTCompiled,
    receipt_kind: ReceiptKind::Compile,
};

/// The census-flip stamp: `Promote` is the receipt for a row changing state
/// (MIGRATION §LANE DELEGATION); the subject of the promotion is LLM work.
const PROMOTE_STAMP: Stamp = Stamp {
    lane: Lane::PriorAuthority,
    source_kind: SourceKind::LLMCandidate,
    receipt_kind: ReceiptKind::Promote,
};

/// Where a crate's live grind transcript lands: `.forge/grind-log/<name>.md`,
/// append-only. This is ARCH000's window into the sidecar's work (ruling
/// 2026-08-09: "anything output is speculative to me" — so the speculation is
/// journaled where an eye and the HUD can watch it, not held in a socket).
pub fn grind_log_path(root: &Path, name: &str) -> std::path::PathBuf {
    root.join(".forge").join("grind-log").join(format!("{name}.md"))
}

/// Append one headed section to the crate's grind transcript. The foreman
/// owns all fs I/O, so the journal lives here and never in the sidecar. A
/// transcript that cannot be written is a loud error — an invisible grind is
/// the defect this journal exists to kill.
fn journal(root: &Path, name: &str, heading: &str, body: &str) -> Result<(), String> {
    use std::io::Write as _;
    let p = grind_log_path(root, name);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
        .map_err(|e| format!("grind journal unwritable at {}: {e}", p.display()))?;
    writeln!(f, "\n## [{ts}] {heading}\n\n{body}").map_err(|e| e.to_string())
}

/// What one loop iteration did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The row travelled the whole way: staged green, promoted, root gate
    /// green, stamped rows on the tape, census flipped.
    Green {
        /// The crate that landed.
        name: String,
        /// Repo-relative paths committed, in commit order.
        committed: Vec<String>,
    },
    /// Red past the retry budget; the brief queue holds the evidence and the
    /// census row reads `queued`.
    Queued {
        /// The crate that went red.
        name: String,
    },
    /// The census has no actionable row — the loop is done, not stuck.
    Drained,
}

/// Run one census item to a terminal state. `Err` means the loop itself is
/// blocked (no census, sidecar down, tape refused) — a state the fail path
/// cannot queue around and must report loudly.
pub fn run_once(root: &Path, d: &Directives) -> Result<Outcome, String> {
    let mut census = Census::load(root)?;
    let Some(row) = census.next_actionable().cloned() else {
        return Ok(Outcome::Drained);
    };

    let sidecar = Sidecar::at(&d.endpoint)?.with_timeout_s(d.reply_timeout_s);
    sidecar
        .status()
        .map_err(|e| format!("blocked: sidecar STATUS failed before any work was taken: {e}"))?;

    let task = brief_for(&row);
    let task_head: String = task.chars().take(2_000).collect();
    journal(root, &row.name, &format!("TAKEN — {} ({})", row.name, row.disposition.as_str()),
        &format!("{task_head}\n\n(v2 reference source rides the brief in full; head shown here)"))?;
    let mut attempts: Vec<Attempt> = Vec::new();
    let mut last_draft: Vec<DraftFile> = Vec::new();

    for n in 1..=d.retry_max {
        let prompt = match attempts.last() {
            None => task.clone(),
            Some(prev) => retry_prompt(&task, &prev.gate_output),
        };

        // Every failure inside an attempt is evidence for the next one, not a
        // loop abort: the sidecar being wrong is the expected case.
        let verdict = attempt(root, &row, &sidecar, d, &prompt, n, &mut last_draft);
        match verdict {
            Ok(committed) => {
                census.set_status(&row.name, Status::Green)?;
                census.store(root)?;
                let mut all = committed;
                all.push(commit_census(root)?);
                journal(root, &row.name, "GREEN — on the tape", &all.join("\n"))?;
                return Ok(Outcome::Green { name: row.name, committed: all });
            }
            Err(output) => {
                journal(root, &row.name, &format!("ATTEMPT {n} RED"), &tail(&output, RETRY_OUTPUT_CAP))?;
                attempts.push(Attempt { number: n, gate_output: output });
            }
        }
    }

    // Fail-closed: the queue survives the night (HANDOFF §11).
    journal(root, &row.name, "QUEUED — retry budget spent",
        "brief and evidence at .forge/brief-queue/; census row flipped to queued")?;
    queue::enqueue(root, &row.name, &task, &attempts, &last_draft)?;
    census.set_status(&row.name, Status::Queued)?;
    census.store(root)?;
    let _ = commit_census(root)?;
    Ok(Outcome::Queued { name: row.name })
}

/// Repeat [`run_once`] until the census drains. Returns every outcome in
/// order, so the caller can report the run as rows, not as a feeling.
pub fn run_all(root: &Path, d: &Directives) -> Result<Vec<Outcome>, String> {
    let mut outcomes = Vec::new();
    loop {
        let o = run_once(root, d)?;
        let done = o == Outcome::Drained;
        outcomes.push(o);
        if done {
            return Ok(outcomes);
        }
    }
}

/// One draft → stage → gate → (green) promote → root gate → commit pass.
/// `Ok` carries the committed paths; `Err` carries the evidence for the retry
/// prompt and the brief queue.
fn attempt(
    root: &Path,
    row: &Row,
    sidecar: &Sidecar,
    d: &Directives,
    prompt: &str,
    n: u32,
    last_draft: &mut Vec<DraftFile>,
) -> Result<Vec<String>, String> {
    journal(root, &row.name, &format!("ATTEMPT {n} — INFER sent"), "waiting on the sidecar…")?;
    // Attempt 1 is greedy; retries sample (seeded from the prompt hash, so a
    // retry is still deterministic). Greedy retries were measured
    // byte-identical across DIFFERING retry prompts (forge-intent-v3,
    // 2026-08-10) — the run lane's copy of the weld lane's rut.
    let reply = if n == 1 {
        sidecar.infer(prompt)
    } else {
        sidecar.infer_t(prompt, d.retry_temp_pmy, d.retry_top_p_pmy)
    }
    .map_err(|e| format!("(no gate run) infer failed: {e}"))?;
    journal(root, &row.name, &format!("ATTEMPT {n} — sidecar replied ({} bytes)", reply.len()), &reply)?;
    land_reply(root, &row.name, d, &reply, &format!("ATTEMPT {n}"), last_draft)
}

/// The reply-to-tape half of an attempt, shared by the sidecar loop and the
/// delegate lane ([`land_external`]): parse, customs lint, stage, stage gate,
/// promote, root gate, stamped commits. `Err` carries the retry evidence.
fn land_reply(
    root: &Path,
    name: &str,
    d: &Directives,
    reply: &str,
    label: &str,
    last_draft: &mut Vec<DraftFile>,
) -> Result<Vec<String>, String> {
    let files = land::extract_files(reply)
        .map_err(|e| format!("(no gate run) reply did not meet the FILE contract: {e}"))?;
    *last_draft = files.clone();

    // The customs lint: an illegal import is refused before any gate run,
    // and the retry prompt carries the ACTION, not just E0432's symptom.
    let foreign = land::foreign_imports(&files);
    if !foreign.is_empty() {
        return Err(format!(
            "(no gate run) the draft imports crate(s) this workspace does not allow: {}. \
             Allowed imports beyond core/alloc/std: {}. Delete the illegal import(s) and \
             hand-roll that behavior in plain Rust; resend EVERY file in full.",
            foreign.join(", "),
            land::DRAFT_DEPS
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }

    let stage_dir = land::stage(root, name, &files)?;
    let staged = gate::run(&d.gate, name, &stage_dir)?;
    journal(root, name,
        &format!("{label} — stage gate {}", if staged.green { "GREEN" } else { "RED" }),
        &tail(&staged.output, RETRY_OUTPUT_CAP))?;
    if !staged.green {
        return Err(staged.output);
    }

    // Green in staging: promote into the workspace and prove it again where
    // it will actually live — stage-green predicts root-green, it is not it.
    let written = land::promote(root, name, &files)?;
    let rooted = gate::run(&d.gate, name, root)?;
    if !rooted.green {
        land::demote(root, name)?;
        return Err(format!(
            "stage gate was green but the root gate went red — draft demoted.\n{}",
            rooted.output
        ));
    }

    // Both gates green: the tape records the travel, stamped truthfully.
    let vcs = VcsRoot::open(root.join(".forge").join("vcs")).map_err(|e| e.to_string())?;
    for rel in &written {
        let stamp = if rel.ends_with(".rs") { DRAFT_STAMP } else { SCAFFOLD_STAMP };
        let bytes = std::fs::read(root.join(rel)).map_err(|e| format!("read {rel}: {e}"))?;
        vcs.commit_bytes_stamped(rel, &bytes, stamp)
            .map_err(|e| format!("tape refused {rel}: {e}"))?;
    }
    Ok(written)
}

/// Land an externally produced draft — the L11 delegate lane (ARCH000
/// 2026-08-10: forge-tui via `claude -p --model haiku` after the brief proved
/// structurally over the sidecar's card). Same travel as a sidecar attempt —
/// parse, lint, stage, gate, promote, root gate, stamped rows, census flip,
/// journal — with the reply read from a file instead of a socket. The stamps
/// stay truthful: an LLM drafted it, the gate is the receipt.
pub fn land_external(
    root: &Path,
    d: &Directives,
    name: &str,
    draft_path: &Path,
) -> Result<Outcome, String> {
    let mut census = Census::load(root)?;
    if !census.rows.iter().any(|r| r.name == name) {
        return Err(format!("census has no row named {name:?} — the delegate lane lands census rows only"));
    }
    let reply = std::fs::read_to_string(draft_path)
        .map_err(|e| format!("cannot read draft {}: {e}", draft_path.display()))?;
    journal(root, name,
        &format!("DELEGATE — external draft received ({} bytes)", reply.len()),
        &format!("source file: {}\n\n{reply}", draft_path.display()))?;

    let mut last_draft: Vec<DraftFile> = Vec::new();
    match land_reply(root, name, d, &reply, "DELEGATE", &mut last_draft) {
        Ok(committed) => {
            census.set_status(name, Status::Green)?;
            census.store(root)?;
            let mut all = committed;
            all.push(commit_census(root)?);
            journal(root, name, "GREEN — on the tape (delegate lane)", &all.join("\n"))?;
            Ok(Outcome::Green { name: name.to_string(), committed: all })
        }
        Err(output) => {
            journal(root, name, "DELEGATE RED — draft refused", &tail(&output, RETRY_OUTPUT_CAP))?;
            Err(format!("delegate draft for {name} went red: {}", tail(&output, RETRY_OUTPUT_CAP)))
        }
    }
}

/// Commit the census file itself with the Promote stamp, returning its path key.
fn commit_census(root: &Path) -> Result<String, String> {
    let key = ".forge/census.tsv".to_string();
    let vcs = VcsRoot::open(root.join(".forge").join("vcs")).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(root.join(&key)).map_err(|e| e.to_string())?;
    vcs.commit_bytes_stamped(&key, &bytes, PROMOTE_STAMP)
        .map_err(|e| format!("tape refused the census flip: {e}"))?;
    Ok(key)
}

/// The grind brief: the reply contract, the tree's law as it binds a draft,
/// and the v2 reference source (capped, truncation stated aloud).
fn brief_for(row: &Row) -> String {
    let mut b = String::new();
    b.push_str(&format!(
        "You are the syntax grinder for the 13forge v3 migration. {} the v2 crate \
         below as a new small Rust crate named `{}`.\n\
         Reply format, exactly: for each file, a line `// FILE: src/<name>.rs` followed by a \
         ```rust fenced code block with the complete file. `src/lib.rs` is required. \
         Do NOT write Cargo.toml or anything outside src/.\n\
         Rules the gate enforces: every module starts with //! doc lines and every pub item \
         has /// docs (missing_docs is deny). No `unsafe`. The ONLY external dependencies \
         allowed are: {deps} (already declared in the manifest the foreman writes). \
         No f32/f64 — integer arithmetic only. Include at least one #[test] fn that proves \
         real behavior. Zero warnings.\n\
         Task note: {}\n\n",
        match row.disposition {
            crate::census::Disposition::Rewrite => "Rewrite",
            _ => "Port",
        },
        row.name,
        row.note,
        deps = allowed_deps(),
    ));
    b.push_str("V2 REFERENCE SOURCE (read-only):\n");
    b.push_str(&v2_source_of(row));
    // Recency is the mechanism (measured 2026-08-10: the model parrots
    // whatever it read last, and the reference source above is ~10-50k tokens
    // of "last"). The binding constraints are restated HERE, after the
    // reference, so they are the most recent thing before the reply begins.
    b.push_str(&format!(
        "\nEND OF V2 REFERENCE. Everything above the END marker is v2 REFERENCE ONLY — \
         do not copy it where the task note demands change.\n\
         The task note, restated and binding: {}\n\
         Allowed imports beyond core/alloc/std: {} — nothing else. No f32/f64. No unsafe. \
         //! module docs and /// on every pub item. At least one #[test]. \
         Reply NOW in the exact // FILE format stated at the top. Do NOT repeat or echo \
         any of these instructions — your reply is code files only.\n",
        row.note,
        allowed_deps(),
    ));
    b
}

/// The approved draft-dependency roots, comma-joined for brief prose.
fn allowed_deps() -> String {
    land::DRAFT_DEPS.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ")
}

/// The last `cap` bytes of a gate's output, on a char boundary.
fn tail(s: &str, cap: usize) -> String {
    if s.len() <= cap {
        return s.to_string();
    }
    let mut at = s.len() - cap;
    while !s.is_char_boundary(at) {
        at += 1;
    }
    s[at..].to_string()
}

/// The retry brief: the standing task plus the gate's own words.
fn retry_prompt(task: &str, gate_output: &str) -> String {
    let tail = tail(gate_output, RETRY_OUTPUT_CAP);
    format!(
        "{task}\n\nYour previous draft FAILED the gate. Compiler/test output:\n\
         ```text\n{tail}\n```\n\
         Fix the defects and resend EVERY file in full, same // FILE format.\n"
    )
}

/// Gather the v2 crate's `Cargo.toml` and `src/**.rs` under the byte cap.
fn v2_source_of(row: &Row) -> String {
    let mut out = String::new();
    let push_file = |label: &str, path: &Path, out: &mut String| {
        if out.len() >= V2_SOURCE_CAP {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            let room = V2_SOURCE_CAP - out.len();
            if text.len() > room {
                out.push_str(&format!("--- {label} (TRUNCATED at {room} bytes) ---\n"));
                out.push_str(&text[..room]);
            } else {
                out.push_str(&format!("--- {label} ---\n{text}\n"));
            }
        }
    };

    push_file("Cargo.toml", &row.v2_path.join("Cargo.toml"), &mut out);
    let src = row.v2_path.join("src");
    let mut rs: Vec<_> = std::fs::read_dir(&src)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    rs.sort();
    for p in rs {
        let label = format!("src/{}", p.file_name().unwrap_or_default().to_string_lossy());
        push_file(&label, &p, &mut out);
    }
    if out.is_empty() {
        out.push_str(&format!(
            "(no v2 source found at {} — draft from the task note alone)\n",
            row.v2_path.display()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::census;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::path::PathBuf;

    /// A scratch root carrying everything the loop touches: a members list, a
    /// one-row census, a v2 reference dir, and room for `.forge/vcs`.
    fn fixture_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("foreman-run-{tag}-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        std::fs::create_dir_all(root.join(".forge")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\n    \"crates/existing\",\n]\n",
        )
        .unwrap();

        let v2 = root.join("v2-ref");
        std::fs::create_dir_all(v2.join("src")).unwrap();
        std::fs::write(v2.join("src").join("lib.rs"), "pub fn two() -> u8 { 2 }\n").unwrap();

        let census_text = format!(
            "forge-tiny-v3\t{}\tport\tpending\tproposed: fixture\n",
            v2.display()
        );
        std::fs::write(root.join(".forge").join("census.tsv"), census_text).unwrap();
        root
    }

    /// A fake sidecar answering every connection with `reply`, forever.
    fn fake_sidecar(reply: &'static str) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            for conn in l.incoming() {
                let Ok(mut s) = conn else { continue };
                let mut len = [0u8; 4];
                if s.read_exact(&mut len).is_err() {
                    continue;
                }
                let mut buf = vec![0u8; u32::from_be_bytes(len) as usize];
                if s.read_exact(&mut buf).is_err() {
                    continue;
                }
                let _ = s.write_all(&(reply.len() as u32).to_be_bytes());
                let _ = s.write_all(reply.as_bytes());
                let _ = s.flush();
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        });
        let _ = TcpStream::connect(&addr); // absorb nothing; just ensure it's up
        addr
    }

    const GOOD_DRAFT: &str = "// FILE: src/lib.rs\n\
        ```rust\n\
        //! Fixture draft.\n\
        /// Two.\n\
        pub fn two() -> u8 { 2 }\n\
        ```\n";

    /// The whole M2 travel, deterministically: census -> draft -> gate ->
    /// promotion -> stamped tape rows -> census flip. The gate command is a
    /// real spawned process; only its verdict is scripted.
    #[test]
    fn a_row_travels_census_to_tape_and_lands_stamped_llm_candidate() {
        let root = fixture_root("green");
        let d = Directives {
            endpoint: fake_sidecar(GOOD_DRAFT),
            nde_endpoint: "127.0.0.1:13018".into(),
            gate: "cmd /c exit 0".into(),
            retry_max: 3,
            reply_timeout_s: 600,
            sip_cap_bytes: 2048,
            sip_anchor_context_lines: 8,
            retry_temp_pmy: 1500,
            retry_top_p_pmy: 9000,
            weld_pairs_path: ".forge/distill/weld-pairs.ndjson".into(),
            bqr_path: ".forge/distill/router.bqr".into(),
            journal_enabled: true,
        };

        let out = run_once(&root, &d).unwrap();
        let Outcome::Green { name, committed } = out else { panic!("wanted Green, got {out:?}") };
        assert_eq!(name, "forge-tiny-v3");

        // Promoted into the workspace…
        assert!(root.join("crates/forge-tiny-v3/src/lib.rs").exists());
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(manifest.contains("\"crates/forge-tiny-v3\""), "member registered");

        // …stamped truthfully on the tape…
        let vcs = VcsRoot::open(root.join(".forge/vcs")).unwrap();
        let rows = vcs.log_all().unwrap();
        let lib = rows.iter().find(|r| r.path.ends_with("src/lib.rs")).expect("lib.rs row");
        assert_eq!(lib.source_kind, SourceKind::LLMCandidate);
        assert_eq!(lib.receipt_kind, ReceiptKind::Compile);
        let census_row = rows.iter().find(|r| r.path == ".forge/census.tsv").expect("census row");
        assert_eq!(census_row.receipt_kind, ReceiptKind::Promote);
        assert!(committed.iter().any(|p| p == "Cargo.toml"), "root manifest edit committed");

        // …the grind was journaled where an eye can watch it…
        let log = std::fs::read_to_string(grind_log_path(&root, "forge-tiny-v3")).unwrap();
        assert!(log.contains("ATTEMPT 1 — sidecar replied"), "the reply is on the record");
        assert!(log.contains("GREEN — on the tape"), "and so is the landing");

        // …and the census reads green, so the loop reports Drained next.
        let c = Census::load(&root).unwrap();
        assert_eq!(c.rows[0].status, Status::Green);
        assert_eq!(run_once(&root, &d).unwrap(), Outcome::Drained);
        std::fs::remove_dir_all(&root).ok();
    }

    /// The DONE clause's other half: a forced red exhausts the retry budget
    /// and lands in the brief queue instead of vanishing — and the workspace
    /// never learns the red draft's name.
    #[test]
    fn a_forced_red_exhausts_retries_and_lands_in_the_brief_queue() {
        let root = fixture_root("red");
        let d = Directives {
            endpoint: fake_sidecar(GOOD_DRAFT),
            nde_endpoint: "127.0.0.1:13018".into(),
            gate: "cmd /c exit 1".into(),
            retry_max: 2,
            reply_timeout_s: 600,
            sip_cap_bytes: 2048,
            sip_anchor_context_lines: 8,
            retry_temp_pmy: 1500,
            retry_top_p_pmy: 9000,
            weld_pairs_path: ".forge/distill/weld-pairs.ndjson".into(),
            bqr_path: ".forge/distill/router.bqr".into(),
            journal_enabled: true,
        };

        let out = run_once(&root, &d).unwrap();
        assert_eq!(out, Outcome::Queued { name: "forge-tiny-v3".into() });

        let qdir = crate::queue::queue_dir(&root, "forge-tiny-v3");
        let brief = std::fs::read_to_string(qdir.join("BRIEF.md")).unwrap();
        assert!(brief.contains("2 attempt(s)"), "the budget was spent, then queued: {brief}");
        assert!(qdir.join("draft/src/lib.rs").exists(), "the failing draft is preserved");

        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
        assert!(!manifest.contains("forge-tiny-v3"), "a red draft never joins the workspace");
        assert_eq!(Census::load(&root).unwrap().rows[0].status, Status::Queued);
        assert_eq!(run_once(&root, &d).unwrap(), Outcome::Drained, "queued rows are not re-taken");
        std::fs::remove_dir_all(&root).ok();
    }

    /// The delegate lane travels the same road as a sidecar attempt: an
    /// external draft file lands staged, gated, promoted, stamped
    /// LLMCandidate, and flips the census — with no sidecar involved at all
    /// (the endpoint below answers nothing).
    #[test]
    fn an_external_draft_lands_through_the_delegate_lane() {
        let root = fixture_root("delegate");
        let d = Directives {
            endpoint: "127.0.0.1:1".into(), // nothing listens; the lane must not care
            nde_endpoint: "127.0.0.1:13018".into(),
            gate: "cmd /c exit 0".into(),
            retry_max: 3,
            reply_timeout_s: 600,
            sip_cap_bytes: 2048,
            sip_anchor_context_lines: 8,
            retry_temp_pmy: 1500,
            retry_top_p_pmy: 9000,
            weld_pairs_path: ".forge/distill/weld-pairs.ndjson".into(),
            bqr_path: ".forge/distill/router.bqr".into(),
            journal_enabled: true,
        };
        let draft = root.join("delegate-draft.md");
        std::fs::write(&draft, GOOD_DRAFT).unwrap();

        let out = land_external(&root, &d, "forge-tiny-v3", &draft).unwrap();
        let Outcome::Green { name, .. } = out else { panic!("wanted Green, got {out:?}") };
        assert_eq!(name, "forge-tiny-v3");
        assert!(root.join("crates/forge-tiny-v3/src/lib.rs").exists());

        let vcs = VcsRoot::open(root.join(".forge/vcs")).unwrap();
        let rows = vcs.log_all().unwrap();
        let lib = rows.iter().find(|r| r.path.ends_with("src/lib.rs")).expect("lib.rs row");
        assert_eq!(lib.source_kind, SourceKind::LLMCandidate, "an LLM drafted it — the stamp says so");
        assert_eq!(Census::load(&root).unwrap().rows[0].status, Status::Green);

        let log = std::fs::read_to_string(grind_log_path(&root, "forge-tiny-v3")).unwrap();
        assert!(log.contains("DELEGATE — external draft received"), "the travel is journaled");
        assert!(log.contains("GREEN — on the tape (delegate lane)"));

        assert!(
            land_external(&root, &d, "no-such-row", &draft).is_err(),
            "the lane lands census rows only"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// Blocked is loud, not queued: with no sidecar listening, the loop
    /// refuses before taking any work.
    #[test]
    fn a_dead_sidecar_blocks_the_loop_before_work_is_taken() {
        let root = fixture_root("dead");
        let d = Directives {
            endpoint: "127.0.0.1:1".into(), // nothing listens on port 1
            nde_endpoint: "127.0.0.1:13018".into(),
            gate: "cmd /c exit 0".into(),
            retry_max: 1,
            reply_timeout_s: 600,
            sip_cap_bytes: 2048,
            sip_anchor_context_lines: 8,
            retry_temp_pmy: 1500,
            retry_top_p_pmy: 9000,
            weld_pairs_path: ".forge/distill/weld-pairs.ndjson".into(),
            bqr_path: ".forge/distill/router.bqr".into(),
            journal_enabled: true,
        };
        let e = run_once(&root, &d).unwrap_err();
        assert!(e.contains("blocked"), "got: {e}");
        assert_eq!(
            Census::load(&root).unwrap().rows[0].status,
            census::Status::Pending,
            "no work was taken, so nothing was flipped"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_brief_names_the_contract_and_carries_the_v2_source() {
        let root = fixture_root("brief");
        let row = Census::load(&root).unwrap().rows[0].clone();
        let b = brief_for(&row);
        assert!(b.contains("// FILE: src/"), "reply contract stated");
        assert!(b.contains("pub fn two()"), "v2 reference rides along");
        assert!(b.contains("missing_docs"), "the law the gate enforces is named");
        assert!(b.contains("bytemuck"), "the approved dep set is named");
        // The recency fix (2026-08-10): the binding task note must be the
        // most recent statement — restated AFTER the reference source.
        let source_at = b.find("V2 REFERENCE SOURCE").expect("reference block");
        let restated_at = b.rfind("restated and binding").expect("post-reference restatement");
        assert!(
            restated_at > source_at && b[restated_at..].contains(&row.note),
            "the task note must bind after the reference, not only before it"
        );
        let retry = retry_prompt(&b, "error[E0999]: fixture");
        assert!(retry.contains("E0999"), "the gate's own words reach the retry");
        std::fs::remove_dir_all(&root).ok();
    }
}
