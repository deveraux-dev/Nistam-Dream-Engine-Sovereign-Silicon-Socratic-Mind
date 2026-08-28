//! # tokens.rs — canonical design-token store + seed resolver (Phase 1)
//!
//! Defines [`DesignTokens`], shaped by `docs/forge-design/token_taxonomy.md`,
//! as the intended eventual replacement for the workspace's competing token
//! schemas (`forge-canvas` `TokenId`/`TokenSheet` and `ColorTheme`, `forge-ast`
//! `TokenDef`/`ThemeDef`) — NOT a consolidation today: this crate imports none
//! of them, `DesignTokens` stands on its own until that wire lands (L03
//! receipt, glyph 144E `colour-doc-lie-fix`). A Mulberry32 seed resolver perturbs a
//! base profile along the `variation_axes.md` axes, clamps each axis, and re-checks
//! the `floor_rules.md` floors (contrast ≥ 4500, hit-target ≥ 44). The output
//! feeds [`crate::layout::TokenCtx`], replacing the provisional sizing in
//! `layout.rs`.
//!
//! ## UNIT CONVENTION (pinned here — resolves the ONBOARDING open-risk)
//! All spatial token values are **MilliUnit, `1000` = 1 logical px** (matching
//! `layout.rs` and `forge_canvas::geom::UiRect`). The taxonomy's bare integers
//! ("density 12", "ramp 12", "hit-target 44") are read as **px** and stored
//! ×1000. So: density comfy = `12_000`, body ramp floor = `12_000`,
//! hit-target floor = `44_000`. The taxonomy's `chrome.thickness 0..400` reads
//! as sub-pixel under this convention, so it is sanitized to a sane px range
//! (`0..8_000`, default `1_000`); flagged as an open unit question for the
//! `.vibe.vixi` author to pin.
//!
//! **Determinism:** same `(BaseProfile, seed, locks)` → byte-identical
//! `DesignTokens`. Mulberry32 is the only RNG in this path (`variation_axes.md`).
//! Integer-only; no float anywhere in the resolve path.

use forge_core_v3::Mulberry32;

use crate::kinetic::IntegerSpring;
use crate::layout::TokenCtx;

// ---------------------------------------------------------------------------
// Pinned floors (px·1000 = MilliUnit)
// ---------------------------------------------------------------------------

/// `floor_rules.md` contrast floor (integer ratio ×1000). 4.5:1 → 4500.
pub const CONTRAST_FLOOR: i64 = 4500;
/// `floor_rules.md` hit-target floor on the shortest axis (44 px → MilliUnit).
pub const HIT_TARGET_FLOOR_MU: i64 = 44_000;
/// `variation_axes.md` body-size floor: `ramp[1]` ≥ 12 px.
pub const BODY_RAMP_FLOOR_MU: i64 = 12_000;

// ---------------------------------------------------------------------------
// palette (8 slots, RGB integer triples 0..255)
// ---------------------------------------------------------------------------

/// An sRGB color slot. Integer 0..=255 per channel (no float).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb8 {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb8 {
    /// Build from raw channel bytes.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// `0xRRGGBB` hex literal → `Rgb8` (alpha ignored).
    pub const fn hex(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xFF) as u8,
            g: ((rgb >> 8) & 0xFF) as u8,
            b: (rgb & 0xFF) as u8,
        }
    }

    /// `0xRRGGBBAA` packed (for forge-canvas interop — alpha forced opaque).
    pub const fn packed_rgba(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | 0xFF
    }

    /// Integer relative luminance (`floor_rules.md`): `(R*299+G*587+B*114)/1000`.
    pub fn luminance(self) -> i64 {
        (self.r as i64 * 299 + self.g as i64 * 587 + self.b as i64 * 114) / 1000
    }
}

/// The eight canonical palette slots (`token_taxonomy.md §palette`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Farthest background layer.
    pub bg_far: Rgb8,
    /// Nearest background layer (text sits directly on this).
    pub bg_near: Rgb8,
    /// Primary text colour.
    pub fg_text: Rgb8,
    /// De-emphasized/secondary text colour.
    pub fg_muted: Rgb8,
    /// The dominant accent colour.
    pub accent_primary: Rgb8,
    /// The secondary accent colour.
    pub accent_secondary: Rgb8,
    /// Success/positive-state colour.
    pub success: Rgb8,
    /// Warning-or-danger colour (a single shared slot).
    pub warning_danger: Rgb8,
}

/// Resolve an authored `color=palette.<slot>` name (`WidgetNode::chrome_color`,
/// `ir.rs:194`) against a live [`Palette`]'s 8 slots. `None` on an unknown name —
/// the caller falls back to its own default, this never invents a colour.
pub fn palette_slot(p: &Palette, name: &str) -> Option<Rgb8> {
    Some(match name {
        "bg_far" => p.bg_far,
        "bg_near" => p.bg_near,
        "fg_text" => p.fg_text,
        "fg_muted" => p.fg_muted,
        "accent_primary" => p.accent_primary,
        "accent_secondary" => p.accent_secondary,
        "success" => p.success,
        "warning_danger" => p.warning_danger,
        _ => return None,
    })
}

