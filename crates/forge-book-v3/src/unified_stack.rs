//! The Unified Stack — 4 layers, one engine (Sean 07-27, post ghostharness exorcism).
//! Compiled const, NOT prose: every row carries its disk anchor and its ladder tag, so the
//! map gauges itself. A row that reads GREEN without an anchor is the false-green this
//! session was spent killing — see `rank`: DECLARED != EXERCISED.
//! SPEC-BINDS (Sean 07-27): quarry-spec names bound to their live structs — the
//! RAG-confabulation audit folded to canon so the names orient instead of haunt.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// Proof ladder for one stack row (root#proof-ladder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proof {
    /// Traced to disk this session, or green under test.
    Proven,
    /// Stated by Sean as lineage or intent (ARCH-020 §2). NOT a weaker `Proven`:
    /// it is off-ladder. Testimony needs no code anchor and is exempt from anchor
    /// resolution — marking authored heritage `Unproven` is a category error.
    Authored,
    /// Named in law/tablet; the anchor exists, the mechanism is untraced.
    Estimate,
    /// Claimed in the stack narrative, no anchor found. Never render as aligned.
    Unproven,
}

impl Proof {
    /// The tag the board and any renderer prints. Never a checkmark for `Unproven`.
    pub fn tag(self) -> &'static str {
        match self {
            Proof::Proven => "[PROVEN]",
            Proof::Authored => "[AUTHORED]",
            Proof::Estimate => "[ESTIMATE]",
            Proof::Unproven => "[UNPROVEN]",
        }
    }

    /// True when the row makes no code claim, so no anchor may be demanded of it.
    pub fn exempt_from_anchor(self) -> bool {
        matches!(self, Proof::Authored)
    }
}

/// One row of the stack: which layer, what it is, where it lives, how well it is known.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// One of the four architecture layers: GOVERNANCE, INGESTION, LATENT-5D, or NDE.
    pub layer: &'static str,
    /// Descriptive name of what this row represents.
    pub name: &'static str,
    /// File path or code location serving as proof this row exists.
    pub anchor: &'static str,
    /// Proof level: Proven, Authored, Estimate, or Unproven.
    pub proof: Proof,
}

