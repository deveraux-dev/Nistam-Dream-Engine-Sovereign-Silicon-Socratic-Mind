//! Golden Vixi — the 16 canon UI/UX exemplars (`crates/scc/golden/vixi/`), encoded.
//!
//! Sean 2026-07-31: "move all of these into canon, encode them, make them loud when
//! we do UIUX". The files were already on disk; what was missing was a machine home,
//! so they read as ore instead of law. Each row carries its dialect, surface, profile,
//! classification and the ONE doctrine line the surface teaches — and the source is
//! bound by `include_str!`, so a row cannot drift from the file it describes (the
//! gates below re-derive the banner/surface/classification from the text itself).
//!
//! This is the LOUD set: [`loud_for_uiux`] is what any UI/UX pass reads before
//! authoring a new surface. See also `.claude/skills/vixi-uiux/SKILL.md`.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// One golden exemplar: what it is, and the law its shape carries.
pub struct Golden {
    /// Repo-relative path — the SoT file.
    pub path: &'static str,
    /// `#vixi:<dialect>` banner word.
    pub dialect: &'static str,
    /// The `surface:` / `name:` this document declares.
    pub surface: &'static str,
    /// The vibe register it inherits (`profile:`), empty for a register itself.
    pub profile: &'static str,
    /// The `classification:` line, empty when the dialect declares none.
    pub classification: &'static str,
    /// The one line a UI/UX pass must carry away from this file.
    pub doctrine: &'static str,
    /// The file, bound at compile time — row and disk cannot drift.
    pub source: &'static str,
}

