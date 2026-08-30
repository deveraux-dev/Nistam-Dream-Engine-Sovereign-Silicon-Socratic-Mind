//! v1_fold_map — THE ROAD TO V1 (Sean 2026-08-05, drained from the intent doc the
//! same night the web-frame launcher shipped). V1 = the BLANK SLATE console: Sean's
//! forge stays his; creators get 3 clicks to their own. Machine data only — edit
//! here, never re-derive from prose. Harvested into `seed::full_atlas` beside
//! `one_engine` so ray + board + book align.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// The V1 sentence, verbatim intent: ship something that builds itself with the
/// user's intent while Sean builds his own — his narrative is not the product,
/// the forge is.
pub const V1_MEANS: &str = "V1 = blank-slate sovereign console: 3 clicks to create, \
every capability folded as a facet before release; Sean's room is the pack-in, \
never the platform.";

/// One of the five launcher pillars (CLICK 1). Resonance-bound per the substrate
/// law's 4 axes — the atom TYPE is owned by forge-core#atom-substrate (8/28/18/64B
/// ladder, resonance≠essence); the intent doc's 8B "ForgeAtom" sketch DEFERS to
/// that owner (drift ruled 2026-08-05: no new struct, the ladder stands).
pub struct Pillar {
    /// Pillar category: PICTURE, SOUND, WORLD, BOOK, or SHARE.
    pub name: &'static str,
    /// What this pillar encompasses and enables.
    pub covers: &'static str,
    /// Resonance frequency in Hz associated with this pillar.
    pub resonance_hz: u16,
    /// Material resonance type (glass, crystal, stone, wood, metal).
    pub material: &'static str,
    /// Core essence identifier for this pillar's function.
    pub essence: &'static str,
}

/// CLICK 1 — the five story pillars, ages 6..60, zero jargon.
pub const PILLARS: &[Pillar] = &[
    Pillar { name: "PICTURE", covers: "2D art · 3D paint · 2.5D photo-pop · canva layout · web graphics", resonance_hz: 220, material: "glass", essence: "canvas" },
    Pillar { name: "SOUND", covers: "music · live DJ · DSP · voice synth · sound FX", resonance_hz: 330, material: "crystal", essence: "audio_graph" },
    Pillar { name: "WORLD", covers: "3D games · 2D games · voxel maps · toybox · atlas", resonance_hz: 440, material: "stone", essence: "voxel_grid" },
    Pillar { name: "BOOK", covers: "lore · authoring · coding IDE (neuro-hud, singing terminal) · themes", resonance_hz: 550, material: "wood", essence: "code_ast" },
    Pillar { name: "SHARE", covers: "youtube-forge · the drop · export game/app · HTML5/webgpu · offline mirror", resonance_hz: 660, material: "metal", essence: "media_stream" },
];

/// CLICK 2 — the three sparks. CLICK 3 is launch; there is no click 4.
pub const SPARKS: &[&str] = &["blank_page", "guided_starter", "story_sandbox"];

/// One capability family that MUST be folded (wired facet, not feature) before V1.
/// `home` = the owner already on disk (EXISTS != REACHABLE — the fold is the wire);
/// `wave` = 0.3..=0.9 minor; `proof` = what green means.
pub struct Fold {
    /// Capability family name.
    pub family: &'static str,
    /// Where it lives in the codebase (crate/module).
    pub home: &'static str,
    /// Version number (0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0).
    pub wave: &'static str,
    /// What constitutes proof this fold is complete.
    pub proof: &'static str,
}

