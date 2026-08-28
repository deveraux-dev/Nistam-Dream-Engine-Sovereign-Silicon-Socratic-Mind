//! Pad glyph atlas: procedural family table. One silhouette per family;
//! rotation is applied at draw, so variants never fork atlas memory.

/// The six silhouette families. Each owns exactly one atlas cell,
/// shared by multiple glyph variants via rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadFamily {
    /// Face buttons (A/B/X/Y equivalents).
    Face,
    /// Shoulder buttons (left/right bumpers).
    Shoulder,
    /// Trigger buttons (left/right triggers).
    Trigger,
    /// Analog sticks.
    Stick,
    /// D-Pad.
    DPad,
    /// System buttons (Start/Select).
    System,
}

/// A stable atlas cell: one per family, rotation/fill applied at draw time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasCell {
    /// Index into the glyph atlas (0..5 for six families).
    pub index: u8,
    /// Width in pixels.
    pub w: u16,
    /// Height in pixels.
    pub h: u16,
}

/// All 16 controller glyphs. Variants share a family silhouette and
/// differ only by rotation_quarter (or mirrored fill for shoulders).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadGlyph {
    /// Face button: down.
    FaceDown,
    /// Face button: right.
    FaceRight,
    /// Face button: left.
    FaceLeft,
    /// Face button: up.
    FaceUp,
    /// Left bumper (shoulder button).
    LB,
    /// Right bumper (shoulder button).
    RB,
    /// Left trigger button.
    LT,
    /// Right trigger button.
    RT,
    /// Left analog stick.
    LStick,
    /// Right analog stick.
    RStick,
    /// D-Pad: up.
    DPadUp,
    /// D-Pad: down.
    DPadDown,
    /// D-Pad: left.
    DPadLeft,
    /// D-Pad: right.
    DPadRight,
    /// Start/Menu button (system button).
    Start,
    /// Select/Options button (system button).
    Select,
}

impl PadGlyph {
    /// Which silhouette family this glyph draws from.
    pub fn family(&self) -> PadFamily {
        match self {
            PadGlyph::FaceDown | PadGlyph::FaceRight | PadGlyph::FaceLeft | PadGlyph::FaceUp => {
                PadFamily::Face
            }
            PadGlyph::LB | PadGlyph::RB => PadFamily::Shoulder,
            PadGlyph::LT | PadGlyph::RT => PadFamily::Trigger,
            PadGlyph::LStick | PadGlyph::RStick => PadFamily::Stick,
            PadGlyph::DPadUp | PadGlyph::DPadDown | PadGlyph::DPadLeft | PadGlyph::DPadRight => {
                PadFamily::DPad
            }
            PadGlyph::Start | PadGlyph::Select => PadFamily::System,
        }
    }

    /// Quarter-turn rotation (0..3) applied to the family silhouette at draw.
    /// 0 = 0°, 1 = 90° CW, 2 = 180°, 3 = 270° CW.
    pub fn rotation_quarter(&self) -> u8 {
        match self {
            PadGlyph::FaceDown => 0,
            PadGlyph::FaceRight => 1,
            PadGlyph::FaceUp => 2,
            PadGlyph::FaceLeft => 3,
            PadGlyph::LB | PadGlyph::RB => 0,
            PadGlyph::LT | PadGlyph::RT => 0,
            PadGlyph::LStick | PadGlyph::RStick => 0,
            PadGlyph::DPadUp => 0,
            PadGlyph::DPadRight => 1,
            PadGlyph::DPadDown => 2,
            PadGlyph::DPadLeft => 3,
            PadGlyph::Start | PadGlyph::Select => 0,
        }
    }

    /// Plain-words label a six-year-old can read. No dev jargon, no vendor names.
    pub fn label(&self) -> &'static str {
        match self {
            PadGlyph::FaceDown => "Bottom button",
            PadGlyph::FaceRight => "Right button",
            PadGlyph::FaceLeft => "Left button",
            PadGlyph::FaceUp => "Top button",
            PadGlyph::LB => "Left bumper",
            PadGlyph::RB => "Right bumper",
            PadGlyph::LT => "Left trigger",
            PadGlyph::RT => "Right trigger",
            PadGlyph::LStick => "Left stick",
            PadGlyph::RStick => "Right stick",
            PadGlyph::DPadUp => "D-pad up",
            PadGlyph::DPadDown => "D-pad down",
            PadGlyph::DPadLeft => "D-pad left",
            PadGlyph::DPadRight => "D-pad right",
            PadGlyph::Start => "Start",
            PadGlyph::Select => "Select",
        }
    }
}

