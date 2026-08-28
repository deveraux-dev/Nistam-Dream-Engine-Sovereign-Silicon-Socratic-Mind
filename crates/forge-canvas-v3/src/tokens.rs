//! Design token system — CSS-like cascade, zero-alloc resolution.
//! Fixed vocabulary, O(1) lookup by TokenId.
//! Resolution: Base < Profile < Celestial < Override.

#![allow(unused_imports)]
use crate::theme::{pack_rgba, lighten_u32, darken_u32};

/// Maximum token vocabulary size.
///
/// Raised 128 -> 256 on 2026-05-27 (Sean approval) to give the Forge-variant
/// work (Prairie / Goblin / Frost / Spirit / Highcontrast / N more) headroom
/// for variant-specific tokens without renumber-pain. Current usage ~75;
/// Patches 1+7+12 of the substrate plan project ~110; variants project +30-60
/// over the lifetime. 256 is a 10-year ceiling, not a 2026 ceiling.
pub const TOKEN_CAPACITY: usize = 256;

/// Cascade layer — higher layer wins during resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Layer {
    /// Base layer — lowest precedence.
    Base = 0,
    /// Profile layer — overrides base.
    Profile = 1,
    /// Celestial layer — overrides profile (sky state).
    Celestial = 2,
    /// Override layer — highest precedence (user/persistent choice).
    Override = 3,
}

/// Semantic token IDs. Stable across versions — append only, never reorder.
///
/// Each variant represents a single token slot (0..256) in the TokenSheet.
/// Appending is safe; reordering breaks deserialized sheets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum TokenId {
    // Backgrounds
    /// Main window background / void.
    BgVoid = 0,
    /// Nebula / atmospheric layer.
    BgNebula = 1,
    /// Dust / tertiary background.
    BgDust = 2,
    /// Hover state background.
    BgHover = 3,
    /// Active state background.
    BgActive = 4,

    // Text
    /// Primary high-contrast text.
    TextPrimary = 10,
    /// Muted / secondary text.
    TextMuted = 11,
    /// Disabled / low-contrast text.
    TextDisabled = 12,

    // Semantic accents
    /// Primary accent — Forge creation / world-building identity.
    AccentCreation = 20,
    /// Secondary accent — curiosity / discovery (NDE content).
    AccentCuriosity = 21,
    /// Tertiary accent — magnificence / loot / reward.
    AccentMagnificence = 22,
    /// Quaternary accent — wonder / keyframe.
    AccentWonder = 23,

    // Functional
    /// Error / critical signal.
    Danger = 30,
    /// Warning signal.
    Warning = 31,
    /// Success / completion signal.
    Success = 32,
    /// Informational signal.
    Info = 33,

    // Borders
    /// Default border / separator.
    Border = 40,
    /// Focus ring / active border.
    BorderFocus = 41,
    /// Subtle divider line.
    Separator = 42,

    // Waveform bands
    /// Audio low-frequency band (bass).
    BandLow = 50,
    /// Audio mid-frequency band.
    BandMid = 51,
    /// Audio high-frequency band (treble).
    BandHigh = 52,

    // Spacing (value = milliunit pixels)
    /// Extra-small spacing unit (MilliUnit).
    SpaceXs = 60,
    /// Small spacing unit (MilliUnit).
    SpaceSm = 61,
    /// Medium spacing unit (MilliUnit).
    SpaceMd = 62,
    /// Large spacing unit (MilliUnit).
    SpaceLg = 63,
    /// Extra-large spacing unit (MilliUnit).
    SpaceXl = 64,

    // Timing (value = milliseconds)
    /// Fast animation duration (ms).
    DurationFast = 70,
    /// Normal animation duration (ms).
    DurationNormal = 71,
    /// Slow animation duration (ms).
    DurationSlow = 72,

    // Celestial dynamic
    /// Moon phase state (0=new, 127=full).
    CelestialPhase = 80,
    /// Planetary conjunction indicator.
    CelestialConjunction = 81,
    /// Sky darkness from clouds + time of day.
    CelestialDarkness = 82,

    // ── Smithy substrate (Patch 1, 2026-05-26) ─────────────────────────────
    /// Chrome curvature geometry (Permyriad / MilliUnit).
    ChromeCurvature = 83,
    /// Chrome thickness geometry (Permyriad / MilliUnit).
    ChromeThickness = 84,
    /// Chrome elevation geometry (Permyriad / MilliUnit).
    ChromeElevation = 85,

    // Tab states — Smithy doctrine: baseline accent on active (NOT fill-invert).
    /// Tab background when active.
    TabBgActive = 90,
    /// Tab background when inactive.
    TabBgInactive = 91,
    /// Tab background on hover.
    TabBgHover = 92,
    /// Tab text when active.
    TabTextActive = 93,
    /// Tab text when inactive.
    TabTextInactive = 94,
    /// Tab baseline accent strip (gold).
    TabBaselineAccent = 95,

    // Window-bar states
    /// Window title bar background.
    WindowBarBg = 100,
    /// Window title bar active state.
    WindowBarActive = 101,
    /// Window title bar pulse / attention.
    WindowBarPulse = 102,

    // Status bar
    /// Status bar background.
    StatusBg = 110,
    /// Status bar text colour.
    StatusText = 111,
    /// Status bar heartbeat / activity indicator.
    StatusHeartbeat = 112,

    // ── System Toggle / Sentinel substrate (Pass 5, 2026-05-29) ────────────────
    /// Admin/monitoring panel background.
    SystemToggleBg = 113,
    /// System toggle button gaming mode fill.
    SystemToggleButtonGamingFill = 114,
    /// System toggle button gaming mode hover fill.
    SystemToggleButtonGamingFillHover = 115,
    /// System toggle button dev mode fill.
    SystemToggleButtonDevFill = 116,
    /// System toggle button dev mode hover fill.
    SystemToggleButtonDevFillHover = 117,
    /// System toggle stats label colour.
    SystemToggleStatsLabel = 118,
    /// System toggle log text colour.
    SystemToggleLogText = 119,
    /// Sentinel critical alert colour (RAM>90%).
    SentinelCritical = 120,

    // ── v10 finished-surface tokens (2026-06-03) — signature gold→ember gradient
    // + World/Blueprint semantic node colours.
    /// Gold — headings, active elements, gradient top.
    Gold = 121,
    /// Deep — hover border, gradient tail.
    Deep = 122,
    /// Node colour for character type.
    NodeCharacter = 123,
    /// Node colour for dialogue type.
    NodeDialogue = 124,
    /// Node colour for lore type.
    NodeLore = 125,
    /// Node colour for town type.
    NodeTown = 126,
    /// Node colour for place type.
    NodePlace = 127,

    // ── SYN_SKIN palette (syn_skin.profile.sheet.vixi, 2026-06-22) ──────────
    /// Synthesia HUD chrome — window void behind panels.
    Ground = 128,
    /// Synthesia HUD chrome — panel / bar fill.
    Bar = 129,
    /// Synthesia HUD chrome — resting border.
    Frame = 130,
    /// Synthesia HUD chrome — bright cool text.
    Title = 131,
    /// Synthesia HUD chrome — mid slate-blue body text.
    Status = 132,
    /// Synthesia HUD chrome — NEON CYAN (live / active / hot edge).
    Accent = 133,
    /// Synthesia HUD chrome — NEON GREEN (app mark / readback anchor / "on").
    Mark = 134,
    /// Synthesia HUD chrome — neon magenta (playhead / special accent).
    Violet = 135,

    // ── Canvas tool rail (canvas_tools.rs Gate C migration, 2026-06-23) ─────
    /// Tool rail panel background (lighter panel).
    CanvasRailBg = 136,
    /// Tool rail panel border (visible warm).
    CanvasRailBorder = 137,
    /// Inactive tool icon colour.
    CanvasIconFaint = 138,
}

