//! Theme — book palettes as integer rgba tokens. The deveraux 8-slot brand
//! sheet (harvested from deveraux.sheet.vixi) plus per-era tints.

use serde::{Deserialize, Serialize};

/// The four world eras — the narrative clock the sky answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era {
    /// The age of stone and forgotten greatness.
    Ancient,
    /// The age of prosperity and power.
    Golden,
    /// The age of decline and crumbling walls.
    Decay,
    /// The age of silence and entropy.
    Void,
}

impl Era {
    /// Return the era's name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Era::Ancient => "Ancient",
            Era::Golden => "Golden",
            Era::Decay => "Decay",
            Era::Void => "Void",
        }
    }
}

/// The eight canonical palette slots (mirrors the forge-vix SHEET_PALETTE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeSlot {
    /// Far background color.
    BgFar,
    /// Near background color.
    BgNear,
    /// Primary text color.
    FgText,
    /// Muted or secondary text color.
    FgMuted,
    /// Primary accent color.
    AccentPrimary,
    /// Secondary accent color.
    AccentSecondary,
    /// Success state color.
    Success,
    /// Warning or danger state color.
    WarningDanger,
}

/// An 8-slot rgba palette — the book's brand sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    /// Far background color in RGBA.
    pub bg_far: [u8; 4],
    /// Near background color in RGBA.
    pub bg_near: [u8; 4],
    /// Primary text color in RGBA.
    pub fg_text: [u8; 4],
    /// Muted text color in RGBA.
    pub fg_muted: [u8; 4],
    /// Primary accent color in RGBA.
    pub accent_primary: [u8; 4],
    /// Secondary accent color in RGBA.
    pub accent_secondary: [u8; 4],
    /// Success state color in RGBA.
    pub success: [u8; 4],
    /// Warning or danger state color in RGBA.
    pub warning_danger: [u8; 4],
}

impl Palette {
    /// The deveraux underground brand — the 8 tokens harvested from disk.
    pub fn deveraux() -> Self {
        Self {
            bg_far: [0x0E, 0x0B, 0x12, 0xFF],
            bg_near: [0x1A, 0x13, 0x20, 0xFF],
            fg_text: [0xF3, 0xE2, 0xC7, 0xFF],
            fg_muted: [0x8A, 0x6F, 0x52, 0xFF],
            accent_primary: [0xE2, 0x54, 0x2B, 0xFF],
            accent_secondary: [0x9B, 0x5C, 0xFF, 0xFF],
            success: [0x4F, 0xB2, 0x86, 0xFF],
            warning_danger: [0xFF, 0x3B, 0x6E, 0xFF],
        }
    }

    /// Read a slot's rgba.
    pub fn slot(&self, s: ThemeSlot) -> [u8; 4] {
        match s {
            ThemeSlot::BgFar => self.bg_far,
            ThemeSlot::BgNear => self.bg_near,
            ThemeSlot::FgText => self.fg_text,
            ThemeSlot::FgMuted => self.fg_muted,
            ThemeSlot::AccentPrimary => self.accent_primary,
            ThemeSlot::AccentSecondary => self.accent_secondary,
            ThemeSlot::Success => self.success,
            ThemeSlot::WarningDanger => self.warning_danger,
        }
    }

    /// `#rrggbb` for a slot.
    pub fn hex(&self, s: ThemeSlot) -> String {
        let [r, g, b, _] = self.slot(s);
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// A per-era tint of the brand — the sky's age colours the page.
    pub fn era(era: Era) -> Self {
        let mut p = Palette::deveraux();
        p.accent_primary = match era {
            Era::Ancient => [0x8A, 0x70, 0x30, 0xFF], // gold-dim
            Era::Golden => [0xC8, 0xA0, 0x40, 0xFF],  // gold
            Era::Decay => [0xE2, 0x54, 0x2B, 0xFF],   // blood
            Era::Void => [0x9B, 0x5C, 0xFF, 0xFF],    // spectral-violet
        };
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deveraux_tokens_match_sheet() {
        let p = Palette::deveraux();
        assert_eq!(p.hex(ThemeSlot::AccentPrimary), "#e2542b");
        assert_eq!(p.hex(ThemeSlot::FgText), "#f3e2c7");
        assert_eq!(p.slot(ThemeSlot::WarningDanger), [0xFF, 0x3B, 0x6E, 0xFF]);
    }

    #[test]
    fn era_retints_accent_only() {
        let base = Palette::deveraux();
        let golden = Palette::era(Era::Golden);
        assert_eq!(golden.hex(ThemeSlot::AccentPrimary), "#c8a040");
        assert_eq!(golden.bg_far, base.bg_far); // background unchanged
    }
}