/// The stack, top (governance) to bottom (inference).
pub const STACK: &[Row] = &[
    // L1 GOVERNANCE — gate/harness are the source; settings.json is the projection.
    Row { layer: "GOVERNANCE", name: "settings.json wiring SoT + drift observer",
          anchor: "crates/forge-daemon/src/harness_config.rs", proof: Proof::Proven },
    Row { layer: "GOVERNANCE", name: "build observer -> harness_status.ron",
          anchor: "crates/forge-daemon/src/harness.rs", proof: Proof::Proven },
    Row { layer: "GOVERNANCE", name: "CLAUDE.md 4608B cap gate (2 of 132 files; both OVER)",
          anchor: "crates/forge-daemon/src/gate.rs:775", proof: Proof::Proven },
    Row { layer: "GOVERNANCE", name: "doctrine mine: 132 files, 432 laws, 76 unexercised",
          anchor: "crates/forge-recovery/scripts/claude_md_scan.py", proof: Proof::Proven },

    // L2 INGESTION — MIDI 1.0 stores at rest, MIDI 2.0 moves on the wire.
    Row { layer: "INGESTION", name: "atom ladder 8/28/18/64B, little-nistam LSB-first",
          anchor: "crates/forge-daemon-types/src/atom.rs", proof: Proof::Estimate },
    Row { layer: "INGESTION", name: "SoulWord store (MIDI 1.0 generation, at rest)",
          anchor: "crates/outland/src/lib.rs", proof: Proof::Estimate },
    Row { layer: "INGESTION", name: "UMP packets (MIDI 2.0 generation, in motion)",
          anchor: "crates/forge-harmonics/src/ump/message.rs", proof: Proof::Estimate },
    Row { layer: "INGESTION", name: "RoutedUmp / UmpAuthorityTicket (both live types, 07-27 relocate)",
          anchor: "crates/forge-core/src/ump.rs:68 + crates/forge-ump/src/ticket.rs:27", proof: Proof::Proven },

    // L3 LATENT — 5D frames, ternary sheet, phase collapse.
    Row { layer: "LATENT-5D", name: "5D raycast origin/dir over river.idx + tape.idx",
          anchor: "crates/forge-daemon/src/repo_query.rs", proof: Proof::Proven },
    Row { layer: "LATENT-5D", name: "105-trit hamming sheet, EQUAL-perp tie-break",
          anchor: "repo_query::trit_hamming_sheet", proof: Proof::Proven },
    Row { layer: "LATENT-5D", name: "SPCC soliton-phase context collapse",
          anchor: "crates/forge-book/src/tablets/ARCH-018-compute-at-rest.md", proof: Proof::Estimate },
    Row { layer: "LATENT-5D", name: "integer determinism: MilliUnit i64 / Permyriad i32",
          anchor: "crates/pp-math/src/fixed_point.rs", proof: Proof::Estimate },
    Row { layer: "LATENT-5D", name: "HEAR collapse: Point5D (X,Y,Z,W,theta wraps 360) -> StereoField, HearDecoder",
          anchor: "crates/forge-audio/src/dimensional_collapse.rs (caller broadcast_booth.rs:30)", proof: Proof::Proven },
    Row { layer: "LATENT-5D", name: "SEE collapse twin: run_unified -> unified::draw_terminal, THE live TKNO glass path",
          anchor: "crates/technothesia/src/unified.rs + tests/r2_rosetta_onglass.rs + r3_world_reacts.rs", proof: Proof::Proven },

    // L4 NDE — 3 tiers, resident muscle, bit-quantized routing.
    Row { layer: "NDE", name: "escalation ladder student->teacher->master->gemma->oracle",
          anchor: "crates/forge-daemon/src/tiers.rs", proof: Proof::Estimate },
    Row { layer: "NDE", name: "resident Gemma engine (no llama_cpp_rs, no port, no FFI)",
          anchor: "crates/forge-daemon/src/gemma_engine.rs", proof: Proof::Estimate },
    Row { layer: "NDE", name: "BqRouter meta-bake (meta_router.bqr, ADR-0020)",
          anchor: "crates/technothesia/src/unified.rs:1866", proof: Proof::Proven },
    Row { layer: "NDE", name: "nde-ladder resident loads (student/teacher/master, .nde only; law renamed to match code 07-27)",
          anchor: "crates/forge-daemon/src/daemon/infer_thread.rs:914,948 + tiers.rs:8", proof: Proof::Proven },
    Row { layer: "NDE", name: "gemma's home: root#nde-ladder bans it in repo/client, forge-daemon#embedded-brain ships it IN the bin",
          anchor: "two laws contradict — UNLOCATED verdict, no code read settles which recipe is canon", proof: Proof::Unproven },
];

/// One spec-name bind: a name that lives in quarry spec canon, mapped to the
/// struct that answers it in the SoT (epistemic-parity: name-by-struct).
#[derive(Debug, Clone, Copy)]
pub struct Bind {
    /// Name from quarry spec or documentation.
    pub spec: &'static str,
    /// The live struct or mechanism that implements this spec name.
    pub live: &'static str,
    /// File path proving the live implementation exists.
    pub anchor: &'static str,
    /// Proof level: Proven, Authored, Estimate, or Unproven.
    pub proof: Proof,
}