/// The SINGLE-DRAW LAW, ported 2026-08-26 from v2's
/// `F:\NewRepo\crates\forge-gui\src\vix_runtime.rs:454-464` + `:499-514`: a
/// region that names no resolvable palette slot paints NOTHING — "unauthored
/// regions stay pixel-free". `None` here means DO NOT PAINT, which is a
/// different answer from [`palette_slot`]'s `None` (that one means "unknown
/// name, caller keeps its own default").
///
/// This is the only way to author a sparse plane: `alpha=permyriad(0)` cannot
/// do it, because CSS opacity and the packed alpha channel both inherit to the
/// subtree, so a transparent ground takes its own cards with it.
pub fn authored_fill(chrome_color: Option<&str>, palette: &Palette) -> Option<Rgb8> {
    palette_slot(palette, chrome_color?)
}

/// Integer contrast ratio ×1000 (`floor_rules.md`): `(max+50)*1000/(min+50)`.
pub fn contrast_ratio(a: Rgb8, b: Rgb8) -> i64 {
    let la = a.luminance();
    let lb = b.luminance();
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 50) * 1000 / (lo + 50)
}

// ---------------------------------------------------------------------------
// density / chrome / motion / type.ramp / accent.bias / brush
// ---------------------------------------------------------------------------

/// `token_taxonomy.md §density` — base spacing presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    /// Tightest spacing preset (8px).
    Compact,
    /// Default spacing preset (12px).
    Comfy,
    /// Loosest spacing preset (20px).
    Spacious,
}

impl Density {
    /// Base spacing in MilliUnit (8/12/20 px under the pinned convention).
    pub const fn base_spacing_mu(self) -> i64 {
        match self {
            Density::Compact => 8_000,
            Density::Comfy => 12_000,
            Density::Spacious => 20_000,
        }
    }
}

/// `token_taxonomy.md §chrome`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chrome {
    /// Permyriad 0..10000 (0 = square, 10000 = full radius).
    pub curvature: i32,
    /// Border/stroke width, MilliUnit.
    pub thickness: i64,
    /// Permyriad 0..10000 (shadow-intensity proxy).
    pub elevation: i32,
}

/// `token_taxonomy.md §motion` (SignalSpring dials).
///
/// `stiffness`/`damping` are **permyriad** in [`crate::kinetic::IntegerSpring`]
/// tick units (10000 = 1.0), the deterministic 120Hz motion grid the live UI
/// runs on — NOT the wall-clock `forge_canvas::spring::Spring` units. Feed them
/// to a spring via [`MotionSnap::spring`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionSnap {
    /// Spring stiffness, Permyriad-scale tick units.
    pub stiffness: i32,
    /// Spring damping, Permyriad-scale tick units.
    pub damping: i32,
    /// Transition duration cap, milliseconds.
    pub duration_cap_ms: i32,
}

impl MotionSnap {
    /// The proven UI snap (the book-open tuning, `kinetic::book_open_spring`):
    /// converges with no integer limit-cycle. ~200ms is the canonical UI
    /// transition cap (`docs/forge-design/dreammaker.md`).
    pub const fn comfy() -> Self {
        Self { stiffness: 450, damping: 3_000, duration_cap_ms: 200 }
    }

    /// Build a deterministic 120Hz [`IntegerSpring`] seeded at `start` from these
    /// dials — the binding that was missing (the token existed, bound to nothing).
    pub const fn spring(&self, start: i32) -> IntegerSpring {
        IntegerSpring::new(start, self.stiffness, self.damping)
    }
}

/// `token_taxonomy.md §type` — 5-stop size ramp (MilliUnit):
/// caption / body / subhead / head / display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeRamp(pub [i64; 5]);

/// `token_taxonomy.md §accent.bias` — focal point in Permyriad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccentBias {
    /// Horizontal focal point, Permyriad.
    pub x_perm: i32,
    /// Vertical focal point, Permyriad.
    pub y_perm: i32,
}

/// `token_taxonomy.md §brush`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brush {
    /// `.vixi` brush id (`None` → no procedural brush).
    pub id: Option<String>,
    /// Audio bus name (`None` → static, non-reactive).
    pub bus_in: Option<String>,
}

/// The fully-resolved canonical token set. Drives `TokenCtx` (sizing) and the
/// forge-canvas render layer (color/chrome).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignTokens {
    /// The eight canonical colour slots.
    pub palette: Palette,
    /// Base spacing preset.
    pub density: Density,
    /// Corner/border/elevation dials.
    pub chrome: Chrome,
    /// Spring motion tuning.
    pub motion: MotionSnap,
    /// The 5-stop text-size ladder.
    pub ramp: TypeRamp,
    /// Focal-point bias.
    pub accent_bias: AccentBias,
    /// Procedural paint binding.
    pub brush: Brush,
}

