//! NDE sovereign weight-ladder census — encoded so it is never lost again (Sean
//! 2026-07-27). Every number is a disk readback receipt, never a size-reflex guess;
//! the probe tests re-read the artifacts, so drift goes RED instead of silent.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;
use std::path::Path;

/// Ship law (root#nde-ladder + forge-daemon#embedded-brain; Sean 07-27 "we CANNOT
/// SHIP WITH GEMMA — that is why she is a feature"): the ship bin links sovereign
/// weights only; gemma is a feature-gated out-of-process dev sidecar, never a rung.
pub const SHIP_LAW: &str =
    "gemma = dev feature, never shipped; the ladder is sovereign: student -> teacher -> master -> apex";

/// One rung of the ladder. `receipt` names HOW the numbers were read (2026-07-27):
/// safetensors = 8-byte-LE u64 length + JSON header readback (pure bytes, no torch);
/// roadmap = F:/.reposold/13engine/ROADMAP.json:10 compile receipt.
pub struct NdeRung {
    /// Tier name (e.g., "student", "teacher", "master", "apex").
    pub tier: &'static str,
    /// Path to the model artifact file.
    pub artifact: &'static str,
    /// Model size in megabytes.
    pub size_mb: u32,
    /// Parameter count in millions.
    pub params_m: u32,
    /// Vocabulary size.
    pub vocab: u32,
    /// Model dimension (embedding width).
    pub d_model: u32,
    /// How the numbers were obtained (e.g., "safetensors header" or "roadmap").
    pub receipt: &'static str,
    /// Descriptive note about the rung's role and characteristics.
    pub note: &'static str,
}

/// The ladder, coder seat included. Byte-vocab rungs (vocab 256) are
/// `gbnf_sampler`-exact — token==byte, deterministic masking — the ghost-byte-clamp
/// thesis (GHOST-BYTE-CLAMP-2026-07-07) at every scale.
pub const LADDER: &[NdeRung] = &[
    NdeRung {
        tier: "student",
        artifact: "nde-models/best_model.safetensors",
        size_mb: 91,
        params_m: 24,
        vocab: 256,
        d_model: 256,
        receipt: "safetensors header 05-14: 268 tensors F32, 7 experts x 3 layers, GQA, 7-700-7 routing",
        note: "byte-level generative ghost — sharp, deterministic, owns its vocab",
    },
    NdeRung {
        tier: "teacher (cut)",
        artifact: "nde-models/teacher-ppl108.safetensors",
        size_mb: 628,
        params_m: 157,
        vocab: 16384,
        d_model: 512,
        receipt: "safetensors header 07-27: 372 tensors F32",
        note: "ppl108 teacher extraction; live twin = nde-models/teacher.nde 185MB",
    },
    NdeRung {
        tier: "master",
        artifact: "nde-models/master-ppl108-q8.nde",
        size_mb: 513,
        params_m: 467,
        vocab: 16384,
        d_model: 512,
        receipt: "roadmap 06-16: compile-nde --quant q8, 9 experts, 20 layers, sha 4d50a0d5...",
        note: "source = nde-models/best_model.pt 1785MB torch zip (ppl108, 3 in-SoT twins); live master.nde 857MB",
    },
    NdeRung {
        tier: "apex (coder)",
        artifact: "nde-models/apex.safetensors",
        size_mb: 2927,
        params_m: 731,
        vocab: 256,
        d_model: 768,
        receipt: "safetensors header 07-27: 823 tensors F32, expert-structured (GQA k 192x768)",
        note: "byte-level giant — THE coder rung; gbnf byte-mask exact; compile-nde q8 => ~780MB owed",
    },
];

/// Read a safetensors JSON header (8-byte LE u64 length prefix, then UTF-8 JSON) —
/// the proven pure-bytes recipe. Returns up to `cap` bytes of the manifest.
pub fn safetensors_header(path: &Path, cap: usize) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut len8 = [0u8; 8];
    f.read_exact(&mut len8)?;
    let len = u64::from_le_bytes(len8).min(cap as u64) as usize;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Build the "NDE Ladder" chapter for the assembled atlas — the structured twin of
/// the 2026-07-27 deep read over nde-models/, nde-live/, lora_swarm/, .reposold, E:/.
pub fn ladder_atlas() -> Chapter {
    let mut ch = Chapter::new("NDE Ladder", AtlasSection::Custom("Architecture".into()));
    ch.add_lore(
        "The sovereign weight ladder, read back from disk 2026-07-27 — never re-derived. \
         Lineage: ppl108 best_model.pt (467M, three in-SoT twins incl. live/teacher.pt) \
         begat the teacher cut and the q8 master; apex (731M, byte vocab) is the coder \
         rung. Byte-vocab rungs mask EXACTLY through gbnf_sampler (token==byte). Gemma \
         is a feature-gated dev sidecar and never ships. Training state rides nde-live/ \
         (flywheel.jsonl + distill-queue.ndjson, feeding live) and lora_swarm/active.lora; \
         the candle-core train verb is the FLY-STUDENT -> FLY-TEACHER -> FLY-CAPACITY \
         board lane, and best_model.py's train loop is the piece still owed to the bin.",
    );
    let mut p = Page::new(1);
    p.add(Block::text(SHIP_LAW.to_string()));
    ch.add_page(p);
    let mut n: u32 = 2;
    for r in LADDER {
        let mut p = Page::new(n);
        p.add(Block::text(format!(
            "{} — {} ({}MB, ~{}M params, vocab {}, d_model {})",
            r.tier, r.artifact, r.size_mb, r.params_m, r.vocab, r.d_model
        )));
        p.add(Block::text(format!("  receipt: {}", r.receipt)));
        p.add(Block::text(format!("  {}", r.note)));
        ch.add_page(p);
        n += 1;
    }
    let mut p = Page::new(n);
    p.add(Block::text(
        "Capacity gauge (FLY-CAPACITY) — DECLARED vs EXERCISED, read from disk at press time:"
            .to_string(),
    ));
    for line in capacity_table(&crate::state_board::repo_root()) {
        p.add(Block::text(format!("  {line}")));
    }
    ch.add_page(p);
    ch
}