/// The 2026-07-27 audit: four names a RAG pass over the quarries presented as
/// engine mechanisms. Three were already live under other spellings; one is the
/// open May-20 Q3. A bind flips to `Proven` ONLY with a code anchor.
pub const SPEC_BINDS: &[Bind] = &[
    Bind { spec: "TritTree5D (5D balanced trinary index)",
           live: "repo_query 5D raycast + trit_hamming_sheet (105 trits)",
           anchor: ".agents/skills/tractor-beam/SKILL.md:39 -> crates/forge-daemon/src/repo_query.rs",
           proof: Proof::Proven },
    Bind { spec: "forge-harness",
           live: "harness observer verb + settings.json SoT mirror (13forge-studio harness)",
           anchor: "crates/forge-daemon/src/harness.rs:81 + crates/forge-daemon/src/harness_config.rs",
           proof: Proof::Proven },
    Bind { spec: "AuthorityTicket (Executable Memory primitive, canon 05-20)",
           live: "forge_core::spine::AuthorityTicket + forge_ump::UmpAuthorityTicket 16B Pod",
           anchor: "crates/forge-core/src/lib.rs:146 + crates/forge-ump/src/ticket.rs:27 + crates/forge-anim/src/cue/envelope.rs:41",
           proof: Proof::Proven },
    Bind { spec: "dauer_state (Allostatic OODA Hypervisor survival mode)",
           live: "VERBED 07-27: DauerState + fail_streak ladder in the harness observer, verb `13forge-studio dauer` \
                  (0 Active / 2 Dauer / 3 no-row), Stop-hook wire compiled in harness_config::WIRING; hook-LIVE at the \
                  release rebuild. Synonym kin: persist_census census.rs:153, gate [STRIKE 1/3], dormant marks intel_drain.rs:42",
           anchor: "crates/forge-daemon/src/harness.rs (DauerState, DAUER_THRESHOLD=3) + crates/forge-studio/src/main.rs dauer arm",
           proof: Proof::Proven },
    Bind { spec: "RoadScan (Process census diagnostic loop)",
           live: "UNBUILT; harness_status.ron build observer",
           anchor: "ARCH-006-process-ops.md:150",
           proof: Proof::Estimate },
    Bind { spec: "PolishGate (Backend to frontend UI status enforcement)",
           live: "UNBUILT; CLAUDE.md cap gate",
           anchor: "ARCH-006-process-ops.md:27",
           proof: Proof::Estimate },
    Bind { spec: "DesignTasteCorpus (Queried design doctrine)",
           live: "UNBUILT; CLAUDE.md manifest laws",
           anchor: "CLAUDE.md + ARCH-006-process-ops.md:29",
           proof: Proof::Estimate },
];

/// T1 ENCODE — `unified_sieve_multidimensional_architecture_v2` (Sean 2026-07-28), bound
/// name-by-struct like [`SPEC_BINDS`]. The spec is a SUPERSET MAP, not a build order: every
/// row below already answers on disk, so v2.0 is a WIRE_B4_NEW pass. A row flips to
/// `Proven` only with a code anchor; `Estimate` means the anchor exists and the mechanism
/// is untraced this session.
pub const UNIFIED_V2: &[Bind] = &[
    Bind { spec: "§1 universal primitive (1 Pixel = 1 Vixel = 1 Voxel)",
           live: "FOUR declared faces, 0 folds — see PRIM_FACES",
           anchor: "crates/forge-book/src/unified_stack.rs PRIM_FACES",
           proof: Proof::Proven },
    Bind { spec: "§3 consequence dispatch (per-tick BTreeMap, frame budget)",
           live: "forge-consequence dispatch + budget",
           anchor: "crates/forge-consequence/src/dispatch.rs + budget.rs",
           proof: Proof::Estimate },
    Bind { spec: "§4 phonetic sound engine (vowel->formant, onset->transient, trit->ADSR)",
           live: "forge_calligraphy::audio_bridge (cree_sound_engine_v1, Sean 07-28)",
           anchor: "crates/forge-calligraphy/src/lib.rs:211",
           proof: Proof::Estimate },
    Bind { spec: "§4 physics->audio event triggers (collision/slide/impact/occlusion)",
           live: "forge_harmonics::physics_audio",
           anchor: "crates/forge-harmonics/src/lib.rs:43",
           proof: Proof::Estimate },
    Bind { spec: "§5 Cosmic Dissonance Kernel (elemental/alchemical/aspect resolution stack)",
           live: "dissonance_sieve — stateless deterministic universal interaction gate",
           anchor: "crates/forge-game-systems/src/dissonance_sieve.rs + _book/17-cosmic-dissonance-kernel.md",
           proof: Proof::Estimate },
    Bind { spec: "§5 SoulWord 64B (8 hash | 4 parent | 52 trits, radix-3 base-243)",
           live: "outland::soulword",
           anchor: "crates/outland/src/soulword.rs",
           proof: Proof::Estimate },
    Bind { spec: "§5 trit-tree 5D partitioning, O(log N) raycast proximity",
           live: "PackedPoint105 + TritTree5D (same index the tractor-beam bind names)",
           anchor: "crates/outland/src/trit_tree.rs:14",
           proof: Proof::Estimate },
    Bind { spec: "§6 ASP sieve (loot curve / boss scaling / secret placement on ship)",
           live: "forge_game_systems::asp_constraints",
           anchor: "crates/forge-game-systems/src/asp_constraints.rs",
           proof: Proof::Estimate },
    Bind { spec: "§7 perceivability sieve (channel prune -> MUD/audio/haptic fallback)",
           live: "forge_sieve::perceivability (PerceivabilitySieve, multimodal floor)",
           anchor: "crates/forge-sieve/src/lib.rs:30",
           proof: Proof::Estimate },
];