impl TokenId {
    /// Lookup by name (build-time resolution of $token refs).
    ///
    /// Returns the TokenId variant matching the snake_case name,
    /// or None if the name is unrecognized.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "bg_void" => Some(Self::BgVoid),
            "bg_nebula" => Some(Self::BgNebula),
            "bg_dust" => Some(Self::BgDust),
            "bg_hover" => Some(Self::BgHover),
            "bg_active" => Some(Self::BgActive),
            "text_primary" => Some(Self::TextPrimary),
            "text_muted" => Some(Self::TextMuted),
            "text_disabled" => Some(Self::TextDisabled),
            "accent_creation" => Some(Self::AccentCreation),
            "accent_curiosity" => Some(Self::AccentCuriosity),
            "accent_magnificence" => Some(Self::AccentMagnificence),
            "accent_wonder" => Some(Self::AccentWonder),
            "danger" => Some(Self::Danger),
            "warning" => Some(Self::Warning),
            "success" => Some(Self::Success),
            "info" => Some(Self::Info),
            "border" => Some(Self::Border),
            "border_focus" => Some(Self::BorderFocus),
            "separator" => Some(Self::Separator),
            "band_low" => Some(Self::BandLow),
            "band_mid" => Some(Self::BandMid),
            "band_high" => Some(Self::BandHigh),
            "space_xs" => Some(Self::SpaceXs),
            "space_sm" => Some(Self::SpaceSm),
            "space_md" => Some(Self::SpaceMd),
            "space_lg" => Some(Self::SpaceLg),
            "space_xl" => Some(Self::SpaceXl),
            "duration_fast" => Some(Self::DurationFast),
            "duration_normal" => Some(Self::DurationNormal),
            "duration_slow" => Some(Self::DurationSlow),
            // Smithy substrate (Patch 1, 2026-05-26)
            "chrome_curvature" => Some(Self::ChromeCurvature),
            "chrome_thickness" => Some(Self::ChromeThickness),
            "chrome_elevation" => Some(Self::ChromeElevation),
            "tab_bg_active" => Some(Self::TabBgActive),
            "tab_bg_inactive" => Some(Self::TabBgInactive),
            "tab_bg_hover" => Some(Self::TabBgHover),
            "tab_text_active" => Some(Self::TabTextActive),
            "tab_text_inactive" => Some(Self::TabTextInactive),
            "tab_baseline_accent" => Some(Self::TabBaselineAccent),
            "window_bar_bg" => Some(Self::WindowBarBg),
            "window_bar_active" => Some(Self::WindowBarActive),
            "window_bar_pulse" => Some(Self::WindowBarPulse),
            "status_bg" => Some(Self::StatusBg),
            "status_text" => Some(Self::StatusText),
            "status_heartbeat" => Some(Self::StatusHeartbeat),
            // System Toggle / Sentinel substrate (Pass 5, 2026-05-29)
            "system_toggle_bg" => Some(Self::SystemToggleBg),
            "system_toggle_button_gaming_fill" => Some(Self::SystemToggleButtonGamingFill),
            "system_toggle_button_gaming_fill_hover" => Some(Self::SystemToggleButtonGamingFillHover),
            "system_toggle_button_dev_fill" => Some(Self::SystemToggleButtonDevFill),
            "system_toggle_button_dev_fill_hover" => Some(Self::SystemToggleButtonDevFillHover),
            "system_toggle_stats_label" => Some(Self::SystemToggleStatsLabel),
            "system_toggle_log_text" => Some(Self::SystemToggleLogText),
            "sentinel_critical" => Some(Self::SentinelCritical),
            "gold" => Some(Self::Gold),
            "deep" => Some(Self::Deep),
            "node_character" => Some(Self::NodeCharacter),
            "node_dialogue" => Some(Self::NodeDialogue),
            "node_lore" => Some(Self::NodeLore),
            "node_town" => Some(Self::NodeTown),
            "node_place" => Some(Self::NodePlace),
            // SYN_SKIN palette (syn_skin.profile.sheet.vixi, 2026-06-22)
            "ground" => Some(Self::Ground),
            "bar" => Some(Self::Bar),
            "frame" => Some(Self::Frame),
            "title" => Some(Self::Title),
            "status" => Some(Self::Status),
            "accent" => Some(Self::Accent),
            "mark" => Some(Self::Mark),
            "violet" => Some(Self::Violet),
            // Canvas tool rail
            "canvas_rail_bg" => Some(Self::CanvasRailBg),
            "canvas_rail_border" => Some(Self::CanvasRailBorder),
            "canvas_icon_faint" => Some(Self::CanvasIconFaint),
            _ => None,
        }
    }

    /// True if this token stores a **dimension** scalar (MilliUnit / Permyriad /
    /// milliseconds) rather than a packed RGBA colour.
    ///
    /// This is the type-punning boundary: dimension slots and colour slots share one `u32` array.
    /// A colour sentinel (e.g. `MAGENTA_UNRESOLVED`) cast into a dimension slot is silent garbage.
    /// The typed getters (`TokenSheet::get_dim_or` / `get_color_or`) debug-assert against this
    /// so a miscategorised read trips in dev, never ships magenta layout unit.
    pub const fn is_dimension(self) -> bool {
        matches!(
            self,
            TokenId::SpaceXs
                | TokenId::SpaceSm
                | TokenId::SpaceMd
                | TokenId::SpaceLg
                | TokenId::SpaceXl
                | TokenId::DurationFast
                | TokenId::DurationNormal
                | TokenId::DurationSlow
                | TokenId::ChromeCurvature
                | TokenId::ChromeThickness
                | TokenId::ChromeElevation
        )
    }
}