/// FLY-CAPACITY: one row of the capability-capacity gauge — what a tier DECLARES
/// against the disk evidence that the capability is EXERCISED (root#rank:
/// DECLARED != EXERCISED; a rung with no live evidence is a claim, not a capability).
pub struct CapacityRow {
    /// Tier name corresponding to a rung in `LADDER`.
    pub tier: &'static str,
    /// The capability the tier declares.
    pub declared: &'static str,
    /// Disk paths that provide evidence the capability is exercised.
    pub exercised_by: &'static [&'static str],
}

/// Capacity table, 1:1 with `LADDER` in tier order. Evidence paths are lanes the
/// daemon already produces/consumes: distill-queue.ndjson feeds student retraining
/// (forge-daemon flywheel_distill), flywheel.jsonl carries the pair log and
/// master-grade retrieval (flywheel_retrieve), the live `.nde` twins are the
/// deployed rungs, and apex's q8 cut is still owed to compile-nde.
pub const CAPACITY: &[CapacityRow] = &[
    CapacityRow {
        tier: "student",
        declared: "byte-exact generation, gbnf mask token==byte (vocab 256)",
        exercised_by: &["nde-models/student.nde", "nde-live/distill-queue.ndjson"],
    },
    CapacityRow {
        tier: "teacher (cut)",
        declared: "16k-vocab distillation source for the student lane",
        exercised_by: &["nde-models/teacher.nde", "nde-live/flywheel.jsonl"],
    },
    CapacityRow {
        tier: "master",
        declared: "16k-vocab q8 inference rung + master-grade pair retrieval",
        exercised_by: &["nde-models/master.nde", "nde-live/flywheel.jsonl"],
    },
    CapacityRow {
        tier: "apex (coder)",
        declared: "731M byte-vocab coder rung",
        exercised_by: &["nde-models/apex-q8.nde"],
    },
];

/// Probe the capacity table against disk — one line per tier, sizes read back at
/// call time. Zero live evidence reads `DECLARED-ONLY`; that is the finding, not
/// an error (the census tests pin the declared artifacts separately).
pub fn capacity_table(root: &Path) -> Vec<String> {
    CAPACITY
        .iter()
        .map(|row| {
            let mut live = Vec::new();
            let mut missing = Vec::new();
            for rel in row.exercised_by {
                match std::fs::metadata(root.join(rel)) {
                    Ok(m) => live.push(format!("{rel} {}MB", m.len() / 1_048_576)),
                    Err(_) => missing.push(*rel),
                }
            }
            let verdict = if live.is_empty() { "DECLARED-ONLY" } else { "EXERCISED" };
            let mut line = format!(
                "FLY-CAPACITY {}: {} {}/{} — declared: {}",
                row.tier,
                verdict,
                live.len(),
                row.exercised_by.len(),
                row.declared
            );
            if !live.is_empty() {
                line.push_str(&format!(" | live: {}", live.join(", ")));
            }
            if !missing.is_empty() {
                line.push_str(&format!(" | missing: {}", missing.join(", ")));
            }
            line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: NDE-LADDER-CENSUS]
    #[test]
    fn every_rung_is_on_disk_at_its_stated_size() {
        let root = crate::state_board::repo_root();
        for r in LADDER {
            let p = root.join(r.artifact);
            let meta =
                std::fs::metadata(&p).unwrap_or_else(|e| panic!("{} ABSENT: {e}", r.artifact));
            let mb = (meta.len() / 1_048_576) as i64;
            let tol = (r.size_mb as i64 / 20).max(2);
            assert!(
                (mb - r.size_mb as i64).abs() <= tol,
                "{} drifted: disk {}MB vs census {}MB — weights changed, update the census",
                r.artifact,
                mb,
                r.size_mb
            );
        }
    }

    // [BOARD: NDE-LADDER-CENSUS]
    #[test]
    fn byte_rungs_readback_vocab_and_width_from_disk() {
        let root = crate::state_board::repo_root();
        for (art, vocab, d) in [
            ("nde-models/apex.safetensors", 256u32, 768u32),
            ("nde-models/teacher-ppl108.safetensors", 16384, 512),
        ] {
            let h = safetensors_header(&root.join(art), 400_000).expect("header readback");
            assert!(
                h.contains(&format!("[{vocab},{d}]")),
                "{art}: embed shape [{vocab},{d}] not found in live header"
            );
        }
    }

    // [BOARD:FLY-CAPACITY]
    #[test]
    fn capacity_table_gauges_declared_vs_exercised_from_disk() {
        assert_eq!(CAPACITY.len(), LADDER.len(), "capacity table drifted from the ladder census");
        for (c, l) in CAPACITY.iter().zip(LADDER) {
            assert_eq!(c.tier, l.tier, "tier order drifted from the ladder census");
            assert!(!c.exercised_by.is_empty(), "{}: no evidence lane named", c.tier);
        }
        let lines = capacity_table(&crate::state_board::repo_root());
        for l in &lines {
            println!("{l}");
        }
        let exercised = lines.iter().filter(|l| l.contains(": EXERCISED ")).count();
        assert!(
            exercised >= 3,
            "flywheel dead — {exercised}/4 rungs exercised; live .nde twins or nde-live logs are gone"
        );
    }
}