/// The universal primitive's FOUR declared faces — and the verdict that NONE of them is a
/// fold. Named here because "1 Pixel = 1 Vixel = 1 Voxel" reads as one struct and is not:
/// the 28B pair is a deliberate host/device mirror (see [`PRIM_FOLD_VERDICT`]), and the 8B
/// and authoring faces carry fields the others do not. Folding by name would break the
/// SPIR-V build; the correspondence is the deliverable, not a merge.
pub const PRIM_FACES: &[Bind] = &[
    Bind { spec: "IPC face, 8B (material_id/color_id/resonance_id/local_z/router_tag)",
           live: "forge_daemon_types::atom::VixelAtom — size_of == 8 asserted at compile time",
           anchor: "crates/forge-daemon-types/src/atom.rs:9 (assert :23)",
           proof: Proof::Proven },
    Bind { spec: "KERNEL face, 28B — the SPIR-V SoT whose offsets bake into the .spv blob",
           live: "forge_shaders::gpu_types::VixelAtom (no_std, bytemuck-free by Firewall Law)",
           anchor: "crates/forge-shaders/src/gpu_types.rs:77",
           proof: Proof::Proven },
    Bind { spec: "HOST face, 28B — the wgpu-upload Pod mirror of the kernel face",
           live: "forge_gpu::vixel_pass::VixelAtom, bound `type Kernel = forge_shaders::…` + drift test",
           anchor: "crates/forge-gpu/src/vixel_pass.rs:47 (bind :741)",
           proof: Proof::Proven },
    Bind { spec: "AUTHORING face (coord[i32;3] + colour + material + resonance + essence)",
           live: "forge_level_editor::VixelAtom — the owned-export authoring SoT",
           anchor: "crates/forge-level-editor/src/lib.rs:67",
           proof: Proof::Proven },
];

/// Why the 28B pair must NOT be folded, kept next to the faces so the next pass reads it
/// before reaching for a merge. `forge-shaders` is `no_std`/rust-gpu: it cannot take a
/// `bytemuck` derive, and its field offsets are compiled INTO the vendored SPIR-V. The
/// host mirror exists so wgpu can upload; the drift test is what keeps them one primitive.
pub const PRIM_FOLD_VERDICT: &str =
    "4 faces, 0 folds — host/device mirror is by design (drift-tested), 8B IPC face is \
     ORPHAN and wants WIRING not folding; spec §1 names essence_id where the 8B face \
     carries resonance_id (reconcile before either moves)";

/// The four layer ids, top to bottom.
pub const LAYERS: [&str; 4] = ["GOVERNANCE", "INGESTION", "LATENT-5D", "NDE"];

/// `(proven, authored, estimate, unproven)` across the whole stack — the honest headline.
pub fn tally() -> (usize, usize, usize, usize) {
    STACK.iter().fold((0, 0, 0, 0), |(p, a, e, u), r| match r.proof {
        Proof::Proven => (p + 1, a, e, u),
        Proof::Authored => (p, a + 1, e, u),
        Proof::Estimate => (p, a, e + 1, u),
        Proof::Unproven => (p, a, e, u + 1),
    })
}

/// Rows of one layer, in declaration order.
pub fn layer(id: &str) -> Vec<&'static Row> {
    STACK.iter().filter(|r| r.layer == id).collect()
}

/// Every row that is narrative-only — the work queue, never rendered as aligned.
pub fn unproven() -> Vec<&'static Row> {
    STACK.iter().filter(|r| r.proof == Proof::Unproven).collect()
}

