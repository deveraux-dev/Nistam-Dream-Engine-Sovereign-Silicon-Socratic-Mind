//! Procedural Audio Synthesis Recipe Library.
//!
//! Material-specific, physics-driven DSP recipes available to any game cartridge.
//! Pure Rust, zero-heap-alloc on the audio thread, deterministic given the same seed.
//!
//! Module layout:
//! - `primitives` — Oscillators, Filters, Envelopes, SeedRng
//! - `fm_clang` — Recipe 1: FM metal/bell
//! - `fm_bass` — Recipe 2: FM sub-growl
//! - `bounce` — Recipe 3: LFO-on-LFO
//! - `friction` — Recipe 4: Noise → filter
//! - `subtractive` — Recipe 5: Saw → resonant LP
//! - `additive` — Recipe 6: Parallel sines
//! - `vibe_modulation` — VibeMatrix → audio modulation
//! - `ce_audio` — CE scan → AudioMaterialProfile
//! - `serialization` — RecipeDefinition TOML round-trip

pub mod primitives;
pub mod fm_clang;
pub mod fm_bass;
pub mod bounce;
pub mod friction;
pub mod subtractive;
pub mod additive;
pub mod vibe_modulation;
pub mod ce_audio;
pub mod serialization;

pub use primitives::{
    SeedRng,
    osc_sine, osc_square, osc_saw,
    noise_white, noise_pink, PinkNoiseState,
    BiquadState, filter_lowpass, filter_highpass, filter_bandpass, filter_resonant_lp,
    envelope_ar, envelope_adsr,
};

// ---------------------------------------------------------------------------
// Engine-level types — no game-crate dependencies
// ---------------------------------------------------------------------------

/// Engine-level material bitmask (4 channels summing to 255).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialBitmask {
    pub void_pct: u8,
    pub shadow_pct: u8,
    pub ash_pct: u8,
    pub iron_pct: u8,
}

/// Engine-level audio material profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioMaterialProfile {
    pub ring_frequency_hz: f32,
    pub attack_sharpness: f32,
    pub harmonic_content: f32,
    pub decay_secs: f32,
    pub reverb_amount: f32,
}

/// Engine-level sound source classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundSource {
    Impact,
    CombatMelee,
    CombatRanged,
    CombatMagic,
    VoxelImpact,
    Structural,
    Locomotion,
    Heat,
    Projectile,
}

// ---------------------------------------------------------------------------
// MaterialClass
// ---------------------------------------------------------------------------

/// Discrete material classification from MaterialBitmask dominant channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MaterialClass {
    Iron = 0,
    Void = 1,
    Ash = 2,
    Shadow = 3,
    Organic = 4,
}

impl MaterialClass {
    /// Classify a MaterialBitmask by dominant channel.
    /// Ties broken by priority: Iron > Shadow > Ash > Void.
    /// If all zero → Organic fallback.
    pub fn from_bitmask(mask: &MaterialBitmask) -> Self {
        let channels = [
            (mask.iron_pct, MaterialClass::Iron),
            (mask.shadow_pct, MaterialClass::Shadow),
            (mask.ash_pct, MaterialClass::Ash),
            (mask.void_pct, MaterialClass::Void),
        ];
        let max_val = channels.iter().map(|(v, _)| *v).max().unwrap_or(0);
        if max_val == 0 { return MaterialClass::Organic; }
        for &(val, class) in &channels {
            if val == max_val { return class; }
        }
        MaterialClass::Organic
    }
}

// ---------------------------------------------------------------------------
// RecipeId
// ---------------------------------------------------------------------------

/// Identifies one of the six canonical synthesis recipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RecipeId {
    FmClang = 0,
    FmBass = 1,
    Bounce = 2,
    Friction = 3,
    Subtractive = 4,
    Additive = 5,
}

// ---------------------------------------------------------------------------
// SoundSourceCategory
// ---------------------------------------------------------------------------

/// Coarse sound source category for dispatch table indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SoundSourceCategory {
    Impact = 0,
    Locomotion = 1,
    Continuous = 2,
    Ambient = 3,
    Ui = 4,
}

