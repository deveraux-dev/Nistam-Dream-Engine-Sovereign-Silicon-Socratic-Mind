//! Variable-size brush tip/pressure/jitter engine — the stroke-to-pixel path.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-gui\src\brush_engine.rs`.
//! Zero-alloc in the hot path: the tip mask is pre-computed on settings
//! change, not per-stamp. `f32` is admitted throughout — this is canvas
//! rasterization (tip falloff curves, stroke-spacing interpolation, alpha
//! blending), not replay-critical sim state, the same category `forge-vix-v3`
//! already draws its own float boundary at.

/// Maximum brush diameter in pixels.
pub const MAX_BRUSH_DIAMETER: usize = 128;
/// Pre-allocated tip buffer size (alpha mask).
pub const TIP_BUFFER_SIZE: usize = MAX_BRUSH_DIAMETER * MAX_BRUSH_DIAMETER;

/// 8x8 Bayer ordered dithering threshold matrix (values 0..63).
pub const BAYER8: [[u8; 8]; 8] = [
    [ 0, 32,  8, 40,  2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44,  4, 36, 14, 46,  6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [ 3, 35, 11, 43,  1, 33,  9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47,  7, 39, 13, 45,  5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// 8x8 Lookup table representing spatial glaze density variance.
/// Values are in permyriad (0..=10_000).
pub const GLAZE_OPACITY_LUT: [[u16; 8]; 8] = [
    [10000,  8500,  9200,  7800,  9800,  8300,  9000,  7600],
    [ 8000,  9500,  7500,  9000,  7900,  9300,  7400,  8800],
    [ 9100,  7700,  9900,  8400,  8900,  7500,  9700,  8200],
    [ 7600,  8900,  8100,  9600,  7300,  8700,  8000,  9400],
    [ 9700,  8200,  9000,  7500, 10000,  8400,  9100,  7700],
    [ 7800,  9400,  7400,  8800,  7900,  9600,  7500,  8900],
    [ 8900,  7600,  9800,  8300,  9000,  7800,  9900,  8500],
    [ 7400,  8800,  7900,  9300,  7600,  9100,  8100,  9700],
];

/// Threshold value for 8x8 Bayer dithering in permyriad (0..=10_000).
#[inline]
pub fn threshold8_pmy(x: u32, y: u32) -> u32 {
    let val = BAYER8[(y % 8) as usize][(x % 8) as usize] as u32;
    (val * 10_000) / 64
}

/// Should the pixel at `(x, y)` receive glaze coverage given baseline `glaze_intensity_pmy`?
#[inline]
pub fn on_glaze(glaze_intensity_pmy: u32, x: u32, y: u32) -> bool {
    let opacity_mod = GLAZE_OPACITY_LUT[(y % 8) as usize][(x % 8) as usize] as u32;
    let adjusted_intensity = (glaze_intensity_pmy * opacity_mod) / 10_000;
    adjusted_intensity > threshold8_pmy(x, y)
}

/// Pressure application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureMode {
    /// Pressure scales brush size.
    Size,
    /// Pressure scales brush opacity.
    Opacity,
    /// Pressure scales both size and opacity.
    Both,
}

/// Pressure settings for pen tablet input.
#[derive(Debug, Clone, Copy)]
pub struct PressureSettings {
    /// Whether pressure sensitivity is active.
    pub enabled: bool,
    /// Which brush property pressure drives.
    pub mode: PressureMode,
    /// Minimum brush size at zero pressure (`1..=128`).
    pub min_size: u8,
    /// Maximum brush size at full pressure (`1..=128`).
    pub max_size: u8,
    /// Minimum opacity at zero pressure (`0..=255`).
    pub min_opacity: u8,
}

impl Default for PressureSettings {
    fn default() -> Self {
        Self { enabled: false, mode: PressureMode::Size, min_size: 1, max_size: 32, min_opacity: 30 }
    }
}

/// Jitter settings for texture brush variation.
#[derive(Debug, Clone, Copy, Default)]
pub struct JitterSettings {
    /// Size variation (`0..=100`, percentage of base size).
    pub size_jitter: u8,
    /// Rotation variation (`0..=180` degrees).
    pub rotation_jitter: u8,
    /// Scatter distance (`0..=100`, percentage of brush size).
    pub scatter: u8,
}

/// A brush tip: grayscale alpha mask defining the brush shape.
/// Values `0..=255` where `255` = full paint, `0` = no paint.
#[derive(Clone)]
pub struct BrushTip {
    /// Alpha mask data, row-major. Only `[0..size*size]` is valid.
    pub mask: [u8; TIP_BUFFER_SIZE],
    /// Actual tip diameter (`1..=128`).
    pub size: u8,
}

impl BrushTip {
    /// Generate a round brush tip with given hardness.
    /// `hardness`: `0` = fully soft (Gaussian-like falloff), `100` = fully hard (binary circle).
    pub fn round(diameter: u8, hardness: u8) -> Self {
        let mut tip = Self { mask: [0u8; TIP_BUFFER_SIZE], size: diameter };
        let d = diameter as f32;
        let radius = d / 2.0;
        let center = radius - 0.5;
        let hard = hardness as f32 / 100.0;

        for y in 0..diameter as usize {
            for x in 0..diameter as usize {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();
                let normalized = dist / radius;

                let alpha = if normalized >= 1.0 {
                    0.0
                } else if hard >= 0.99 {
                    255.0
                } else {
                    let inner_radius = hard;
                    if normalized <= inner_radius {
                        255.0
                    } else {
                        let t = (normalized - inner_radius) / (1.0 - inner_radius);
                        ((1.0 - t) * core::f32::consts::FRAC_PI_2).cos() * 255.0
                    }
                };

                tip.mask[y * diameter as usize + x] = alpha.clamp(0.0, 255.0) as u8;
            }
        }
        tip
    }

    /// Generate a square brush tip (fully opaque).
    pub fn square(diameter: u8) -> Self {
        let mut tip = Self { mask: [0u8; TIP_BUFFER_SIZE], size: diameter };
        for i in 0..(diameter as usize * diameter as usize) {
            tip.mask[i] = 255;
        }
        tip
    }

    /// Generate a chalk/noise brush tip using a simple xorshift PRNG.
    pub fn chalk(diameter: u8, density: u8) -> Self {
        let mut tip = Self { mask: [0u8; TIP_BUFFER_SIZE], size: diameter };
        let d = diameter as f32;
        let radius = d / 2.0;
        let center = radius - 0.5;
        let threshold = density as u32;
        let mut seed: u32 = 0xDEAD_BEEF;

        for y in 0..diameter as usize {
            for x in 0..diameter as usize {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < radius {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    let rand_val = seed % 100;
                    if rand_val < threshold {
                        tip.mask[y * diameter as usize + x] = 200;
                    }
                }
            }
        }
        tip
    }

    /// Load a brush tip from a grayscale image buffer.
    /// Input: RGBA pixels, width, height. Uses the red channel as alpha.
    pub fn from_rgba(rgba: &[u8], width: u32, height: u32) -> Self {
        let size = width.min(height).min(MAX_BRUSH_DIAMETER as u32) as u8;
        let mut tip = Self { mask: [0u8; TIP_BUFFER_SIZE], size };
        for y in 0..size as usize {
            for x in 0..size as usize {
                if y < height as usize && x < width as usize {
                    let src_idx = (y * width as usize + x) * 4;
                    if src_idx < rgba.len() {
                        tip.mask[y * size as usize + x] = rgba[src_idx];
                    }
                }
            }
        }
        tip
    }
}

/// Built-in brush presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushPreset {
    /// Fully opaque round tip.
    HardRound,
    /// Soft-falloff round tip.
    SoftRound,
    /// Fully opaque square tip.
    Square,
    /// Chalk/noise textured tip.
    Chalk,
    /// Sparser chalk/noise textured tip.
    Splatter,
    /// A single hard pixel.
    Pixel,
}

impl BrushPreset {
    /// Human-readable preset label.
    pub fn label(self) -> &'static str {
        match self {
            Self::HardRound => "Hard Round",
            Self::SoftRound => "Soft Round",
            Self::Square => "Square",
            Self::Chalk => "Chalk",
            Self::Splatter => "Splatter",
            Self::Pixel => "Pixel (1px)",
        }
    }

    /// Every preset, in menu order.
    pub const ALL: [BrushPreset; 6] =
        [Self::HardRound, Self::SoftRound, Self::Square, Self::Chalk, Self::Splatter, Self::Pixel];
}

