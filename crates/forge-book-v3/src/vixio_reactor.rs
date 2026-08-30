//! Session 2026-07-27 — the gemma lane driven green, and the VIXIO decision that
//! came out of watching it cost context. Two halves, one page: what landed with a
//! receipt, and what Sean ruled next (`[PLANNED]`, no receipt, never read as done).

use crate::atlas::{AtlasSection, CapabilityEntry, CapabilityStatus};
use crate::chapter::Chapter;

/// What this session actually proved, receipt = the command that gated it.
pub fn session_2026_07_27_gemma() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "moe_train alloc fold — train_epoch's per-token backward() (12MB allocate-and-zero at 3M params, one core pegged, GPU idle) replaced by backward_into on a hoisted buffer; both accum.flush().to_vec() sites fold into one reused clip buffer",
            S::Capabilities,
            "forge-ml/src/moe_train.rs:501,504,514 · -p forge-ml lib 624 green (floor 608)",
        ),
        CapabilityEntry::proven(
            "NDE student checkpoint — the flywheel's first trained artifact; the 25-min CPU stall was the allocation, an epoch now runs ~65s (1.6M params, vocab 256 d=64 experts=7 layers=3)",
            S::Learning,
            "nde-models/student-distill.safetensors 6.4MB · loss 5.5452 init -> 3.9711 (e1) -> 3.2256 (e9) · 13forge-studio nde-train --resume",
        ),
        CapabilityEntry::proven(
            "gemma sliding-window mask — local layers (5 of every 6) were attending the WHOLE kv cache; arch.sliding_window was parsed and never read. window_first() is now the one bound both forwards call: the dequant reference AND ResidentGemma::forward, which is the live lane gemma_infer answers on",
            S::Capabilities,
            "forge-daemon/src/gemma_gpu.rs window_first + is_global · window_tests 1/1 · gemma_fwd_parity 2/2",
        ),
        CapabilityEntry::proven(
            "gemma window parity past the window — 1088 tokens over a 1024 window, 34 layers: our incremental GPU loop lands the same final-position argmax as candle's batched masked forward. Below the window window_first returns 0 and a masked layer is indistinguishable from an unmasked one, so only this length can referee the fix",
            S::Capabilities,
            "forge-daemon/tests/gemma_fwd_parity.rs::gemma_fwd_window_parity_past_the_window · --ignored --release · 1 passed 791.16s",
        ),
        CapabilityEntry::proven(
            "gemma context cap reads the weights — the 131072 ceiling was a hardcoded constant, right only for the 4b that happened to be on disk (a 1b would have left it 4x wide, silently). native_context() now reads gemma3.context_length off the gguf header, cached; FORGE_GEMMA_CONTEXT can still lower it, never raise it past the model",
            S::Capabilities,
            "forge-daemon/src/gemma_engine.rs::native_context · -p forge-daemon lib 470 green",
        ),
    ]
}

