//! ARCH-tablet chapters — doctrine that cannot drift: each chapter quotes its
//! tablet from encoded data (`include_str!` of crates/forge-book/src/tablets/*.md),
//! never a live `_plans/` read. `_plans` retired ARCHIVE-ONLY 2026-07-22 (Sean
//! ruling): there is no such thing as a live tablet — the canon content is
//! encoded in-crate so the chapter compiles the doctrine in, byte-for-byte.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

const ARCH_000_TABLET: &str = include_str!("tablets/ARCH-000-neuro-hud-is-the-engine.md");
const CODIFY_QUEUE_TABLET: &str = include_str!("tablets/CODIFY-QUEUE.md");
const ARCH_001_TABLET: &str = include_str!("tablets/ARCH-001-the-creation-lifecycle.md");

const ARCH_002_TABLET: &str = include_str!("tablets/ARCH-002-atomic-substrate.md");
const ARCH_003_TABLET: &str = include_str!("tablets/ARCH-003-inference-stack.md");
const ARCH_004_TABLET: &str = include_str!("tablets/ARCH-004-creation-surface.md");
const ARCH_005_TABLET: &str = include_str!("tablets/ARCH-005-sovereignty-security.md");
const ARCH_006_TABLET: &str = include_str!("tablets/ARCH-006-process-ops.md");
const ARCH_007_TABLET: &str = include_str!("tablets/ARCH-007-biological-architecture.md");
const ARCH_008_TABLET: &str = include_str!("tablets/ARCH-008-outside-sot-law.md");
const ARCH_009_TABLET: &str = include_str!("tablets/ARCH-009-two-drums.md");
const ARCH_010_TABLET: &str = include_str!("tablets/ARCH-010-circulatory-authoring.md");
const ARCH_011_TABLET: &str = include_str!("tablets/ARCH-011-crate-collapse-vascular-mods.md");
const ARCH_012_TABLET: &str = include_str!("tablets/ARCH-012-cache-line-uniformity.md");
const ARCH_013_TABLET: &str = include_str!("tablets/ARCH-013-river-watershed.md");
const ARCH_014_TABLET: &str = include_str!("tablets/ARCH-014-intent-axis-sieve.md");
const ARCH_015_TABLET: &str = include_str!("tablets/ARCH-015-fan-out-law.md");
const ARCH_016_TABLET: &str = include_str!("tablets/ARCH-016-sovereign-triage.md");
const ARCH_017_TABLET: &str = include_str!("tablets/ARCH-017-latent-space-collider.md");
const ARCH_018_TABLET: &str = include_str!("tablets/ARCH-018-compute-at-rest.md");

/// Helper to build a generic architecture chapter and extract invariant lines.
fn build_generic_arch_chapter(title: &str, lore: &[&str], tablet_content: &str, path_err_msg: &str) -> Chapter {
    let mut chapter = Chapter::new(title, AtlasSection::Custom("Doctrine".into()));
    for l in lore {
        chapter.add_lore(l.to_string());
    }

    let mut page = Page::new(1);
    for line in tablet_content.lines() {
        let t = line.trim();
        if t.starts_with("[VERIFIED]")
            || t.starts_with("- MUST NOT")
            || t.starts_with("MUST NOT")
            || t.starts_with("**Known truth:**")
            || t.starts_with("##")
            || t.starts_with("- ")
            || t.starts_with("**")
        {
            page.add(Block::text(t.to_string()));
        }
    }
    if page.blocks.is_empty() {
        page.add(Block::text(format!("TABLET SHAPE DRIFTED: no structural lines found in {path_err_msg}")));
    }
    chapter.add_page(page);
    chapter
}

