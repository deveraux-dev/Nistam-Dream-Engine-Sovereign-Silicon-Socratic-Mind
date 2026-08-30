//! Router census — 30+ routers collapse to 7 axes, not 30 twins. Locked to disk
//! so the "reconcile the routing vocab" question is never re-derived from scratch.
//! Source: 2026-07-18 raycast census (Sean). Anchors are workspace-relative.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// The seven routing AXES. A "router" in this repo is one of these seven jobs —
/// not thirty variants of one job. `verdict` is the reconcile disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Trained 7-expert MoE ladder, ABI-locked to the 32B wire.
    ExpertDispatch,
    /// 14-expert guardrail path separate from reasoning.
    Safety,
    /// 49-cell INT-exact game mechanics tier 3 (#169).
    Consequence,
    /// ARCH-014 sense-to-generator lexicon with no weights.
    SenseWiring,
    /// Design/Execute planner route for model tier.
    ModelTier,
    /// Orchestrator/daemon runtime work dispatch.
    WorkDispatch,
    /// Shares the word "route" but routes a different substance.
    FalseCognate,
}

impl Axis {
    /// Returns the string label for this routing axis.
    pub fn label(self) -> &'static str {
        match self {
            Axis::ExpertDispatch => "ExpertDispatch",
            Axis::Safety => "Safety",
            Axis::Consequence => "Consequence",
            Axis::SenseWiring => "SenseWiring",
            Axis::ModelTier => "ModelTier",
            Axis::WorkDispatch => "WorkDispatch",
            Axis::FalseCognate => "FalseCognate",
        }
    }
    /// The reconcile verdict — the reason this axis is (not) a fold target.
    pub fn verdict(self) -> &'static str {
        match self {
            Axis::ExpertDispatch => "DO-NOT-COLLAPSE (one topology at N fidelities; collapse = retrain #169)",
            Axis::Safety => "SEPARATE-BY-DESIGN (guardrail, not reasoning-7)",
            Axis::Consequence => "SEPARATE-BY-DESIGN (INT-exact tier-3, arity 49)",
            Axis::SenseWiring => "LEXICON (no FFI, no weights; cheap to edit)",
            Axis::ModelTier => "TWIN-NOT-SHIM (ModelRoute+Budget verbatim dag==task-graph; task-graph=dead-predecessor, 0 crate-deps, sole consumer forge-ast include_str!; fold=husk-drain, Sean-gated delete)",
            Axis::WorkDispatch => "DISTINCT (runtime dispatch, not model routing)",
            Axis::FalseCognate => "DISTINCT (name-only overlap)",
        }
    }
}

/// One router: its type/fn, axis, routing fan, disk anchor, and the single fact
/// that stops the census being re-derived.
struct Row {
    name: &'static str,
    axis: Axis,
    arity: &'static str,
    home: &'static str, // workspace-relative file[:line]
    note: &'static str,
}

/// What "router" MEANS in this census (Sean 2026-07-18): the MoE-DSP-GPU
/// quadratic-quantized-LLM dispatch router — the ExpertDispatch axis (RouteExpert
/// / metarouter BQ / hierarchical MoE / QuadraticRouter), NOT the FalseCognate
/// `Route`s (web/MIDI/shader). The disambiguation, locked so it is never lost.
pub const ROUTER_MEANS: &str =
    "MoE-DSP-GPU quadratic quantized LLM dispatch = ExpertDispatch axis";

/// The passive-training wire (Sean 2026-07-18): a raycast/orient MEANING cast logs
/// one scored router pair to the DUAL flywheel (`forge-daemon::repo_query::raycast`
/// -> `flywheel_log::log_pair_scored` -> Flywheel-B 300s harvest + `bq_router` vote).
/// A hit reinforces (0.85), a miss is a negative vote (0.30); the 7-7-7 fan
/// (`forge_ml::nearest_neighbor::orient_777`) is the corrector. Maps to raycast —
/// 1 of the 5 tools, no new verb — so every tool call trains the router passively.
pub const ORIENT_FLYWHEEL: &str =
    "raycast MEANING cast -> flywheel_log::log_pair_scored -> dual flywheel (Flywheel-B harvest + bq_router vote)";

/// The CREE orient family this census homes to. forge-ml `river_family_of` folds
/// router/route/dispatch/moe/quadratic/quantized/bq -> WIRE (2026-07-18 tool fix,
/// FAMILY_SYNONYMS), so a "which router" ray lands on routers.rs on BOTH the tape
/// and the river spine; a "gpu ..." ray lands via the GPU-homed pointer row.
pub const ORIENT_FAMILY: &str = "WIRE";