impl DesignTokens {
    /// Project the sizing-relevant tokens into the layout engine's [`TokenCtx`].
    /// This is the seam that replaces `layout.rs`'s provisional defaults.
    pub fn to_token_ctx(&self) -> TokenCtx {
        TokenCtx {
            density_base: self.density.base_spacing_mu(),
            ramp: self.ramp.0,
            chrome_thickness: self.chrome.thickness,
            motion: self.motion,
            faces: forge_canvas_v3::text::RAMP_FACES,
        }
    }
}

// ---------------------------------------------------------------------------
// S channel (one dial: colour + radius + padding)
// ---------------------------------------------------------------------------

/// The S channel in Permyriad: 0 = flat desaturated chrome, 10000 = the
/// profile's own colour at full strength. One dial moves three token families
/// together — chromatic saturation, chrome curvature, density padding — so a
/// generated panel reads as ONE decision instead of three unrelated knobs.
pub const S_MIN: i32 = 0;
/// Ceiling of the S dial — full-strength profile colour.
pub const S_MAX: i32 = 10_000;

/// The enumerated S stops a generated panel picks from.
pub const S_PRESETS: [(&str, i32); 4] =
    [("flat", 0), ("quiet", 3_500), ("studio", 7_000), ("molten", 10_000)];

/// Resolve a preset name to its S value.
pub fn s_preset(name: &str) -> Option<i32> {
    S_PRESETS.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// Pull one colour toward its own luminance grey by the S dial. Integer end to
/// end: `out = grey + (c - grey) * s / 10000`, so S=10000 returns the input
/// byte-identical and S=0 returns the flat grey.
pub fn saturate(c: Rgb8, s_perm: i32) -> Rgb8 {
    let s = s_perm.clamp(S_MIN, S_MAX) as i64;
    let grey = c.luminance().clamp(0, 255);
    let mix = |v: u8| -> u8 {
        let out = grey + (v as i64 - grey) * s / 10_000;
        out.clamp(0, 255) as u8
    };
    Rgb8::new(mix(c.r), mix(c.g), mix(c.b))
}

impl DesignTokens {
    /// Apply the S channel. The greys (bg/fg) keep their authored values — the
    /// contrast floor is theirs to hold — while the four chromatic slots ride
    /// the dial. Curvature scales with S, so flat chrome squares its corners,
    /// and density steps down with it, so a low-S panel reads tight, not airy.
    pub fn with_s(&self, s_perm: i32) -> Self {
        let s = s_perm.clamp(S_MIN, S_MAX);
        let mut out = self.clone();
        out.palette.accent_primary = saturate(self.palette.accent_primary, s);
        out.palette.accent_secondary = saturate(self.palette.accent_secondary, s);
        out.palette.success = saturate(self.palette.success, s);
        out.palette.warning_danger = saturate(self.palette.warning_danger, s);
        out.chrome.curvature =
            (self.chrome.curvature as i64 * s as i64 / S_MAX as i64) as i32;
        out.density = if s < 3_500 {
            Density::Compact
        } else if s < 7_000 {
            Density::Comfy
        } else {
            Density::Spacious
        };
        out
    }
}

// ---------------------------------------------------------------------------
// Base profile + per-axis locks (the .vibe.vixi inputs)
// ---------------------------------------------------------------------------

/// The un-perturbed base a seed varies from (parsed from a `.vibe.vixi`, or one
/// of the built-in studio profiles). The resolver perturbs the *chromatic* and
/// dial fields; `palette`'s grays (bg/fg) are preserved except for floor clamps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseProfile {
    /// The eight canonical colour slots.
    pub palette: Palette,
    /// Base spacing preset.
    pub density: Density,
    /// Corner/border/elevation dials.
    pub chrome: Chrome,
    /// Spring motion tuning.
    pub motion: MotionSnap,
    /// The 5-stop text-size ladder.
    pub ramp: TypeRamp,
    /// Focal-point bias.
    pub accent_bias: AccentBias,
    /// Procedural paint binding.
    pub brush: Brush,
    /// `true` if widgets draw a visible stroke (thickness clamps to ≥ 8 px).
    pub has_visible_stroke: bool,
}