/// ARCH-000 — "The Neuro-HUD is the Engine" (Sean [VERIFIED] 2026-07-04; codified
/// into the Book 2026-07-20). The apex tablet: accommodations are structural
/// invariants, never willpower-dependent surfaces; the user is the environment.
pub fn arch_000_chapter() -> Chapter {
    let mut chapter = Chapter::new(
        "ARCH-000 — The Neuro-HUD is the Engine",
        AtlasSection::Custom("Doctrine".into()),
    );

    // Machine face rows (terse, dense — the invariant mapping is the law).
    chapter.add_lore("PRIME-USER@0 = Sean = Brain A; the engine is Brain A's cognition made structural");
    chapter.add_lore("attention-budget=67k-envelope; switch-cost=Aperture-Law; frustration-threshold=Signal-Law");
    chapter.add_lore("cognitive seam = forge-sieve::cognitive ONLY; nine retired names never revive");

    let mut page = Page::new(1);
    // Human face: the tablet's canon + containment sections, quoted from encoded data.
    for line in ARCH_000_TABLET.lines() {
        let t = line.trim();
        if t.starts_with("[VERIFIED]") || t.starts_with("- MUST NOT") {
            page.add(Block::text(t.to_string()));
        }
    }
    if page.blocks.is_empty() {
        page.add(Block::text(
            "TABLET SHAPE DRIFTED: no [VERIFIED]/MUST-NOT lines found in tablets/ARCH-000-neuro-hud-is-the-engine.md".to_string(),
        ));
    }
    chapter.add_page(page);
    chapter
}

/// CODIFY-QUEUE — RATCHET R3(b) 2026-07-20: laws-never-landed found by the vault
/// archive census, held for Sean's word (sonnet never edits canon). Same encoded
/// idiom as `arch_000_chapter`: the Book quotes the tablet from `include_str!`, never a live read.
pub fn codify_queue_chapter() -> Chapter {
    let mut chapter = Chapter::new(
        "CODIFY-QUEUE — laws never landed",
        AtlasSection::Custom("Doctrine".into()),
    );
    chapter.add_lore("RATCHET R3(b) vault census: ADR/tablet claims vs disk truth, Sean-gated rulings");

    let mut page = Page::new(1);
    for line in CODIFY_QUEUE_TABLET.lines() {
        if line.contains('\t') {
            page.add(Block::text(line.to_string()));
        }
    }
    if page.blocks.is_empty() {
        page.add(Block::text(
            "TABLET SHAPE DRIFTED: no TSV rows found in tablets/CODIFY-QUEUE.md".to_string(),
        ));
    }
    chapter.add_page(page);
    chapter
}

/// ARCH-001 — "The Creation Lifecycle: One Atom, Six Stages" (Sean 2026-07-21,
/// codified as T1 infra from the live pipeline map). The engine spine: every
/// creation op reduces to the shared VixelAtom field across six stages, and SEE
/// (PBR/LUT/bloom over the atom lanes) + HEAR both fall out of that one atom.
/// Same encoded idiom as `arch_000_chapter`: the Book quotes the tablet from `include_str!`, never a live read.
pub fn pipeline_lifecycle_chapter() -> Chapter {
    let mut chapter = Chapter::new(
        "ARCH-001 — The Creation Lifecycle: One Atom, Six Stages",
        AtlasSection::Custom("Doctrine".into()),
    );

    // Machine face (syntactic): the six-stage spine + the trinity + the lanes, dense.
    chapter.add_lore("ATOM = ForgeAtom/VibeBuffer (1 pixel = 1 voxel = 1 atom); colour_id = material_id = essence_id = resonanceID (Rosetta)");
    chapter.add_lore("SIX STAGES = import -> atomize -> modify -> physics -> export -> play; all = ops on ONE atom field, nothing else");
    chapter.add_lore("lanes = material_id.colour_id.normal.bloom.light.coverage.phase; PBR/LUT/bloom = the SEE-read of these lanes; SEE == HEAR");
    chapter.add_lore("2D = z=0 slice of the voxel world; blend = field-CSG OR renderer 8x8 overlay; physics reads the atom, never a flag table");
    chapter.add_lore("TWO CLOCKS / TWO WRITERS: DET-CLOCK(120Hz CPU integer)=atom-truth plane, byte-replayable; CREATIVE-LANE(GPU uncapped)=pixel plane; lock-free bridge forge-hal::TripleBuffer(1-producer/2-consumer); float/wall-clock into DET-CLOCK = firewall breach");
    chapter.add_lore("HEAR speaks MIDI + MIDI2/UMP (forge-harmonics music_speak::ump_codec, forge-ump); resonanceID -> note/voice, SEE==HEAR from the one atom");

    let mut page = Page::new(1);
    // Human face: the tablet's canon + containment, quoted from encoded data.
    for line in ARCH_001_TABLET.lines() {
        let t = line.trim();
        if t.starts_with("[VERIFIED]") || t.starts_with("- MUST NOT") {
            page.add(Block::text(t.to_string()));
        }
    }
    if page.blocks.is_empty() {
        page.add(Block::text(
            "TABLET SHAPE DRIFTED: no [VERIFIED]/MUST-NOT lines found in tablets/ARCH-001-the-creation-lifecycle.md".to_string(),
        ));
    }
    chapter.add_page(page);
    chapter
}

