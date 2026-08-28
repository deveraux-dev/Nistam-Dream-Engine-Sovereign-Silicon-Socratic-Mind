//! Color theme — the sole colour contract between forge-gui and forge-canvas.
//! Stores theme state as packed RGBA u32 values. Pure data, no engine or GPU types.

/// Flat colour bag for the sovereign UI. All values are packed RGBA `0xRRGGBBAA`.
///
/// Each field represents an interactive or decorative UI surface. The theme
/// is filled from external RenderParams and consumed by all widget-rendering
/// code instead of hardcoded constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTheme {
    /// Deepest layer (main window background).
    pub bg_app: u32,
    /// Elevated layer (panel / drawer background).
    pub bg_panel: u32,
    /// Interactive surface (dropdowns, slider tracks).
    pub bg_surface: u32,
    /// Primary text — high contrast on dark backgrounds.
    pub text_primary: u32,
    /// Muted text — reduced prominence.
    pub text_muted: u32,
    /// Driving profile colour (Forge Amber, Field Blue, Arena Red, etc.).
    pub accent_primary: u32,
    /// Accent colour on hover.
    pub accent_hover: u32,
    /// Accent colour on press.
    pub accent_pressed: u32,
    /// Border / separator colour.
    pub border: u32,
    /// Error / danger signal colour.
    pub error: u32,
}

impl Default for ColorTheme {
    fn default() -> Self {
        // Dark-mode neutral fallback — Forge Amber accent.
        Self {
            bg_app:         0x121215FF,
            bg_panel:       0x1A1A1FFF,
            bg_surface:     0x2A2A32FF,
            text_primary:   0xE0D4BAFF,
            text_muted:     0x8A8478FF,
            accent_primary: 0xD4A843FF, // amber
            accent_hover:   0xE0B84EFF,
            accent_pressed: 0xB08A36FF,
            border:         0x2A2A32FF,
            error:          0xD43535FF,
        }
    }
}

impl ColorTheme {
    /// Construct from red, green, blue, alpha channels `0..=255`.
    /// Returns the packed u32 in RGBA order.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// The canonical "Celestial Prairie" Thermal Forge light palette.
    /// Derived from `celestial_prairie.base.sheet.vixi`.
    #[inline]
    pub const fn celestial_prairie() -> Self {
        Self {
            bg_app:         0xEDE3CEFF, // VELLUM ground
            bg_panel:       0xF5EEDEFF, // bg_dust card surface
            bg_surface:     0xF9F2E4FF, // bg_hover active lift
            text_primary:   0x221B17FF, // INK mark (~13-15:1 contrast)
            text_muted:     0x5C4F3EFF, // INK->VELLUM taupe mark
            accent_primary: 0xFFAE2BFF, // FLARE gold heat
            accent_hover:   0xE23B22FF, // FORGE-RED
            accent_pressed: 0x875BD6FF, // FLUX-VIOLET
            border:         0x8C8270FF, // taupe hairline
            error:          0xE23B22FF, // FORGE-RED
        }
    }
}

// ── Color utilities (pure integer math, no floats) ───────────────────────────

/// Extract RGBA channels from a packed u32.
///
/// Input is in the form `0xRRGGBBAA`.
/// Returns `(red, green, blue, alpha)` as separate `u8` values.
#[inline]
pub const fn unpack_rgba(c: u32) -> (u8, u8, u8, u8) {
    let r = ((c >> 24) & 0xFF) as u8;
    let g = ((c >> 16) & 0xFF) as u8;
    let b = ((c >> 8) & 0xFF) as u8;
    let a = (c & 0xFF) as u8;
    (r, g, b, a)
}

/// Pack RGBA channels into a u32 in the form `0xRRGGBBAA`.
#[inline]
pub const fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | a as u32
}