/// A resolved token sheet. Fixed-size array, fits in L1 cache.
/// 128 × 4 bytes = 512 bytes for values + 128 bytes for layers = 640 bytes total.
#[derive(Clone, Debug)]
pub struct TokenSheet {
    /// Packed RGBA values indexed by TokenId. 0 = unset.
    pub values: [u32; TOKEN_CAPACITY],
    /// Which layer set each value.
    pub layers: [Layer; TOKEN_CAPACITY],
}

impl Default for TokenSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenSheet {
    /// Create a new empty token sheet (all slots zero, all layers Base).
    pub const fn new() -> Self {
        Self {
            values: [0u32; TOKEN_CAPACITY],
            layers: [Layer::Base; TOKEN_CAPACITY],
        }
    }

    /// Set a token value at a given layer. Higher layer wins.
    ///
    /// If the new layer has equal or higher precedence than the existing layer,
    /// the value and layer are updated. Otherwise no change.
    #[inline]
    pub fn set(&mut self, id: TokenId, value: u32, layer: Layer) {
        let idx = id as usize;
        if idx < TOKEN_CAPACITY && layer as u8 >= self.layers[idx] as u8 {
            self.values[idx] = value;
            self.layers[idx] = layer;
        }
    }

    /// Get resolved value. O(1). Raw, untyped — prefer [`get_color_or`] /
    /// [`get_dim_or`] at consumption sites so the colour↔dimension boundary is
    /// explicit and a missing slot can't silently type-pun.
    ///
    /// [`get_color_or`]: TokenSheet::get_color_or
    /// [`get_dim_or`]: TokenSheet::get_dim_or
    #[inline]
    pub fn get(&self, id: TokenId) -> u32 {
        self.values[id as usize]
    }

    /// Get resolved **colour** value, or `fallback` when the slot is unset (0).
    ///
    /// Pass `MAGENTA_UNRESOLVED` as `fallback` so a missing colour screams on screen
    /// instead of rendering transparent-black.
    ///
    /// # Panics
    ///
    /// Debug-asserts the token is NOT a dimension — reading a layout unit through
    /// the colour getter is the type-pun this split exists to kill.
    #[inline]
    pub fn get_color_or(&self, id: TokenId, fallback: u32) -> u32 {
        assert!(
            !id.is_dimension(),
            "get_color_or called on dimension token {id:?} — use get_dim_or"
        );
        let v = self.values[id as usize];
        if v != 0 { v } else { fallback }
    }

    /// Get resolved **dimension** value, or `fallback` when the slot is unset (0).
    ///
    /// Pass a safe layout unit — `0` for brutalist-square / no-border,
    /// or e.g. `10_000` (1px MilliUnit) — NEVER a colour sentinel.
    /// This is the guard: a dimension can never fall back to `MAGENTA_UNRESOLVED`.
    ///
    /// # Panics
    ///
    /// Debug-asserts the token IS a dimension — reading a colour through the
    /// dimension getter is the inverse type-pun.
    #[inline]
    pub fn get_dim_or(&self, id: TokenId, fallback: u32) -> u32 {
        assert!(
            id.is_dimension(),
            "get_dim_or called on colour token {id:?} — use get_color_or"
        );
        let v = self.values[id as usize];
        if v != 0 { v } else { fallback }
    }

    /// Resolve by overlaying sheets. Each successive sheet overrides if layer >= current.
    ///
    /// Base sheet is cloned, then each overlay is applied in order.
    /// Only non-zero values in higher layers override lower layers.
    pub fn resolve(base: &TokenSheet, overlays: &[&TokenSheet]) -> Self {
        let mut result = base.clone();
        for overlay in overlays {
            for i in 0..TOKEN_CAPACITY {
                if overlay.values[i] != 0 && overlay.layers[i] as u8 >= result.layers[i] as u8 {
                    result.values[i] = overlay.values[i];
                    result.layers[i] = overlay.layers[i];
                }
            }
        }
        result
    }