/// Findings that outlive the code they came from — each one cost a wrong assumption
/// to buy, and each would be re-bought by the next session that assumes otherwise.
pub fn session_2026_07_27_findings() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::new(
            "candle is NOT an oracle for incremental decode — quantized_gemma3.rs:447 passes mask None whenever seq_len == 1, so candle's own step-by-step decode ignores the sliding window entirely. Only its BATCHED path builds the mask (:152-156). Anything checking a long-context decode against candle step-decode is checking against a wrong answer",
            S::Learning,
            CapabilityStatus::Proven,
            "candle-transformers-0.10.2 quantized_gemma3.rs:447 vs :152 · read on disk 2026-07-27",
        ),
        CapabilityEntry::new(
            "the window span is window+1 — candle masks j only when j + window < i, so distance == window is KEPT. An off-by-one here is invisible under the window and wrong past it, which is the exact shape of bug that ships",
            S::Learning,
            CapabilityStatus::Proven,
            "quantized_gemma3.rs:156 · gemma_gpu window_tests asserts 5000 - window_first(5000,false,1024) == 1025",
        ),
        CapabilityEntry::new(
            "the .grain spill is the anti-pattern — grep_roots over ~8KB writes a pointer file the reader must then Read WHOLE. A size guard that turns a big response into a bigger one plus a round-trip. Three of those in one session; the door was 65% of a week's usage",
            S::Runbook,
            CapabilityStatus::Proven,
            "Sean 2026-07-27 usage read · .forge/spill/*.grain · settings.json:123 matcher Read|Grep|Glob -> Read",
        ),
        CapabilityEntry::new(
            "the hook was the leak, not the door — gate search-ladder matched Grep|Glob and struck every native search, force-feeding each one into the MCP lane. Un-forcing it restored native search at a few hundred bytes per call. raycast stays: 6 rows of digest is the shape everything should match",
            S::Runbook,
            CapabilityStatus::Proven,
            ".claude/settings.json:123 · mcp__forge__grep_roots dropped from allow",
        ),
        CapabilityEntry::new(
            "there is no reactor to replace — forge-daemon has ZERO tokio (dev-dep removed 2026-07-02, dual-oracle: 0 .await, 0 block_on). The door is already hand-rolled sync std::net::TcpListener + threads, so VIXIO displaces no async runtime; the socket loop is already sovereign",
            S::Runbook,
            CapabilityStatus::Proven,
            "forge-daemon/Cargo.toml:93-94 · forge-daemon/src/lib.rs:818",
        ),
    ]
}

/// VIXIO — Sean's 2026-07-27 ruling: MCP is dead, the daemon is not.
///
/// The distinction the name has to carry: "door" bundles the :13016 MCP listener
/// with the :13013 process that owns lifecycle, sidecar chat (:13017) and the gates.
/// Only the first one dies. Every hook already runs `13forge-studio gate <verb>`
/// over stdin-JSON (root#binary-verb) — MCP is the last surface doing it the other
/// way, so this is a sweep, not a rewrite.
///
/// Step 1 flipped Proven 2026-07-28 with its receipt in-row; steps 2-5 stay
/// `[PLANNED]` — a row here is never a receipt without one.
pub fn vixio_plan() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    use CapabilityStatus::Planned as P;
    vec![
        CapabilityEntry::new(
            "1. BYTE CEILING in repo_query::dispatch — one hard ceiling (~2KB) enforced centrally, not per-tool. Search default path:line only, no line text, <=10 hits. Over ceiling TRUNCATE + 'narrow the aim', NEVER spill-to-file. Every response carries its own byte count so the cost is visible at the call site",
            S::Runbook,
            CapabilityStatus::Proven,
            "LANDED 2026-07-28: repo_query::vixio_gate — ceiling 2048 head 512, spill retired; search default path:line x10, text opt-in; raycast digest default (debug opt-in); [BOARD:VIXIO-CEILING] tests incl 5-thread knock; welded via Weld RON, gate cargo test -p forge-daemon",
        ),
        CapabilityEntry::new(
            "2. VIXIO VERB TABLE — verb surface declared in VixiScript, not registered as MCP schemas: name, args, byte ceiling, receipt shape, one table one home. A ceiling declared beside its verb cannot drift from it; a ceiling buried in dispatch can. Open question Sean holds: earns its own dialect, or rides `sheet` (existing dialects are all UI/shader/asset, so a protocol dialect is NET-NEW = pivot#sentinel-WATCHED, flagged post-hoc, not blocked)",
            S::Runbook,
            P,
            "root#vixi-t1 T1-JEWEL prefer-vixi-over-hand-Rust · SoT forge-vix::grammar",
        ),
        CapabilityEntry::new(
            "3. FLIP THE CALLERS — hooks, skills, settings allow to the verb. This is the part that bites if done half-way: mcp__forge__* is referenced across prime-context's NEXT-line, the welder agent tool list, the M2a recon contract, and settings.allow",
            S::Runbook,
            P,
            "grep mcp__forge__ across .claude/** before cutting anything",
        ),
        CapabilityEntry::new(
            "4. DROP THE :13016 LISTENER — MCP schema registration and .mcp.json go with it; 44 MCP tool names stop standing in every prompt and the round-trips to load their schemas go to zero. :13013 ctrl, :13017 chat, gates all unaffected",
            S::Runbook,
            P,
            "forge-daemon door-process law: :13013 + :13016 are ONE process — only the MCP half dissolves",
        ),
        CapabilityEntry::new(
            "5. BOARD ROWS — the sweep lands as harvested rows, not prose. GEMMA-FWD-WINDOW from this session is still an undeclared tag: the proof is green and lands nowhere until worldmerge_tasks names it",
            S::Runbook,
            P,
            "forge_book::board_sync::worldmerge_tasks · one harvest lands them all",
        ),
    ]
}

