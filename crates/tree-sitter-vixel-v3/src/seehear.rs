//! seehear — the canonical `NodeKind -> (colour ⊕ sound)` vocabulary.
//!
//! THE THESIS (Sean 2026-06-16): "people who create code with intent — not a wall
//! of text, but colours and sounds." This module is the foundation: every CST node
//! kind maps to BOTH a material weight (what it LOOKS like) and a musical note (what
//! it SOUNDS like). Parse source -> walk the [`crate::Cst`] -> each node emits its
//! voice. The CST *is* the score: you see AND hear the shape of your logic.
//!
//! ## Why this lives here (zero-dep, integer-only)
//! [`crate`] is the zero-alloc CST parser that GBNF-constrained AI output parses
//! into — so it is the ONE place that owns "what a node IS". Keeping the voice table
//! here keeps the crate zero-dependency: it emits integer IDs, never concrete colours
//! or frequencies. Downstream:
//!   * `material_id` aligns with `forge_gui::vixel_highlight::VixelMaterial`
//!     (Void=0 Structural=1 Control=2 Immutable=3 Kinetic=4 Acoustic=5); the actual
//!     COLOUR resolves from the authored `TokenSheet` (`VixelMaterial::token`) — Rust
//!     assigns the semantic WEIGHT, VixiScript authors the colour. [[render-shade-authored-in-vixiscript-not-rust]]
//!   * `note` is a MIDI number; the actual FREQUENCY resolves via
//!     `forge_harmonics::scale_voice::note_to_mhz` (integer mHz).
//!   * `voice` aligns with `forge_harmonics::scale_voice::VoicePreset` (Glass=0 Reed=1 Hearth=2).
//!
//! ## The consonance guarantee
//! Every non-silent note is drawn from the C-major pentatonic (pitch class ∈
//! {0,2,4,7,9} = C D E G A). A pentatonic has no semitone clashes, so ANY CST —
//! any code, any AI output — renders to a consonant chord *by construction*. Code
//! cannot sound ugly. The pitch also RISES with activity: definitions sit low
//! (foundation), values mid, conditions/actions high (bright) — so you literally
//! hear a file's structure climb from its declarations to its logic.

use crate::NodeKind;

// Material weights (align with VixelMaterial discriminants)
/// Base text / values — the canvas truth.
pub const MAT_VOID: u8 = 0;
/// Braces, separators, refs — the grid anchors.
pub const MAT_STRUCTURAL: u8 = 1;
/// Keywords / branches — load-bearing control flow.
pub const MAT_CONTROL: u8 = 2;
/// Definitions / types — the immutable foundation.
pub const MAT_IMMUTABLE: u8 = 3;
/// Actions / colour values — kinetic, interactive heat.
pub const MAT_KINETIC: u8 = 4;
/// Comments / timing — passive acoustic resonance.
pub const MAT_ACOUSTIC: u8 = 5;
// ── Fine-grain bands (append-only; 0..=5 above stay stable for the live
// forge-gui highlighter). These let distinct node kinds carry distinct hues
// instead of collapsing onto the six coarse bands. Colour still resolves
// downstream from the authored TokenSheet (`VixelMaterial::token`).
/// Numeric / tuned values — integers, permyriad. A tuned value pops.
pub const MAT_VALUE: u8 = 6;
/// Invocations — function calls, then-clauses. Kinetic action.
pub const MAT_ACTION: u8 = 7;
/// Conditions — comparisons, when-clauses. The question.
pub const MAT_QUERY: u8 = 8;
/// References — identifiers, properties, arrays. Named bindings.
pub const MAT_REFERENCE: u8 = 9;
/// Parse faults — Error nodes. Must SCREAM (never dim / grey).
pub const MAT_FAULT: u8 = 10;

// Voice presets (align with VoicePreset)
/// Crystalline bell — bright, for values + logic.
pub const VOICE_GLASS: u8 = 0;
/// Breath/reed — mid, for structure + refs.
pub const VOICE_REED: u8 = 1;
/// Warm body — low, for definitions + the file drone.
pub const VOICE_HEARTH: u8 = 2;

/// Silence — `note == 0` means a node is rendered but emits no pitch (errors).
pub const SILENCE: u8 = 0;

/// The look + sound of one CST node. Pure integer: colour and frequency resolve
/// downstream from the authored sheet / pitch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeVoice {
    /// Material weight — index into `VixelMaterial` (0..=5).
    pub material_id: u8,
    /// MIDI note (0..=127). `0` = [`SILENCE`].
    pub note: u8,
    /// Timbre — index into `VoicePreset` (0..=2).
    pub voice: u8,
}