    /// Re-stamp every SET (non-zero) token of this sheet to `layer`, leaving unset
    /// slots untouched.
    ///
    /// Use to promote a viewer-chosen skin (the `.forge` vibe) to `Layer::Override`
    /// so it DOMINATES the live-sky celestial overlay instead of losing to it —
    /// a chosen skin is the top of the cascade, not the middle.
    /// Stack-only: copies two fixed arrays, no heap (mirrors `resolve`'s 640B copy).
    pub fn promoted(&self, layer: Layer) -> Self {
        let mut out = TokenSheet::new();
        out.values = self.values;
        for i in 0..TOKEN_CAPACITY {
            out.layers[i] = if self.values[i] != 0 { layer } else { self.layers[i] };
        }
        out
    }
}

/// Spectral temperature buckets from forge-celestial lore.
/// Maps star spectral class to palette temperature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpectralTemp {
    /// O/B class — cool blue.
    Frost,
    /// F/G class — neutral warm.
    Gold,
    /// K class — warm orange.
    Iron,
    /// M class — hot red.
    Ember,
    /// Unknown — deep purple.
    Void,
}

/// Sky state snapshot — populated by API fetch or game tick.
/// Passed into overlay generators. No network code here.
///
/// # Float Justification
/// This struct uses f32 because it lives at the **GPU dispatch boundary** —
/// it feeds CanvasUniforms (shader uniforms) and token color selection.
/// It is part of the Automata branch (visual-only), NOT the physics kernel.
/// The game's Spatial branch stores equivalent values as MilliUnits/Permyriads
/// and converts to SkyState at the boundary via `/ 1000.0` or `/ 10000.0`.
#[derive(Clone, Debug)]
pub struct SkyState {
    /// Spectral temperature of dominant star (drives accent warmth).
    pub spectral: SpectralTemp,
    /// Moon phase 0.0=new, 0.5=full, 1.0=new again.
    pub moon_phase: f32,
    /// Cloud cover 0-8 oktas.
    pub cloud_cover: u8,
    /// Wind speed m/s (drives vibe_shake).
    pub wind_speed: f32,
    /// Is it currently daytime.
    pub is_daytime: bool,
    /// Stability class A-F (turbulence). 0=A(unstable) .. 5=F(very stable).
    pub stability: u8,
}

impl Default for SkyState {
    fn default() -> Self {
        Self {
            spectral: SpectralTemp::Gold,
            moon_phase: 0.0,
            cloud_cover: 2,
            wind_speed: 1.5,
            is_daytime: false,
            stability: 3, // D = neutral
        }
    }
}

/// Generate a Celestial-layer overlay from live sky state.
///
/// Shifts accent colors warm/cool based on spectral temperature,
/// adjusts nebula opacity from cloud cover, writes CelestialPhase/Darkness.
///
/// The primary chrome accent (AccentCreation) is intentionally NOT set —
/// it stays locked to the per-window profile (warm ember/amber).
pub fn celestial_overlay(sky: &SkyState) -> TokenSheet {
    let mut s = TokenSheet::new();

    // Spectral temperature → accent color shift.
    // v10 chrome lock (2026-06-03): the PRIMARY chrome accent (AccentCreation) is NOT
    // sky-reactive — it stays locked to the per-window profile (warm ember/amber) so
    // World/Forge/Play/Ship/Blueprint match Canvas's locked v10 treatment instead of
    // drifting cool/violet under Frost/Void skies. The sky still shifts the *semantic*
    // accents (curiosity/magnificence/wonder = NDE/loot/keyframe content) + nebula.
    let (_creation, curiosity, magnificence, wonder): (u32, u32, u32, u32) = match sky.spectral {
        SpectralTemp::Frost => (0x64B5F6FF, 0x90CAF9FF, 0xBB86FCFF, 0x80DEEAFF), // cool
        SpectralTemp::Gold => (0xF0A840FF, 0x64B5F6FF, 0xD040FFFF, 0x00D4B8FF), // neutral (base)
        SpectralTemp::Iron => (0xFFAB40FF, 0xF0A840FF, 0xFF6E40FF, 0x64FFAAFF), // warm
        SpectralTemp::Ember => (0xFF6E40FF, 0xFFAB40FF, 0xFF5252FF, 0xFFD740FF), // hot
        SpectralTemp::Void => (0x7C4DFFFF, 0x536DFEFF, 0xEA80FCFF, 0x448AFFFF), // deep
    };
    // AccentCreation intentionally NOT set (v10 chrome lock above) — profile wins.
    s.set(TokenId::AccentCuriosity, curiosity, Layer::Celestial);
    s.set(TokenId::AccentMagnificence, magnificence, Layer::Celestial);
    s.set(TokenId::AccentWonder, wonder, Layer::Celestial);

    // Cloud cover → BgNebula darkness (more clouds = darker/more opaque)
    // Format: 0xRRGGBBAA. Warm-dark nebula (v3 ink/panel family, R>G>B); darken
    // with cloud cover. Cool-blue baseline retired 2026-06-02 (whole app warm dark).
    let darken = (sky.cloud_cover as u32) * 2; // 0..16 subtracted from RGB channels
    let r = 0x12u32.saturating_sub(darken);
    let g = 0x0Eu32.saturating_sub(darken);
    let b = 0x0Au32.saturating_sub(darken);
    s.set(TokenId::BgNebula, (r << 24) | (g << 16) | (b << 8) | 0xFF, Layer::Celestial);

    // Moon phase → CelestialPhase token (0=new moon dark, 127=full bright)
    let phase_val = (sky.moon_phase * 255.0) as u32;
    s.set(TokenId::CelestialPhase, phase_val, Layer::Celestial);

    // Cloud + daytime → CelestialDarkness (higher = darker sky)
    let darkness = if sky.is_daytime { 0 } else { 128u32 + sky.cloud_cover as u32 * 16 };
    s.set(TokenId::CelestialDarkness, darkness.min(255), Layer::Celestial);

    // Planetary conjunction detection — 0 = none, 1-255 = strength
    let conjunction_val = detect_conjunction(sky.moon_phase);
    s.set(TokenId::CelestialConjunction, conjunction_val, Layer::Celestial);

    s
}

