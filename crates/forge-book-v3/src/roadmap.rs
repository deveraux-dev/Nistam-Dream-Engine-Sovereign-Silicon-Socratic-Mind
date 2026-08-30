//! Roadmap chapter — what is open and what is aspired, condensed to terse prose
//! behind code. FLEXIBLE, NOT LAW: the live source is `_plans/BACKLOG-INDEX-*.md`
//! + the RUN-BOARDs (drained each workflow cycle); this chapter is the narrative
//! snapshot. Aspirations ride the HORIZON page — far, revisable, not scheduled.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// One open head: what it is, its verdict, and where it lives.
struct Head {
    what: &'static str,
    verdict: &'static str,
    at: &'static str,
}

const OPEN: &[Head] = &[
    Head { what: "MAGIC CANVAS — open-world physics/paint/audio-brush/alchemy web tool", verdict: "DONE + DEPLOYED 07-14", at: "13forge.com/magic-canvas · sf-wasm + magic-canvas-app" },
    Head { what: "mamawapiwin (G1) — the ghost-deck Gathering, 5D-indexed", verdict: "FUN-CLOSED 07-13 (1,682 LoC folded)", at: "BIGPLAN-BOOKFOLD-2026-07-13.md · forge-daemon/ghost_network.rs" },
    Head { what: "forge-vcs — wire into forge-daemon, or ship as storefront no-git SKU", verdict: "SEAN-GATED (9/9 green, decision open)", at: "crates/forge-vcs" },
    Head { what: "\"is X already built?\" pull-gate as one door question before net-new/ABSENT", verdict: "PENDING (engine landed, front-discipline open)", at: "forge-daemon/repo_query.rs Oracle B :1383-1447" },
    Head { what: "SOVEREIGN STACK — Gemma drafts / Claude executes / flywheel learns", verdict: "PARTIAL — foreman-runner LANDED 07-15; resident gemma-brain + dream-arm tool surface still open", at: "forge-daemon/dream_wire.rs · tools/foreman-runner" },
    Head { what: "UMSSP GEMMA-GPU — resident-VRAM sovereign forward: Q4_K layer weights + Q6_K tied lm_head on GPU, dequant in-shader, argmax == candle oracle (818/669)", verdict: "GPU FORWARD LANDED 07-22 — 2.1 tok/s, 2.08GB VRAM. ACTIVATION-RESIDENT RUNG (collapse 238 readbacks = 7 matvec×34L, each submit+poll(Wait)+readback → 1/token; weights already resident, only the ACTIVATION round-trips 7×/layer; path to 100+ tok/s; risk = GQA 8q/4kv + growing on-GPU KV): legs 1-3a LANDED 07-22 (rmsnorm; per-head rmsnorm+NeoX-rope; GQA multi-head attention kernel), bit-exact vs gemma_forward_cpu; open = 3b resident-KV append plumbing · silu·up · residual-add (+leg3.1 flash online-softmax for global long-ctx past the 2048 workgroup-scores cap); then forward_resident twin (one encoder/token) under umssp_phase6 holding 818/669, forward left untouched as parity fallback", at: "forge-daemon/gemma_gpu.rs ResidentGemma::forward · forge-ml gpu_matmul.rs+shaders/{rmsnorm,rope_qknorm,attention_gqa,matvec_q6k,matvec_f32}.wgsl · tests/gpu_dispatch_smoke {rmsnorm,rope_qknorm,attention_gqa}_gpu_matches_cpu_oracle · example umssp_phase5" },
    Head { what: "visual-gate 6up — 4/6 panels drifted from blessed baseline (real content confirmed, not a black-capture bug)", verdict: "SEAN-GATED — bless or investigate first", at: "forge-studio/visual_gate.rs run_6up" },
    Head { what: "The Book wears the product's skin (G2) / first cart falls out (G3, ironroot reel)", verdict: "COLD — Sean ranks next", at: "BIGPLAN-BOOKFOLD-2026-07-13.md candidates" },
    Head { what: "physics-effect TYPED routing (sound->AudioCmd, damage->HP)", verdict: "PENDING (Sean design — generic CartSink)", at: "forge-game-host/arena_host.rs physics_effects()" },
    Head { what: "cockpit re-source (orb / constellation / sliders)", verdict: "DESIGN-FRAMED (Sean) — PULL-BOARD G21", at: "technothesia/orb_swarm.rs · forge-gui/constellation_kit.rs" },
    Head { what: "CORE FOLD (3 canvas editors -> 1 CanvasDoc) / AUDIO-ONE-BUS / pixel_sequencer move / 9 lost-capability orphans / Pasquill DET-CLOCK legs", verdict: "NOT RE-WALKED since 07-13 — RUN-BOARD fold5000 absent from disk, anchor is stale", at: "PASS2 carry-forward, BACKLOG-INDEX-2026-07-13-PASS2.md" },
];

/// Horizon aspirations — far, not scheduled; the direction, revisable.
const HORIZON: &[&str] = &[
    "R3 sovereign-model real forward: text -> learned BQ code wired (weights on E:/F:), lifting R3 off its R2 fallback.",
    "Semantic index over the full code corpus, not just the river spine — dense clustering where the ray has material.",
    "The manual as the single condensed home: aspirations / roadmap / design as terse-code-behind-prose, sources quarantined off-repo once proven.",
];

/// Build the "Roadmap" chapter: the open heads (with verdicts) + the horizon
/// aspirations. Narrative snapshot of the live backlog; revisable, not law.
pub fn roadmap() -> Chapter {
    let mut chapter = Chapter::new("Roadmap", AtlasSection::Custom("Roadmap".into()));
    chapter.add_lore(
        "What is open and what is aspired. Flexible, not law — the live source is \
         _plans/BACKLOG-INDEX-*.md + the RUN-BOARDs, drained each workflow cycle; this \
         chapter is the narrative snapshot.",
    );

    let mut open = Page::new(1);
    open.add(Block::text("OPEN heads (drain order is the workflow's call):"));
    for h in OPEN {
        open.add(Block::text(format!("  [{}] {} — {}", h.verdict, h.what, h.at)));
    }
    chapter.add_page(open);

    let mut horizon = Page::new(2);
    horizon.add(Block::text("HORIZON (aspirations — far, revisable):"));
    for a in HORIZON {
        horizon.add(Block::text(format!("  - {a}")));
    }
    chapter.add_page(horizon);

    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roadmap_is_the_roadmap_section() {
        let ch = roadmap();
        assert_eq!(ch.title(), "Roadmap");
        assert_eq!(ch.section, AtlasSection::Custom("Roadmap".into()));
        assert_eq!(ch.page_count(), 2);
    }

    #[test]
    fn roadmap_carries_open_heads_and_horizon() {
        let ch = roadmap();
        let text: String = ch
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\n");
        for needle in ["CORE FOLD", "SEAN-GATED", "HORIZON", "R3 sovereign-model"] {
            assert!(text.contains(needle), "roadmap missing '{needle}'");
        }
    }
}
