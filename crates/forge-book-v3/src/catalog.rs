//! Catalog — the living capabilities index. Seeds "this is what I can do" from
//! the engine's real proven surfaces, each with an on-disk receipt.

use crate::atlas::{AtlasSection, CapabilityEntry, CapabilityStatus};

/// The forge capabilities catalog — surfaces this session verified, with receipts.
pub fn forge_capabilities() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    use CapabilityStatus as St;
    vec![
        // The 13forge-studio product surfaces — Sean's final six tabs (2026-07-09
        // "THATS IT"): the 14-in-1 tool's own front door, each a live top-bar
        // Surface the folded panels drain into (forge_vision_lab::Surface).
        CapabilityEntry::proven("Paint surface — voxel-acrylic paint HUD (brush / fill / line / layers)", S::Capabilities, "forge-studio/src/paint_host.rs · Surface::Paint"),
        CapabilityEntry::proven("Create surface — Magic-Canvas 2D editor + Font Sandbox (F5 2D/3D flip)", S::Capabilities, "forge-studio Surface::Create · font_sandbox_kit + create_2d_kit (live 2026-07-09)"),
        CapabilityEntry::proven("Audio surface — 4-deck DJ/DAW workshop (mixer / waveforms / transport)", S::Capabilities, "forge-studio Surface::Audio -> forge_gui::recording_studio_kit (HITL 2026-07-09)"),
        CapabilityEntry::proven("Terminal surface (TKNO) — PS7 terminal spine, stylized glyph grid", S::Capabilities, "technothesia::unified::draw_terminal · Surface::Terminal"),
        CapabilityEntry::proven("Hub surface — Command Hub page-maker (3 portals + 20-chip era vibe rail)", S::Capabilities, "forge-studio Surface::Playground lowers forge-vix/panels/playground.kit.vixi"),
        // Webview UI face folded INTO the binary (2026-07-23): the design HTML
        // panels ship inside 13forge-studio.exe (one-bin law), served from memory.
        // Honest status = Wired: embedded + tests-green, but NOT yet lowered to
        // native primitives / rendered as live surfaces (that fold is pending).
        CapabilityEntry::new("webview UI face — design panels folded INTO the binary as one PIPELINE (bottom→top: substrate→dissect→lift→animate→author→compose→frame→command), each mock mapped to its native .kit.vixi twin, NOT 12 flat panels", S::Capabilities, St::Wired, "forge-studio/src/ui_embed.rs PANEL_FOLD (12 mocks -> registered twins, fact-locked) · ui_embed 4/4 green 2026-07-23"),
        // The genuine gap authored + registered this session — the shared substrate rail.
        CapabilityEntry::proven("palette rail — the persistent 64-material substrate (one pick = one material everywhere: MaterialSelection idx -> material + rgb; palette_idx byte-locked to the sprite-dissector's own palette)", S::Capabilities, "forge-vix/panels/palette.kit.vixi (STUDIO_PANELS + readback proof, forge-vix 323 green) · forge-core::MaterialSelection · palette parity fact-lock all-64 green 2026-07-23"),
        CapabilityEntry::proven("The five legs at one index — colour · material · essence · resonance · SURFACE all resolve from one palette_idx 0..=63 (the fifth leg landed 2026-07-28: 220 PBR sets bound 64/64, 10 by material name and 54 by nearest mean albedo, each slot wearing a DISTINCT set)", S::Capabilities, "forge-materials/src/slot_correspondence.rs::resolve_all + receipt_tsv · texture_registry::bind_all_slots · _book/09-the-five-legs.md -> seed::full_atlas (include_str!) · 43 green -p forge-materials"),
        CapabilityEntry::proven("Texture browser surface — cycle the PBR library, bind a surface to a palette slot", S::Capabilities, "forge-vix/panels/texture_browser.kit.vixi (STUDIO_PANELS 103 + readback proof root.controls.next) · host forge-studio/src/texture_browser_kit.rs (5 green 2026-07-28)"),
        CapabilityEntry::proven("Ghostmoon 5D box primitive (Sean-named, was HyperBox5D)", S::Capabilities, "pp-math/src/ghostmoon.rs · 118 tests -p pp-math green 2026-07-21"),
        CapabilityEntry::proven("formation geometry (thirds / quincunx / superior-dexter / Yod apex / Weyl stride)", S::Capabilities, "pp-math/src/formation.rs · 6/6 release 2026-07-21"),
        CapabilityEntry::proven("R3 SovereignCoder — BQ centroid topic half + thirds-stride wording half", S::Learning, "forge-ml/src/sovereign_coder.rs · 3/3 + r3 4/4 release; live seam repo_query.rs sovereign arm"),
        CapabilityEntry::proven("tas-de-charge vault course-work (rise 1/3, free span 2/3)", S::Capabilities, "forge-tile-crawler/src/architecture/verticality.rs · crate 16/16 release 2026-07-21"),
        CapabilityEntry::proven("folding book / living Atlas", S::Capabilities, "forge-book (this crate)"),
        CapabilityEntry::proven("deterministic fold state-machine", S::Capabilities, "forge-book/src/fold.rs"),
        CapabilityEntry::proven("hash-seal hide/reveal", S::Capabilities, "forge-book/src/seal.rs"),
        CapabilityEntry::proven("grow-with-the-person unlock", S::Learning, "forge-book/src/grow.rs"),
        CapabilityEntry::proven("HTML export (author -> site)", S::Capabilities, "forge-book/src/export_html.rs"),
        CapabilityEntry::proven("sovereign UI floor (integer layout/draw/text)", S::Capabilities, "forge-canvas/src/{draw,layout,text}.rs"),
        CapabilityEntry::proven("8-layer zplane compositor", S::Shaders, "forge-canvas/src/compositor.rs:11"),
        // Honest status = Wired: source reachable + naga-green, but nothing binds
        // CorrosionUniforms to a live pass yet, so no pixel has been drawn by it.
        CapabilityEntry::new("corrosion pass — clean steel -> rust off one corrosion_pct (FBM mask, roughness 0.3->0.9 / metallic 0.9->0.1, CUI-risk tint at >=1.5 and pulsing red border at >=2.5); drained off the airgap quarry 2026-07-28, where it had a live producer and no home in this tree", S::Shaders, St::Wired, "forge-render/shaders/core/corrosion.wgsl · consumer forge-render/src/corrosion.rs (CORROSION_WGSL + CorrosionUniforms, 142 green -p forge-render) · naga gate forge-shader-build/tests/corrosion_naga_gate.rs (1 green; the quarry .spv was a month NEWER than the source, so the binary was never proof it parsed)"),
        CapabilityEntry::proven("VixiScript grammar + sovereign LSP", S::Capabilities, "forge-vix, forge-vix-lsp"),
        CapabilityEntry::proven("dialogue lore codex model", S::Dialogue, "forge-lore/src/codex.rs"),
        CapabilityEntry::proven("Camelot harmonic key wheel", S::Learning, "forge-audio/src/vocal_studio/camelot.rs"),
        CapabilityEntry::proven("integer shaderbind DSL (permyriad signals)", S::Shaders, "forge-gpu/shaderbind_dsl::parse_shaderbind"),
        CapabilityEntry::proven("concept assimilator (8 ForgeAtom families)", S::Learning, "ffi-ui-assimilator-001 (deveraux_mud, exit 0)"),
        CapabilityEntry::new("atlas dialogue authoring (ASP/clingo)", S::Dialogue, St::Planned, "clingo .lp (deveraux_mud walk)"),
        CapabilityEntry::new("sovereign canvas book render", S::Capabilities, St::Wired, "forge-book/src/render.rs (forge-canvas)"),
        CapabilityEntry::new("13-axis governor", S::Learning, St::Study, "Sean design head 2026-07-10"),
        CapabilityEntry::proven("integer sim doctrine (3 clocks, 1 membrane)", S::Capabilities, "forge-book/src/physics.rs"),
        CapabilityEntry::proven("20-bone Mobometric rig", S::Capabilities, "forge-geo / forge-book/src/geometry.rs"),
        CapabilityEntry::proven("deterministic combat resolver", S::Learning, "forge-book/src/combat.rs"),
        CapabilityEntry::proven("quest FSM + objectives", S::Dialogue, "forge-book/src/{fsm,quest}.rs"),
        CapabilityEntry::proven("BFS zone pathfinding", S::Learning, "forge-book/src/pathfind.rs"),
        CapabilityEntry::proven("hash-chained evidence ledger", S::Capabilities, "forge-book/src/evidence.rs (tamper-evident)"),
        CapabilityEntry::proven("procedural zone generator", S::Capabilities, "forge-book/src/zone_gen.rs"),
        CapabilityEntry::proven("markdown import + export round-trip", S::Capabilities, "forge-book/src/{markdown,export_md}.rs"),
        CapabilityEntry::proven("13 Moons — Take Too Much (paranormal chapter, voice-linted 4.54, embedded in the one book)", S::Dialogue, "_book/03-take-too-much.md -> seed::full_atlas (include_str!)"),
        CapabilityEntry::proven("Front Matter — Ward / Signal / Map (register, condensed signal, source index)", S::Capabilities, "_book/{00-front,01-signal,02-map}.md -> seed::full_atlas (include_str!)"),
        CapabilityEntry::proven("Voice Corpus — Threads gold set (linter calibration source)", S::Capabilities, "forge-dialogue/voice-corpus-threads.md -> seed::full_atlas · read live by forge-dialogue/voice_lint.py"),
        CapabilityEntry::proven("Storydrop — 1000-PNG story engine (dwell/blink/McCloud/Cohn/kishotenketsu, haiku pillow shots, style rotation)", S::Capabilities, "_book/04-storydrop-forge.md -> seed::full_atlas · tools/storydrop-forge/storydrop.py (live, 12-frame proof render)"),
        CapabilityEntry::proven("Cree Syllabics — the Star Alphabet (orientation=vowel rotation rule, our 26 glyphs + finals, proportions/tiers, double-oracle sourced, cultural floor honest)", S::Capabilities, "_book/06-cree-syllabics.md -> seed::full_atlas (include_str!) · _plans/cree-syllabics-research-2026-07-18.md"),
        CapabilityEntry::proven("Sphere Pixelizer — integer HEALPix-nested star index (12 faces × Morton quadtree × Mersenne 2^13-1 fold to 5 lanes; npix 196608), indexes the CREE codebook", S::Capabilities, "forge-ml/src/sphere_index.rs (7 tests green) · forge-book/src/sphere_index_chapter.rs -> seed::full_atlas · _plans/sphere-pixelizer-plan-2026-07-18.md"),
        CapabilityEntry::proven("Vixio deterministic tick-reactor", S::Shaders, "crates/vixio (Sean-named 2026-07-10)"),
        CapabilityEntry::proven("Fred sentinel watches every beat", S::Learning, "forge-daemon/src/sentinel.rs:193"),
        CapabilityEntry::proven("Weld ratchet — RON diff parses, plans, applies atomically, runs its own gate, reverts itself on red (oracle1_governor.rs:21 gets its machine)", S::Runbook, "forge-daemon/src/weld.rs 13 green · forge-core/src/line_diff.rs LCS hunks 6 green · `13forge-studio weld [--apply]` dogfooded live both directions"),
        // Semantic 5D codebook (this session) — Cree banks lead, projection refines.
        CapabilityEntry::proven("semantic 5D codebook — Cree banks lead, projection refines (R1 lexical + R2 distributional)", S::Runbook, "forge-ml/src/nearest_neighbor.rs map5_refine · 530 lib green · live door raycast embedding:\"semantic\""),
        CapabilityEntry::new("R3 sovereign-model refinement (learned BQ code)", S::Runbook, St::Wired, "forge-ml nearest_neighbor::SovereignCoder seam · projection+fallback proven · real model forward UNVERIFIED"),
        // Photo→asset + item-geometry lane (2026-07-17): the CUI drain closed.
        CapabilityEntry::proven("photo → relief pipeline — forward Poisson shape-from-shading + watertight-by-construction staircase mesher (bg-cut, stride-as-decimation)", S::Capabilities, "forge-geo/src/relief.rs + sterometric.rs cell-complete walls · photo_to_relief GREEN watertight ×3 (F:/output/photo-relief)"),
        CapabilityEntry::proven("mesh catalog bridge — forge-items part ids -> forge-geo prim recipes (lathe-capable) -> assembled item glb", S::Capabilities, "forge-items/data/mesh_catalog.json + forge-export/src/item_mesh.rs · forge_item_glb sword-42.glb GREEN"),
        CapabilityEntry::proven("mint renders live — items mint emits the sword's glb + preview beside the BLAKE3 fingerprint, same XorShift64 walk as the Item", S::Capabilities, "forge-studio/src/items_tool.rs · item-1-seed-42.glb + preview (F:/output/forge-items)"),
        // Ground-material correspondence norm (2026-07-23): id = the 64-slot palette_idx,
        // so material_id=colour_id=essence_id=resonance_id is one number, never four guesses.
        CapabilityEntry::proven("ground materials wired through correspondence — Smithy 5/5 layers live (parchment/vellum filled), .material.vixi id = material_registry palette_idx (Basalt 19, Sandstone 24), physics/acoustic = material_atom(id) exactly", S::Capabilities, "forge-gpu/src/canvas_renderer.rs::smithy_default_bindings · vixi-corpus/pool/{ground081,gravel030}.material.vixi · cargo test -p forge-gpu smithy 7/7 green 2026-07-23"),
        // Developmental Voxel-Resomorphic Sieve (DV-RS) (2026-07-26)
        CapabilityEntry::proven("Developmental Voxel-Resomorphic Sieve (DV-RS) — integer-deterministic cellular automaton grid for multi-generational spatial rendering and UIUX refinements", S::Capabilities, "_book/08-latent-space-collider.md -> seed::full_atlas (include_str!)"),
        // Singing-terminal route lane (2026-07-28): the PTY decision reaches pixels.
        CapabilityEntry::proven("UMP wired PTY -> vibe — BqRouter's route decision leaves the terminal in both widths: the 16-byte MoM word (UmpWord::from_pty_route, 4th Hamming-separated family) and the scalar margin the pixels see (term.route_margin), quantized against ONE edge so the word and the glow never disagree", S::Capabilities, "forge-core/src/ump.rs FAMILY_PTY_ROUTE 0xA9 + margin_bucket (edge 50 = the live UNCERTAIN threshold at technothesia/src/unified.rs) · forge-gpu/src/shaderbind_dsl.rs SignalSource::TermRouteMargin · technothesia route_vibe · 14 + 19 + 32 green 2026-07-28"),
        // ARCH-015 — THE FAN-OUT LAW (2026-07-26)
        CapabilityEntry::proven("Fan-Out Law — limiting parallel subagent execution to design/location phases and direct inline conductor mutations via receipts-per-token metrics", S::Capabilities, "crates/forge-book/src/tablets/ARCH-015-fan-out-law.md -> seed::full_atlas (include_str!)"),
        // ARCH-016 — THE SOVEREIGN TRIAGE & THE ASPIRE MATRIX (2026-07-26)
        CapabilityEntry::proven("Sovereign Triage — read-only automated intent validation via cargo xtask triage-check cross-referencing aspire.rs and board_status.json", S::Capabilities, "crates/forge-book/src/tablets/ARCH-016-sovereign-triage.md -> seed::full_atlas (include_str!)"),
        // ARCH-017 — THE LATENT-SPACE COLLIDER & LATERAL SYMBIOSIS (2026-07-26)
        CapabilityEntry::proven("Latent-Space Collider — the dual-triage lateral symbiosis of the Machine (internal substrate) and the Ghost (external model lateral weights) via cargo xtask triage-lateral and TritTree5D", S::Capabilities, "crates/forge-book/src/tablets/ARCH-017-latent-space-collider.md -> seed::full_atlas (include_str!)"),
        // ARCH-018 — COMPUTE-AT-REST & EMERGENT TOKENIZATION (2026-07-26)
        CapabilityEntry::proven("Compute-At-Rest — shifting runtime footprint to static author-time compilers (.shaderbind, DFA state-machines, packed VFS cartridges) and async emergent tokenizers", S::Capabilities, "crates/forge-book/src/tablets/ARCH-018-compute-at-rest.md -> seed::full_atlas (include_str!)"),
        // Sovereign PKM and the Autonomous Flywheel (2026-07-26)
        CapabilityEntry::proven("Sovereign PKM & Autonomous Flywheel — background knowledge distillation cascade (7-7-7 structure) driven autonomously by Gemma and ORACLE_B", S::Capabilities, "_book/10-sovereign-pkm-flywheel.md -> seed::full_atlas (include_str!)"),
        // Sovereign Routing Plane and Offline Inference Topology (2026-07-26)
        CapabilityEntry::proven("Sovereign Routing Plane & Offline Inference Topology — multi-tier expert, safety, and consequence routing coupled to in-process local inference and Six-Pattern DAG task orchestration", S::Capabilities, "_book/11-sovereign-routing-topology.md -> seed::full_atlas (include_str!)"),
        // Cosmic Dissonance Kernel & Modality Mapping (2026-07-27)
        CapabilityEntry::proven("Cosmic Dissonance Kernel & Modality Mapping — mapping asymmetric thermodynamic forces, elemental qualities, and alchemical resonance substrates to SoulWord, RoutedUmp, and TritTree5D", S::Capabilities, "_book/17-cosmic-dissonance-kernel.md -> seed::full_atlas (include_str!)"),
        // Fae World Overlay (2026-07-27)
        CapabilityEntry::proven("Fae World Overlay — mid-game folklore pressure layer (12+1) mapping ethical pressures and substrate crafting", S::Capabilities, "_book/18-fae-world-overlay.md -> seed::full_atlas (include_str!)"),
        // Thornhaven — The Thousand-Hour City (2026-08-18)
        CapabilityEntry::proven("Thornhaven — the four-era tutorial city as ROOTLESS' 1000-hour content anchor: corpus inventory (era-variant art, Parish loop, questline, Architect thread), budget frame, W-THORN1-6 work queue", S::Capabilities, "_book/27-thornhaven-thousand-hours.md -> seed::full_atlas (include_str!)"),
        // Foreign-vocab boundary — three tokenizers, one ours (2026-07-26 gauge)
        CapabilityEntry::proven("foreign-vocab boundary — cremantic trit pack (index) / GGUF SPM (resident Gemma) / HF BPE (Teacher) are three separate vocabs; a pretrained embedding matrix is keyed to its own ids, so swapping under borrowed weights is a RETRAIN not a wiring job — Gemma keeps its foreign SPM by law", S::Capabilities, "gemma_engine.rs:76-124 (GGUF tokenizer.ggml.tokens) · infer_thread.rs:908 TIER2_TOKENIZER · repo_query::pack_point_trits -> cremantic · ARCH-018 §5"),
        CapabilityEntry::proven("master.nde decoder emits the tokenizer geometry — decode_to_5d_frames -> Frame [x,y,z,θ_mdeg,w], the same 5 lanes pack_point_trits collapses to 105 trits", S::Capabilities, "crates/forge-ml/src/master_decode.rs:571 · sniff_ext:62 (8 media/symbolic formats)"),
        CapabilityEntry::new("emergent tokenizer TEXT leg — prose -> 5D frames -> trit pack, feeding the .nde ladder (never Gemma). No code home today: sniff_ext has no text format", S::Learning, St::Planned, "ARCH-018 §5 -> master_decode::sniff_ext (gap gauged 2026-07-26)"),
        // READ_LADDER compiled (2026-07-26)
        CapabilityEntry::proven("READ_LADDER gate — PreToolUse search-ladder WALL (Grep/Glob floor 1, door UP=deny naming raycast, RED=allow said loud, NO self-issued override)", S::Capabilities, "crates/forge-ast/src/search_gate.rs + forge-daemon/src/gate.rs::SearchLadder · [BOARD: SEARCH-GATE] · nudge->wall 2026-07-26 · LADDER-OVERRIDE hatch struck + grep_roots retired 2026-07-31"),
        // door aim knob (2026-07-26 — aim is the DOOR's scoping concept, not grep's; raycast aim = declared next)
        CapabilityEntry::proven("door aim — subtree scoping knob, home content_search (query kind=content, the ONE face since grep_roots retired 07-31); unknown params + dead aims LOUD, silent global sweep dead; a capped sweep says NOT_A_NEGATIVE", S::Capabilities, "crates/forge-daemon/src/repo_query.rs::content_search · [BOARD: AIM] green 2026-07-26 · one-face + partial-verdict 2026-07-31"),
        // Offline Arbiter (2026-07-26)
        CapabilityEntry::proven("Offline Arbiter — `13forge-studio arbiter`: stdin candidates -> STANDING-brain infer :13013 bounded, quota-proof, zero resident add (prime-symbiosis Step 5 primary)", S::Capabilities, "crates/forge-studio/src/door_wire.rs::{ask_local,arbiter_contract} · [BOARD: ARBITER-LOCAL] · e2e exit0 63s 2026-07-26"),
    ]
}