impl BaseProfile {
    /// Canonical `studio_dark` (the `ColorTheme` defaults from
    /// `tokens_inventory.md`, re-homed into the 8-slot palette).
    pub fn studio_dark() -> Self {
        Self {
            palette: Palette {
                // Tuned near-black so amber text clears the (strict) integer
                // contrast floor: with the floor formula on a 0..255 luminance
                // scale, max possible contrast is ~6100 and 4500 forces bg
                // luminance ≤ ~17 under near-white text. Depth order
                // bg_far < bg_near preserved. (Original 0x1A1A1F panel maxed at
                // 4013 — below floor — and is too light for a scotopic engine.)
                bg_far: Rgb8::hex(0x060609),
                bg_near: Rgb8::hex(0x0A0A0E),
                fg_text: Rgb8::hex(0xEADFC8),
                fg_muted: Rgb8::hex(0x8A8478),
                accent_primary: Rgb8::hex(0xD4A843), // Forge Amber
                accent_secondary: Rgb8::hex(0x4A78D4),
                success: Rgb8::hex(0x43D46A),
            // DISPLAY is a HERO stop (Sean 2026-07-30 "13FORGE needs to be about
            // 20x bigger"). It sat at 40px — 1.4x the heading stop — so a wordmark
            // authored at ramp[4] rendered ~18px on glass and no slot size could
            // change it: `size=mu(N)` sets the slot BOX, the glyph comes from here.
            // This is the live ramp the exe actually wears (molten inherits it via
            // `..studio_dark()`, and a roll outranks every profile sheet —
            // live.rs:76), which is why editing TokenCtx::comfy was a no-op.
                warning_danger: Rgb8::hex(0xD43535),
            },
            density: Density::Comfy,
            chrome: Chrome { curvature: 1200, thickness: 1_000, elevation: 2000 },
            motion: MotionSnap { stiffness: 220, damping: 12, duration_cap_ms: 240 },
            // 190_000 -> 102_000 (2026-08-03): the display stop was set to make the
            // wordmark huge, and it did — 190mu of glyph in the launcher's 100mu box,
            // painting straight down over the tagline, the doors line and all four nav
            // cards. `diagnostics::check_layout` now measures this instead of trusting
            // it. 102 is MEASURED, not derived: 115 bled the doors line 13mu past the
            // hero band (`no_launcher_slot_crosses_its_parents_floor`), and this kit's
            // own note records 110 bleeding 8 — both land on 102 as the ceiling.
            // Still ~4x the next stop, so the mark keeps the focal it was raised for.
            ramp: TypeRamp([12_000, 16_000, 20_000, 28_000, 102_000]),
            accent_bias: AccentBias { x_perm: 5000, y_perm: 5000 },
            brush: Brush { id: None, bus_in: None },
            has_visible_stroke: true,
        }
    }

    /// Bright **Thermal-Forge LIGHT** base — VELLUM ground + INK marks, the dtokens
    /// twin of `themes/celestial_prairie.base.sheet.vixi` (the canonical light sheet).
    /// Pairs with the bright TokenSheet so `render_lowered_full`'s node fills + chrome
    /// read light when the studio theme is switched to Vellum — the bright/airy/
    /// HIGH-CONTRAST UI law (Sean 2026-06-14, supersedes warm-dark). INK-on-VELLUM is
    /// ~13:1, well clear of the contrast floor (so `enforce_contrast` is a no-op).
    pub fn studio_light() -> Self {
        Self {
            palette: Palette {
                bg_far: Rgb8::hex(0xE3D9C0),           // VELLUM ground, slightly recessed
                bg_near: Rgb8::hex(0xF0E7D4),          // panel surface lifts off the ground
                fg_text: Rgb8::hex(0x221B17),          // INK mark (~13:1 on vellum)
                fg_muted: Rgb8::hex(0x5C4F3E),         // INK→VELLUM muted (~7:1)
                accent_primary: Rgb8::hex(0xE8843C),   // EMBER warm accent
                accent_secondary: Rgb8::hex(0x3E8FD0), // TEMPER-BLUE
                success: Rgb8::hex(0x19B6A0),          // VERDIGRIS
                warning_danger: Rgb8::hex(0xE23B22),   // FORGE-RED
            },
            density: Density::Comfy,
            chrome: Chrome { curvature: 1200, thickness: 1_000, elevation: 2000 },
            motion: MotionSnap { stiffness: 220, damping: 12, duration_cap_ms: 240 },
            ramp: TypeRamp([12_000, 16_000, 20_000, 28_000, 40_000]),
            accent_bias: AccentBias { x_perm: 5000, y_perm: 5000 },
            brush: Brush { id: None, bus_in: None },
            has_visible_stroke: true,
        }
    }

    /// **MOLTEN** — cold soot → hot bronze → white-hot spark (Sean 2026-07-17).
    /// The 8-slot palette drained verbatim from `design/molten/molten.sheet.vixi`
    /// (no second source of truth); dials mirror `studio_dark`. This is the warm
    /// forge identity the studio boots wearing — the native twin of the (retired)
    /// forge-shell amber/bronze splash. bg_far #0A0705 vs fg_text #F7E9D2 clears
    /// the integer contrast floor with room to spare (near-black vs struck bone).
    pub fn molten() -> Self {
        Self {
            palette: Palette {
                bg_far: Rgb8::hex(0x0A0705),           // cold soot
                bg_near: Rgb8::hex(0x1A0F09),          // warmed ash
                fg_text: Rgb8::hex(0xF7E9D2),          // struck bone
                fg_muted: Rgb8::hex(0xB08A63),         // cooled bronze
                accent_primary: Rgb8::hex(0xFF6A1A),   // MOLTEN core — the one focal heat
                accent_secondary: Rgb8::hex(0xC8791E), // hot bronze
                success: Rgb8::hex(0x7FB86A),          // forge-cooled patina
                warning_danger: Rgb8::hex(0xFFD54A),   // white-hot spark
            },
            ..Self::studio_dark()
        }
    }