/// The doctrine underneath the plan, in Sean's words (2026-07-27) — the reason a
/// ceiling is a correctness feature and not a nicety.
pub const CONTEXT_DOCTRINE: &str = "The door should be context-sipping. Context is terrible and has to be managed at very low levels to be successful: the more context you get, the more agents hallucinate, the fuzzier details get, the more things get focused on and lost. It needs to be flushed and cleared and managed.";

/// The session as a book chapter — proven rows, findings, then the plan, in that
/// order so a reader hits receipts before intentions.
pub fn vixio_chapter() -> Chapter {
    let mut ch = Chapter::new(
        "Session 2026-07-27 — the gemma lane, and VIXIO",
        AtlasSection::Runbook,
    );
    ch.add_lore(CONTEXT_DOCTRINE);
    for cap in session_2026_07_27_gemma() {
        ch.add_lore(&format!("{} {} — {}", cap.status.badge(), cap.name, cap.receipt));
    }
    for cap in session_2026_07_27_findings() {
        ch.add_lore(&format!("FINDING {} — {}", cap.name, cap.receipt));
    }
    for cap in vixio_plan() {
        ch.add_lore(&format!("{} {} — {}", cap.status.badge(), cap.name, cap.receipt));
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD:VIXIO-PLAN]
    #[test]
    fn the_plan_is_five_steps_and_none_of_them_claim_to_be_done() {
        let plan = vixio_plan();
        assert_eq!(plan.len(), 5, "VIXIO is a 5-step sweep");
        assert_eq!(
            plan[0].status,
            CapabilityStatus::Proven,
            "step 1 landed 2026-07-28 — a flip is legal ONLY with an on-disk receipt"
        );
        assert!(
            plan[0].receipt.contains("vixio_gate"),
            "the flip must name its receipt, not just wear the badge"
        );
        assert!(
            plan[1..].iter().all(|c| c.status == CapabilityStatus::Planned),
            "steps 2-5 are not built — a plan row is NEVER a receipt"
        );
        assert!(
            plan[0].name.contains("BYTE CEILING"),
            "step 1 is the ceiling: the vixi table only declares what dispatch enforces"
        );
    }

    // [BOARD:SESSION-2026-07-27]
    #[test]
    fn every_landed_row_carries_a_receipt() {
        let landed = session_2026_07_27_gemma();
        assert_eq!(landed.len(), 5);
        assert!(landed.iter().all(|c| c.status == CapabilityStatus::Proven));
        assert!(
            landed.iter().all(|c| !c.receipt.is_empty()),
            "PROVEN with no receipt is [UNPROVEN] wearing a badge"
        );
    }

    #[test]
    fn the_candle_oracle_finding_survives_gemma() {
        let f = session_2026_07_27_findings();
        assert_eq!(f.len(), 5);
        assert!(
            f.iter().any(|c| c.name.contains("seq_len == 1")),
            "candle's unmasked incremental decode is the finding most likely to be re-bought"
        );
    }

    #[test]
    fn chapter_carries_doctrine_then_every_row() {
        let ch = vixio_chapter();
        // 1 doctrine + 5 landed + 5 findings + 5 plan.
        assert_eq!(ch.lore_count(), 16);
    }
}