/// ARCH-002 — Atomic Substrate: VixelAtom · Vocabulary · Scoping · Semantic Prim
pub fn arch_002_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-002 — Atomic Substrate: VixelAtom · Vocabulary · Scoping · Semantic Prim",
        &[
            "VixelAtom is the single engine primitive. UI = World = Physics = AST.",
            "1 pixel = 1 atom = 1 vixel. This is the Voxel Principle. It does not bend.",
            "Derived faces (all come FROM the atom): Appearance, Matter, Sound, Semantics, Motion.",
            "Six words name multiple distinct things: spine, sieve, timeline, lane, authority, oracle. Strictly avoid name violation.",
        ],
        ARCH_002_TABLET,
        "tablets/ARCH-002-atomic-substrate.md",
    )
}

/// ARCH-003 — Inference Stack: MCP Door · NDE Ladder · BqRouter · Egress
pub fn arch_003_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-003 — Inference Stack: MCP Door · NDE Ladder · BqRouter · Egress",
        &[
            "One standing HTTP door on :13016, TCP daemon on :13013. Client is ephemeral cattle.",
            "Four inference tiers: T1 Student -> T2 Teacher -> T3 Master -> T4 Oracle.",
            "BqRouter: pre-dispatch classification of keystrokes and terminal bytes (route first).",
            "Procedural generation is author-time + advisory; it MAY write the artifact, it MUST NOT write the tick.",
        ],
        ARCH_003_TABLET,
        "tablets/ARCH-003-inference-stack.md",
    )
}

/// ARCH-004 — Creation Surface: Audio · SCC · Egress · Terminal · Animation · Sand-to-Glass
pub fn arch_004_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-004 — Creation Surface: Audio · SCC · Egress · Terminal · Animation · Sand-to-Glass",
        &[
            "Unified Audio Format: born-symbolic stream synthesized on the 120Hz integer tick.",
            "DAW_NO_AUDIO=1 headless mode mandatory for testing; zero-alloc Sound Gate on hot threads.",
            "TripleBuffer bridges CPU DET-CLOCK and GPU CREATIVE-LANE safely.",
        ],
        ARCH_004_TABLET,
        "tablets/ARCH-004-creation-surface.md",
    )
}

/// ARCH-005 — Sovereignty & Security: Author-Time Axis · VFS Seam · Artifact Intake · Recovery
pub fn arch_005_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-005 — Sovereignty & Security: Author-Time Axis · VFS Seam · Artifact Intake · Recovery",
        &[
            "Sovereignty Axis = Author-time commit boundary (stroke-z, radio, SCC). Clear-store in .vixi.",
            "VFS Write Seam: guest reaches disk only through WASI host write with receipt ticket.",
            "Red-Team posture on artifact intake; zero-trust on all incoming payloads.",
        ],
        ARCH_005_TABLET,
        "tablets/ARCH-005-sovereignty-security.md",
    )
}

/// ARCH-006 — Process & Operations: Lineage · Skills · Daemon Loop · Governance
pub fn arch_006_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-006 — Process & Operations: Lineage · Skills · Daemon Loop · Governance",
        &[
            "Lineage Gate: a quarry asset is UNPROVEN until it builds and tests green in NewRepo.",
            "Node Lineage DAG: records tier + parents + proof in ROADMAP.json, not in chat.",
            "Polish Gate and queried-not-loaded design corpus.",
        ],
        ARCH_006_TABLET,
        "tablets/ARCH-006-process-ops.md",
    )
}