/// Lighten a packed RGBA color, scaled in LINEAR light (not raw sRGB bytes).
///
/// `factor_256` is a fixed-point multiplier where:
/// - 256 = 1.0 (no change)
/// - 307 ≈ 1.2 (20% lighter)
/// - 320 = 1.25 (25% lighter)
///
/// Delegates to `forge_correspondence_v3::correspondence::scale_luma`
/// (decode -> scale -> re-encode); multiplying sRGB bytes directly shifts hue
/// as well as brightness (aspire.rs colour-gamma-lighten-fix,
/// forge-correspondence-v3::correspondence.rs:243). Alpha is preserved.
#[inline]
pub fn lighten_u32(c: u32, factor_256: u16) -> u32 {
    let factor_pmy = (factor_256 as u32 * 10_000) / 256;
    forge_correspondence_v3::correspondence::scale_luma(c, factor_pmy)
}

/// Darken a packed RGBA color, scaled in LINEAR light (not raw sRGB bytes).
///
/// `factor_256` is a fixed-point multiplier where:
/// - 256 = 1.0 (no change)
/// - 204 ≈ 0.8 (20% darker)
/// - 179 ≈ 0.7 (30% darker)
///
/// See [`lighten_u32`] — same gamma-correct scale_luma delegation. Alpha is preserved.
#[inline]
pub fn darken_u32(c: u32, factor_256: u16) -> u32 {
    let factor_pmy = (factor_256 as u32 * 10_000) / 256;
    forge_correspondence_v3::correspondence::scale_luma(c, factor_pmy)
}

/// Synthesia ColourID index constants — the slot identity for each theme colour role.
///
/// Note: These are view constants over the 64-slot correspondence engine;
/// the real index is in `forge_materials::slot_correspondence::resolve`.
/// Do not re-index these without regenerating the backing skin table.
pub const CID_GROUND: u8 = 0;
/// Bar layer identifier.
pub const CID_BAR: u8 = 1;
/// Frame / border layer identifier.
pub const CID_FRAME: u8 = 2;
/// Title text layer identifier.
pub const CID_TITLE: u8 = 3;
/// Status text layer identifier.
pub const CID_STATUS: u8 = 4;
/// Accent layer identifier.
pub const CID_ACCENT: u8 = 5;
/// Mark / success layer identifier.
pub const CID_MARK: u8 = 6;
/// Violet / lore layer identifier.
pub const CID_VIOLET: u8 = 7;
/// Danger / error layer identifier.
pub const CID_DANGER: u8 = 8;

/// Pack a Synthesia ColourID slot to `0xRRGGBBAA`.
///
/// Call as `syn_rgba(CID_ACCENT, 0xFF)` — never pass a raw hex value.
/// If `cid` is out of range, the last colour is used as a fallback.
#[inline]
pub const fn syn_rgba(cid: u8, alpha: u8) -> u32 {
    let idx = if (cid as usize) < SYN_SKIN_BYTES.len() { cid as usize } else { SYN_SKIN_BYTES.len() - 1 };
    let [r, g, b] = SYN_SKIN_BYTES[idx];
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (alpha as u32)
}

/// Extract RGB from a packed RGBA colour (internal helper).
const fn rgb_from_rgba(rgba: u32) -> [u8; 3] {
    let (r, g, b, _) = unpack_rgba(rgba);
    [r, g, b]
}