    /// **PERMAFROST** — polar night → ice shelf → white-out flare: molten's cold
    /// twin (Sean 2026-07-17 "Permafrost version that is like blue"). Palette
    /// drained from `design/permafrost/permafrost.sheet.vixi`; the proven traded
    /// re-skin — same surfaces, this base swapped in hot.
    pub fn permafrost() -> Self {
        Self {
            palette: Palette {
                bg_far: Rgb8::hex(0x05090F),           // polar night
                bg_near: Rgb8::hex(0x0C1622),          // ice shelf shadow
                fg_text: Rgb8::hex(0xE8F4FF),          // cut ice
                fg_muted: Rgb8::hex(0x7FA3C4),         // frosted steel
                accent_primary: Rgb8::hex(0x3BC7FF),   // PERMAFROST core — the one focal cold
                accent_secondary: Rgb8::hex(0x1E7FB8), // deep floe
                success: Rgb8::hex(0x6FE0B8),          // meltwater
                warning_danger: Rgb8::hex(0xCFF2FF),   // white-out flare
            },
            ..Self::studio_dark()
        }
    }
}

/// Per-axis lock flags (`variation_axes.md §Lock Axes`). A locked axis copies
/// the base value; an unlocked axis receives a fresh perturbation from the seed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LockAxes {
    /// Lock the colour palette to the base.
    pub palette: bool,
    /// Lock chrome dials to the base.
    pub chrome: bool,
    /// Lock motion tuning to the base.
    pub motion: bool,
    /// Lock the type ramp to the base.
    pub ramp: bool,
    /// Lock the accent focal point to the base.
    pub accent_bias: bool,
}

// ---------------------------------------------------------------------------
// Resolver
// ---------------------------------------------------------------------------

/// Inclusive integer draw in `[lo, hi]` from the seed stream.
fn draw_range(rng: &mut Mulberry32, lo: i32, hi: i32) -> i32 {
    debug_assert!(hi >= lo);
    let span = (hi - lo) as u32 + 1;
    lo + (rng.next_u32() % span) as i32
}

/// Integer saturation adjust toward/away from per-pixel luminance gray.
/// `bias` in Permyriad: `+` saturates, `-` desaturates. Channel-clamped 0..255.
fn apply_saturation(c: Rgb8, bias: i32) -> Rgb8 {
    let gray = c.luminance(); // 0..255
    let adj = |ch: u8| -> u8 {
        let delta = ch as i64 - gray;
        let scaled = gray + delta * (10_000 + bias as i64) / 10_000;
        scaled.clamp(0, 255) as u8
    };
    Rgb8 { r: adj(c.r), g: adj(c.g), b: adj(c.b) }
}

fn lighten(c: Rgb8, step: u8) -> Rgb8 {
    Rgb8 { r: c.r.saturating_add(step), g: c.g.saturating_add(step), b: c.b.saturating_add(step) }
}
fn darken(c: Rgb8, step: u8) -> Rgb8 {
    Rgb8 { r: c.r.saturating_sub(step), g: c.g.saturating_sub(step), b: c.b.saturating_sub(step) }
}

/// `floor_rules.md` auto-clamp: spread `fg` and `bg` apart (lighten the lighter,
/// darken the darker) until contrast ≥ floor. Moving BOTH is required — under
/// the strict integer formula, near-white text alone cannot clear the floor
/// against a mid-dark bg (max single-sided contrast ≈ 4013). Converges to the
/// 6100 extreme; guarded. No-op when the pair already passes (the studio
/// defaults are pre-tuned, so this never fires for them).
fn enforce_contrast(fg: &mut Rgb8, bg: &mut Rgb8) {
    let mut guard = 0;
    while contrast_ratio(*fg, *bg) < CONTRAST_FLOOR && guard < 128 {
        if fg.luminance() >= bg.luminance() {
            *fg = lighten(*fg, 8);
            *bg = darken(*bg, 8);
        } else {
            *fg = darken(*fg, 8);
            *bg = lighten(*bg, 8);
        }
        guard += 1;
    }
}

