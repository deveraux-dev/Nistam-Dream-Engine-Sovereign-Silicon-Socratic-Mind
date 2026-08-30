//! Ink palette + quill nib — the grimoire's sepia/blood/spectral inks as integer
//! rgba, plus a pressure-driven nib. The author's tool faces on the book spine.

use serde::{Deserialize, Serialize};

/// A named ink. The three harvested from the grimoire are canonical; `Custom`
/// packs an rgba8 into a u32 so the palette stays integer + `Eq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InkId {
    /// Warm brown sepia from the grimoire.
    Sepia,
    /// Bright blood-red from the grimoire.
    Blood,
    /// Teal spectral ink from the grimoire.
    Spectral,
    /// Bright gold ink.
    Gold,
    /// Arbitrary rgba8 color packed as a u32 (r<<24 | g<<16 | b<<8 | a).
    Custom(u32),
}

/// An ink resolved to rgba8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ink {
    /// The ink's identity and palette key.
    pub id: InkId,
    /// RGBA8 color bytes.
    pub rgba: [u8; 4],
}

impl Ink {
    /// Resolve a named ink to its rgba8 bytes.
    pub fn of(id: InkId) -> Self {
        let rgba = match id {
            InkId::Sepia => [106, 79, 48, 255],
            InkId::Blood => [229, 75, 0, 255],
            InkId::Spectral => [0, 160, 142, 255],
            InkId::Gold => [200, 160, 64, 255],
            InkId::Custom(p) => [(p >> 24) as u8, (p >> 16) as u8, (p >> 8) as u8, p as u8],
        };
        Self { id, rgba }
    }

    /// Pack an rgba8 into a `Custom` ink id.
    pub fn custom(r: u8, g: u8, b: u8, a: u8) -> InkId {
        InkId::Custom(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a as u32)
    }

    /// `#rrggbb` — the export/web face of this ink.
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.rgba[0], self.rgba[1], self.rgba[2])
    }
}

/// The quill nib cursor: which ink, how hard it presses (permyriad `0..=10000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quill {
    /// The current ink selection.
    pub ink: InkId,
    /// Pressure in permyriad (10000ths), clamped to 0..=10000.
    pub pressure_pmy: u32,
}

impl Quill {
    /// Construct a quill with the given ink at zero pressure.
    pub fn new(ink: InkId) -> Self {
        Self { ink, pressure_pmy: 0 }
    }
    /// Set nib pressure, clamped to the permyriad ceiling.
    pub fn press(&mut self, pmy: u32) {
        self.pressure_pmy = pmy.min(10_000);
    }
    /// Resolve the quill's current ink to its rgba8 bytes.
    pub fn resolved(&self) -> Ink {
        Ink::of(self.ink)
    }
}

impl Default for Quill {
    /// Default quill: sepia ink at zero pressure.
    fn default() -> Self {
        Self::new(InkId::Sepia)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sepia_hex() {
        assert_eq!(Ink::of(InkId::Sepia).hex(), "#6a4f30");
    }

    #[test]
    fn blood_and_spectral_match_grimoire() {
        assert_eq!(Ink::of(InkId::Blood).rgba, [229, 75, 0, 255]);
        assert_eq!(Ink::of(InkId::Spectral).rgba, [0, 160, 142, 255]);
    }

    #[test]
    fn custom_round_trips() {
        let id = Ink::custom(18, 52, 86, 255);
        assert_eq!(Ink::of(id).rgba, [18, 52, 86, 255]);
    }

    #[test]
    fn pressure_clamps() {
        let mut q = Quill::default();
        q.press(99_999);
        assert_eq!(q.pressure_pmy, 10_000);
        assert_eq!(q.resolved().id, InkId::Sepia);
    }
}
