//! The fail-closed brief queue: `.forge/brief-queue/<crate>/`.
//!
//! HANDOFF §11: on red or drift the foreman **queues a brief** for a later
//! `claude -p` drainer. It does not ping an attended human — overnight nobody
//! is there, and a fail path that waits on a person is an open loop that
//! stops silently. A queue survives the night.
//!
//! One directory per crate. `BRIEF.md` is the human/Claude-readable statement
//! of what failed and what was tried; the final failing draft's files sit
//! beside it so the drainer reads evidence, not a summary of evidence.

use std::path::{Path, PathBuf};

use crate::land::DraftFile;

/// The queue directory for one crate.
pub fn queue_dir(root: &Path, name: &str) -> PathBuf {
    root.join(".forge").join("brief-queue").join(name)
}

/// One failed attempt's record: which try it was and the gate's uncut output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    /// 1-based attempt number.
    pub number: u32,
    /// The gate's stdout+stderr for this attempt, whole.
    pub gate_output: String,
}

/// Write the brief for a crate that exhausted its retry budget. Replaces any
/// standing brief for the same crate — the newest evidence is the brief; the
/// tape, not the queue, is the historian.
pub fn enqueue(
    root: &Path,
    name: &str,
    task: &str,
    attempts: &[Attempt],
    last_draft: &[DraftFile],
) -> Result<PathBuf, String> {
    let dir = queue_dir(root, name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;

    let mut brief = String::new();
    brief.push_str(&format!(
        "# BRIEF — {name} went red past the retry budget\n\n\
         The foreman's task, verbatim:\n\n{task}\n\n\
         {} attempt(s) were made; each gate run's uncut output follows. The final\n\
         failing draft sits beside this file under `draft/`. Fix the draft (or rule\n\
         the row condemned) and set the census row back to `pending` to re-enter\n\
         the loop.\n",
        attempts.len()
    ));
    for a in attempts {
        brief.push_str(&format!("\n## Attempt {}\n\n```text\n{}\n```\n", a.number, a.gate_output));
    }
    std::fs::write(dir.join("BRIEF.md"), brief).map_err(|e| e.to_string())?;

    for f in last_draft {
        let p = dir.join("draft").join(&f.rel_path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, &f.body).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The M2 DONE clause's second half, in miniature: a forced red must land
    /// in the queue with its evidence, not vanish.
    #[test]
    fn a_forced_red_lands_in_the_queue_with_its_evidence() {
        let root = std::env::temp_dir().join(format!("foreman-q-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();

        let attempts = vec![
            Attempt { number: 1, gate_output: "error[E0425]: cannot find value".into() },
            Attempt { number: 2, gate_output: "error[E0308]: mismatched types".into() },
        ];
        let draft = vec![DraftFile { rel_path: "src/lib.rs".into(), body: "broken".into() }];
        let dir = enqueue(&root, "forge-x-v3", "port the thing", &attempts, &draft).unwrap();

        let brief = std::fs::read_to_string(dir.join("BRIEF.md")).unwrap();
        assert!(brief.contains("E0425") && brief.contains("E0308"), "every attempt's output");
        assert!(brief.contains("port the thing"), "the task travels with the failure");
        let body = std::fs::read_to_string(dir.join("draft").join("src/lib.rs")).unwrap();
        assert_eq!(body, "broken", "the failing draft is evidence, preserved whole");
        std::fs::remove_dir_all(&root).ok();
    }
}