impl NodeVoice {
    const fn new(material_id: u8, note: u8, voice: u8) -> Self {
        Self { material_id, note, voice }
    }
    /// True when this node contributes a pitch to the chord.
    #[inline]
    pub const fn is_audible(self) -> bool {
        self.note != SILENCE
    }
}

/// The canonical voice for a CST node kind. Total, deterministic, const.
///
/// Tiers (pitch rises with activity): file drone < definitions < structure/refs <
/// values < logic/actions. All audible notes are C-major-pentatonic by construction.
pub const fn node_voice(kind: NodeKind) -> NodeVoice {
    match kind {
        // file drone
        NodeKind::SourceFile => NodeVoice::new(MAT_VOID, 48, VOICE_HEARTH), // C3 root drone
        // definitions — foundation, low, warm
        NodeKind::UiDef => NodeVoice::new(MAT_STRUCTURAL, 50, VOICE_HEARTH), // D3
        NodeKind::MaterialDef => NodeVoice::new(MAT_IMMUTABLE, 52, VOICE_HEARTH), // E3
        NodeKind::AutomataDef => NodeVoice::new(MAT_CONTROL, 55, VOICE_HEARTH), // G3 (rule = control)
        NodeKind::SpatialDef => NodeVoice::new(MAT_KINETIC, 57, VOICE_HEARTH), // A3 (placement)
        NodeKind::EnvironmentCall => NodeVoice::new(MAT_CONTROL, 60, VOICE_REED), // C4 (effect)
        // the lowering TARGET — the atom sits at the deep foundation, an octave below MaterialDef.
        NodeKind::AtomDef => NodeVoice::new(MAT_IMMUTABLE, 40, VOICE_HEARTH), // E2 (the atom bedrock)
        // authored paint/canvas primitives — deep foundation voices, all C-pentatonic.
        NodeKind::AcrylicDef => NodeVoice::new(MAT_KINETIC, 43, VOICE_HEARTH),    // G2 (a paint dab)
        NodeKind::PressureDef => NodeVoice::new(MAT_ACOUSTIC, 36, VOICE_HEARTH),  // C2 (pen-feel curve)
        NodeKind::LayersDef => NodeVoice::new(MAT_STRUCTURAL, 38, VOICE_HEARTH),  // D2 (stack depth)
        NodeKind::ViewportDef => NodeVoice::new(MAT_STRUCTURAL, 31, VOICE_HEARTH), // G1 (camera frame)
        NodeKind::BrushDef => NodeVoice::new(MAT_KINETIC, 33, VOICE_HEARTH),      // A1 (brush tip)
        // structure / references — mid, reed (references share the REFERENCE band)
        NodeKind::Property => NodeVoice::new(MAT_REFERENCE, 62, VOICE_REED), // D4
        NodeKind::Identifier => NodeVoice::new(MAT_REFERENCE, 64, VOICE_REED), // E4
        NodeKind::ArrayLiteral => NodeVoice::new(MAT_REFERENCE, 67, VOICE_REED), // G4
        NodeKind::StringLiteral => NodeVoice::new(MAT_ACOUSTIC, 69, VOICE_REED), // A4 (a name spoken)
        NodeKind::TickDelay => NodeVoice::new(MAT_ACOUSTIC, 45, VOICE_REED), // A2 (low timing pulse)
        // values — upper, glass (numeric values carry the VALUE band)
        NodeKind::Integer => NodeVoice::new(MAT_VALUE, 72, VOICE_GLASS), // C5
        NodeKind::Permyriad => NodeVoice::new(MAT_VALUE, 74, VOICE_GLASS), // D5 (a tuned value)
        NodeKind::HexLiteral => NodeVoice::new(MAT_KINETIC, 76, VOICE_GLASS), // E5 (literal colour)
        // logic / actions — high, bright, glass (QUERY = the question, ACTION = the deed)
        NodeKind::Comparison => NodeVoice::new(MAT_QUERY, 79, VOICE_GLASS), // G5
        NodeKind::FunctionCall => NodeVoice::new(MAT_ACTION, 79, VOICE_GLASS), // G5 (acts, kinetic)
        NodeKind::WhenClause => NodeVoice::new(MAT_QUERY, 81, VOICE_GLASS), // A5 (the question)
        NodeKind::ThenClause => NodeVoice::new(MAT_ACTION, 84, VOICE_GLASS), // C6 (the resolution)
        NodeKind::Error => NodeVoice::new(MAT_FAULT, SILENCE, VOICE_HEARTH), // a fault must be SEEN loud
    }
}