/// The brush engine state. Owns the pre-computed tip and all settings.
pub struct BrushEngine {
    /// Current brush tip (pre-computed alpha mask).
    pub tip: BrushTip,
    /// Base brush diameter (before pressure).
    pub base_size: u8,
    /// Hardness (`0..=100`).
    pub hardness: u8,
    /// Base opacity (`0..=255`).
    pub opacity: u8,
    /// Audio-reactive opacity modulation `0..=255`. `None` = no modulation
    /// (full base). Set per-frame by a host audio adapter; never user-written.
    pub audio_opacity_mod: Option<u8>,
    /// Spacing as percentage of brush size (`10..=200`). `25` = stamp every
    /// 25% of diameter.
    pub spacing: u8,
    /// Current preset.
    pub preset: BrushPreset,
    /// Pressure settings.
    pub pressure: PressureSettings,
    /// Jitter settings.
    pub jitter: JitterSettings,
    last_x: i32,
    last_y: i32,
    distance_accum: f32,
    /// Whether a stroke is in progress.
    pub stroking: bool,
    /// Light-modulated brush brightness. `0.0` = full shadow (stroke dimmed),
    /// `1.0` = full illumination (unmodified). `None` = lighting disabled.
    pub light_intensity: Option<f32>,
    /// Dynamic glaze opacity overlay intensity in permyriad (`0..=10_000`).
    /// `None` = glaze overlay disabled (standard alpha blend).
    /// Driven by `combo_heat -> visual.opacity` or manual glaze toggle.
    pub glaze_intensity_pmy: Option<u32>,
}

