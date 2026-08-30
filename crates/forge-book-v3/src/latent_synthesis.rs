//! Latent Synthesis — the three 5D wormholes (Sean 2026-07-27).
//!
//! Not aspirations in prose: each row is a MEASURED trajectory through the 5D
//! index `[x, y, z, theta, w]` between two organs that already exist, with the z
//! band it was found in. The claim each one makes is that the abstraction layer
//! BETWEEN those organs is the waste — routing IPC, UI layout, SQLite history and
//! markdown spec through integer-exact coordinates turns a static directory into
//! an executable spatial mesh.
//!
//! Grounding for the set: 1,226 scanned files, 659 filtered duplicate shadows.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// One latent wormhole: where it was found in z, what it joins, and what it buys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Synthesis {
    /// Stable id — the board/aspire row name.
    pub id: &'static str,
    /// Human-readable title for this wormhole.
    pub title: &'static str,
    /// The z band the trajectory runs through, entry first. A third value = the
    /// hole lands in live source (`crates/`, z=1).
    pub z: &'static [i64],
    /// Source organ (the ray's origin).
    pub from: &'static str,
    /// Target organ (the ray's landing).
    pub to: &'static str,
    /// HOW — the coordinate mechanism, never a metaphor.
    pub mechanism: &'static str,
    /// WHAT IT BUYS — the capability that only exists once the layer is gone.
    pub impact: &'static str,
}

/// The three latent synthesis wormholes, in the order Sean named them.
pub const SYNTHESES: &[Synthesis] = &[
    Synthesis {
        id: "ACOUSTIC-CALLIGRAPHY",
        title: "Acoustic Calligraphy",
        z: &[212_992, 147_456],
        from: "vixi/dream_pipeline_panel.kit.vixi",
        to: "crates/dream_channel.rs",
        mechanism: "Marquee drags and vector strokes convert straight to 5D spatial coordinates [x,y,z] \
                    — no DOM, no event-callback tree between the canvas and the DSP.",
        impact: "The audio engine reads visual layout geometry as physical boundary conditions: dragging a \
                 panel deforms the simulated acoustic chamber live, sweeping ITD and the room impulse \
                 response inside dream_bridge.rs.",
    },
    Synthesis {
        id: "REPLAY-PHASE-LOCK",
        title: "Cognitive Replay Phase-Lock Loop",
        z: &[98_304, 229_376],
        from: "crates/dream_journal_query.rs",
        to: "crates/dream_wire.rs",
        mechanism: "Time-series execution logs in SQLite map to the wrapping angular coordinate \
                    theta in [0, 360000 mdeg) — history becomes an axis, not a table.",
        impact: "The orchestrator acts as a PLL: scrubbing theta lets forge-hal's lock-free TripleBuffer \
                 interpolate and replay past execution deltas, and branch new deterministic paths with \
                 zero floating-point drift.",
    },
    Synthesis {
        id: "SELF-HEAL-WORMHOLE",
        title: "Spec-to-Code Self-Healing Wormhole",
        z: &[114_688, 180_224, 1],
        from: "spec_sheets/dream-worker-methodology.md",
        to: "crates/ (via spec_sheets/Nistram_Dream_Engine)",
        mechanism: "The concept-lexicon parser reads architectural requirements out of the markdown spec; \
                    a compile pass that flags a missing symbol fires a 5D vector search across the airgap \
                    archive (E:/.airgap) for the closest structural match.",
        impact: "Tractor-pull the match into crates/ and verify the integration through a local cargo test \
                 gate before anything commits — an automated recovery cycle, gate-first.",
    },
];

/// `z=<a>-><b>` (or `-><c>` for a landing in live source).
fn z_row(s: &Synthesis) -> String {
    s.z.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("->")
}

/// Bind the three wormholes into a chapter: terse lore rows + one page carrying
/// the mechanism/impact pair per row (the state_board idiom — rows, not prose).
pub fn latent_synthesis_chapter() -> Chapter {
    let mut ch = Chapter::new("Latent Synthesis — The Three Wormholes", AtlasSection::Capabilities);
    ch.add_lore(
        "5D [x,y,z,theta,w] trajectories between organs that already exist. Grounding: 1,226 scanned \
         files, 659 filtered duplicate shadows. The abstraction layer between the two ends is the waste.",
    );
    for s in SYNTHESES {
        ch.add_lore(format!("{} z={} {} -> {}", s.id, z_row(s), s.from, s.to));
    }

    let mut page = Page::new(1);
    for s in SYNTHESES {
        page.add(Block::text(format!("## {} ({})\n\nz={}  {} -> {}", s.title, s.id, z_row(s), s.from, s.to)));
        page.add(Block::text(format!("MECHANISM: {}", s.mechanism)));
        page.add(Block::text(format!("IMPACT: {}", s.impact)));
    }
    ch.add_page(page);
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_wormholes_each_naming_two_real_ends() {
        assert_eq!(SYNTHESES.len(), 3);
        for s in SYNTHESES {
            assert!(s.z.len() >= 2, "{} has no trajectory", s.id);
            assert!(!s.from.is_empty() && !s.to.is_empty(), "{} is missing an end", s.id);
            assert!(s.mechanism.len() > 40 && s.impact.len() > 40, "{} is a slogan, not a spec", s.id);
        }
        // The self-healing hole is the only one that lands in live source.
        assert_eq!(SYNTHESES[2].z.last(), Some(&1));
    }

    #[test]
    fn the_chapter_carries_every_row_and_reaches_the_one_book() {
        let ch = latent_synthesis_chapter();
        assert_eq!(ch.lore_count(), 4, "header + 3 trajectories");
        let text: String = ch.pages[0].blocks.iter().map(|b| b.as_plain()).collect::<Vec<_>>().join("\n");
        for s in SYNTHESES {
            assert!(text.contains(s.id), "{} missing from the page", s.id);
        }
        let html = crate::export_html::export_book(&crate::seed::full_atlas("The Opus", "deveraux"));
        assert!(html.contains("ACOUSTIC-CALLIGRAPHY"), "wormholes did not reach the exported one book");
    }
}