impl crate::Cst {
    /// Walk the CST and emit each node's [`NodeVoice`] — the "score" of the source.
    /// Zero-alloc: borrows the node array, maps lazily. This is the see-and-hear seam.
    pub fn voices(&self) -> impl Iterator<Item = (u16, NodeVoice)> + '_ {
        self.nodes[..self.count as usize]
            .iter()
            .enumerate()
            .map(|(i, n)| (i as u16, node_voice(n.kind)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cst, NodeKind};

    /// The full kind set (Error excluded — it is the silent sentinel, tested apart).
    const AUDIBLE_KINDS: [NodeKind; 24] = [
        NodeKind::SourceFile, NodeKind::MaterialDef, NodeKind::SpatialDef,
        NodeKind::AutomataDef, NodeKind::EnvironmentCall, NodeKind::UiDef,
        NodeKind::Property, NodeKind::WhenClause, NodeKind::ThenClause,
        NodeKind::TickDelay, NodeKind::Integer, NodeKind::Permyriad,
        NodeKind::HexLiteral, NodeKind::StringLiteral, NodeKind::Identifier,
        NodeKind::ArrayLiteral, NodeKind::Comparison, NodeKind::FunctionCall,
        NodeKind::AtomDef, NodeKind::AcrylicDef, NodeKind::PressureDef,
        NodeKind::LayersDef, NodeKind::ViewportDef, NodeKind::BrushDef,
    ];

    #[test]
    fn every_kind_has_a_valid_voice() {
        for k in AUDIBLE_KINDS {
            let v = node_voice(k);
            assert!(v.material_id <= MAT_FAULT, "{k:?}: material_id {} out of range", v.material_id);
            assert!(v.note <= 127, "{k:?}: note {} out of MIDI range", v.note);
            assert!(v.voice <= VOICE_HEARTH, "{k:?}: voice {} out of range", v.voice);
            assert!(v.is_audible(), "{k:?} must sound");
        }
    }

    #[test]
    fn error_is_silence_but_still_visible() {
        let v = node_voice(NodeKind::Error);
        assert_eq!(v.note, SILENCE, "Error must not ring");
        assert!(!v.is_audible());
        // It still has a material so a parse error is SEEN, not invisible —
        // now the FAULT band so a parse error screams instead of dimming to void.
        assert_eq!(v.material_id, MAT_FAULT);
    }

    /// THE guarantee: any CST renders to a consonant chord. Every audible note is
    /// C-major pentatonic (pitch class ∈ {0,2,4,7,9}) — no semitone clashes possible.
    #[test]
    fn no_code_can_sound_ugly() {
        const PENTATONIC: [u8; 5] = [0, 2, 4, 7, 9]; // C D E G A
        for k in AUDIBLE_KINDS {
            let pc = node_voice(k).note % 12;
            assert!(PENTATONIC.contains(&pc), "{k:?}: pitch class {pc} is not pentatonic");
        }
    }

    /// The shape climbs: every definition sits BELOW every logic/action node, so a
    /// file audibly rises from its declarations to its behaviour.
    #[test]
    fn pitch_rises_from_definitions_to_logic() {
        let def_hi = [NodeKind::UiDef, NodeKind::MaterialDef, NodeKind::AutomataDef,
                      NodeKind::SpatialDef, NodeKind::EnvironmentCall]
            .iter().map(|&k| node_voice(k).note).max().unwrap();
        let logic_lo = [NodeKind::Comparison, NodeKind::WhenClause, NodeKind::ThenClause]
            .iter().map(|&k| node_voice(k).note).min().unwrap();
        assert!(def_hi < logic_lo, "definitions ({def_hi}) must sit below logic ({logic_lo})");
    }

    #[test]
    fn voice_is_deterministic() {
        assert_eq!(node_voice(NodeKind::MaterialDef), node_voice(NodeKind::MaterialDef));
    }

    /// The headless proof of the thesis: parse real source, walk the CST, and read
    /// back its score — you SEE (material) and HEAR (note) the shape of the logic.
    #[test]
    fn a_cst_is_a_score() {
        let src = b"material \"wood\" { hardness: 3000p }\n\
                    spawn_grid(\"plank\", 2)\n\
                    rule \"burn\" { when: fire > 5000, then: destroy(), tick_delay: 1 }";
        let cst = Cst::parse(src);
        let score: std::vec::Vec<(u16, NodeVoice)> = cst.voices().collect();
        assert_eq!(score.len(), 3, "three top-level nodes -> three voices");
        // material "wood" -> Immutable foundation, E3.
        assert_eq!(score[0].1, node_voice(NodeKind::MaterialDef));
        // spawn_grid -> Kinetic placement, A3.
        assert_eq!(score[1].1, node_voice(NodeKind::SpatialDef));
        // rule -> Control, G3.
        assert_eq!(score[2].1, node_voice(NodeKind::AutomataDef));
        // every voice in a real file is audible + consonant.
        for (_, v) in &score {
            assert!(v.is_audible());
            assert!([0u8, 2, 4, 7, 9].contains(&(v.note % 12)));
        }
    }
}