/// The 16 golden exemplars, grouped kits → renderpasses → shaderbinds → vibes.
pub const GOLDEN: &[Golden] = &[
    Golden {
        path: "crates/scc/golden/vixi/kits/studio.kit.vixi",
        dialect: "kit",
        surface: "studio",
        profile: "forge_smithy",
        classification: "creative_tool_workspace",
        doctrine: "The shell: three-tier vertical stack (window_bar / sub_bar / content / status_bar), four top-level windows CREATE|DJ|PLAY|SHIP. Active state = baseline accent + idle pulse — NEVER fill the active button with accent_primary. No chrome texture bleeds into the canvas region (Editor Surface Clarity). The command palette is discoverable, not pinned.",
        source: include_str!("../../scc/golden/vixi/kits/studio.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/kits/animation_timeline.kit.vixi",
        dialect: "kit",
        surface: "animation_timeline",
        profile: "forge_primeflow",
        classification: "animation_authoring",
        doctrine: "Author motion in TICKS at 120Hz, never seconds — the ruler's unit is the contract. Curve continuity is required, every major action gets an anticipation marker, and movement alone must not determine mass.",
        source: include_str!("../../scc/golden/vixi/kits/animation_timeline.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/kits/audio_vis.kit.vixi",
        dialect: "kit",
        surface: "audio_vis",
        profile: "forge_primeflow",
        classification: "dj_audio_performance",
        doctrine: "A performance surface inverts one default: progressive disclosure is LIMITED, because muscle memory beats discovery at the deck. The mixer holds a fixed position, meters read green/yellow/red (never colour alone), and meter latency is a strict budget.",
        source: include_str!("../../scc/golden/vixi/kits/audio_vis.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/kits/debug_overlay.kit.vixi",
        dialect: "kit",
        surface: "debug_overlay",
        profile: "forge_primeflow",
        classification: "visual_debugging",
        doctrine: "Telemetry rides OVER the work, so it is capped: an overlay may obscure at most 35% (permyriad 3500), sections collapse, data reads mono. Severity is carried by colour AND shape AND label together.",
        source: include_str!("../../scc/golden/vixi/kits/debug_overlay.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/kits/forgewright_cad.kit.vixi",
        dialect: "kit",
        surface: "forgewright_cad",
        profile: "forge_primeflow",
        classification: "cad_gpu_artifact_proof",
        doctrine: "A proof surface proves: real GPU pixels, a perceptual hash taken FROM the render (not from the source), primitive provenance either known or flagged, mesh manifold. Split view — viewport beside the evidence, never instead of it.",
        source: include_str!("../../scc/golden/vixi/kits/forgewright_cad.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/kits/material_registry.kit.vixi",
        dialect: "kit",
        surface: "material_registry",
        profile: "forge_primeflow",
        classification: "voxel_item_material_slider_authoring",
        doctrine: "Sliders edit PROFILES, never hot-path runtime. A voxel stores a material_id, never a behaviour blob; lookup is O(1) with no runtime string lookup and no hot-path allocation. Every material carries an accessibility profile — it is a required column, not a setting.",
        source: include_str!("../../scc/golden/vixi/kits/material_registry.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/kits/ritual_typography.kit.vixi",
        dialect: "kit",
        surface: "ritual_typography",
        profile: "forge_primeflow",
        classification: "voxel_font_sigil_accessible_ui",
        doctrine: "Colour alone is FORBIDDEN for meaning. Glyphs are layered material_ids, not flat ink. UCAS coverage is required for Cree/Dene syllabics, ASL is gesture notation and not a font, and every surface ships a motion-reduction fallback and a readable name.",
        source: include_str!("../../scc/golden/vixi/kits/ritual_typography.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/kits/vfx_post.kit.vixi",
        dialect: "kit",
        surface: "vfx_post",
        profile: "forge_primeflow",
        classification: "vfx_vfz_visual_effects",
        doctrine: "The one place float is allowed is the visual-only lane — and the price is that it may NEVER mutate authoritative state. Silhouette is preserved through the whole post chain; temporal popping is rejected outright, not tuned down.",
        source: include_str!("../../scc/golden/vixi/kits/vfx_post.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/kits/vision_review.kit.vixi",
        dialect: "kit",
        surface: "vision_review",
        profile: "forge_primeflow",
        classification: "dream_maker_cv_review",
        doctrine: "Four refusals a review surface must show, not assume: a mask is not semantic truth, material needs multi-cue evidence, a shadow is not geometry, and colour is not material. Quad view — candidate beside evidence beside its friction warnings.",
        source: include_str!("../../scc/golden/vixi/kits/vision_review.kit.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/renderpasses/forgewright_proof.renderpass.vixi",
        dialect: "renderpass",
        surface: "forgewright_cad",
        profile: "forge_primeflow",
        classification: "",
        doctrine: "Every pass declares its millisecond budget up front (1+4+2+1+2 = 10ms). Real GPU pixels, a stable phash, and no shader compile in the hot path — the proof chain is load → render → readback → hash → diff.",
        source: include_str!("../../scc/golden/vixi/renderpasses/forgewright_proof.renderpass.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/renderpasses/udle_art.renderpass.vixi",
        dialect: "renderpass",
        surface: "udle",
        profile: "forge_primeflow",
        classification: "",
        doctrine: "Procedural art is deterministic art: a seed is required, hashing is PCG or another deterministic hash, platform RNG is banned, and the pass may not mutate authoritative state. Capture and review are optional passes — the determinism is not.",
        source: include_str!("../../scc/golden/vixi/renderpasses/udle_art.renderpass.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/shaderbinds/audio_vis.shaderbind.vixi",
        dialect: "shaderbind",
        surface: "audio_vis",
        profile: "forge_primeflow",
        classification: "",
        doctrine: "Five permyriad signals (rms, beat_phase, spectral_centroid, crossfader, pen pressure) land on vibematrix channels 0..4. Audio is NOT identity — it modulates the picture and never decides what a thing is.",
        source: include_str!("../../scc/golden/vixi/shaderbinds/audio_vis.shaderbind.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/shaderbinds/udle_vibematrix.shaderbind.vixi",
        dialect: "shaderbind",
        surface: "udle",
        profile: "forge_primeflow",
        classification: "",
        doctrine: "The full eight-channel vibe matrix: audio, world and input all arrive as permyriad 0..10000, never floats. Shader compile in the hot path is forbidden, and the visual-only lane may not mutate authority.",
        source: include_str!("../../scc/golden/vixi/shaderbinds/udle_vibematrix.shaderbind.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/vibes/forge_primeflow.vibe.vixi",
        dialect: "vibe",
        surface: "forge_primeflow",
        profile: "forge",
        classification: "",
        doctrine: "The engine-default register every kit inherits unless it says otherwise: the canonical palette slots, comfy density, snap 220/12 capped at 240ms, a five-step type ramp, golden-ratio accent bias. It closes on the four guardrails — colour alone must not determine material, music alone must not determine identity, movement alone must not determine mass, flow optimisation must not erase style.",
        source: include_str!("../../scc/golden/vixi/vibes/forge_primeflow.vibe.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/vibes/forge_smithy.vibe.vixi",
        dialect: "vibe",
        surface: "forge_smithy",
        profile: "forge",
        classification: "workshop_register",
        doctrine: "A working smithy at dusk — warm-dominant, ember as the FOCAL POINT and not a highlight. Chrome is heavier than primeflow and motion settles with mass (200/11, 280ms cap). Texture amplitude is tiered by surface context: 2% over canvas (load-bearing, protects pixel work), 5% floating, 12% audio-reactive peak, 8% event burst, 80% showcase. Atmosphere over canvas content is forbidden; active-state fill-invert is forbidden.",
        source: include_str!("../../scc/golden/vixi/vibes/forge_smithy.vibe.vixi"),
    },
    Golden {
        path: "crates/scc/golden/vixi/vibes/rune_palettes.vibe.vixi",
        dialect: "vibe",
        surface: "rune_palettes",
        profile: "forge_ritual_typography",
        classification: "",
        doctrine: "Three sigil registers (ember_forge, deep_jade, storm_sigil), five layered slots each. Palettes are layered materials, and no palette may carry meaning by colour alone.",
        source: include_str!("../../scc/golden/vixi/vibes/rune_palettes.vibe.vixi"),
    },
];

/// Rows for one dialect (`kit`, `renderpass`, `shaderbind`, `vibe`).
pub fn by_dialect(dialect: &str) -> Vec<&'static Golden> {
    GOLDEN.iter().filter(|g| g.dialect == dialect).collect()
}

/// Look one exemplar up by its declared surface + dialect.
pub fn find(dialect: &str, surface: &str) -> Option<&'static Golden> {
    GOLDEN.iter().find(|g| g.dialect == dialect && g.surface == surface)
}

/// **The loud set.** What a UI/UX pass reads before authoring any new surface:
/// one `path — doctrine` line per exemplar, mirroring order. Print this, don't
/// paraphrase it.
pub fn loud_for_uiux() -> Vec<String> {
    GOLDEN.iter().map(|g| format!("{} — {}", g.path, g.doctrine)).collect()
}

/// Read a `key:` header value straight out of a golden source (the drift check).
/// Test-only: every caller lives in the drift-check tests below.
#[cfg(test)]
fn header_value(src: &str, key: &str) -> Option<String> {
    src.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(':').map(|v| v.trim().to_string()))
}

/// Build the "Golden Vixi" chapter — the 16 canon UI/UX exemplars and the law each
/// one carries, grouped by dialect.
pub fn golden_vixi_chapter() -> Chapter {
    let mut chapter = Chapter::new("Golden Vixi", AtlasSection::Custom("Design".into()));
    chapter.add_lore(
        "Sixteen authored surfaces under crates/scc/golden/vixi are the canon UI/UX \
         exemplars. Mirror them; do not invent a shape they already answer. Every one \
         is bound into this chapter by include_str!, so the file is the source of truth \
         and this page cannot drift from it.",
    );

    let mut page_no = 1;
    for dialect in ["kit", "renderpass", "shaderbind", "vibe"] {
        let rows = by_dialect(dialect);
        let mut page = Page::new(page_no);
        page.add(Block::text(format!("{} — {} golden exemplar(s):", dialect, rows.len())));
        for g in rows {
            page.add(Block::text(format!("  {} [{}] — {}", g.surface, g.path, g.doctrine)));
        }
        chapter.add_page(page);
        page_no += 1;
    }

    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_golden_exemplars_in_four_dialects() {
        assert_eq!(GOLDEN.len(), 16);
        assert_eq!(by_dialect("kit").len(), 9);
        assert_eq!(by_dialect("renderpass").len(), 2);
        assert_eq!(by_dialect("shaderbind").len(), 2);
        assert_eq!(by_dialect("vibe").len(), 3);
    }

    /// The anti-drift gate: every declared field is re-derived from the bound file.
    /// If someone edits a golden `.vixi`, this fails rather than letting the codex lie.
    #[test]
    fn every_row_is_re_derived_from_its_own_source() {
        for g in GOLDEN {
            assert!(!g.source.is_empty(), "{} is empty", g.path);
            let banner = g.source.lines().next().unwrap_or_default().trim();
            assert_eq!(
                banner,
                format!("#vixi:{} v1", g.dialect),
                "{} banner disagrees with its declared dialect",
                g.path
            );
            // kits/renderpasses/shaderbinds declare `surface:`; a vibe register declares `name:`.
            let declared = header_value(g.source, "surface")
                .or_else(|| header_value(g.source, "name"))
                .unwrap_or_default();
            assert_eq!(declared, g.surface, "{} surface disagrees", g.path);
            assert_eq!(
                header_value(g.source, "profile").unwrap_or_default(),
                g.profile,
                "{} profile disagrees",
                g.path
            );
            assert_eq!(
                header_value(g.source, "classification").unwrap_or_default(),
                g.classification,
                "{} classification disagrees",
                g.path
            );
            assert!(!g.doctrine.is_empty(), "{} carries no doctrine line", g.path);
        }
    }

    #[test]
    fn the_loud_set_is_one_line_per_exemplar() {
        let loud = loud_for_uiux();
        assert_eq!(loud.len(), GOLDEN.len());
        assert!(loud.iter().all(|l| l.contains(" — ")));
        // the three loudest calls a UI pass gets wrong
        let all = loud.join("\n");
        assert!(all.contains("Colour alone is FORBIDDEN for meaning"));
        assert!(all.contains("NEVER fill the active button with accent_primary"));
        assert!(all.contains("permyriad"));
    }

    #[test]
    fn find_reaches_the_shell_and_its_register() {
        let studio = find("kit", "studio").expect("studio kit");
        assert_eq!(studio.profile, "forge_smithy");
        let smithy = find("vibe", "forge_smithy").expect("smithy register");
        assert_eq!(smithy.classification, "workshop_register");
        assert!(smithy.source.contains("texture_amplitude_chrome_over_canvas"));
        assert!(find("kit", "no_such_surface").is_none());
    }

    #[test]
    fn the_chapter_carries_every_exemplar() {
        let ch = golden_vixi_chapter();
        assert_eq!(ch.title(), "Golden Vixi");
        assert_eq!(ch.page_count(), 4);
        let text: String = ch
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\n");
        for g in GOLDEN {
            assert!(text.contains(g.path), "chapter missing {}", g.path);
        }
    }
}