/// Resolve a family to its single stable atlas cell. Six cells total (one per family).
pub fn cell(family: PadFamily) -> AtlasCell {
    match family {
        PadFamily::Face => AtlasCell { index: 0, w: 32, h: 32 },
        PadFamily::Shoulder => AtlasCell { index: 1, w: 32, h: 32 },
        PadFamily::Trigger => AtlasCell { index: 2, w: 32, h: 32 },
        PadFamily::Stick => AtlasCell { index: 3, w: 32, h: 32 },
        PadFamily::DPad => AtlasCell { index: 4, w: 32, h: 32 },
        PadFamily::System => AtlasCell { index: 5, w: 32, h: 32 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [PadGlyph; 16] = [
        PadGlyph::FaceDown,
        PadGlyph::FaceRight,
        PadGlyph::FaceLeft,
        PadGlyph::FaceUp,
        PadGlyph::LB,
        PadGlyph::RB,
        PadGlyph::LT,
        PadGlyph::RT,
        PadGlyph::LStick,
        PadGlyph::RStick,
        PadGlyph::DPadUp,
        PadGlyph::DPadDown,
        PadGlyph::DPadLeft,
        PadGlyph::DPadRight,
        PadGlyph::Start,
        PadGlyph::Select,
    ];

    // ── L07-style determinism: glyph-to-family mapping is stable ────────────

    #[test]
    fn family_is_deterministic() {
        for glyph in &ALL {
            let f1 = glyph.family();
            let f2 = glyph.family();
            assert_eq!(f1, f2, "family() must be deterministic");
        }
    }

    #[test]
    fn rotation_is_deterministic() {
        for glyph in &ALL {
            let r1 = glyph.rotation_quarter();
            let r2 = glyph.rotation_quarter();
            assert_eq!(r1, r2, "rotation_quarter() must be deterministic");
        }
    }

    #[test]
    fn label_is_deterministic() {
        for glyph in &ALL {
            let l1 = glyph.label();
            let l2 = glyph.label();
            assert_eq!(l1, l2, "label() must be deterministic");
        }
    }

    #[test]
    fn cell_is_deterministic() {
        for family in [
            PadFamily::Face,
            PadFamily::Shoulder,
            PadFamily::Trigger,
            PadFamily::Stick,
            PadFamily::DPad,
            PadFamily::System,
        ] {
            let c1 = cell(family);
            let c2 = cell(family);
            assert_eq!(c1, c2, "cell() must be deterministic");
        }
    }

    #[test]
    fn sixteen_glyphs_resolve_to_six_cells() {
        let mut cells = std::collections::HashSet::new();
        for glyph in &ALL {
            cells.insert(cell(glyph.family()).index);
        }
        assert_eq!(cells.len(), 6, "all 16 glyphs must map to exactly 6 atlas cells");
    }

    #[test]
    fn dpad_right_shares_cell_with_dpad_up_by_rotation() {
        assert_eq!(cell(PadGlyph::DPadRight.family()), cell(PadGlyph::DPadUp.family()));
        assert_ne!(
            PadGlyph::DPadRight.rotation_quarter(),
            PadGlyph::DPadUp.rotation_quarter(),
            "same family must differ by rotation"
        );
    }

    #[test]
    fn face_buttons_all_map_to_face_family() {
        let faces = [
            PadGlyph::FaceDown,
            PadGlyph::FaceRight,
            PadGlyph::FaceLeft,
            PadGlyph::FaceUp,
        ];
        for glyph in &faces {
            assert_eq!(glyph.family(), PadFamily::Face);
        }
    }

    #[test]
    fn labels_are_plain_words() {
        for g in &ALL {
            let l = g.label();
            assert!(!l.is_empty(), "label must not be empty");
            assert!(!l.chars().any(|c| c.is_ascii_digit()), "label must contain no digits");
        }
    }

    // ── L18-style sabotage: flip rotation range ─────────────────────────────
    // If rotation_quarter() could return 4 or higher, the 90° rotation math would
    // be wrong. We verify all rotations stay in [0, 3].

    #[test]
    fn rotation_quarter_always_under_four() {
        for g in &ALL {
            let r = g.rotation_quarter();
            assert!(r < 4, "rotation_quarter must be in [0, 3]");
        }
    }

    #[test]
    fn face_four_rotations_are_unique() {
        let faces = [
            PadGlyph::FaceDown,
            PadGlyph::FaceRight,
            PadGlyph::FaceUp,
            PadGlyph::FaceLeft,
        ];
        let rotations: Vec<u8> = faces.iter().map(|g| g.rotation_quarter()).collect();
        // All four rotations should be distinct.
        let mut sorted = rotations.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "four face buttons must have four distinct rotations");
    }

    #[test]
    fn dpad_four_rotations_are_unique() {
        let dpad = [
            PadGlyph::DPadUp,
            PadGlyph::DPadRight,
            PadGlyph::DPadDown,
            PadGlyph::DPadLeft,
        ];
        let rotations: Vec<u8> = dpad.iter().map(|g| g.rotation_quarter()).collect();
        let mut sorted = rotations.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "four d-pad directions must have four distinct rotations");
    }

    #[test]
    fn atlas_cells_have_consistent_dimensions() {
        // All cells are 32×32 in this implementation.
        for family in [
            PadFamily::Face,
            PadFamily::Shoulder,
            PadFamily::Trigger,
            PadFamily::Stick,
            PadFamily::DPad,
            PadFamily::System,
        ] {
            let c = cell(family);
            assert_eq!(c.w, 32, "atlas cell width must be 32");
            assert_eq!(c.h, 32, "atlas cell height must be 32");
        }
    }

    #[test]
    fn atlas_cell_indices_are_unique_and_ordered() {
        let families = [
            PadFamily::Face,
            PadFamily::Shoulder,
            PadFamily::Trigger,
            PadFamily::Stick,
            PadFamily::DPad,
            PadFamily::System,
        ];
        let mut indices: Vec<u8> = families.iter().map(|f| cell(*f).index).collect();
        indices.sort();
        indices.dedup();
        assert_eq!(indices.len(), 6, "all 6 families must have unique cell indices");
        // Indices should be 0..5.
        assert_eq!(indices, vec![0, 1, 2, 3, 4, 5]);
    }
}