impl Default for BrushEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BrushEngine {
    /// Construct a fresh engine. Pixel-art default: a 1px hard pixel, so a
    /// fresh canvas paints exact, contiguous cells rather than a soft blob.
    pub fn new() -> Self {
        let tip = BrushTip::round(1, 100);
        Self {
            tip,
            base_size: 1,
            hardness: 100,
            opacity: 255,
            audio_opacity_mod: None,
            spacing: 25,
            preset: BrushPreset::HardRound,
            pressure: PressureSettings::default(),
            jitter: JitterSettings::default(),
            last_x: 0,
            last_y: 0,
            distance_accum: 0.0,
            stroking: false,
            light_intensity: None,
            glaze_intensity_pmy: None,
        }
    }

    /// Set brush size and regenerate the tip.
    pub fn set_size(&mut self, size: u8) {
        self.base_size = size.clamp(1, MAX_BRUSH_DIAMETER as u8);
        self.regenerate_tip();
    }

    /// Set hardness and regenerate the tip.
    pub fn set_hardness(&mut self, hardness: u8) {
        self.hardness = hardness.min(100);
        self.regenerate_tip();
    }

    /// Set the preset (adjusting hardness/size to its defaults) and regenerate the tip.
    pub fn set_preset(&mut self, preset: BrushPreset) {
        self.preset = preset;
        match preset {
            BrushPreset::Pixel => {
                self.base_size = 1;
                self.hardness = 100;
            }
            BrushPreset::HardRound => self.hardness = 100,
            BrushPreset::SoftRound => self.hardness = 20,
            BrushPreset::Square => self.hardness = 100,
            BrushPreset::Chalk => self.hardness = 60,
            BrushPreset::Splatter => self.hardness = 30,
        }
        self.regenerate_tip();
    }

    fn regenerate_tip(&mut self) {
        self.tip = match self.preset {
            BrushPreset::HardRound | BrushPreset::SoftRound => {
                BrushTip::round(self.base_size, self.hardness)
            }
            BrushPreset::Square => BrushTip::square(self.base_size),
            BrushPreset::Chalk => BrushTip::chalk(self.base_size, 60),
            BrushPreset::Splatter => BrushTip::chalk(self.base_size, 35),
            BrushPreset::Pixel => BrushTip::round(1, 100),
        };
    }

    /// Dim the stroke colour by the frame's light. Shadow scales RGB toward
    /// black, alpha untouched; `None` = identity.
    pub fn lit_color(&self, c: [u8; 4]) -> [u8; 4] {
        match self.light_intensity {
            None => c,
            Some(i) => {
                let i = i.clamp(0.0, 1.0);
                [(c[0] as f32 * i) as u8, (c[1] as f32 * i) as u8, (c[2] as f32 * i) as u8, c[3]]
            }
        }
    }

    /// Compute effective size given pen pressure (Permyriad: `0..=10000`).
    pub fn effective_size(&self, pressure_permyriad: u16) -> u8 {
        if !self.pressure.enabled {
            return self.base_size;
        }
        match self.pressure.mode {
            PressureMode::Size | PressureMode::Both => {
                let p = pressure_permyriad as u32;
                let min = self.pressure.min_size as u32;
                let max = self.pressure.max_size as u32;
                let size = min + (p * (max - min)) / 10000;
                size.clamp(1, MAX_BRUSH_DIAMETER as u32) as u8
            }
            PressureMode::Opacity => self.base_size,
        }
    }