/// ARCH-007 — Biological Architecture (The Vascular Doctrine)
pub fn arch_007_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-007 — Biological Architecture (The Vascular Doctrine)",
        &[
            "Vascular Doctrine: isolated organs survive local unwinding and panics.",
            "The Heart (DET-CLOCK) never shares a lock with a panic-able creative lane.",
            "The Kidneys (decoupled channels/buffers) flush pending data on Drop.",
        ],
        ARCH_007_TABLET,
        "tablets/ARCH-007-biological-architecture.md",
    )
}

/// ARCH-008 — The Outside-SoT Law
pub fn arch_008_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-008 — The Outside-SoT Law",
        &[
            "Outside-SoT Law: any process/artifact living outside .forge/ or F:/NewRepo is presumed stale, poorly written, or unaligned.",
            "Silent failure is the loudest kind of structural betrayal of Brain A by Brain B.",
        ],
        ARCH_008_TABLET,
        "tablets/ARCH-008-outside-sot-law.md",
    )
}

/// ARCH-009 — Two Drums: the Drum is the Tick, and it Must Be Two
pub fn arch_009_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-009 — Two Drums: the Drum is the Tick, and it Must Be Two",
        &[
            "Two Drums: determinism is only provable at the boundary between two orthogonal beats.",
            "Drum-1 (Tick) is integer 120Hz DET-CLOCK for sequences, replay, and order.",
            "Drum-2 (Beat) is f32 wall-clock Creative-Lane for liveness detection.",
        ],
        ARCH_009_TABLET,
        "tablets/ARCH-009-two-drums.md",
    )
}

/// ARCH-010 — The Circulatory System of Authoring (Genre-Agnostic)
pub fn arch_010_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-010 — The Circulatory System of Authoring (Genre-Agnostic)",
        &[
            "Circulatory Authoring: player's intent is blood, ceiling system is the vascular tree.",
            "Heart pumps 16-byte InteractionQuery; genre is a downstream arterial disguise.",
        ],
        ARCH_010_TABLET,
        "tablets/ARCH-010-circulatory-authoring.md",
    )
}

/// ARCH-011 — Crate Collapse: 113 → 10 (Vascular Mod Tree)
pub fn arch_011_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-011 — Crate Collapse: 113 → 10 (Vascular Mod Tree)",
        &[
            "Crate Collapse: fold 113-crate workspace into ~10 core members.",
            "One crate per vascular layer; pub mod and feature gates replace dynamic crate boundaries.",
        ],
        ARCH_011_TABLET,
        "tablets/ARCH-011-crate-collapse-vascular-mods.md",
    )
}

/// ARCH-012 — Cache-Line Uniformity (GPU-Native Layout Streams)
pub fn arch_012_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-012 — Cache-Line Uniformity (GPU-Native Layout Streams)",
        &[
            "Cache-Line Uniformity: lowered VixiScript slot is an identical-stride record.",
            "No heap pointers, no variable-length fields; linear memory = GPU streamable.",
        ],
        ARCH_012_TABLET,
        "tablets/ARCH-012-cache-line-uniformity.md",
    )
}

/// ARCH-013 — THE RIVER + THE WATERSHED (sedimentary context · entropy compaction · flow-as-maintenance)
pub fn arch_013_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-013 — THE RIVER + THE WATERSHED (sedimentary context · entropy compaction · flow-as-maintenance)",
        &[
            "River + Watershed model: write-ahead log / memtable in motion with background compaction.",
            "Data has grain size: silt (<50-byte handle) flows, gravel (whole file) stays at active aperture.",
        ],
        ARCH_013_TABLET,
        "tablets/ARCH-013-river-watershed.md",
    )
}

/// ARCH-014 — THE INTENT-AXIS SIEVE (Layer 0 · 9 Prime Senses · the WHY beneath the fold)
pub fn arch_014_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-014 — THE INTENT-AXIS SIEVE (Layer 0 · 9 Prime Senses · the WHY beneath the fold)",
        &[
            "The Intent-Axis Sieve: why these 10 crates exist to serve the 9 Prime Senses.",
            "Senses: 7 Receptive (Know, Hear, See, Feel, Want, Expect, Valued) + 2 Generative (Make, Own).",
        ],
        ARCH_014_TABLET,
        "tablets/ARCH-014-intent-axis-sieve.md",
    )
}