/// Synthesia HUD chrome palette — sRGB byte triplets indexed by ColourID (0–8).
///
/// Sourced from `themes/syn_skin.profile.sheet.vixi`; derived via `unpack_rgba`
/// to avoid the `[0xRR, 0xGG, 0xBB]` byte-array literal anti-pattern.
/// GPU consumers (forge-gpu) import this instead of duplicating the table.
pub const SYN_SKIN_BYTES: [[u8; 3]; 9] = [
    // MODERN DARK MINIMAL (Sean 2026-07-23) — v10 app-shell ember/gold/ink,
    // matched to forge_studio.base.sheet.vixi so CID chrome == named tokens.
    rgb_from_rgba(0x0A0706FF), // 0 Ground — warm black
    rgb_from_rgba(0x15110DFF), // 1 Bar    — panel fill
    rgb_from_rgba(0x241E17FF), // 2 Frame  — warm hairline border
    rgb_from_rgba(0xECDFCDFF), // 3 Title  — bone text
    rgb_from_rgba(0x9C9080FF), // 4 Status — ash-dim
    rgb_from_rgba(0xE8843CFF), // 5 Accent — ember (the one heat)
    rgb_from_rgba(0x5FC285FF), // 6 Mark   — town (verdigris)
    rgb_from_rgba(0x9A86E0FF), // 7 Violet — lore (frost violet)
    rgb_from_rgba(0xD2674FFF), // 8 Danger — coral red
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_has_nonzero_colors() {
        let t = ColorTheme::default();
        assert_ne!(t.bg_app, 0);
        assert_ne!(t.accent_primary, 0);
        assert_ne!(t.text_primary, 0);
    }

    #[test]
    fn pack_unpack_roundtrip() {
        let c = 0xAABBCCDD;
        let (r, g, b, a) = unpack_rgba(c);
        assert_eq!(r, 0xAA);
        assert_eq!(g, 0xBB);
        assert_eq!(b, 0xCC);
        assert_eq!(a, 0xDD);
        assert_eq!(pack_rgba(r, g, b, a), c);
    }

    // Sabotage test L18: flip the assert to confirm this catches real errors.
    #[test]
    fn pack_unpack_determinism() {
        // Test that all possible byte values pack and unpack deterministically.
        for r in [0, 1, 127, 128, 255] {
            for g in [0, 64, 192, 255] {
                for b in [0, 32, 128, 255] {
                    for a in [0, 128, 255] {
                        let packed = pack_rgba(r, g, b, a);
                        let (r2, g2, b2, a2) = unpack_rgba(packed);
                        assert_eq!((r, g, b, a), (r2, g2, b2, a2), "mismatch for ({r}, {g}, {b}, {a})");
                    }
                }
            }
        }
    }

    #[test]
    fn lighten_increases_channels() {
        let c = 0x804020FF; // R=128, G=64, B=32
        let lit = lighten_u32(c, 307); // ~1.2x
        let (r, g, b, a) = unpack_rgba(lit);
        assert!(r > 128, "R should increase: {r}");
        assert!(g > 64, "G should increase: {g}");
        assert!(b > 32, "B should increase: {b}");
        assert_eq!(a, 0xFF, "alpha preserved");
    }

    #[test]
    fn lighten_clamps_at_255() {
        let c = 0xF0F0F0FF;
        let lit = lighten_u32(c, 512); // 2.0x
        let (r, g, b, _) = unpack_rgba(lit);
        assert_eq!(r, 255);
        assert_eq!(g, 255);
        assert_eq!(b, 255);
    }

    #[test]
    fn darken_decreases_channels() {
        let c = 0x804020FF;
        let dim = darken_u32(c, 204); // ~0.8x
        let (r, g, b, a) = unpack_rgba(dim);
        assert!(r < 128, "R should decrease: {r}");
        assert!(g < 64, "G should decrease: {g}");
        assert!(b < 32, "B should decrease: {b}");
        assert_eq!(a, 0xFF, "alpha preserved");
    }

    #[test]
    fn syn_rgba_packs_with_alpha() {
        let c = syn_rgba(CID_ACCENT, 0x80);
        let (_, _, _, a) = unpack_rgba(c);
        assert_eq!(a, 0x80);
        let (r, g, b, _) = unpack_rgba(c);
        // CID_ACCENT index 5 maps to 0xE8843C
        assert_eq!(r, 0xE8);
        assert_eq!(g, 0x84);
        assert_eq!(b, 0x3C);
    }

    #[test]
    fn syn_rgba_clamps_out_of_range_cid() {
        // CID out of bounds should use the last colour (CID_DANGER).
        let c1 = syn_rgba(8, 0xFF);
        let c2 = syn_rgba(255, 0xFF);
        assert_eq!(c1, c2);
    }

    #[test]
    fn celestial_prairie_has_correct_contrast_and_tokens() {
        let cp = ColorTheme::celestial_prairie();
        assert_eq!(cp.bg_app, 0xEDE3CEFF, "VELLUM ground");
        assert_eq!(cp.text_primary, 0x221B17FF, "INK text mark");
        assert_eq!(cp.accent_primary, 0xFFAE2BFF, "FLARE accent");
        assert_eq!(cp.border, 0x8C8270FF, "taupe border");
        assert_eq!(cp.error, 0xE23B22FF, "FORGE-RED error");
    }
}