/// Detect planetary conjunction from moon phase (used as time proxy).
/// Simulates two planets at different orbital periods; returns 0 when far apart,
/// 1-255 as they approach alignment. Based on moon_phase as a normalized time value
/// (0.0=start, 1.0=end of one cycle), used as lore flavor only, not deterministic state.
fn detect_conjunction(moon_phase: f32) -> u32 {
    // Planet 1: fast orbit (5 year cycle)
    let p1_angle = (moon_phase * 5.0 * 360.0) % 360.0;
    // Planet 2: slow orbit (17 year cycle)
    let p2_angle = (moon_phase * 17.0 * 360.0) % 360.0;

    // Angular separation (shortest arc on the circle)
    let mut diff = (p1_angle - p2_angle).abs();
    if diff > 180.0 {
        diff = 360.0 - diff;
    }

    // Conjunction window: within 20 degrees (narrow event, lore-flavor)
    // Maps 0-20 deg to 255-0 intensity; >20 deg = 0
    if diff <= 20.0 {
        (255.0 * (1.0 - diff / 20.0)) as u32
    } else {
        0
    }
}

/// Vibe modulation values derived from sky state.
/// These feed into CanvasUniforms alongside audio-derived vibes.
#[derive(Clone, Debug)]
pub struct SkyVibes {
    /// Wind-driven micro-tremor (0.0 = calm, 1.0 = gale).
    pub shake: f32,
    /// Moon-phase glow intensity multiplier.
    pub glow_mult: f32,
    /// Star visibility threshold (0.0 = all visible, 1.0 = none).
    pub star_occlusion: f32,
}

/// Compute vibe modulation from sky state.
///
/// - **shake**: 0-2 m/s = 0 shake, 6+ = 0.3 max (subtle, not nauseating)
/// - **glow_mult**: Full moon (0.5) = 1.2x glow, new moon (0.0/1.0) = 0.7x
/// - **star_occlusion**: 0 oktas = 0, 8 oktas = 0.9
pub fn sky_vibes(sky: &SkyState) -> SkyVibes {
    SkyVibes {
        // Wind: 0-2 m/s = 0 shake, 6+ = 0.3 max (subtle, not nauseating)
        shake: ((sky.wind_speed - 2.0).max(0.0) / 12.0).min(0.3),
        // Full moon (0.5) = 1.2x glow, new moon (0.0/1.0) = 0.7x
        // sin(phase * PI) peaks at 0.5
        glow_mult: 0.7 + (sky.moon_phase * core::f32::consts::PI).sin() * 0.5,
        // Cloud cover: 0 oktas = 0 occlusion, 8 = 0.9
        star_occlusion: sky.cloud_cover as f32 / 9.0,
    }
}

// -- TritCell5D Lane Ripple (B11/B12 — lane-unity stroke) ─────────────────────
// The five lanes of TritCell5D (balanced-ternary dimensions) are one substrate
// wearing five faces per B11. This ripple implements the Ripple Engine's cascade
// PATTERN (circular chain of regen/decay) adapted to 5D balanced ternary: each
// lane influences its circular-neighbor, with bounded attenuation. Deterministic,
// no heap alloc, no unsafe. The ripple is not a game-system (no zone health state);
// it is a pure function on the coordinate lattice itself.
//
// PATTERN citation: E:\quarantine\md-dedup-2026-07-14\E\airgap\repos\13moons\legacy
// \pixel-pastures\docs\superpowers\specs\2026-03-21-phase3-adventure-ripple-engine
// -design.md, Section 2 (Ripple Engine circular cascade). The TS implementation is
// not ported; only the circular-chain-of-influence PATTERN is adapted here.

use forge_core_v3::atom::TritCell5D;

/// Ripple a TritCell5D coordinate across its five lanes.
/// Each lane circularly influences the next: lane\[i\] contributes to lane\[(i+1)%5\].
/// Influence is scaled by 1/3 (ternary attenuation), rounded toward zero, to model
/// decay; the effect propagates but diminishes, matching Ripple Engine's cascade
/// decay rule. Input and output are both valid TritCell5D interior (0..242), not
/// sentinels; no validation needed (caller's responsibility).
pub fn ripple_5d(cell: TritCell5D) -> TritCell5D {
    let trits = match cell.trits() {
        Some(t) => t,
        None => return cell, // Sentinel: no ripple
    };

    let mut rippled = trits;
    // Circular propagation: each lane influences the next with attenuation.
    // Scale by 1/3 to model ternary regen limit (bounded cascade).
    for i in 0..5 {
        let next = (i + 1) % 5;
        // Add 1/3 of current lane to next lane (integer division, rounds toward zero).
        rippled[next] = (rippled[next] as i16 + trits[i] as i16 / 3) as i8;
    }

    // Clamp each trit back to [-1, 0, +1] (balanced ternary).
    for i in 0..5 {
        rippled[i] = rippled[i].clamp(-1, 1);
    }

    TritCell5D::from_trits(rippled)
}

// -- Sheet builders (NAMED DEBT — donor's were BUILD-TIME GENERATED) ───────────
// In the v2 donor, `celestial_prairie_base()`, every `profile_*()`, and
// `sheet_by_name()` were AOT-generated by a build.rs from themes/*.sheet.vixi
// via the forge-vix-syntax DSL compiler (a v2 build-dependency). That whole
// DSL/build-script pipeline is NOT ported here — it's a separate, larger
// ARCH000-gated dependency decision (same shape as the fontdue L19 nod this
// session, but for a build-time parser crate), out of scope for CANVAS RASTER
// schema porting. This pass keeps tokens.rs's generation-INDEPENDENT core
// (TokenId, TokenSheet, Layer, cascade/resolve) — real, tested, usable today.
// A caller wanting a themed sheet must build a TokenSheet by hand (TokenSheet::new()
// + set()) until this debt is paid.