/// ARCH-015 — THE FAN-OUT LAW
pub fn arch_015_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-015 — THE FAN-OUT LAW",
        &[
            "The Fan-Out Law: mutation contracts only, zero inline reconnaissance fan-out.",
            "Enforces directive limits and the strict receipts-per-token metric across subagents.",
        ],
        ARCH_015_TABLET,
        "tablets/ARCH-015-fan-out-law.md",
    )
}

/// ARCH-016 — THE SOVEREIGN TRIAGE & THE ASPIRE MATRIX
pub fn arch_016_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-016 — THE SOVEREIGN TRIAGE & THE ASPIRE MATRIX",
        &[
            "Sovereign Triage: fusing internal architectural intent with actual on-disk execution.",
            "Features the 30-item capacity limit of the aspire matrix and cargo xtask triage-check.",
        ],
        ARCH_016_TABLET,
        "tablets/ARCH-016-sovereign-triage.md",
    )
}

/// ARCH-017 — THE LATENT-SPACE COLLIDER & LATERAL SYMBIOSIS
pub fn arch_017_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-017 — THE LATENT-SPACE COLLIDER & LATERAL SYMBIOSIS",
        &[
            "The Latent-Space Collider: the closed-loop symbiosis of the Machine (internal) and the Ghost (external).",
            "Features TritTree5D and the three indicator-level lateral-triage pipeline check rules.",
        ],
        ARCH_017_TABLET,
        "tablets/ARCH-017-latent-space-collider.md",
    )
}