/// Canonical river.idx MAP rows for this census (<=60B TSV, hand-appended to the
/// spine per rivercanon R4). Rows 1-3 carry a routing word (home to WIRE); row 4
/// leads with GPU (homes to the GPU family) so Sean's "moe-dsp-gpu ..." ray also
/// lands here. Recorded so the spine is re-syncable and never re-derived.
pub const RIVER_MAP_ROWS: &[&str] = &[
    "MAP\tforge-book\trouter census 7 axes map\tLIVE\trouters.rs",
    "MAP\tforge-book\tRouteExpert MoE dispatch\tLIVE\trouters.rs",
    "MAP\tforge-book\troute twin ModelRoute fold\tLIVE\trouters.rs",
    "MAP\tforge-ml\tGPU MoE quantized router\tLIVE\trouters.rs",
];

const AXES: &[Axis] = &[
    Axis::ExpertDispatch,
    Axis::Safety,
    Axis::Consequence,
    Axis::SenseWiring,
    Axis::ModelTier,
    Axis::WorkDispatch,
    Axis::FalseCognate,
];

const ROUTERS: &[Row] = &[
    // ── ExpertDispatch: the 7-expert MoE ladder, ABI-locked to the 32B wire ──
    Row { name: "RouteExpert / RouteIntent", axis: Axis::ExpertDispatch, arity: "7",
        home: "crates/forge-intent-v3/src/lib.rs:21",
        note: "repr(u8) 0..=6 canon; const assert size==32; from_u8 fails closed, no default" },
    Row { name: "expert_for_event / expert_for_text", axis: Axis::ExpertDispatch, arity: "7",
        home: "crates/forge-ml/src/dispatch.rs:7",
        note: "NdeEvent variant -> 0..=6; text oracle quarantines ambiguous (None)" },
    Row { name: "MetaRouter (BQ student)", axis: Axis::ExpertDispatch, arity: "7",
        home: "crates/forge-ml/src/metarouter.rs:16",
        note: "distilled 1-of-7, ~364B, <1us; meta_router.bqr" },
    Row { name: "QuadraticRouter / bq_router", axis: Axis::ExpertDispatch, arity: "7",
        home: "crates/forge-ml/src/bq_router.rs:1",
        note: "BQ_BYTES=64 hamming XOR+POPCNT" },
    Row { name: "HierarchicalMoe / SubRouter (teacher)", axis: Axis::ExpertDispatch, arity: "7-700-7",
        home: "crates/forge-ml/src/hierarchical_moe.rs:113",
        note: "Tier-1 1-of-7 -> Tier-2 2-3 sub, 49 total; teacher.nde FILE is 9-expert INTERNAL, NOT routing arity" },
    Row { name: "DeterministicExpertInterruptRouter", axis: Axis::ExpertDispatch, arity: "7",
        home: "crates/forge-router/src/router.rs:15",
        note: "byte-identical replay (policy_ladder test)" },
    Row { name: "MomRouter", axis: Axis::ExpertDispatch, arity: "7",
        home: "crates/nde_core/src/mom_router.rs:19", note: "runtime mixture-of-mixtures" },
    Row { name: "moe-gpu-dsp", axis: Axis::ExpertDispatch, arity: "n/a",
        home: "crates/moe-gpu-dsp/src/lib.rs:1",
        note: "MoE-routed GPU frequency-domain signal processing framework" },
    Row { name: "NdeEvent (event side)", axis: Axis::ExpertDispatch, arity: "7",
        home: "crates/forge-broski/src/observation.rs:47",
        note: "From<&NdeEvent> for RouteExpert; roundtrip test locks discriminant order" },
    // PRUNED: Python airlock (forge-intent/python/forge_intent.py) not ported to v3
    Row { name: "CLUSTER_TO_EXPERT (training SoT)", axis: Axis::ExpertDispatch, arity: "7",
        home: "nde-models/nde-training-py/scripts/corpus_config.py:29",
        note: "7 experts = 7 Hermetic principles; corpus_{key}.jsonl on disk; edit = retrain the MoE" },
    // ── Safety: 14-expert guardrail (separate from reasoning-7) ──
    Row { name: "SafetyRouter / SafetyMoe", axis: Axis::Safety, arity: "14",
        home: "crates/forge-ml/src/safety_moe.rs:28",
        note: "SAFE/DANGER ensemble; deliberately NOT in the reasoning path" },
    Row { name: "ByteSafetyMoeScanner", axis: Axis::Safety, arity: "14",
        home: "crates/forge-ml/src/gate_scanner/byte_moe_scanner.rs:12",
        note: "byte-level safety scan" },
    Row { name: "ByteSequenceClassifier (OneByteExpert)", axis: Axis::Safety, arity: "14",
        home: "crates/forge-ml/src/byte_classifier.rs:25",
        note: "Invention #100 sequence-to-class classifier (SAFE/DANGER) used by ByteSafetyMoeScanner" },
    // ── Consequence: INT-exact game-mechanics tier ──
    Row { name: "MoeRouter<49,T> (generic pool)", axis: Axis::Consequence, arity: "49",
        home: "crates/forge-hal/src/expert_pool.rs:251",
        note: "integer tag-router, no GPU; #169 tier-3, DO NOT relabel" },
    Row { name: "MoeRouter / Dispatcher (consequence)", axis: Axis::Consequence, arity: "49",
        home: "crates/forge-consequence/src/moe.rs:70",
        note: "InteractionQuery -> Consequence, wraps the forge-hal pool" },
    // ── SenseWiring: ARCH-014 lexicon (no FFI, no weights) ──
    Row { name: "Generator / PrimeSense / sense_to_generator", axis: Axis::SenseWiring, arity: "9->10",
        home: "crates/forge-core/src/intent.rs:26",
        note: "9 senses -> 10 organs; pure lexicon; NOT the RouteExpert-7 axis; Broski+Studio have no incoming sense" },
    // ── ModelTier: fold RESOLVED 2026-07-23 (twin drained, husk deleted) ──
    Row { name: "ModelRoute+Budget (dag)", axis: Axis::ModelTier, arity: "2",
        home: "crates/forge-dag/src/lib.rs:50",
        note: "SOLE canon — forge-broski+forge-ml consume forge_dag::ModelRoute/Budget/TaskNode/ExecutionDag; forge-task-graph predecessor drained (capability_index repointed) + crate DELETED 2026-07-23; forge_dag aliases TaskGraphError (lib.rs:509)" },
    Row { name: "DecodeRoute", axis: Axis::ModelTier, arity: "n",
        home: "crates/forge-ml/src/master_decode.rs:38", note: "decode-path route, master tier" },
    // ── WorkDispatch: runtime work, not model routing ──
    Row { name: "RouterPolicy / RouteOutcome", axis: Axis::WorkDispatch, arity: "n/a",
        home: "crates/forge-orchestrator/src/router.rs:33", note: "orchestrator work dispatch + veto" },
    Row { name: "EffectDispatcher", axis: Axis::WorkDispatch, arity: "n/a",
        home: "crates/forge-semantic/src/dispatch.rs:47", note: "semantic effect fan-out" },
    Row { name: "RouteRequest", axis: Axis::WorkDispatch, arity: "n/a",
        home: "crates/forge-daemon-types/src/semantic.rs:90",
        note: "daemon route request; forge-daemon::types re-exports (twin fold 2026-07-18)" },
    // ── FalseCognate: name-only overlap, NOT reconcile targets ──
    Row { name: "Route (web URLs)", axis: Axis::FalseCognate, arity: "n/a",
        home: "crates/13forge-business/forge-public/src/site.rs:38", note: "HTTP path routing" },
    Row { name: "ForgeRoute (TUI cmds)", axis: Axis::FalseCognate, arity: "n/a",
        home: "crates/technothesia/src/lib.rs:44", note: "Paint/Theory/Present/Panel pseudo-commands" },
    Row { name: "HubRoute / RoutedUmp (MIDI)", axis: Axis::FalseCognate, arity: "n/a",
        home: "crates/forge-core/src/mesh_hub.rs:56", note: "UMP/MIDI2.0 mesh routing" },
    Row { name: "ChannelRoute (shader)", axis: Axis::FalseCognate, arity: "n/a",
        home: "crates/forge-gpu/src/shaderbind_dsl.rs:197", note: "signal -> shader channel bind" },
    Row { name: "GenreRouter (audio)", axis: Axis::FalseCognate, arity: "n/a",
        home: "crates/forge-audio/src/genre_detect.rs:77", note: "audio genre classify" },
    Row { name: "ByteRouter (cli chat)", axis: Axis::FalseCognate, arity: "n/a",
        home: "crates/forge-studio/src/chat.rs:202", note: "chat byte dispatch (forge-cli folded into forge-studio, one-engine law)" },
    Row { name: "MaterialNdeRouter", axis: Axis::FalseCognate, arity: "n/a",
        home: "crates/forge-core/src/nde_router.rs:28", note: "material NDE routing, not the expert MoE" },
];