#[cfg(test)]
mod tests {
    use super::*;

    /// The colour sentinel a missing colour falls back to. Must NEVER appear in a
    /// dimension slot — that is the type-pun this split kills.
    const MAGENTA_UNRESOLVED: u32 = 0xFF00_FFFF;

    #[test]
    fn is_dimension_classifies_the_two_kinds() {
        // Dimensions: Space* / Duration* / Chrome*.
        for d in [
            TokenId::SpaceXs,
            TokenId::SpaceXl,
            TokenId::DurationFast,
            TokenId::ChromeCurvature,
            TokenId::ChromeThickness,
            TokenId::ChromeElevation,
        ] {
            assert!(d.is_dimension(), "{d:?} is a dimension");
        }
        // Colours: backgrounds / text / accents / nav.
        for c in [
            TokenId::BgVoid,
            TokenId::TextPrimary,
            TokenId::AccentCreation,
            TokenId::WindowBarActive,
            TokenId::Gold,
            TokenId::Border,
        ] {
            assert!(!c.is_dimension(), "{c:?} is a colour");
        }
    }

    #[test]
    fn dim_getter_never_returns_a_colour_sentinel() {
        // THE DELIVERABLE: an UNSET dimension slot falls back to a typed layout
        // unit (0), NOT magenta. This is the exact 5-day-glitch class: a missing
        // ChromeThickness must read as 0px, never 0xFF00FFFF cast to a MilliUnit.
        let empty = TokenSheet::new();
        assert_eq!(
            empty.get_dim_or(TokenId::ChromeThickness, 0),
            0,
            "missing dimension → 0, never a colour sentinel",
        );
        assert_ne!(
            empty.get_dim_or(TokenId::ChromeThickness, 0),
            MAGENTA_UNRESOLVED,
            "a dimension can NEVER fall back to MAGENTA",
        );
        // A safe non-zero fallback (1px MilliUnit) also passes straight through.
        assert_eq!(empty.get_dim_or(TokenId::ChromeThickness, 10_000), 10_000);
    }

    #[test]
    fn colour_getter_screams_magenta_on_missing() {
        // The inverse: an unset COLOUR falls back to the visible sentinel so a
        // missing token is caught in dev, not silently transparent.
        let empty = TokenSheet::new();
        assert_eq!(
            empty.get_color_or(TokenId::WindowBarActive, MAGENTA_UNRESOLVED),
            MAGENTA_UNRESOLVED,
            "missing colour → magenta (dev-visible)",
        );
    }

    #[test]
    fn set_dimension_reads_back_through_typed_getter() {
        let mut s = TokenSheet::new();
        s.set(TokenId::ChromeThickness, 14_000, Layer::Base); // mu(14)
        assert_eq!(
            s.get_dim_or(TokenId::ChromeThickness, 0),
            14_000,
            "set value wins over fallback"
        );
    }

    #[test]
    #[should_panic(expected = "use get_dim_or")]
    fn colour_getter_rejects_a_dimension_token() {
        // RED guard: reading a dimension through the colour getter trips in debug —
        // the type-pun boundary is enforced at the API, not just documented.
        let s = TokenSheet::new();
        let _ = s.get_color_or(TokenId::ChromeThickness, MAGENTA_UNRESOLVED);
    }

    #[test]
    #[should_panic(expected = "use get_color_or")]
    fn dim_getter_rejects_a_colour_token() {
        let s = TokenSheet::new();
        let _ = s.get_dim_or(TokenId::WindowBarActive, 0);
    }

    // L07 test — bijection: encode/decode determinism.
    #[test]
    fn tokenid_from_name_roundtrip_determinism() {
        // Every variant that has a from_name mapping must roundtrip cleanly.
        // Encode: TokenId variant -> name string
        // Decode: name string -> TokenId variant
        // Property: f(f_inv(x)) == x for all named TokenIds.
        let test_cases: [(&str, TokenId); 12] = [
            ("bg_void", TokenId::BgVoid),
            ("text_primary", TokenId::TextPrimary),
            ("accent_creation", TokenId::AccentCreation),
            ("danger", TokenId::Danger),
            ("border", TokenId::Border),
            ("band_low", TokenId::BandLow),
            ("space_xs", TokenId::SpaceXs),
            ("duration_fast", TokenId::DurationFast),
            ("chrome_curvature", TokenId::ChromeCurvature),
            ("tab_bg_active", TokenId::TabBgActive),
            ("status_bg", TokenId::StatusBg),
            ("canvas_rail_bg", TokenId::CanvasRailBg),
        ];
        for (name, expected_id) in test_cases {
            // Forward: name -> TokenId
            let decoded = TokenId::from_name(name);
            assert_eq!(
                decoded, Some(expected_id),
                "from_name({:?}) should decode to {:?}",
                name, expected_id
            );
        }
    }

    // L18 test — sabotage: flip the invariant check to confirm it catches errors.
    #[test]
    fn layer_cascade_invariant_honored() {
        // INVARIANT: Higher layer (numerically larger) always wins.
        // Sabotage target: change `>= ` to `>` in TokenSheet::set logic.
        // When sabotaged: this test should fail because equal layers stop winning.
        let mut s = TokenSheet::new();
        // Set to Base layer
        s.set(TokenId::AccentCreation, 0xAABBCCFF, Layer::Base);
        // Set same layer again — Base >= Base, so should win
        s.set(TokenId::AccentCreation, 0x11223344, Layer::Base);
        assert_eq!(
            s.get(TokenId::AccentCreation),
            0x11223344,
            "equal layer (Base == Base) must overwrite"
        );

        // Now test higher layer wins
        s.set(TokenId::AccentCreation, 0xAABBCCFF, Layer::Base);
        s.set(TokenId::AccentCreation, 0x11223344, Layer::Override);
        assert_eq!(
            s.get(TokenId::AccentCreation),
            0x11223344,
            "Override must override Base"
        );
        // Reverse: lower layer must NOT win
        s.set(TokenId::AccentCreation, 0x11223344, Layer::Override);
        s.set(TokenId::AccentCreation, 0xAABBCCFF, Layer::Base);
        assert_eq!(
            s.get(TokenId::AccentCreation),
            0x11223344,
            "Base must not override Override"
        );
    }