/// Scale a 5-stop ramp by a Permyriad delta, then clamp so `ramp[1]` (body)
/// stays ≥ floor by lifting the whole ramp proportionally (keeps it monotone).
fn scale_ramp(base: TypeRamp, scale_perm: i32) -> TypeRamp {
    let factor = 10_000 + scale_perm as i64;
    let mut out = [0i64; 5];
    for (i, &v) in base.0.iter().enumerate() {
        out[i] = (v * factor / 10_000).max(1_000);
    }
    if out[1] < BODY_RAMP_FLOOR_MU {
        // Re-lift the whole ramp (ceil division) so body hits the floor exactly
        // while preserving proportions and monotonicity.
        let den = out[1].max(1);
        for v in out.iter_mut() {
            *v = (*v * BODY_RAMP_FLOOR_MU + den - 1) / den;
        }
        out[1] = out[1].max(BODY_RAMP_FLOOR_MU);
    }
    TypeRamp(out)
}

/// Resolve a base profile + seed (+ locks) into canonical [`DesignTokens`].
///
/// Order (`variation_axes.md`): draw each unlocked axis from the Mulberry32
/// stream, clamp per-axis, apply, then re-check the contrast + hit-target floors.
pub fn resolve(base: &BaseProfile, seed: u64, locks: &LockAxes) -> DesignTokens {
    let mut rng = Mulberry32::new(seed);

    // --- palette (saturation_bias; hue_shift DEFERRED — needs integer HSL/YIQ
    //     rotation; saturation + the contrast floor cover v1 chroma needs). ---
    let mut palette = base.palette;
    if !locks.palette {
        let sat_bias = draw_range(&mut rng, -3000, 3000); // clamp range; desat floor -7000 unreached
        palette.accent_primary = apply_saturation(palette.accent_primary, sat_bias);
        palette.accent_secondary = apply_saturation(palette.accent_secondary, sat_bias);
        palette.success = apply_saturation(palette.success, sat_bias);
        palette.warning_danger = apply_saturation(palette.warning_danger, sat_bias);
    }

    // --- chrome ---
    let chrome = if locks.chrome {
        base.chrome
    } else {
        let curvature = draw_range(&mut rng, 0, 10_000);
        let elevation = draw_range(&mut rng, 0, 10_000);
        // Sanitized thickness range (see UNIT CONVENTION): 0..8px, min 1px if
        // visible-stroke. Taxonomy's "0..400 MilliUnit" reads sub-pixel.
        let mut thickness = draw_range(&mut rng, 0, 8_000) as i64;
        if base.has_visible_stroke && thickness < 1_000 {
            thickness = 1_000;
        }
        Chrome { curvature, thickness, elevation }
    };

    // --- motion.snap (hard-clamped at edges) ---
    let motion = if locks.motion {
        base.motion
    } else {
        MotionSnap {
            stiffness: draw_range(&mut rng, 50, 500),
            damping: draw_range(&mut rng, 4, 20),
            duration_cap_ms: base.motion.duration_cap_ms,
        }
    };

    // --- type.ramp.scale (body floor clamp) ---
    let ramp = if locks.ramp {
        base.ramp
    } else {
        let scale = draw_range(&mut rng, -2000, 3000);
        scale_ramp(base.ramp, scale)
    };

    // --- accent.bias ---
    let accent_bias = if locks.accent_bias {
        base.accent_bias
    } else {
        AccentBias {
            x_perm: draw_range(&mut rng, 0, 10_000),
            y_perm: draw_range(&mut rng, 0, 10_000),
        }
    };

    // --- floor re-check: contrast (text on near bg) ---
    let mut bg_near = palette.bg_near;
    enforce_contrast(&mut palette.fg_text, &mut bg_near);
    palette.bg_near = bg_near;

    DesignTokens {
        palette,
        density: base.density,
        chrome,
        motion,
        ramp,
        accent_bias,
        brush: base.brush.clone(),
    }
}

// ---------------------------------------------------------------------------
// authored profile sheets (themes/*.profile.sheet.vixi) → BaseProfile
// ---------------------------------------------------------------------------
//
// REMOVED (gap named, not faked): v1's `profile_by_name` + its `slot()`/
// `rgb_from_packed()` helpers read `forge_canvas::tokens::sheet_by_name`, a
// registry generated at build time by `forge-canvas/build.rs` (75 lines)
// scanning `themes/*.profile.sheet.vixi` theme files. Neither the build.rs
// codegen nor the theme-file assets have a v3 home yet — checked, confirmed
// absent from `forge-canvas-v3` (which has no `build.rs` and no `themes/`
// dir). Porting this one function would mean porting the AOT codegen
// pipeline too, not just a function body. `BaseProfile::studio_dark()` (the
// hardcoded default this function falls back to) is unaffected and stays.