    /// Compute effective opacity given pen pressure (Permyriad: `0..=10000`).
    pub fn effective_opacity(&self, pressure_permyriad: u16) -> u8 {
        if !self.pressure.enabled {
            return self.opacity;
        }
        match self.pressure.mode {
            PressureMode::Opacity | PressureMode::Both => {
                let p = pressure_permyriad as u32;
                let min = self.pressure.min_opacity as u32;
                let max = self.opacity as u32;
                let opacity = min + (p * (max - min)) / 10000;
                opacity.clamp(0, 255) as u8
            }
            PressureMode::Size => self.opacity,
        }
    }

    /// Effective opacity blending base with audio modulation. When
    /// `audio_opacity_mod` is `Some(m)`, scales the pressure-adjusted base by
    /// `m/255`. Silence (`m=0`) -> `0`; full signal (`m=255`) -> unchanged base.
    pub fn effective_opacity_blended(&self, pressure_permyriad: u16) -> u8 {
        let base = self.effective_opacity(pressure_permyriad) as u32;
        match self.audio_opacity_mod {
            None => base as u8,
            Some(m) => ((base * m as u32 + 127) / 255).min(255) as u8,
        }
    }

    /// Set glaze overlay intensity in permyriad (`0..=10_000`), or `None` to disable.
    pub fn set_glaze_intensity(&mut self, intensity_pmy: Option<u32>) {
        self.glaze_intensity_pmy = intensity_pmy.map(|i| i.min(10_000));
    }

    /// Set glaze overlay intensity driven by `combo_heat -> visual.opacity` reactive binding.
    /// Clamps `combo_heat_pmy` against `bounded_pmy` ceiling in integer permyriad space.
    pub fn set_glaze_from_combo_heat(&mut self, combo_heat_pmy: u32, bounded_pmy: u32) {
        let intensity = combo_heat_pmy.min(bounded_pmy).min(10_000);
        self.glaze_intensity_pmy = Some(intensity);
    }

    /// Begin a new stroke at canvas position `(px, py)`.
    pub fn begin_stroke(&mut self, px: u32, py: u32) {
        self.last_x = px as i32;
        self.last_y = py as i32;
        self.distance_accum = 0.0;
        self.stroking = true;
    }

    /// Continue a stroke to `(px, py)`, writing spacing-interpolated stamp
    /// positions into `stamps_out`. Returns the number of stamps written.
    pub fn stroke_to(&mut self, px: u32, py: u32, stamps_out: &mut [(u32, u32); 512]) -> usize {
        if !self.stroking {
            self.begin_stroke(px, py);
            stamps_out[0] = (px, py);
            return 1;
        }

        let x1 = px as i32;
        let y1 = py as i32;
        let dx = (x1 - self.last_x) as f32;
        let dy = (y1 - self.last_y) as f32;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.5 {
            return 0;
        }

        let spacing_px = (self.base_size as f32 * self.spacing as f32 / 100.0).max(1.0);
        let mut count = 0usize;

        self.distance_accum += dist;

        while self.distance_accum >= spacing_px && count < 512 {
            self.distance_accum -= spacing_px;
            let t = 1.0 - (self.distance_accum / dist).min(1.0);
            let sx = self.last_x as f32 + dx * t;
            let sy = self.last_y as f32 + dy * t;
            let stamp_x = sx.round().max(0.0) as u32;
            let stamp_y = sy.round().max(0.0) as u32;
            stamps_out[count] = (stamp_x, stamp_y);
            count += 1;
        }

        self.last_x = x1;
        self.last_y = y1;
        count
    }

    /// End the current stroke.
    pub fn end_stroke(&mut self) {
        self.stroking = false;
        self.distance_accum = 0.0;
    }