    #[test]
    fn higher_layer_wins() {
        let mut s = TokenSheet::new();
        s.set(TokenId::AccentCreation, 0xAABBCCFF, Layer::Base);
        s.set(TokenId::AccentCreation, 0x11223344, Layer::Override);
        assert_eq!(s.get(TokenId::AccentCreation), 0x11223344);
    }

    #[test]
    fn lower_layer_does_not_override() {
        let mut s = TokenSheet::new();
        s.set(TokenId::AccentCreation, 0xAABBCCFF, Layer::Override);
        s.set(TokenId::AccentCreation, 0x11223344, Layer::Base);
        assert_eq!(s.get(TokenId::AccentCreation), 0xAABBCCFF);
    }

    #[test]
    fn resolve_cascade() {
        // NOTE: celestial_prairie_base() is generated; this test is BLOCKED without build.rs.
        // Once generated_sheets.rs lands, uncomment:
        // let base = celestial_prairie_base();
        // let mut user = TokenSheet::new();
        // user.set(TokenId::AccentCreation, 0xFF6600FF, Layer::Override);
        // let resolved = TokenSheet::resolve(&base, &[&user]);
        // assert_eq!(resolved.get(TokenId::AccentCreation), 0xFF6600FF);
        // assert_eq!(resolved.get(TokenId::BgVoid), 0xEDE3CEFF);

        // Self-contained fallback: test resolve() with manually constructed sheets.
        let mut base = TokenSheet::new();
        base.set(TokenId::BgVoid, 0xAABBCCFF, Layer::Base);
        base.set(TokenId::TextPrimary, 0x11223344, Layer::Base);

        let mut overlay = TokenSheet::new();
        overlay.set(TokenId::BgVoid, 0xFF0000FF, Layer::Profile);

        let resolved = TokenSheet::resolve(&base, &[&overlay]);
        assert_eq!(
            resolved.get(TokenId::BgVoid),
            0xFF0000FF,
            "overlay must override base"
        );
        assert_eq!(
            resolved.get(TokenId::TextPrimary),
            0x11223344,
            "unoverridden tokens keep base value"
        );
    }

    #[test]
    fn from_name_lookup() {
        assert_eq!(TokenId::from_name("accent_creation"), Some(TokenId::AccentCreation));
        assert_eq!(TokenId::from_name("bg_void"), Some(TokenId::BgVoid));
        assert_eq!(TokenId::from_name("nonexistent"), None);
    }

    #[test]
    fn smithy_token_ids_fit_in_sheet_capacity() {
        // All new variants must address a slot inside the 256-wide sheet.
        let ids: [TokenId; 15] = [
            TokenId::ChromeCurvature,
            TokenId::ChromeThickness,
            TokenId::ChromeElevation,
            TokenId::TabBgActive,
            TokenId::TabBgInactive,
            TokenId::TabBgHover,
            TokenId::TabTextActive,
            TokenId::TabTextInactive,
            TokenId::TabBaselineAccent,
            TokenId::WindowBarBg,
            TokenId::WindowBarActive,
            TokenId::WindowBarPulse,
            TokenId::StatusBg,
            TokenId::StatusText,
            TokenId::StatusHeartbeat,
        ];
        for id in ids {
            assert!(
                (id as usize) < TOKEN_CAPACITY,
                "TokenId variant {:?} (discriminant {}) outside TOKEN_CAPACITY ({})",
                id,
                id as u16,
                TOKEN_CAPACITY
            );
        }
    }

    #[test]
    fn smithy_token_ids_have_unique_discriminants() {
        // Defense against accidental renumbering colliding with existing variants.
        // Existing occupied: 0-4, 10-12, 20-23, 30-33, 40-42, 50-52, 60-64, 70-72, 80-82.
        let new_ids: [u16; 15] = [83, 84, 85, 90, 91, 92, 93, 94, 95, 100, 101, 102, 110, 111, 112];
        let existing: [u16; 30] = [
            0, 1, 2, 3, 4, 10, 11, 12, 20, 21, 22, 23, 30, 31, 32, 33, 40, 41, 42, 50, 51, 52, 60,
            61, 62, 63, 64, 70, 71, 72,
        ];
        // Also include 80, 81, 82 (celestial) by checking outside the existing 30-array bounds.
        let celestial: [u16; 3] = [80, 81, 82];
        for &n in &new_ids {
            assert!(
                !existing.contains(&n) && !celestial.contains(&n),
                "Smithy variant discriminant {} collides with existing TokenId",
                n
            );
        }
        // Check the new variants themselves don't collide.
        for (i, &a) in new_ids.iter().enumerate() {
            for &b in &new_ids[i + 1..] {
                assert_ne!(a, b, "duplicate discriminant {} in Smithy variants", a);
            }
        }
    }