/// ARCH-018 — COMPUTE-AT-REST & EMERGENT TOKENIZATION
pub fn arch_018_chapter() -> Chapter {
    build_generic_arch_chapter(
        "ARCH-018 — COMPUTE-AT-REST & EMERGENT TOKENIZATION",
        &[
            "Compute-At-Rest: shifting expensive allocations and parsing entirely to author-time compilers.",
            "Features domain-expert routing and the asynchronous emergent tokenization model.",
        ],
        ARCH_018_TABLET,
        "tablets/ARCH-018-compute-at-rest.md",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anti-drift gate: the chapter must carry the canon verdict line and at
    /// least one containment MUST-NOT, quoted from the encoded tablet — a
    /// gutted/edited tablet fails here LOUD instead of shipping a hollow chapter.
    #[test]
    fn arch_000_chapter_reads_the_encoded_tablet() {
        let ch = arch_000_chapter();
        let text: String = ch.pages[0]
            .blocks
            .iter()
            .map(|b| format!("{b:?}"))
            .collect();
        assert!(
            text.contains("Neuro-HUD") && text.contains("MUST NOT"),
            "chapter must quote the encoded tablet's canon + containment; got: {text}"
        );
    }

    /// Same anti-drift gate for the CODIFY-QUEUE tablet.
    #[test]
    fn codify_queue_chapter_reads_the_encoded_tablet() {
        let ch = codify_queue_chapter();
        let text: String = ch.pages[0]
            .blocks
            .iter()
            .map(|b| format!("{b:?}"))
            .collect();
        assert!(
            text.contains("ADR-0032") && text.contains("tablet"),
            "chapter must quote the encoded CODIFY-QUEUE rows; got: {text}"
        );
    }

    /// Anti-drift gate for the ARCH-001 creation-lifecycle tablet: the chapter must
    /// quote the encoded tablet's canon (the ForgeAtom spine) + at least one MUST-NOT.
    #[test]
    fn pipeline_lifecycle_chapter_reads_the_encoded_tablet() {
        let ch = pipeline_lifecycle_chapter();
        let text: String = ch.pages[0]
            .blocks
            .iter()
            .map(|b| format!("{b:?}"))
            .collect();
        assert!(
            text.contains("ForgeAtom") && text.contains("MUST NOT"),
            "chapter must quote the encoded tablet's canon + containment; got: {text}"
        );
    }

    /// Comprehensive anti-drift and structural integrity test for all newly encoded tablets.
    #[test]
    fn all_tablets_are_encoded_and_valid() {
        let ch_2 = arch_002_chapter();
        let txt_2: String = ch_2.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_2.contains("One Atom") || txt_2.contains("Voxel Principle"), "ARCH-002 loaded incorrectly: {txt_2}");

        let ch_3 = arch_003_chapter();
        let txt_3: String = ch_3.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_3.contains("MCP Door") || txt_3.contains("BqRouter"), "ARCH-003 loaded incorrectly: {txt_3}");

        let ch_4 = arch_004_chapter();
        let txt_4: String = ch_4.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_4.contains("Audio") || txt_4.contains("Sound Gate") || txt_4.contains("TripleBuffer"), "ARCH-004 loaded incorrectly: {txt_4}");

        let ch_5 = arch_005_chapter();
        let txt_5: String = ch_5.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_5.contains("Sovereignty") || txt_5.contains("VFS Write Seam"), "ARCH-005 loaded incorrectly: {txt_5}");

        let ch_6 = arch_006_chapter();
        let txt_6: String = ch_6.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_6.contains("Lineage Gate") || txt_6.contains("DAG"), "ARCH-006 loaded incorrectly: {txt_6}");

        let ch_7 = arch_007_chapter();
        let txt_7: String = ch_7.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_7.contains("Heart") || txt_7.contains("Kidneys") || txt_7.contains("organ"), "ARCH-007 loaded incorrectly: {txt_7}");

        let ch_8 = arch_008_chapter();
        let txt_8: String = ch_8.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_8.contains("Outside-SoT") || txt_8.contains("betrayal"), "ARCH-008 loaded incorrectly: {txt_8}");

        let ch_9 = arch_009_chapter();
        let txt_9: String = ch_9.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_9.contains("Two Drums") || txt_9.contains("MetronomeClock"), "ARCH-009 loaded incorrectly: {txt_9}");

        let ch_10 = arch_010_chapter();
        let txt_10: String = ch_10.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_10.contains("heart") || txt_10.contains("circulatory") || txt_10.contains("InteractionQuery"), "ARCH-010 loaded incorrectly: {txt_10}");

        let ch_11 = arch_011_chapter();
        let txt_11: String = ch_11.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_11.contains("Crate Collapse") || txt_11.contains("Workspace"), "ARCH-011 loaded incorrectly: {txt_11}");

        let ch_12 = arch_012_chapter();
        let txt_12: String = ch_12.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_12.contains("Cache-Line") || txt_12.contains("stride") || txt_12.contains("VixiScript"), "ARCH-012 loaded incorrectly: {txt_12}");

        let ch_13 = arch_013_chapter();
        let txt_13: String = ch_13.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_13.contains("river") || txt_13.contains("silt") || txt_13.contains("beaver"), "ARCH-013 loaded incorrectly: {txt_13}");

        let ch_14 = arch_014_chapter();
        let txt_14: String = ch_14.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_14.contains("Senses") || txt_14.contains("receptive") || txt_14.contains("generative"), "ARCH-014 loaded incorrectly: {txt_14}");

        let ch_15 = arch_015_chapter();
        let txt_15: String = ch_15.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_15.contains("Fan-Out") || txt_15.contains("Mutation") || txt_15.contains("Subagents"), "ARCH-015 loaded incorrectly: {txt_15}");

        let ch_16 = arch_016_chapter();
        let txt_16: String = ch_16.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_16.contains("Sovereign Triage") || txt_16.contains("aspire.rs") || txt_16.contains("board_status.json"), "ARCH-016 loaded incorrectly: {txt_16}");

        let ch_17 = arch_017_chapter();
        let txt_17: String = ch_17.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_17.contains("Ghost") || txt_17.contains("TritTree5D") || txt_17.contains("Indicator"), "ARCH-017 loaded incorrectly: {txt_17}");

        let ch_18 = arch_018_chapter();
        let txt_18: String = ch_18.pages[0].blocks.iter().map(|b| format!("{b:?}")).collect();
        assert!(txt_18.contains("COMPUTATION") || txt_18.contains("TOKENIZATION") || txt_18.contains("CollisionBridge"), "ARCH-018 loaded incorrectly: {txt_18}");
    }
}