    /// Stamp the brush tip onto a raw RGBA pixel buffer at `(cx, cy)`,
    /// alpha-blending the brush colour through the tip mask (src-over).
    #[allow(clippy::too_many_arguments)]
    pub fn stamp(
        &self,
        canvas: &mut [u8],
        canvas_w: u32,
        canvas_h: u32,
        cx: u32,
        cy: u32,
        color: [u8; 4],
        opacity_override: u8,
        size_override: u8,
    ) {
        let size = size_override.max(1) as i32;
        let half = size / 2;
        let tip_size = self.tip.size as i32;

        for dy in 0..size {
            for dx in 0..size {
                let px = cx as i32 - half + dx;
                let py = cy as i32 - half + dy;

                if px < 0 || py < 0 || px >= canvas_w as i32 || py >= canvas_h as i32 {
                    continue;
                }

                if let Some(glaze_pmy) = self.glaze_intensity_pmy {
                    if !on_glaze(glaze_pmy, px as u32, py as u32) {
                        continue;
                    }
                }

                let tx = if tip_size > 0 { (dx * tip_size / size) as usize } else { 0 };
                let ty = if tip_size > 0 { (dy * tip_size / size) as usize } else { 0 };
                let tip_alpha = if tip_size > 0 {
                    self.tip.mask[ty * tip_size as usize + tx] as u32
                } else {
                    255u32
                };

                if tip_alpha == 0 {
                    continue;
                }

                let light_mod = self.light_intensity.unwrap_or(1.0);
                let final_alpha = ((tip_alpha * opacity_override as u32 * color[3] as u32) as f32
                    * light_mod
                    / (255.0 * 255.0)) as u32;
                if final_alpha == 0 {
                    continue;
                }

                let idx = (py as u32 * canvas_w + px as u32) as usize * 4;
                if idx + 3 >= canvas.len() {
                    continue;
                }

                let src_a = final_alpha;
                let inv_src_a = 255 - src_a;

                canvas[idx] = ((color[0] as u32 * src_a + canvas[idx] as u32 * inv_src_a) / 255) as u8;
                canvas[idx + 1] =
                    ((color[1] as u32 * src_a + canvas[idx + 1] as u32 * inv_src_a) / 255) as u8;
                canvas[idx + 2] =
                    ((color[2] as u32 * src_a + canvas[idx + 2] as u32 * inv_src_a) / 255) as u8;
                canvas[idx + 3] = (src_a + (canvas[idx + 3] as u32 * inv_src_a) / 255).min(255) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_tip_center_is_opaque() {
        let tip = BrushTip::round(16, 100);
        let center = 16 / 2;
        assert!(tip.mask[center * 16 + center] > 200);
    }

    #[test]
    fn lit_color_dims_rgb_keeps_alpha() {
        let mut e = BrushEngine::new();
        let c = [200u8, 100, 50, 255];
        assert_eq!(e.lit_color(c), c, "None (lighting off) is identity");
        e.light_intensity = Some(1.0);
        assert_eq!(e.lit_color(c), c, "full light is identity");
        e.light_intensity = Some(0.5);
        assert_eq!(e.lit_color(c), [100, 50, 25, 255], "half light halves RGB, alpha untouched");
        e.light_intensity = Some(0.0);
        assert_eq!(e.lit_color(c), [0, 0, 0, 255], "full shadow floors RGB");
        e.light_intensity = Some(7.0);
        assert_eq!(e.lit_color(c), c, "over-range clamps to identity");
    }

    #[test]
    fn soft_tip_edge_is_transparent() {
        let tip = BrushTip::round(32, 0);
        assert_eq!(tip.mask[0], 0);
    }

    #[test]
    fn pressure_size_scales() {
        let mut engine = BrushEngine::new();
        engine.pressure.enabled = true;
        engine.pressure.mode = PressureMode::Size;
        engine.pressure.min_size = 4;
        engine.pressure.max_size = 32;
        let size = engine.effective_size(5000);
        assert_eq!(size, 18);
    }

    #[test]
    fn stroke_produces_stamps() {
        let mut engine = BrushEngine::new();
        engine.base_size = 10;
        engine.spacing = 25;
        engine.begin_stroke(0, 0);
        let mut stamps = [(0u32, 0u32); 512];
        let count = engine.stroke_to(20, 0, &mut stamps);
        assert!(count > 0);
    }

    #[test]
    fn stamp_paints_pixels() {
        let mut canvas = vec![0u8; 64 * 64 * 4];
        let engine = BrushEngine::new();
        engine.stamp(&mut canvas, 64, 64, 32, 32, [255, 0, 0, 255], 255, 8);
        let idx = (32 * 64 + 32) * 4;
        assert!(canvas[idx] > 100);
    }

    #[test]
    fn default_engine_is_hard_pixel() {
        let engine = BrushEngine::new();
        assert_eq!(engine.base_size, 1);
        assert_eq!(engine.hardness, 100);
        assert_eq!(engine.tip.size, 1);
        assert_eq!(engine.tip.mask[0], 255);
    }

    #[test]
    fn hard_round_preset_has_no_partial_alpha() {
        let mut engine = BrushEngine::new();
        engine.set_size(9);
        engine.set_preset(BrushPreset::HardRound);
        assert_eq!(engine.hardness, 100);
        let d = engine.tip.size as usize;
        for v in &engine.tip.mask[..d * d] {
            assert!(*v == 0 || *v == 255, "hard round tip has partial alpha: {v}");
        }
        let c = d / 2;
        assert_eq!(engine.tip.mask[c * d + c], 255);
    }

    #[test]
    fn audio_mod_none_is_identity() {
        let engine = BrushEngine::new();
        assert_eq!(engine.audio_opacity_mod, None);
        assert_eq!(engine.effective_opacity_blended(10000), engine.effective_opacity(10000));
    }

    #[test]
    fn audio_mod_zero_silences_brush() {
        let mut engine = BrushEngine::new();
        engine.audio_opacity_mod = Some(0);
        assert_eq!(engine.effective_opacity_blended(10000), 0);
    }

    #[test]
    fn audio_mod_full_preserves_base() {
        let mut engine = BrushEngine::new();
        engine.audio_opacity_mod = Some(255);
        assert_eq!(engine.effective_opacity_blended(10000), engine.effective_opacity(10000));
    }

    #[test]
    fn audio_mod_scales_proportionally() {
        let mut engine = BrushEngine::new();
        engine.opacity = 200;
        engine.audio_opacity_mod = Some(128);
        let blended = engine.effective_opacity_blended(10000);
        assert!((98..=102).contains(&blended), "expected ~100, got {blended}");
    }

    #[test]
    fn glaze_threshold_and_lut_integrity() {
        assert_eq!(BAYER8.len(), 8);
        assert_eq!(BAYER8[0].len(), 8);
        assert_eq!(GLAZE_OPACITY_LUT.len(), 8);
        assert_eq!(GLAZE_OPACITY_LUT[0].len(), 8);
        assert_eq!(threshold8_pmy(0, 0), 0);
        assert_eq!(threshold8_pmy(0, 7), (63 * 10_000) / 64);
    }

    #[test]
    fn on_glaze_evaluates_threshold() {
        // Zero intensity -> never on
        assert!(!on_glaze(0, 0, 0));
        assert!(!on_glaze(0, 4, 4));

        // Full 10,000 intensity -> on wherever LUT > threshold
        for y in 0..8 {
            for x in 0..8 {
                let lut = GLAZE_OPACITY_LUT[y][x] as u32;
                let thresh = threshold8_pmy(x as u32, y as u32);
                if lut > thresh {
                    assert!(on_glaze(10_000, x as u32, y as u32));
                }
            }
        }
    }

    #[test]
    fn glaze_from_combo_heat_clamps_and_sets() {
        let mut engine = BrushEngine::new();
        assert_eq!(engine.glaze_intensity_pmy, None);

        // combo_heat=5000, bounded=9000 -> 5000
        engine.set_glaze_from_combo_heat(5000, 9000);
        assert_eq!(engine.glaze_intensity_pmy, Some(5000));

        // combo_heat=9000, bounded=3000 -> 3000
        engine.set_glaze_from_combo_heat(9000, 3000);
        assert_eq!(engine.glaze_intensity_pmy, Some(3000));

        // over-range clamp
        engine.set_glaze_from_combo_heat(20_000, 50_000);
        assert_eq!(engine.glaze_intensity_pmy, Some(10_000));
    }

    #[test]
    fn stamp_with_glaze_applies_stippled_pattern() {
        let mut canvas_no_glaze = vec![0u8; 16 * 16 * 4];
        let mut canvas_glazed = vec![0u8; 16 * 16 * 4];

        let mut engine = BrushEngine::new();
        engine.stamp(&mut canvas_no_glaze, 16, 16, 8, 8, [255, 255, 255, 255], 255, 8);

        // Enable partial glaze overlay
        engine.set_glaze_intensity(Some(3000));
        engine.stamp(&mut canvas_glazed, 16, 16, 8, 8, [255, 255, 255, 255], 255, 8);

        let painted_no_glaze: usize = canvas_no_glaze.chunks(4).filter(|c| c[3] > 0).count();
        let painted_glazed: usize = canvas_glazed.chunks(4).filter(|c| c[3] > 0).count();

        assert!(painted_glazed < painted_no_glaze, "glaze must stipple/skip some pixels");
        assert!(painted_glazed > 0, "glaze must still paint pixels with low thresholds");
    }
}