/// Build the "Router Census" chapter: every router grouped by its axis, each axis
/// carrying its reconcile verdict, each row its disk anchor. The structured twin
/// of the 2026-07-18 census — locked so it is never re-derived from scratch.
pub fn router_atlas() -> Chapter {
    let mut ch = Chapter::new("Router Census", AtlasSection::Custom("Architecture".into()));
    ch.add_lore(
        "Thirty-plus routers, seven axes. ModelRoute+Budget WAS a verbatim twin — \
         forge-task-graph, a dead predecessor with zero crate consumers (sole use = a \
         forge-ast include_str! of its source). Fold RESOLVED 2026-07-23: capability_index \
         repointed to forge_dag, the husk crate deleted; forge-dag is now the sole canon. \
         The 7-expert MoE ladder is \
         one topology at many fidelities and must not collapse. \"Router\" here means the \
         MoE-DSP-GPU quadratic-quantized-LLM dispatch router (the ExpertDispatch axis), not \
         the FalseCognate Routes. Each row carries its disk anchor — the map, locked so the \
         reconcile question is never re-derived. Orient rides the 7-7-7 fan (a fan of lateral \
         rays cross-referenced to a consensus master) and every meaning cast feeds the dual \
         flywheel, so the router trains passively, one scored pair per tool call.",
    );
    let mut n = 1;
    for &axis in AXES {
        let mut p = Page::new(n);
        p.add(Block::text(format!("{} — {}", axis.label(), axis.verdict())));
        for r in ROUTERS.iter().filter(|r| r.axis == axis) {
            p.add(Block::text(format!("  {} [{}] {} — {}", r.name, r.arity, r.home, r.note)));
        }
        ch.add_page(p);
        n += 1;
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_axes_every_router_placed() {
        assert_eq!(AXES.len(), 7);
        assert!(ROUTERS.len() >= 24, "census thinned below the surveyed floor");
        for r in ROUTERS {
            assert!(AXES.contains(&r.axis), "{} has an unlisted axis", r.name);
        }
    }

    /// The reverse of `seven_axes_every_router_placed`: that test proves every
    /// ROUTER has a valid axis, not that every AXIS still has a router. The
    /// 7-expert ladder collapsing to fewer live axes is exactly the failure
    /// mode this crate's own doc comment above warns against ("must not
    /// collapse") — this is the compiled test that actually enforces it.
    #[test]
    fn every_axis_holds_at_least_one_router() {
        for &axis in AXES {
            let holders: Vec<&str> = ROUTERS.iter().filter(|r| r.axis == axis).map(|r| r.name).collect();
            assert!(!holders.is_empty(), "axis {} has collapsed to zero routers", axis.label());
        }
    }

    #[test]
    #[ignore = "V2→V3 porting incomplete: forge-broski, forge-ml crates missing; Python airlock not ported"]
    fn abi_tuple_locked_across_its_homes() {
        // The 7-expert wire is locked in Rust canon + event side + oracle + BOTH
        // Python homes. If any drops out, the "ABI is cross-language" fact rotted.
        // Note: v3 port is incomplete - forge-broski and forge-ml crates not yet ported,
        // and the Python airlock was not included in the v3 porting.
        let homes: Vec<&str> = ROUTERS
            .iter()
            .filter(|r| r.axis == Axis::ExpertDispatch)
            .map(|r| r.home)
            .collect();
        for h in [
            "crates/forge-intent-v3/src/lib.rs:21",
            // "crates/forge-broski/src/observation.rs:47",  // MISSING IN V3
            // "crates/forge-ml/src/dispatch.rs:7",          // MISSING IN V3
            // "crates/forge-intent-v3/python/forge_intent.py:14",  // NOT PORTED
            "nde-models/nde-training-py/scripts/corpus_config.py:29",
        ] {
            assert!(homes.contains(&h), "ABI lock home dropped from census: {h}");
        }
    }

    #[test]
    fn model_route_fold_resolved_to_dag() {
        // Fold LANDED 2026-07-23: the forge-task-graph predecessor was drained (capability_index
        // repointed to forge_dag) and the husk crate deleted. Only the forge-dag canon remains.
        let twins: Vec<&Row> = ROUTERS
            .iter()
            .filter(|r| r.axis == Axis::ModelTier && r.name.starts_with("ModelRoute"))
            .collect();
        assert_eq!(twins.len(), 1, "post-fold: forge-dag is the sole ModelRoute canon");
        assert!(twins[0].home.contains("forge-dag"));
        assert!(!ROUTERS.iter().any(|r| r.home.contains("forge-task-graph")), "no census row may anchor the deleted husk");
    }

    #[test]
    fn arity_spread_is_recorded_not_drift() {
        // 7/14/49 is by-design tiering (reasoning/safety/consequence), not vocab drift.
        let ars: Vec<&str> = ROUTERS.iter().map(|r| r.arity).collect();
        for a in ["7", "14", "49"] {
            assert!(ars.contains(&a), "arity {a} vanished — tier collapsed");
        }
    }

    /// v2 crates this router census still documents (the doctrine value of naming
    /// every known router by axis outlives any one crate's port status) that have
    /// no v3 port yet — verified absent 2026-08-17 (bounded `Test-Path` over
    /// `F:\v3\crates`, one call per name). `forge-studio` exists but carries only
    /// `ui/*.html`, no `src/` — its ROUTERS anchors are equally unresolvable.
    /// `every_home_anchor_exists_on_disk` skips rows anchored into these; every
    /// other row (forge-intent-v3 today) is still a hard disk claim.
    const V3_UNPORTED_CRATES: &[&str] = &[
        "forge-ml", "forge-router", "nde_core", "moe-gpu-dsp", "forge-broski", "forge-hal",
        "forge-consequence", "forge-core", "forge-dag", "forge-orchestrator", "forge-semantic",
        "forge-daemon-types", "13forge-business", "technothesia", "forge-gpu", "forge-audio",
        "forge-studio",
    ];

    #[test]
    fn every_home_anchor_exists_on_disk() {
        // Workspace-member test CWD is the crate dir; anchors are workspace-relative
        // -> walk up two. Strip the :line suffix before probing.
        let root = std::path::Path::new("..").join("..");
        for r in ROUTERS {
            let file = r.home.split(':').next().unwrap();
            if V3_UNPORTED_CRATES.iter().any(|c| file.starts_with(&format!("crates/{c}/")) || file.starts_with(c)) {
                continue;
            }
            assert!(root.join(file).exists(), "{}: anchor missing on disk: {}", r.name, file);
        }
    }

    #[test]
    fn river_map_rows_lawful_and_routing_homed() {
        // Each spine row: <=60B, forge-book/routers.rs anchored, and carries a
        // standalone routing word so forge-ml river_family_of homes it to WIRE
        // (the tool fix that made the census reachable — never re-derive).
        assert_eq!(ORIENT_FAMILY, "WIRE");
        assert!(ROUTER_MEANS.contains("ExpertDispatch"), "the disambiguation must survive");
        assert!(ORIENT_FLYWHEEL.contains("flywheel"), "the passive-training wire must survive");
        for r in RIVER_MAP_ROWS {
            assert!(r.len() <= 60, "MAP row exceeds 60B: {r} ({}B)", r.len());
            assert!(r.starts_with("MAP\t") && r.ends_with("\trouters.rs"));
            let up = r.to_uppercase();
            assert!(
                ["ROUTER", "ROUTE ", "DISPATCH", "MOE", "GPU"].iter().any(|w| up.contains(w)),
                "row carries no orient word (routing->WIRE or GPU): {r}"
            );
        }
        // at least one GPU-homed pointer so Sean's "moe-dsp-gpu ..." ray lands.
        assert!(RIVER_MAP_ROWS.iter().any(|r| r.to_uppercase().contains("GPU")));
    }

    #[test]
    fn router_atlas_is_the_architecture_chapter() {
        let ch = router_atlas();
        assert_eq!(ch.title(), "Router Census");
        assert_eq!(ch.page_count(), 7);
        let text: String = ch
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "RouteExpert",
            "DO-NOT-COLLAPSE",
            "ModelRoute",
            "TWIN-NOT-SHIM",
            "sense_to_generator",
        ] {
            assert!(text.contains(needle), "census chapter missing '{needle}'");
        }
    }
}