/// The subset of a catalog filed under `section`.
pub fn by_section<'a>(caps: &'a [CapabilityEntry], section: &AtlasSection) -> Vec<&'a CapabilityEntry> {
    caps.iter().filter(|c| &c.section == section).collect()
}

/// Count of proven capabilities — the honest headline number.
pub fn proven_count(caps: &[CapabilityEntry]) -> usize {
    caps.iter().filter(|c| c.status == CapabilityStatus::Proven).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_populated_with_receipts() {
        let caps = forge_capabilities();
        assert!(caps.len() >= 12);
        assert!(caps.iter().all(|c| !c.receipt.is_empty()));
        assert!(caps.iter().all(|c| !c.name.is_empty()));
    }

    #[test]
    fn most_are_proven() {
        let caps = forge_capabilities();
        assert!(proven_count(&caps) >= 10);
    }

    #[test]
    fn semantic_codebook_capability_is_indexed() {
        let caps = forge_capabilities();
        assert!(
            caps.iter().any(|c| c.name.contains("semantic 5D codebook")
                && c.section == AtlasSection::Runbook),
            "semantic codebook must be indexed as a Runbook capability"
        );
    }

    #[test]
    fn section_filter_works() {
        let caps = forge_capabilities();
        assert!(!by_section(&caps, &AtlasSection::Shaders).is_empty());
        assert!(!by_section(&caps, &AtlasSection::Dialogue).is_empty());
    }

    #[test]
    fn studio_surfaces_reach_the_atlas_page() {
        // Pixel-forward anti-drift: the five product surfaces must render into
        // the exported HUB page ("This is what I can do"). Path: forge_capabilities
        // -> seed::full_atlas b.index(cap) -> export_html::capabilities_html.
        // Rename a surface entry and this catches it.
        let html = crate::export_html::export_book(&crate::seed::full_atlas("The Opus", "deveraux"));
        for surface in [
            "Paint surface",
            "Create surface",
            "Audio surface",
            "Terminal surface (TKNO)",
            "Hub surface",
        ] {
            assert!(html.contains(surface), "atlas page missing studio surface: {surface}");
        }
    }

    #[test]
    fn item_geometry_lane_reaches_the_atlas_page() {
        // Same anti-drift gate as the studio surfaces: the 2026-07-17 photo→asset
        // + item-geometry rows must render into the exported atlas page.
        let html = crate::export_html::export_book(&crate::seed::full_atlas("The Opus", "deveraux"));
        for cap in ["photo → relief pipeline", "mesh catalog bridge", "mint renders live"] {
            assert!(html.contains(cap), "atlas page missing capability: {cap}");
        }
    }

    #[test]
    fn moons_chapter_capability_is_indexed() {
        // GATE (2026-07-11, "never again"): content added to the book must ALSO be a
        // capability with a receipt — no mechanic without a capability row.
        let caps = forge_capabilities();
        assert!(
            caps.iter().any(|c| c.name.contains("13 Moons") && !c.receipt.is_empty()),
            "13 Moons chapter must be indexed as a capability (no orphan content)"
        );
    }
}