    #[test]
    fn smithy_from_name_roundtrip() {
        // Every new variant must resolve via from_name with its snake_case name.
        let pairs: [(&str, TokenId); 15] = [
            ("chrome_curvature", TokenId::ChromeCurvature),
            ("chrome_thickness", TokenId::ChromeThickness),
            ("chrome_elevation", TokenId::ChromeElevation),
            ("tab_bg_active", TokenId::TabBgActive),
            ("tab_bg_inactive", TokenId::TabBgInactive),
            ("tab_bg_hover", TokenId::TabBgHover),
            ("tab_text_active", TokenId::TabTextActive),
            ("tab_text_inactive", TokenId::TabTextInactive),
            ("tab_baseline_accent", TokenId::TabBaselineAccent),
            ("window_bar_bg", TokenId::WindowBarBg),
            ("window_bar_active", TokenId::WindowBarActive),
            ("window_bar_pulse", TokenId::WindowBarPulse),
            ("status_bg", TokenId::StatusBg),
            ("status_text", TokenId::StatusText),
            ("status_heartbeat", TokenId::StatusHeartbeat),
        ];
        for (name, expected) in pairs {
            assert_eq!(
                TokenId::from_name(name),
                Some(expected),
                "from_name({:?}) failed to resolve",
                name
            );
        }
    }

    #[test]
    fn celestial_overlay_frost_shifts_cool() {
        let sky = SkyState {
            spectral: SpectralTemp::Frost,
            ..Default::default()
        };
        let overlay = celestial_overlay(&sky);
        // v10 chrome lock: celestial must NOT re-tint the chrome accent (AccentCreation)
        // — it stays on the per-window profile so chrome never drifts cool under the sky.
        assert_eq!(
            overlay.get(TokenId::AccentCreation),
            0,
            "celestial must not set the chrome accent (v10 lock — profile owns it)"
        );
        // The *semantic* accents still shift cool under a Frost sky.
        assert_eq!(overlay.get(TokenId::AccentCuriosity), 0x90CAF9FF);
        assert_eq!(overlay.layers[TokenId::AccentCuriosity as usize], Layer::Celestial);
    }

    #[test]
    fn celestial_overlay_integrates_deterministically() {
        // L07 test: celestial_overlay is deterministic.
        // Same SkyState -> same overlay TokenSheet.
        let sky1 = SkyState {
            spectral: SpectralTemp::Ember,
            moon_phase: 0.75,
            cloud_cover: 4,
            wind_speed: 3.5,
            is_daytime: true,
            stability: 2,
        };
        let overlay1 = celestial_overlay(&sky1);

        let sky2 = SkyState {
            spectral: SpectralTemp::Ember,
            moon_phase: 0.75,
            cloud_cover: 4,
            wind_speed: 3.5,
            is_daytime: true,
            stability: 2,
        };
        let overlay2 = celestial_overlay(&sky2);

        // Bit-exact match on all values and layers
        for i in 0..TOKEN_CAPACITY {
            assert_eq!(overlay1.values[i], overlay2.values[i], "value[{i}] mismatch");
            assert_eq!(overlay1.layers[i], overlay2.layers[i], "layer[{i}] mismatch");
        }
    }

    #[test]
    fn sky_vibes_calm_night() {
        let sky = SkyState {
            wind_speed: 1.0,
            moon_phase: 0.5,
            cloud_cover: 0,
            ..Default::default()
        };
        let v = sky_vibes(&sky);
        assert_eq!(v.shake, 0.0, "below 2 m/s threshold");
        assert_eq!(v.star_occlusion, 0.0, "clear sky");
        assert!(v.glow_mult > 1.0, "full moon boost");
    }

    #[test]
    fn promoted_layer_stamps_set_tokens() {
        // TokenSheet::promoted() must promote only non-zero slots.
        let mut s = TokenSheet::new();
        s.set(TokenId::BgVoid, 0xAABBCCFF, Layer::Base);
        s.set(TokenId::TextPrimary, 0x11223344, Layer::Profile);
        // BgDust stays unset (zero)

        let promoted = s.promoted(Layer::Override);

        // Set tokens promoted to Override
        assert_eq!(promoted.layers[TokenId::BgVoid as usize], Layer::Override);
        assert_eq!(promoted.layers[TokenId::TextPrimary as usize], Layer::Override);
        // Unset tokens stay Base
        assert_eq!(promoted.layers[TokenId::BgDust as usize], Layer::Base);
        // Values unchanged
        assert_eq!(promoted.values[TokenId::BgVoid as usize], 0xAABBCCFF);
        assert_eq!(promoted.values[TokenId::TextPrimary as usize], 0x11223344);
    }

    #[test]
    fn conjunction_detection_aligns_at_zero_mod_gcd() {
        // Two planets at periods 5 and 17 years should align when moon_phase makes both angles equal.
        // LCM(5, 17) = 85, so they align every 85 cycles. At moon_phase=0, both are at 0°.
        let conj_at_zero = detect_conjunction(0.0);
        assert!(conj_at_zero > 200, "at alignment (0.0), conjunction strength should be high: {}", conj_at_zero);

        // At moon_phase=0.5, planets are at 5*0.5*360 = 900 (180 mod 360) and 17*0.5*360 = 3060 (180 mod 360).
        // They should still align (both at 180°).
        let conj_at_half = detect_conjunction(0.5);
        assert!(conj_at_half > 200, "at opposition (0.5), conjunction strength should be high: {}", conj_at_half);

        // At moon_phase=0.25, planets are at 5*0.25*360 = 450 (90 mod 360) and 17*0.25*360 = 1530 (90 mod 360).
        // Again aligned.
        let conj_at_quarter = detect_conjunction(0.25);
        assert!(conj_at_quarter > 200, "at quarter (0.25), conjunction strength should be high: {}", conj_at_quarter);

        // At moon_phase=0.1, planets diverge: 5*0.1*360 = 180° vs 17*0.1*360 = 612° (252 mod 360) = 252°.
        // Separation is 72°, well outside 20° window.
        let conj_at_tenth = detect_conjunction(0.1);
        assert_eq!(conj_at_tenth, 0, "at moon_phase=0.1, planets are 72° apart, no conjunction: {}", conj_at_tenth);
    }
}