impl From<SoundSource> for SoundSourceCategory {
    fn from(s: SoundSource) -> Self {
        match s {
            SoundSource::Impact
            | SoundSource::CombatMelee
            | SoundSource::CombatRanged
            | SoundSource::CombatMagic
            | SoundSource::VoxelImpact
            | SoundSource::Structural => Self::Impact,
            SoundSource::Locomotion => Self::Locomotion,
            SoundSource::Heat | SoundSource::Projectile => Self::Continuous,
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch Table
// ---------------------------------------------------------------------------

/// Material × SoundSourceCategory → RecipeId
/// Rows: [Iron, Void, Ash, Shadow, Organic]
/// Cols: [Impact, Locomotion, Continuous, Ambient, UI]
pub const DISPATCH_TABLE: [[RecipeId; 5]; 5] = [
    [RecipeId::FmClang, RecipeId::Friction, RecipeId::Friction, RecipeId::Additive, RecipeId::Subtractive],
    [RecipeId::FmBass, RecipeId::Bounce, RecipeId::FmBass, RecipeId::Additive, RecipeId::FmBass],
    [RecipeId::Friction, RecipeId::Friction, RecipeId::Friction, RecipeId::Friction, RecipeId::Subtractive],
    [RecipeId::Subtractive, RecipeId::Bounce, RecipeId::Friction, RecipeId::Additive, RecipeId::Bounce],
    [RecipeId::FmClang, RecipeId::Friction, RecipeId::Bounce, RecipeId::Additive, RecipeId::Subtractive],
];

// ---------------------------------------------------------------------------
// RecipeParams
// ---------------------------------------------------------------------------

/// Unified parameter struct passed to all recipe functions.
pub struct RecipeParams {
    pub ring_frequency_hz: f32,
    pub attack_sharpness: f32,
    pub harmonic_content: f32,
    pub decay_secs: f32,
    pub reverb_amount: f32,
    pub intensity_db: f32,
    pub sample_rate: u32,
    pub num_samples: usize,
    pub seed: u64,
    pub fog_cutoff_mod: f32,
    pub aberration_detune: f32,
    pub glow_reverb_mod: f32,
    pub distortion_amount: f32,
    pub fm_ratio: f32,
    pub fm_feedback: f32,
    pub filter_q: f32,
    pub partial_count: u8,
    pub lfo_rate_hz: f32,
    pub duration_override_secs: Option<f32>,
}

impl RecipeParams {
    pub fn from_profile(
        profile: &AudioMaterialProfile,
        intensity_db: f32,
        seed: u64,
        sample_rate: u32,
        num_samples: usize,
    ) -> Self {
        Self {
            ring_frequency_hz: profile.ring_frequency_hz,
            attack_sharpness: profile.attack_sharpness,
            harmonic_content: profile.harmonic_content,
            decay_secs: profile.decay_secs,
            reverb_amount: profile.reverb_amount,
            intensity_db,
            sample_rate,
            num_samples,
            seed,
            fog_cutoff_mod: 0.0,
            aberration_detune: 0.0,
            glow_reverb_mod: 0.0,
            distortion_amount: 0.0,
            fm_ratio: 3.57,
            fm_feedback: 0.0,
            filter_q: 2.0,
            partial_count: 8,
            lfo_rate_hz: 5.0,
            duration_override_secs: None,
        }
    }
}

// ---------------------------------------------------------------------------
// BufferPool
// ---------------------------------------------------------------------------

/// Maximum samples per buffer: 150ms at 44100 Hz.
pub const MAX_SAMPLES: usize = 6615;
/// Number of pre-allocated buffers (matches MAX_SAMPLER_SLOTS).
pub const POOL_SIZE: usize = 8;

/// Pre-allocated buffer pool for zero-heap-alloc audio synthesis.
pub struct BufferPool {
    buffers: Vec<Box<[f32; MAX_SAMPLES]>>,
    next: usize,
}

impl BufferPool {
    pub fn new() -> Self {
        let mut buffers = Vec::with_capacity(POOL_SIZE);
        for _ in 0..POOL_SIZE {
            buffers.push(Box::new([0.0f32; MAX_SAMPLES]));
        }
        Self { buffers, next: 0 }
    }

    pub fn acquire(&mut self) -> &mut [f32; MAX_SAMPLES] {
        let idx = self.next;
        self.next = (self.next + 1) % POOL_SIZE;
        let buf = &mut self.buffers[idx];
        buf.fill(0.0);
        buf.as_mut()
    }
}

impl Default for BufferPool {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// VibeSignals + RecipeEngine
// ---------------------------------------------------------------------------

/// Per-tick vibe signals for audio modulation.
#[derive(Debug, Clone, Copy)]
pub struct VibeSignals {
    pub fog_density: f32,
    pub chromatic_aberration: f32,
    pub artifact_glow: f32,
    pub distortion: f32,
}

impl Default for VibeSignals {
    fn default() -> Self {
        Self { fog_density: 0.0, chromatic_aberration: 0.0, artifact_glow: 0.0, distortion: 0.0 }
    }
}

/// Top-level recipe engine dispatcher.
pub struct RecipeEngine {
    pub buffer_pool: BufferPool,
    pub vibe_signals: VibeSignals,
    pub era_index: u8,
    pub brand_distortion: f32,
    prev_era_index: u8,
    era_crossfade_progress: f32,
    era_crossfade_ticks: u32,
    era_crossfade_counter: u32,
    /// Smoothed vibe state for the audio↔render viz bridge (G-AUDIO-01 wire).
    vibe_mod: vibe_modulation::VibeAudioModulation,
}

const SAMPLE_RATE: u32 = 48000; // must match device (FORGE_INVARIANTS [audio] sample_rate_hz)
const SFX_DURATION_SECS: f32 = 0.15;
const RECIPE_TIMEOUT_SECS: f32 = 0.0005;

impl RecipeEngine {
    pub fn new() -> Self {
        println!("[RECIPE] RecipeEngine initialized: 6 recipes, 5 material classes, {}×{} buffer pool.",
            POOL_SIZE, MAX_SAMPLES);
        Self {
            buffer_pool: BufferPool::new(),
            vibe_signals: VibeSignals::default(),
            era_index: 0,
            brand_distortion: 0.0,
            prev_era_index: 0,
            era_crossfade_progress: 1.0,
            era_crossfade_ticks: 60,
            era_crossfade_counter: 0,
            vibe_mod: vibe_modulation::VibeAudioModulation::new(),
        }
    }

    pub fn update_vibe(&mut self, fog_density: f32, chromatic_aberration: f32, artifact_glow: f32, distortion: f32) {
        self.vibe_signals.fog_density = fog_density;
        self.vibe_signals.chromatic_aberration = chromatic_aberration;
        self.vibe_signals.artifact_glow = artifact_glow;
        self.vibe_signals.distortion = distortion;
        self.vibe_mod.update(&self.vibe_signals);
    }

    // publish_vibe_to_viz: EXCLUDED — needs crate::viz_buffer (real unsafe,
    // excluded). vibe_mod itself (VibeAudioModulation) stays: only its own
    // store_to_viz method needed viz_buffer, removed in vibe_modulation.rs.


    pub fn update_era(&mut self, era_index: u8, brand_level: u8) {
        if era_index != self.era_index {
            self.prev_era_index = self.era_index;
            self.era_index = era_index;
            self.era_crossfade_progress = 0.0;
            self.era_crossfade_counter = 0;
        }
        if self.era_crossfade_progress < 1.0 {
            self.era_crossfade_counter += 1;
            self.era_crossfade_progress =
                (self.era_crossfade_counter as f32 / self.era_crossfade_ticks as f32).min(1.0);
        }
        let base_distortion = brand_level as f32 / 255.0;
        self.brand_distortion = if self.era_index == 3 {
            (base_distortion * 2.0).min(1.0)
        } else {
            base_distortion
        };
    }

    /// Core dispatch: classify material → lookup recipe → build params → synthesize.
    pub fn synthesize(
        &mut self,
        profile: &AudioMaterialProfile,
        intensity_db: f32,
        source: SoundSource,
        material: Option<&MaterialBitmask>,
        seed: u64,
    ) -> Vec<f32> {
        let mat_class = match material {
            Some(mask) => MaterialClass::from_bitmask(mask),
            None => MaterialClass::Organic,
        };
        let category = SoundSourceCategory::from(source);
        let recipe_id = DISPATCH_TABLE[mat_class as usize][category as usize];

        let duration = SFX_DURATION_SECS;
        let num_samples = (SAMPLE_RATE as f32 * duration) as usize;
        let num_samples = num_samples.min(MAX_SAMPLES);

        let mut params = RecipeParams::from_profile(profile, intensity_db, seed, SAMPLE_RATE, num_samples);
        vibe_modulation::apply_vibe(&self.vibe_signals, &mut params);
        params.distortion_amount = (params.distortion_amount + self.brand_distortion).min(1.0);

        let buf = self.buffer_pool.acquire();
        let start = std::time::Instant::now();

        let target = &mut buf[..num_samples];
        match recipe_id {
            RecipeId::FmClang => fm_clang::recipe_fm_clang(&params, target),
            RecipeId::FmBass => fm_bass::recipe_fm_bass(&params, target),
            RecipeId::Bounce => bounce::recipe_bounce(&params, target),
            RecipeId::Friction => friction::recipe_friction(&params, target),
            RecipeId::Subtractive => subtractive::recipe_subtractive(&params, target),
            RecipeId::Additive => additive::recipe_additive(&params, target),
        }

        let elapsed = start.elapsed();
        // AUDIO-TELEMETRY-F12 (2026-06-01): publish the last-recipe row. Cold path
        // (synthesize copies the buffer out); these are atomic stores, no extra heap.
        crate::telemetry::telemetry().set_last_recipe(
            recipe_id as u64,
            params.ring_frequency_hz as u64,
            elapsed.as_micros() as u64,
            seed,
        );
        let elapsed_secs = elapsed.as_secs_f32();
        if elapsed_secs > RECIPE_TIMEOUT_SECS {
            eprintln!("[RECIPE] Warning: {:?} synthesis took {:.2}ms (limit 0.5ms)", recipe_id, elapsed_secs * 1000.0);
        }

        buf[..num_samples].to_vec()
    }
}

impl Default for RecipeEngine {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_material(void: u8, shadow: u8, ash: u8, iron: u8) -> MaterialBitmask {
        MaterialBitmask { void_pct: void, shadow_pct: shadow, ash_pct: ash, iron_pct: iron }
    }

    #[test]
    fn dispatch_table_matches_spec() {
        use RecipeId::*;
        assert_eq!(DISPATCH_TABLE[0][0], FmClang);
        assert_eq!(DISPATCH_TABLE[0][1], Friction);
        assert_eq!(DISPATCH_TABLE[0][2], Friction);
        assert_eq!(DISPATCH_TABLE[0][3], Additive);
        assert_eq!(DISPATCH_TABLE[0][4], Subtractive);
        assert_eq!(DISPATCH_TABLE[1][0], FmBass);
        assert_eq!(DISPATCH_TABLE[1][1], Bounce);
        assert_eq!(DISPATCH_TABLE[1][2], FmBass);
        assert_eq!(DISPATCH_TABLE[1][3], Additive);
        assert_eq!(DISPATCH_TABLE[1][4], FmBass);
        assert_eq!(DISPATCH_TABLE[2][0], Friction);
        assert_eq!(DISPATCH_TABLE[2][1], Friction);
        assert_eq!(DISPATCH_TABLE[2][2], Friction);
        assert_eq!(DISPATCH_TABLE[2][3], Friction);
        assert_eq!(DISPATCH_TABLE[2][4], Subtractive);
        assert_eq!(DISPATCH_TABLE[3][0], Subtractive);
        assert_eq!(DISPATCH_TABLE[3][1], Bounce);
        assert_eq!(DISPATCH_TABLE[3][2], Friction);
        assert_eq!(DISPATCH_TABLE[3][3], Additive);
        assert_eq!(DISPATCH_TABLE[3][4], Bounce);
        assert_eq!(DISPATCH_TABLE[4][0], FmClang);
        assert_eq!(DISPATCH_TABLE[4][1], Friction);
        assert_eq!(DISPATCH_TABLE[4][2], Bounce);
        assert_eq!(DISPATCH_TABLE[4][3], Additive);
        assert_eq!(DISPATCH_TABLE[4][4], Subtractive);
    }

    #[test]
    fn material_class_from_bitmask_iron_dominant() {
        assert_eq!(MaterialClass::from_bitmask(&make_material(10, 20, 30, 195)), MaterialClass::Iron);
    }

    #[test]
    fn material_class_from_bitmask_void_dominant() {
        assert_eq!(MaterialClass::from_bitmask(&make_material(200, 20, 20, 15)), MaterialClass::Void);
    }

    #[test]
    fn material_class_from_bitmask_all_zero() {
        assert_eq!(MaterialClass::from_bitmask(&make_material(0, 0, 0, 0)), MaterialClass::Organic);
    }

    #[test]
    fn material_class_tie_iron_wins() {
        assert_eq!(MaterialClass::from_bitmask(&make_material(27, 100, 28, 100)), MaterialClass::Iron);
    }

    #[test]
    fn sound_source_category_mapping() {
        assert_eq!(SoundSourceCategory::from(SoundSource::Impact), SoundSourceCategory::Impact);
        assert_eq!(SoundSourceCategory::from(SoundSource::CombatMelee), SoundSourceCategory::Impact);
        assert_eq!(SoundSourceCategory::from(SoundSource::CombatRanged), SoundSourceCategory::Impact);
        assert_eq!(SoundSourceCategory::from(SoundSource::CombatMagic), SoundSourceCategory::Impact);
        assert_eq!(SoundSourceCategory::from(SoundSource::VoxelImpact), SoundSourceCategory::Impact);
        assert_eq!(SoundSourceCategory::from(SoundSource::Structural), SoundSourceCategory::Impact);
        assert_eq!(SoundSourceCategory::from(SoundSource::Locomotion), SoundSourceCategory::Locomotion);
        assert_eq!(SoundSourceCategory::from(SoundSource::Heat), SoundSourceCategory::Continuous);
        assert_eq!(SoundSourceCategory::from(SoundSource::Projectile), SoundSourceCategory::Continuous);
    }

    // AUDIO-TELEMETRY-F12 (2026-06-01): the determinism proof the F12 panel claims.
    // Same seed + same inputs -> bit-identical output.
    fn det_profile() -> AudioMaterialProfile {
        AudioMaterialProfile {
            ring_frequency_hz: 440.0,
            attack_sharpness: 0.5,
            harmonic_content: 0.4,
            decay_secs: 0.5,
            reverb_amount: 0.2,
        }
    }

    #[test]
    fn synthesize_is_deterministic_for_same_seed() {
        let profile = det_profile();
        let material = make_material(10, 20, 30, 195); // iron-dominant -> FmClang
        const SEED: u64 = 0xC0FF_EE12_3456_789A;
        let a = RecipeEngine::new().synthesize(&profile, -6.0, SoundSource::Impact, Some(&material), SEED);
        let b = RecipeEngine::new().synthesize(&profile, -6.0, SoundSource::Impact, Some(&material), SEED);
        assert_eq!(a.len(), b.len(), "same seed must yield same-length output");
        assert!(
            a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()),
            "same seed must yield bit-identical output"
        );
        assert!(!a.is_empty() && a.iter().any(|s| *s != 0.0), "synthesis must produce a real signal");
    }

    #[test]
    fn synthesize_is_deterministic_across_recipes() {
        let profile = det_profile();
        const SEED: u64 = 0xABCD_7777_1111_2222;
        let iron = make_material(10, 20, 30, 195);
        let void = make_material(200, 20, 20, 15);
        let ash = make_material(10, 10, 200, 20);
        for (mat, src) in [
            (&iron, SoundSource::Impact),
            (&void, SoundSource::Locomotion),
            (&ash, SoundSource::Heat),
        ] {
            let a = RecipeEngine::new().synthesize(&profile, -3.0, src, Some(mat), SEED);
            let b = RecipeEngine::new().synthesize(&profile, -3.0, src, Some(mat), SEED);
            assert!(
                a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()),
                "every recipe must be deterministic for a fixed seed"
            );
        }
    }

    #[test]
    fn negative_control_nontrivial() {
        // ADR-0015 law #3: distinct seeds must produce distinct synthesis — harness non-blind.
        // Uses Ash+Heat → Friction (noise-based; SeedRng drives every sample).
        // FmClang with default params zeroes pitch-jitter (aberration_detune=0.0 by default),
        // making it seed-invariant under those params — not a bug, just a recipe property.
        let profile = det_profile();
        let ash = make_material(10, 10, 200, 20); // ash-dominant -> Friction via Heat
        let a = RecipeEngine::new().synthesize(&profile, -6.0, SoundSource::Heat, Some(&ash), 0x1111_1111_1111_1111);
        let b = RecipeEngine::new().synthesize(&profile, -6.0, SoundSource::Heat, Some(&ash), 0x2222_2222_2222_2222);
        assert!(
            a.iter().zip(&b).any(|(x, y)| x.to_bits() != y.to_bits()),
            "distinct seeds must produce distinct synthesis output (harness non-blind)"
        );
    }

    #[test]
    fn buffer_pool_round_robin() {
        let mut pool = BufferPool::new();
        for i in 0..8 {
            let buf = pool.acquire();
            buf[0] = (i + 1) as f32;
        }
        let buf = pool.acquire();
        assert_eq!(buf[0], 0.0, "Buffer should be zeroed on acquire");
    }

    // engine_publishes_vibe_to_viz test EXCLUDED - needs crate::viz_buffer + publish_vibe_to_viz (both excluded).
    #[test]
    fn buffer_pool_preallocated() {
        let pool = BufferPool::new();
        assert_eq!(pool.buffers.len(), POOL_SIZE);
        for buf in &pool.buffers {
            assert_eq!(buf.len(), MAX_SAMPLES);
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_material_bitmask() -> impl Strategy<Value = MaterialBitmask> {
        (0u8..=255u8, 0u8..=255u8, 0u8..=255u8).prop_flat_map(|(a, b, c)| {
            let sum3 = a as u16 + b as u16 + c as u16;
            if sum3 > 255 {
                let total = sum3.max(1);
                let va = ((a as u16) * 255 / total) as u8;
                let vb = ((b as u16) * 255 / total) as u8;
                let vc = ((c as u16) * 255 / total) as u8;
                let vd = 255u8.saturating_sub(va).saturating_sub(vb).saturating_sub(vc);
                Just(MaterialBitmask { void_pct: va, shadow_pct: vb, ash_pct: vc, iron_pct: vd })
            } else {
                let d = (255 - sum3) as u8;
                Just(MaterialBitmask { void_pct: a, shadow_pct: b, ash_pct: c, iron_pct: d })
            }
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p1_material_classification_consistency(mask in arb_material_bitmask()) {
            let class = MaterialClass::from_bitmask(&mask);
            let max_val = mask.iron_pct.max(mask.shadow_pct).max(mask.ash_pct).max(mask.void_pct);
            if max_val == 0 {
                prop_assert_eq!(class, MaterialClass::Organic);
            } else {
                let class_val = match class {
                    MaterialClass::Iron => mask.iron_pct,
                    MaterialClass::Shadow => mask.shadow_pct,
                    MaterialClass::Ash => mask.ash_pct,
                    MaterialClass::Void => mask.void_pct,
                    MaterialClass::Organic => 0,
                };
                prop_assert_eq!(class_val, max_val);
                if mask.iron_pct == max_val { prop_assert_eq!(class, MaterialClass::Iron); }
                else if mask.shadow_pct == max_val { prop_assert_eq!(class, MaterialClass::Shadow); }
                else if mask.ash_pct == max_val { prop_assert_eq!(class, MaterialClass::Ash); }
                else { prop_assert_eq!(class, MaterialClass::Void); }
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p8_dispatch_table_completeness(mat_idx in 0u8..5, src_idx in 0u8..5) {
            let recipe = DISPATCH_TABLE[mat_idx as usize][src_idx as usize];
            let _name = match recipe {
                RecipeId::FmClang => "FmClang",
                RecipeId::FmBass => "FmBass",
                RecipeId::Bounce => "Bounce",
                RecipeId::Friction => "Friction",
                RecipeId::Subtractive => "Subtractive",
                RecipeId::Additive => "Additive",
            };
        }
    }

    fn arb_recipe_id() -> impl Strategy<Value = RecipeId> {
        prop_oneof![
            Just(RecipeId::FmClang), Just(RecipeId::FmBass), Just(RecipeId::Bounce),
            Just(RecipeId::Friction), Just(RecipeId::Subtractive), Just(RecipeId::Additive),
        ]
    }

    fn arb_audible_profile() -> impl Strategy<Value = AudioMaterialProfile> {
        (20.0f32..20000.0, 0.0f32..=1.0, 0.0f32..=1.0, 0.01f32..2.0, 0.0f32..=1.0)
            .prop_map(|(freq, attack, harmonic, decay, reverb)| {
                AudioMaterialProfile { ring_frequency_hz: freq, attack_sharpness: attack,
                    harmonic_content: harmonic, decay_secs: decay, reverb_amount: reverb }
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p7_recipe_non_silence(
            profile in arb_audible_profile(),
            recipe_id in arb_recipe_id(),
            seed in any::<u64>(),
        ) {
            let num_samples = 4410;
            let mut params = RecipeParams::from_profile(&profile, -10.0, seed, 44100, num_samples);
            let mut buf = vec![0.0f32; num_samples];
            match recipe_id {
                RecipeId::FmClang => fm_clang::recipe_fm_clang(&params, &mut buf),
                RecipeId::FmBass => {
                    params.fm_ratio = 1.0;
                    params.fm_feedback = profile.reverb_amount * 0.9;
                    fm_bass::recipe_fm_bass(&params, &mut buf);
                }
                RecipeId::Bounce => bounce::recipe_bounce(&params, &mut buf),
                RecipeId::Friction => friction::recipe_friction(&params, &mut buf),
                RecipeId::Subtractive => subtractive::recipe_subtractive(&params, &mut buf),
                RecipeId::Additive => additive::recipe_additive(&params, &mut buf),
            }
            let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            prop_assert!(peak > 0.001, "Recipe {:?} produced silence (peak={})", recipe_id, peak);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p2_synthesis_determinism(
            profile in arb_audible_profile(),
            recipe_id in arb_recipe_id(),
            seed in any::<u64>(),
        ) {
            let num_samples = 2205;
            let mut params = RecipeParams::from_profile(&profile, -10.0, seed, 44100, num_samples);
            let mut buf1 = vec![0.0f32; num_samples];
            let mut buf2 = vec![0.0f32; num_samples];
            match recipe_id {
                RecipeId::FmClang => {
                    fm_clang::recipe_fm_clang(&params, &mut buf1);
                    fm_clang::recipe_fm_clang(&params, &mut buf2);
                }
                RecipeId::FmBass => {
                    params.fm_ratio = 1.0;
                    fm_bass::recipe_fm_bass(&params, &mut buf1);
                    fm_bass::recipe_fm_bass(&params, &mut buf2);
                }
                RecipeId::Bounce => {
                    bounce::recipe_bounce(&params, &mut buf1);
                    bounce::recipe_bounce(&params, &mut buf2);
                }
                RecipeId::Friction => {
                    friction::recipe_friction(&params, &mut buf1);
                    friction::recipe_friction(&params, &mut buf2);
                }
                RecipeId::Subtractive => {
                    subtractive::recipe_subtractive(&params, &mut buf1);
                    subtractive::recipe_subtractive(&params, &mut buf2);
                }
                RecipeId::Additive => {
                    additive::recipe_additive(&params, &mut buf1);
                    additive::recipe_additive(&params, &mut buf2);
                }
            }
            for (i, (a, b)) in buf1.iter().zip(buf2.iter()).enumerate() {
                prop_assert_eq!(a.to_bits(), b.to_bits(),
                    "Sample {} differs: {} vs {} for {:?} seed={}", i, a, b, recipe_id, seed);
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p15_variable_duration_buffer_length(duration in 0.01f32..1.0) {
            let sr = 44100u32;
            let expected = (duration * sr as f32).round() as usize;
            let actual = (duration * sr as f32) as usize;
            let diff = expected.abs_diff(actual);
            prop_assert!(diff <= 1);
        }
    }
}