/// One dense line per row: `LAYER  [TAG]  name  <- anchor`.
pub fn render() -> String {
    let (p, a, e, u) = tally();
    let mut s = format!(
        "UNIFIED STACK  rows={}  proven={p} authored={a} estimate={e} unproven={u}\n",
        STACK.len()
    );
    for id in LAYERS {
        for r in layer(id) {
            s.push_str(&format!("{id:<11} {:<11} {}  <- {}\n", r.proof.tag(), r.name, r.anchor));
        }
    }
    s.push_str(&format!("SPEC-BINDS  n={}  (quarry name -> live struct)\n", SPEC_BINDS.len()));
    for b in SPEC_BINDS {
        s.push_str(&format!("BIND        {:<11} {} -> {}  <- {}\n", b.proof.tag(), b.spec, b.live, b.anchor));
    }
    s
}

/// The atlas chapter — the live caller that drains this module's orphanhood:
/// one page, the whole self-gauging map, wired by `seed::full_atlas`.
pub fn stack_atlas() -> Chapter {
    let mut ch = Chapter::new("Unified Stack & Spec Binds", AtlasSection::Custom("Architecture".into()));
    ch.add_lore(
        "The 4-layer engine map plus the spec-name binds, compiled const with disk \
         anchors — assembled 2026-07-27 after the RAG-confabulation audit: names that \
         haunt the quarries are bound to the structs that answer them, and dauer_state \
         stays [UNPROVEN] until code lands.",
    );
    let mut p = Page::new(1);
    p.add(Block::text(render()));
    ch.add_page(p);
    let mut p2 = Page::new(2);
    p2.add(Block::text(render_v2()));
    ch.add_page(p2);
    ch
}