/// A `profile:` header name → the sheet it names. `None` on an unknown name —
/// the caller keeps its own floor rather than inventing a theme.
///
/// Ported 2026-08-26 from v2's `F:\NewRepo\crates\forge-vix\src\tokens.rs:601-630`
/// (`profile_by_name`), the half v2's `live::ctx_for` (`live.rs:75-83`) called.
/// Every `.kit.vixi` in the tree authors a `profile:` line and until now NOTHING
/// read it: [`crate::loader::load_kit_comfy`] hard-coded the comfy floor, so a
/// kit asking for `molten` wore exactly the same tokens as one asking for
/// `permafrost`. The header was a statement no loader honoured.
pub fn profile_by_name(name: &str) -> Option<BaseProfile> {
    Some(match name {
        "studio_dark" => BaseProfile::studio_dark(),
        "studio_light" => BaseProfile::studio_light(),
        "molten" => BaseProfile::molten(),
        "permafrost" => BaseProfile::permafrost(),
        _ => return None,
    })
}

impl BaseProfile {
    /// This profile verbatim, no dice applied — the authored values as
    /// [`DesignTokens`]. `resolve` is the seeded path; this is the "wear exactly
    /// what the sheet says" path a `profile:` kit header takes.
    pub fn to_tokens(&self) -> DesignTokens {
        DesignTokens {
            palette: self.palette,
            density: self.density,
            chrome: self.chrome,
            motion: self.motion,
            ramp: self.ramp,
            accent_bias: self.accent_bias,
            brush: self.brush.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens_of(base: &BaseProfile) -> DesignTokens {
        DesignTokens {
            palette: base.palette,
            density: base.density,
            chrome: base.chrome,
            motion: base.motion,
            ramp: base.ramp,
            accent_bias: base.accent_bias,
            brush: base.brush.clone(),
        }
    }

    /// One dial, three families: at full S nothing moves, at zero S the
    /// chromatic slots go grey, corners square, and padding tightens — while
    /// the authored bg/fg greys stay exactly where the profile put them.
    #[test]
    fn s_channel_moves_colour_radius_and_padding_together() {
        assert_eq!(s_preset("studio"), Some(7_000));
        assert_eq!(s_preset("nope"), None);

        let base = BaseProfile::molten();
        let t = tokens_of(&base);

        let full = t.with_s(S_MAX);
        assert_eq!(full.palette.accent_primary, base.palette.accent_primary);
        assert_eq!(full.chrome.curvature, base.chrome.curvature);
        assert_eq!(full.density, Density::Spacious);

        let flat = t.with_s(S_MIN);
        let a = flat.palette.accent_primary;
        assert!(a.r == a.g && a.g == a.b, "S=0 must flatten to grey, got {a:?}");
        assert_eq!(flat.chrome.curvature, 0, "flat chrome squares its corners");
        assert_eq!(flat.density, Density::Compact);
        assert_eq!(flat.palette.bg_far, base.palette.bg_far, "greys are untouched");
        assert_eq!(flat.palette.fg_text, base.palette.fg_text, "greys are untouched");

        // Out-of-range input clamps instead of wrapping.
        assert_eq!(t.with_s(-5_000), flat);
        assert_eq!(t.with_s(50_000), full);
    }

    #[test]
    fn studio_dark_meets_contrast_floor() {
        let t = resolve(&BaseProfile::studio_dark(), 1, &LockAxes::default());
        assert!(
            contrast_ratio(t.palette.fg_text, t.palette.bg_near) >= CONTRAST_FLOOR,
            "contrast {} < {}",
            contrast_ratio(t.palette.fg_text, t.palette.bg_near),
            CONTRAST_FLOOR
        );
    }

    /// The boot-look bases (Sean 2026-07-23) resolved palette-locked (exactly how
    /// the exe boots molten) must clear the contrast floor — the studio wears this
    /// from the first frame, so a low-contrast HUD would ship. molten's authored
    /// text/bg pair sits just under floor (~4208); the resolver's enforce_contrast
    /// legitimately spreads it to clear 4500 while leaving the accents untouched,
    /// so it stays visually molten. This is the exact palette apply_live_theme
    /// overlays onto every rendered sheet.
    #[test]
    fn molten_and_permafrost_boot_bases_meet_contrast_floor() {
        let locks = LockAxes { palette: true, ..Default::default() };
        for (name, base) in [("molten", BaseProfile::molten()), ("permafrost", BaseProfile::permafrost())] {
            let t = resolve(&base, 0, &locks);
            assert!(
                contrast_ratio(t.palette.fg_text, t.palette.bg_near) >= CONTRAST_FLOOR,
                "{name}: text/bg contrast {} < {}",
                contrast_ratio(t.palette.fg_text, t.palette.bg_near),
                CONTRAST_FLOOR
            );
            // The focal accent is the theme's identity — it must survive verbatim
            // (the floor clamp only ever moves the text/bg pair apart).
            assert_eq!(
                t.palette.accent_primary, base.palette.accent_primary,
                "{name}: focal accent must pass through unclamped"
            );
        }
    }

    #[test]
    fn resolve_populates_chrome_curvature_axis() {
        // The `.vibe.vixi` → DesignTokens curvature axis (Permyriad 0..10000). The
        // forge-canvas render bridge maps it through curvature_to_radius; the widget
        // path uses the parallel forge-canvas ChromeCurvature token. Here we prove the
        // resolver populates the curvature axis (its forge-vix source-of-truth).
        let t = resolve(&BaseProfile::studio_dark(), 7, &LockAxes::default());
        assert!(
            (0..=10_000).contains(&t.chrome.curvature),
            "chrome.curvature {} must be a clamped permyriad (the VixiScript radius axis)",
            t.chrome.curvature
        );
    }

    #[test]
    fn resolve_is_deterministic() {
        let base = BaseProfile::studio_dark();
        let a = resolve(&base, 0xDEAD_BEEF, &LockAxes::default());
        let b = resolve(&base, 0xDEAD_BEEF, &LockAxes::default());
        assert_eq!(a, b, "same seed must produce byte-identical tokens");
    }

    #[test]
    fn different_seeds_diverge() {
        let base = BaseProfile::studio_dark();
        let a = resolve(&base, 1, &LockAxes::default());
        let b = resolve(&base, 999_999, &LockAxes::default());
        assert_ne!(a, b);
    }

    #[test]
    fn locked_axis_copies_base() {
        let base = BaseProfile::studio_dark();
        let locks = LockAxes { chrome: true, motion: true, ramp: true, accent_bias: true, palette: true };
        let t = resolve(&base, 12345, &locks);
        assert_eq!(t.chrome, base.chrome);
        assert_eq!(t.motion, base.motion);
        assert_eq!(t.ramp, base.ramp);
        assert_eq!(t.accent_bias, base.accent_bias);
        // palette locked → accents unperturbed (only fg_text may be floor-clamped)
        assert_eq!(t.palette.accent_primary, base.palette.accent_primary);
    }

    #[test]
    fn motion_axes_clamped_in_range() {
        let base = BaseProfile::studio_dark();
        for seed in 0..200u64 {
            let t = resolve(&base, seed, &LockAxes::default());
            assert!((50..=500).contains(&t.motion.stiffness), "stiffness {}", t.motion.stiffness);
            assert!((4..=20).contains(&t.motion.damping), "damping {}", t.motion.damping);
        }
    }

    #[test]
    fn ramp_body_floor_holds_under_max_shrink() {
        // Smallest base body that, scaled down, would dip below floor.
        let mut base = BaseProfile::studio_dark();
        base.ramp = TypeRamp([10_000, 12_000, 14_000, 18_000, 24_000]);
        for seed in 0..200u64 {
            let t = resolve(&base, seed, &LockAxes::default());
            assert!(t.ramp.0[1] >= BODY_RAMP_FLOOR_MU, "body {} < floor", t.ramp.0[1]);
            // ramp stays monotone non-decreasing
            assert!(t.ramp.0.windows(2).all(|w| w[1] >= w[0]));
        }
    }

    #[test]
    fn to_token_ctx_projects_sizing() {
        let t = resolve(&BaseProfile::studio_dark(), 7, &LockAxes { ramp: true, ..Default::default() });
        let ctx = t.to_token_ctx();
        assert_eq!(ctx.density_base, 12_000); // comfy
        // Display stop tracks `studio_dark()` (:319, 190_000 -> 102_000 on 2026-08-03,
        // the measured ceiling of the launcher's folded hero band).
        assert_eq!(ctx.ramp, [12_000, 16_000, 20_000, 28_000, 102_000]);
        // motion.snap is now PROJECTED (it used to be dropped on the floor here).
        assert_eq!(ctx.motion, t.motion);
    }

    #[test]
    fn motion_snap_spring_actually_animates_and_settles() {
        // The deliverable's own proof: a spring seeded from the resolved
        // motion.snap token must produce real motion AND latch to its target on
        // the 120Hz grid — not coast, not limit-cycle.
        let snap = MotionSnap::comfy();
        let mut s = snap.spring(0);
        s.set_target(10_000);

        let mut ticks = 0;
        let mut moved = false;
        while !s.settled() && ticks < 600 {
            s.tick();
            if s.value > 0 {
                moved = true; // it left the start — real animation, not a snap-jump
            }
            ticks += 1;
        }
        assert!(moved, "spring never moved off start — token didn't drive motion");
        assert!(s.settled(), "spring did not settle within {ticks} ticks");
        assert_eq!(s.value, 10_000, "settled off-target at {}", s.value);
        assert!(ticks > 1, "reached target in one tick — not a spring");
    }

    #[test]
    fn contrast_enforcer_is_monotone() {
        // A deliberately low-contrast pair must be lifted to the floor. Single-
        // sided lift can't reach it (bg too light), so the enforcer spreads both.
        let mut bg = Rgb8::hex(0x1A1A1F);
        let mut fg = Rgb8::hex(0x202025); // near-bg, fails floor
        assert!(contrast_ratio(fg, bg) < CONTRAST_FLOOR);
        enforce_contrast(&mut fg, &mut bg);
        assert!(contrast_ratio(fg, bg) >= CONTRAST_FLOOR);
    }
}