/// THE FOLD LIST — everything folds in before the slate goes blank.
pub const FOLDS: &[Fold] = &[
    // 0.3 — THE DOOR HOLDS
    Fold { family: "launcher/3-click gateway", home: "forge-studio::web_frame + ui/launcher.html", wave: "0.3", proof: "pillar->spark->launch crossing captured, one process" },
    Fold { family: "paint chrome (prototype face)", home: "forge-vix::panels/paint_chrome.kit.vixi", wave: "0.3", proof: "wired live over dual_loop rail, side column clean" },
    // 0.4 — THE ENVELOPE
    Fold { family: "viewport ladder goldens", home: "forge-studio::visual_gate + forge-vix::emit_layout5d", wave: "0.4", proof: "layout5d manifest + phash per rung, all kits" },
    Fold { family: "vixi themes (molten/permafrost/swamp)", home: "forge-vix::tokens + themes/*.sheet.vixi", wave: "0.4", proof: "one kit re-skinned by token swap only" },
    Fold { family: "canva layout / HTML5 emit", home: "forge-vix::emit_html + forge-studio::page_layout", wave: "0.4", proof: "authored kit -> self-contained page, parity gate" },
    // 0.5 — THE HANDS
    Fold { family: "controller (pad tape)", home: "forge-input::gamepad + sovereign_window poll", wave: "0.5", proof: "replay parity + p99 photon on HUD" },
    Fold { family: "MIDI in -> HarmonicEvent", home: "forge-audio::game_midi + forge-harmonics", wave: "0.5", proof: "note in -> sound out, taped" },
    Fold { family: "resonance-bound UI (zero silent elements)", home: "forge-gpu::shaderbind_dsl + forge-harmonics", wave: "0.5", proof: "hover drone + click pulse on launcher, latency bar met" },
    Fold { family: "brush synesthesia (gesture->sound/colour, FFT->hue, material->acoustic)", home: "forge-core::gesture_brush + forge-gui::{brush_engine,audio_brush} + *.brush.vixi panels", wave: "0.5", proof: "one stroke heard, coloured and rung from the same atom axes" },
    Fold { family: "harmonics ONE HOME (Sean 08-05 'theory window, terminal, midi 1.0/2.0, 1 home I can fuck with it')", home: "`harmonics` verb (landed 08-05) + theory_panel + technothesia terminal + forge-ump UMP spine", wave: "0.5", proof: "gesture/hue/ring knobs playable live in ONE desk, MIDI in/out both speak them" },
    // 0.6 — THE TAPE
    Fold { family: "session record/replay/verify", home: "forge-input tapes + forge-core::lockstep + forge-vision", wave: "0.6", proof: "same tape -> same frame hash in CI" },
    Fold { family: "vision QA automation", home: "forge-vision (forgewright) + f3 harness", wave: "0.6", proof: "recorded flow replayed against live window" },
    // 0.7 — THE ONE ROOM
    Fold { family: "world/room creator loop", home: "forge-zones + `room place` + forge-game-systems", wave: "0.7", proof: "a stranger builds a room start-to-finish" },
    Fold { family: "atlas/lore/cartography", home: "forge-book::cartography + lore", wave: "0.7", proof: "room shows on atlas, lore attached" },
    Fold { family: "2D/3D game lanes", home: "forge-tile-crawler + forge-game-systems + ironroot", wave: "0.7", proof: "one playable slice each, tape-verified" },
    // 0.8 — THE BOOK
    Fold { family: "authoring/IDE (egui text lane)", home: "forge-book editor + forge-tui (law vixi-t1 EGUI-EXCEPTION)", wave: "0.8", proof: "devlog script written inside the studio" },
    Fold { family: "singing terminal", home: "technothesia + forge-studio::termi_launch", wave: "0.8", proof: "terminal sings in the BOOK pillar" },
    Fold { family: "build-your-own-model + distillation", home: "forge-ml (nde ladder, distill queue) + forge-daemon::tiers", wave: "0.8", proof: "user-trained student.nde answers in-studio" },
    // 0.9 — THE CARTRIDGE
    Fold { family: "media/multimedia export", home: "forge-export + pipe1000 + mp3_sovereign + glb_writer", wave: "0.9", proof: "png/gif/mp4/glb/midi/mp3 from one source" },
    Fold { family: "youtube-forge + the drop", home: "youtube-forge pipeline + storydrop reel engine", wave: "0.9", proof: "one devlog produced end-to-end in-studio" },
    Fold { family: "cartridge seal/share", home: "forge-cart-* + Ship door", wave: "0.9", proof: "cart runs on a second machine, hash-verified" },
    // 1.0 — THE CONSOLE
    Fold { family: "blank slate + envelope spec", home: "the launcher + published ladder law", wave: "1.0", proof: "someone else's narrative on the machine" },
];

/// Build the chapter — harvested by `seed::full_atlas`.
pub fn v1_fold_atlas() -> Chapter {
    let mut ch = Chapter::new("Road To V1", AtlasSection::Custom("Architecture".into()));
    ch.add_lore(V1_MEANS);
    let mut pillars = Page::new(1);
    pillars.add(Block::text("CLICK 1 — five pillars (then spark, then launch; no click 4):"));
    for p in PILLARS {
        pillars.add(Block::text(format!(
            "  {} ({}Hz {}, essence {}) — {}",
            p.name, p.resonance_hz, p.material, p.essence, p.covers
        )));
    }
    pillars.add(Block::text(format!("CLICK 2 — sparks: {}", SPARKS.join(" · "))));
    ch.add_page(pillars);
    let mut folds = Page::new(2);
    folds.add(Block::text("THE FOLD LIST — wave · family [home] — proof bar:"));
    for f in FOLDS {
        folds.add(Block::text(format!("  {} · {} [{}] — {}", f.wave, f.family, f.home, f.proof)));
    }
    ch.add_page(folds);
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_pillars_three_sparks_no_click_four() {
        assert_eq!(PILLARS.len(), 5, "the launcher deck is 5 cards (gate card_count_max)");
        assert_eq!(SPARKS.len(), 3);
    }

    #[test]
    fn every_fold_names_a_wave_and_a_proof() {
        for f in FOLDS {
            assert!(matches!(f.wave, "0.3" | "0.4" | "0.5" | "0.6" | "0.7" | "0.8" | "0.9" | "1.0"), "{} wave {}", f.family, f.wave);
            assert!(!f.proof.is_empty() && !f.home.is_empty());
        }
    }

    #[test]
    fn atlas_chapter_builds() {
        let ch = v1_fold_atlas();
        assert_eq!(ch.pages.len(), 2);
    }
}