/// One dense line per v2.0 bind, then the four faces and the fold verdict. The live caller
/// for [`UNIFIED_V2`] / [`PRIM_FACES`] — a const nobody renders is a map nobody reads.
pub fn render_v2() -> String {
    let mut s = format!(
        "UNIFIED v2.0 (Sean 07-28)  binds={}  faces={}\n",
        UNIFIED_V2.len(),
        PRIM_FACES.len()
    );
    for b in UNIFIED_V2 {
        s.push_str(&format!("  {}  {}\n    -> {}  <- {}\n", b.proof.tag(), b.spec, b.live, b.anchor));
    }
    s.push_str("PRIMITIVE FACES\n");
    for b in PRIM_FACES {
        s.push_str(&format!("  {}  {}\n    -> {}  <- {}\n", b.proof.tag(), b.spec, b.live, b.anchor));
    }
    s.push_str("VERDICT ");
    s.push_str(PRIM_FOLD_VERDICT);
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: UNIFIED-STACK] the map must never claim more than the disk carries.
    #[test]
    fn every_row_has_a_layer_and_an_anchor_and_unproven_rows_say_so() {
        for r in STACK {
            assert!(LAYERS.contains(&r.layer), "row outside the 4 layers: {r:?}");
            assert!(!r.name.is_empty() && !r.anchor.is_empty(), "anchorless row: {r:?}");
            if r.proof == Proof::Unproven {
                assert!(
                    r.anchor.contains("UNLOCATED") || r.anchor.contains("0 code mentions"),
                    "an UNPROVEN row must name WHY it is unproven: {r:?}"
                );
            }
        }
        assert_eq!(Proof::Unproven.tag(), "[UNPROVEN]", "never a checkmark");
    }

    /// The v2.0 encode is a MAP, so every row must carry a disk anchor and a live name —
    /// a spec row with neither is the confabulation this module exists to catch.
    // [BOARD: UNIFIED-V2]
    #[test]
    fn the_v2_encode_binds_every_spec_name_to_a_struct_and_an_anchor() {
        for b in UNIFIED_V2.iter().chain(PRIM_FACES) {
            assert!(!b.spec.is_empty() && !b.live.is_empty(), "unbound spec row: {b:?}");
            assert!(b.anchor.contains('/'), "a bind must anchor to a path: {b:?}");
            assert_ne!(b.proof, Proof::Unproven, "v2.0 rows all answer on disk — that IS the finding");
        }
        assert!(UNIFIED_V2.len() >= 9, "the spec has 9 named sections to bind");
    }

    /// The primitive has four faces and zero folds. If someone merges the 28B pair the
    /// SPIR-V offsets stop matching the host upload, so the verdict is pinned, not advisory.
    // [BOARD: UNIFIED-V2]
    #[test]
    fn the_primitive_keeps_four_faces_and_the_no_fold_verdict_stays_loud() {
        assert_eq!(PRIM_FACES.len(), 4, "1px=1vixel=1voxel reads as one struct and is not");
        assert!(PRIM_FOLD_VERDICT.contains("0 folds"), "{PRIM_FOLD_VERDICT}");
        assert!(PRIM_FOLD_VERDICT.contains("ORPHAN"), "the 8B face wants wiring, not folding");
        // The kernel face must never be described without its no_std/SPIR-V reason.
        let kernel = PRIM_FACES.iter().find(|b| b.spec.starts_with("KERNEL")).expect("kernel face");
        assert!(kernel.live.contains("no_std"), "{:?}", kernel);
    }

    /// [`render_v2`] is the live caller — the consts are reachable, not shelf-ware.
    // [BOARD: UNIFIED-V2]
    #[test]
    fn render_v2_reaches_every_bind_and_the_verdict() {
        let out = render_v2();
        for b in UNIFIED_V2.iter().chain(PRIM_FACES) {
            assert!(out.contains(b.anchor), "bind missing from the render: {b:?}");
        }
        assert!(out.contains(PRIM_FOLD_VERDICT));
        assert!(stack_atlas().page_count() >= 2, "the v2 page must be wired into the atlas");
    }

    // [BOARD: UNIFIED-STACK] a saturated all-green tally is the false-green signature
    // (07-27: a 5-axis 'all Sovereign Anchor (0)' scan hid the whole ghostharness split).
    #[test]
    fn tally_is_honest_and_the_stack_is_not_saturated() {
        let (p, a, e, u) = tally();
        assert_eq!(p + a + e + u, STACK.len(), "tally covers every row");
        assert!(u > 0, "SATURATED: 0 unproven rows means the map stopped gauging");
        assert_eq!(u, unproven().len());
        assert!(p > 0 && e > 0, "a map that is all one tag is not measuring anything");
    }

    // [BOARD: UNIFIED-STACK]
    #[test]
    fn all_four_layers_are_populated_and_render_lists_them_all() {
        for id in LAYERS {
            assert!(!layer(id).is_empty(), "empty layer {id}");
        }
        let out = render();
        assert!(out.starts_with("UNIFIED STACK"));
        assert_eq!(
            out.lines().count(),
            STACK.len() + SPEC_BINDS.len() + 2,
            "one line per row and per bind + two headers"
        );
        assert!(out.contains("harness_config.rs"), "the governance projection is on the map");
    }

    // [BOARD: SPEC-BINDS] the audit fold: every bind carries receipts, a Proven
    // bind is anchored in the SoT, and exactly the dauer rung stays open.
    #[test]
    fn spec_binds_carry_receipts_and_dauer_stays_honest() {
        let mut open = 0;
        for b in SPEC_BINDS {
            assert!(!b.spec.is_empty() && !b.live.is_empty() && !b.anchor.is_empty(), "bare bind: {b:?}");
            match b.proof {
                Proof::Proven => assert!(b.anchor.contains("crates/"), "Proven bind without a SoT code anchor: {b:?}"),
                Proof::Unproven => {
                    open += 1;
                    assert!(b.anchor.contains("0 code mentions"), "an open bind must say why: {b:?}");
                }
                // Testimony makes no code claim, so no anchor may be demanded of it.
                Proof::Authored => assert!(b.proof.exempt_from_anchor()),
                Proof::Estimate => {}
            }
        }
        assert_eq!(open, 0, "all four audit binds landed 07-27 — a new phantom name reopens this via the spec-binds miner");
    }

    // [BOARD: SPEC-BINDS] the chapter is the module's live caller (orphan-wire).
    #[test]
    fn stack_atlas_chapter_renders_the_whole_map() {
        let ch = stack_atlas();
        assert_eq!(ch.title(), "Unified Stack & Spec Binds");
        let text = format!("{ch:?}");
        assert!(text.contains("TritTree5D") && text.contains("dauer_state"), "binds ride the chapter");
    }
}
