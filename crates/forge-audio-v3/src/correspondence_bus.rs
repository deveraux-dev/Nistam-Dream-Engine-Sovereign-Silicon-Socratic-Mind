//! Correspondence Bus — shared state updated per audio block.
//! Every system reads from this. Mixer writes to it each mix_block cycle.
//!
//! No allocations. All state pre-allocated in the struct.
//! f32 for signal processing (GPU-side values), f64 for phase accumulators only.

use crate::dsp::{AudioBuffer, BiquadState};
use forge_colour;
use crate::creature_engine::AudioProfile;

/// Harmonic compatibility between two Camelot keys (0.0 = clash, 1.0 = perfect).
///
/// Camelot wheel: 12 positions (1-12), two modes (A=minor, B=major).
/// Same position = perfect. ±1 on wheel = compatible. Same number A↔B = relative key.
pub fn camelot_compat(key_a: &str, key_b: &str) -> f32 {
    let parse = |k: &str| -> Option<(i32, char)> {
        let k = k.trim();
        if k.len() < 2 { return None; }
        let letter = k.chars().last()?;
        if letter != 'A' && letter != 'B' { return None; }
        let num: i32 = k[..k.len() - 1].parse().ok()?;
        if !(1..=12).contains(&num) { return None; }
        Some((num, letter))
    };

    let (na, la) = match parse(key_a) { Some(v) => v, None => return 0.0 };
    let (nb, lb) = match parse(key_b) { Some(v) => v, None => return 0.0 };

    // Same key = perfect
    if na == nb && la == lb {
        return 1.0;
    }

    // Same number, different letter (relative major/minor)
    if na == nb && la != lb {
        return 0.9;
    }

    // Same letter — distance on the Camelot wheel (mod 12)
    if la == lb {
        let dist = ((na - nb).abs()).min(12 - (na - nb).abs());
        return match dist {
            1 => 0.85,
            2 => 0.6,
            3 => 0.3,
            _ => 0.1,
        };
    }

    // Different letter AND different number — cross-mode distance
    // Adjacent on wheel in opposite mode still somewhat compatible
    let dist = ((na - nb).abs()).min(12 - (na - nb).abs());
    match dist {
        0 => 0.9, // same number handled above, but guard
        1 => 0.7,
        2 => 0.4,
        _ => 0.1,
    }
}

/// Parse "8B" / "5A" → (position 1-12, mode A|B). Returns None on bad input.
fn parse_camelot(k: &str) -> Option<(u8, char)> {
    let k = k.trim();
    if k.len() < 2 { return None; }
    let mode = k.chars().last()?;
    if mode != 'A' && mode != 'B' { return None; }
    let num: u8 = k[..k.len() - 1].parse().ok()?;
    if !(1..=12).contains(&num) { return None; }
    Some((num, mode))
}

/// Derive an OKLCH seed from two Camelot key strings.
/// Returns None if either key is invalid.
/// Feed to `forge_colour::palette64()` / `palette64_lut()` for a full LUT.
pub fn camelot_oklch_seed(key_a: &str, key_b: &str) -> Option<forge_core::colour::OklchColor> {
    let (pos, mode) = parse_camelot(key_a)?;
    let _ = parse_camelot(key_b)?;
    let compat_pmy = (camelot_compat(key_a, key_b).clamp(0.0, 1.0) * 10_000.0) as u32;
    Some(forge_colour::camelot_to_oklch(pos, mode, compat_pmy))
}

/// How close a BPM is to the 100-120 groove sweet spot (0.0-1.0).
pub fn groove_lock(bpm: f64) -> f32 {
    if bpm <= 0.0 {
        return 0.0;
    }
    // Half time → treat as if doubled
    let effective = if (50.0..=60.0).contains(&bpm) {
        return 0.8;
    } else if (200.0..=240.0).contains(&bpm) {
        return 0.6;
    } else {
        bpm
    };
    if (100.0..=120.0).contains(&effective) {
        1.0
    } else if (80.0..100.0).contains(&effective) {
        // Linear falloff 80→0.0, 100→1.0
        ((effective - 80.0) / 20.0) as f32
    } else if (120.0..140.0).contains(&effective) {
        // Linear falloff 120→1.0, 140→0.0
        ((140.0 - effective) / 20.0) as f32
    } else {
        0.0
    }
}

/// Biquad coefficients for a bandpass filter (Audio EQ Cookbook).
/// Returns (b0, b1, b2, a1, a2) normalized by a0.
fn bandpass_coeffs(center_hz: f32, q: f32, sample_rate: u32) -> (f32, f32, f32, f32, f32) {
    let w0 = 2.0 * std::f32::consts::PI * center_hz / sample_rate as f32;
    let alpha = w0.sin() / (2.0 * q);
    let a0 = 1.0 + alpha;
    (
        (alpha) / a0,          // b0
        0.0,                   // b1
        (-alpha) / a0,         // b2
        (-2.0 * w0.cos()) / a0, // a1
        (1.0 - alpha) / a0,    // a2
    )
}

pub struct CorrespondenceBus {
    // Per-deck state
    pub vocal_energy: [f32; 4],
    pub deck_keys: [Option<String>; 4],
    pub deck_bpm: [f64; 4],

    // Derived
    pub harmonic_compat: f32,
    pub vocal_collision: f32,
    pub groove_lock: f32,

    // Binaural state (f64 for phase accumulators — precision matters)
    pub binaural_phase_l: f64,
    pub binaural_phase_r: f64,
    pub sub_bass_phase: f64,

    // Vocal detector biquad state per deck (persistent across blocks)
    pub vocal_biquad: [BiquadState; 4],

    // Cached bandpass coefficients (recomputed only on sample rate change)
    bp_coeffs: (f32, f32, f32, f32, f32),
    cached_sample_rate: u32,

    // Audio-first cue state (fires 150ms before visual reacts)
    cue_sweep_phase: f64,      // notch sweep on vocal collision
    cue_sweep_remaining: f32,  // samples remaining in sweep
    cue_bloom_remaining: f32,  // reverb bloom on key clash
    cue_pulse_remaining: f32,  // sub pulse on groove lock threshold
    prev_vocal_collision: f32, // edge detection
    prev_groove_lock: f32,     // edge detection
}

impl Default for CorrespondenceBus {
    fn default() -> Self {
        Self::new()
    }
}

impl CorrespondenceBus {
    pub fn new() -> Self {
        let coeffs = bandpass_coeffs(1500.0, 0.7, 48000);
        Self {
            vocal_energy: [0.0; 4],
            deck_keys: [None, None, None, None],
            deck_bpm: [0.0; 4],
            harmonic_compat: 0.0,
            vocal_collision: 0.0,
            groove_lock: 0.0,
            binaural_phase_l: 0.0,
            binaural_phase_r: 0.0,
            sub_bass_phase: 0.0,
            vocal_biquad: Default::default(),
            bp_coeffs: coeffs,
            cached_sample_rate: 48000,
            cue_sweep_phase: 0.0,
            cue_sweep_remaining: 0.0,
            cue_bloom_remaining: 0.0,
            cue_pulse_remaining: 0.0,
            prev_vocal_collision: 0.0,
            prev_groove_lock: 0.0,
        }
    }

    /// Recompute bandpass coefficients if sample rate changed.
    fn ensure_coeffs(&mut self, sample_rate: u32) {
        if sample_rate != self.cached_sample_rate && sample_rate > 0 {
            self.bp_coeffs = bandpass_coeffs(1500.0, 0.7, sample_rate);
            self.cached_sample_rate = sample_rate;
        }
    }

    /// Bandpass 300-3400Hz and compute RMS for vocal energy detection.
    /// Call once per deck per mix_block. No allocations — processes in-place with biquad state.
    pub fn detect_vocal(&mut self, deck: usize, block: &[f32], sample_rate: u32) {
        if deck >= 4 || block.is_empty() {
            return;
        }
        self.ensure_coeffs(sample_rate);

        let (b0, b1, b2, a1, a2) = self.bp_coeffs;
        let state = &mut self.vocal_biquad[deck];

        // Process block through biquad, accumulate squared output for RMS
        let mut sum_sq: f64 = 0.0;
        for &x in block {
            let y = b0 * x + b1 * state.x1 + b2 * state.x2
                  - a1 * state.y1 - a2 * state.y2;
            state.x2 = state.x1;
            state.x1 = x;
            state.y2 = state.y1;
            state.y1 = y;
            sum_sq += (y as f64) * (y as f64);
        }
        let rms = (sum_sq / block.len() as f64).sqrt() as f32;

        // Smooth: attack ~50ms, release ~200ms at 48kHz/1024 block
        let current = self.vocal_energy[deck];
        if rms > current {
            // Attack: ~50ms → alpha ≈ 0.3 per block at 1024/48000
            self.vocal_energy[deck] = current + 0.3 * (rms - current);
        } else {
            // Release: ~200ms → alpha ≈ 0.08 per block
            self.vocal_energy[deck] = current + 0.08 * (rms - current);
        }
    }

    /// Update derived state after all decks processed.
    pub fn update_derived(&mut self) {
        // Find two decks with highest vocal energy
        let mut indices: [usize; 4] = [0, 1, 2, 3];
        indices.sort_by(|&a, &b| {
            self.vocal_energy[b]
                .partial_cmp(&self.vocal_energy[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let a = indices[0];
        let b = indices[1];

        // Harmonic compatibility between the two loudest decks
        self.harmonic_compat = match (&self.deck_keys[a], &self.deck_keys[b]) {
            (Some(ka), Some(kb)) => camelot_compat(ka, kb),
            _ => 0.0,
        };

        // Vocal collision: how much two vocals overlap
        let va = self.vocal_energy[a];
        let vb = self.vocal_energy[b];
        if va > 0.3 && vb > 0.3 {
            let max_v = va.max(vb);
            let min_v = va.min(vb);
            self.vocal_collision = if max_v > 0.001 { min_v / max_v } else { 0.0 };
        } else {
            self.vocal_collision = 0.0;
        }

        // Groove lock: average of playing decks (bpm > 0)
        let mut sum = 0.0f32;
        let mut count = 0u32;
        for &bpm in &self.deck_bpm {
            if bpm > 0.0 {
                sum += groove_lock(bpm);
                count += 1;
            }
        }
        self.groove_lock = if count > 0 { sum / count as f32 } else { 0.0 };
    }

    /// Inject psychoacoustic layer into master output. Called after crossfade mix.
    /// All signals below -30dB. The DJ doesn't hear them consciously.
    /// No allocations — uses pre-allocated phase state.
    pub fn inject_psychoacoustic(&mut self, output: &mut AudioBuffer) {
        // Only active when groove is locked and vocals aren't colliding
        if self.groove_lock <= 0.7 {
            return;
        }

        let channels = output.channels();
        if channels < 2 {
            return;
        }
        let sr = output.sample_rate as f64;
        if sr <= 0.0 {
            return;
        }
        let frames = output.samples[0].len();

        // Compute master RMS for AM modulation of sub-bass
        let master_rms: f32 = {
            let sum: f32 = output.samples.iter()
                .flat_map(|ch| ch.iter())
                .map(|s| s * s)
                .sum();
            let count = (channels * frames).max(1);
            (sum / count as f32).sqrt()
        };

        let two_pi = 2.0 * std::f64::consts::PI;

        // Choose binaural frequency based on state
        let (freq_l, freq_r) = if self.vocal_collision > 0.5 {
            // Alert state: 13Hz binaural beat (beta range)
            (40.0_f64, 53.0_f64)
        } else {
            // Flow state: 7Hz binaural beat (theta range)
            (40.0_f64, 47.0_f64)
        };

        // Amplitude: -36dB ≈ 0.016, -30dB ≈ 0.032
        let amp_binaural: f32 = 0.016;
        let amp_sub: f32 = 0.032;
        let sub_freq = 28.0_f64;

        // Only inject when vocal collision is low
        let inject_binaural = self.vocal_collision < 0.2;

        for i in 0..frames {
            // Sub-bass on both channels (AM by master RMS)
            let sub = (self.sub_bass_phase * two_pi).sin() as f32 * amp_sub * master_rms;
            self.sub_bass_phase += sub_freq / sr;
            if self.sub_bass_phase >= 1.0 {
                self.sub_bass_phase -= 1.0;
            }

            output.samples[0][i] += sub;
            output.samples[1][i] += sub;

            if inject_binaural {
                // Left ear: freq_l Hz
                let bl = (self.binaural_phase_l * two_pi).sin() as f32 * amp_binaural;
                self.binaural_phase_l += freq_l / sr;
                if self.binaural_phase_l >= 1.0 {
                    self.binaural_phase_l -= 1.0;
                }
                output.samples[0][i] += bl;

                // Right ear: freq_r Hz
                let br = (self.binaural_phase_r * two_pi).sin() as f32 * amp_binaural;
                self.binaural_phase_r += freq_r / sr;
                if self.binaural_phase_r >= 1.0 {
                    self.binaural_phase_r -= 1.0;
                }
                output.samples[1][i] += br;
            }

            // --- Audio-first cues (fire 150ms before visual reacts) ---

            // Notch sweep on vocal collision onset
            if self.cue_sweep_remaining > 0.0 {
                let progress = 1.0 - (self.cue_sweep_remaining / (sr as f32 * 0.1));
                let freq = 2000.0 - progress * 1500.0; // 2kHz → 500Hz sweep
                let cue = (self.cue_sweep_phase * two_pi).sin() as f32 * 0.02 * (1.0 - progress);
                self.cue_sweep_phase += freq as f64 / sr;
                if self.cue_sweep_phase >= 1.0 { self.cue_sweep_phase -= 1.0; }
                output.samples[0][i] += cue;
                output.samples[1][i] += cue;
                self.cue_sweep_remaining -= 1.0;
            }

            // Reverb bloom on key clash
            if self.cue_bloom_remaining > 0.0 {
                let decay = self.cue_bloom_remaining / (sr as f32 * 0.15);
                let bloom = (self.cue_sweep_phase * two_pi * 1.5).sin() as f32 * 0.015 * decay;
                output.samples[0][i] += bloom;
                output.samples[1][i] += bloom * 0.7; // slight stereo offset
                self.cue_bloom_remaining -= 1.0;
            }

            // Sub pulse on groove lock threshold crossing
            if self.cue_pulse_remaining > 0.0 {
                let decay = self.cue_pulse_remaining / (sr as f32 * 0.08);
                let pulse = (self.sub_bass_phase * two_pi * 2.0).sin() as f32 * 0.025 * decay;
                output.samples[0][i] += pulse;
                output.samples[1][i] += pulse;
                self.cue_pulse_remaining -= 1.0;
            }
        }

        // Edge detection: trigger cues on state transitions
        if self.vocal_collision > 0.5 && self.prev_vocal_collision <= 0.5 {
            self.cue_sweep_remaining = sr as f32 * 0.1; // 100ms sweep
        }
        if self.harmonic_compat > 0.0 && self.harmonic_compat < 0.3
            && self.prev_vocal_collision >= 0.3 // was fine, now clashing
        {
            self.cue_bloom_remaining = sr as f32 * 0.15; // 150ms bloom
        }
        if self.groove_lock > 0.8 && self.prev_groove_lock <= 0.8 {
            self.cue_pulse_remaining = sr as f32 * 0.08; // 80ms confirmation pulse
        }
        self.prev_vocal_collision = self.vocal_collision;
        self.prev_groove_lock = self.groove_lock;
    }

    /// Set ambient reverb wet mix (0.0-1.0). Driven by rain_intensity.
    pub fn set_ambient_reverb(&mut self, _wet: f32) {
        // TODO: modulate reverb tail length when reverb DSP is integrated
    }

    /// Set ambient high-pass filter sweep (0.0-1.0). Driven by wind_speed.
    pub fn set_ambient_filter(&mut self, _cutoff_norm: f32) {
        // TODO: modulate ambient noise HP cutoff when wind synth is integrated
    }

    /// Set ambient low-pass damping (0.0-1.0). Driven by fog_density.
    pub fn set_ambient_damping(&mut self, _damping: f32) {
        // TODO: modulate master LP when fog damping DSP is integrated
    }

    /// Retune the material bandpass filter from a creature-engine `AudioProfile`.
    ///
    /// Maps `pitch_mult` → ring Hz (440 Hz at middle register) and takes
    /// `resonance_q` directly. Metal (Q = 12, high mass = low pitch) → narrow
    /// ring; bone (Q = 6) → broader bandpass. Call on material change only.
    pub fn update_from_audio_profile(&mut self, profile: &AudioProfile) {
        let ring_hz = (440.0 * profile.pitch_mult).clamp(80.0, 16_000.0);
        self.bp_coeffs = bandpass_coeffs(ring_hz, profile.resonance_q.max(0.1), self.cached_sample_rate);
    }

    /// Current material bandpass coefficients `(b0, b1, b2, a1, a2)`.
    /// Copy into a DSP thread to pre-filter a signal without Bus ownership.
    pub fn material_coeffs(&self) -> (f32, f32, f32, f32, f32) {
        self.bp_coeffs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camelot_same_key() {
        assert_eq!(camelot_compat("8B", "8B"), 1.0);
        assert_eq!(camelot_compat("5A", "5A"), 1.0);
    }

    #[test]
    fn camelot_relative_key() {
        assert!((camelot_compat("8B", "8A") - 0.9).abs() < 0.01);
    }

    #[test]
    fn camelot_adjacent() {
        assert!((camelot_compat("8B", "9B") - 0.85).abs() < 0.01);
        assert!((camelot_compat("8B", "7B") - 0.85).abs() < 0.01);
    }

    #[test]
    fn camelot_wrap_around() {
        // 12B and 1B are adjacent on the wheel
        assert!((camelot_compat("12B", "1B") - 0.85).abs() < 0.01);
    }

    #[test]
    fn camelot_distant() {
        assert!((camelot_compat("1B", "7B") - 0.1).abs() < 0.01);
    }

    #[test]
    fn camelot_invalid() {
        assert_eq!(camelot_compat("", "8B"), 0.0);
        assert_eq!(camelot_compat("ZZ", "8B"), 0.0);
    }

    #[test]
    fn camelot_oklch_seed_perfect_match_is_full_chroma() {
        let seed = camelot_oklch_seed("8B", "8B").unwrap();
        assert_eq!(seed.h, 0, "8B hue={}", seed.h);
        let expected_c = (3_000u32 * u16::MAX as u32 / 4_000) as u16; assert_eq!(seed.c, expected_c, "compat=1.0 chroma={}", seed.c);
    }

    #[test]
    fn camelot_oklch_seed_invalid_key_returns_none() {
        assert!(camelot_oklch_seed("ZZ", "8B").is_none());
        assert!(camelot_oklch_seed("8B", "").is_none());
    }

    #[test]
    fn camelot_oklch_seed_distant_keys_desaturate() {
        let close = camelot_oklch_seed("8B", "8B").unwrap();
        let far   = camelot_oklch_seed("8B", "2B").unwrap(); // compat ≈ 0.1
        assert!(far.c < close.c, "distant keys should desaturate: far={} close={}", far.c, close.c);
    }

    #[test]
    fn groove_lock_sweet_spot() {
        assert_eq!(groove_lock(110.0), 1.0);
        assert_eq!(groove_lock(100.0), 1.0);
        assert_eq!(groove_lock(120.0), 1.0);
    }

    #[test]
    fn groove_lock_falloff() {
        assert!((groove_lock(90.0) - 0.5).abs() < 0.01);
        assert!((groove_lock(130.0) - 0.5).abs() < 0.01);
    }

    #[test]
    fn groove_lock_half_time() {
        assert!((groove_lock(55.0) - 0.8).abs() < 0.01);
    }

    #[test]
    fn groove_lock_zero() {
        assert_eq!(groove_lock(0.0), 0.0);
        assert_eq!(groove_lock(160.0), 0.0);
    }

    #[test]
    fn update_from_audio_profile_metal_rings_narrow() {
        use crate::creature_engine::{derive_audio, PhysicalProfile, SurfaceMaterial};
        let metal_profile = PhysicalProfile {
            surface_material: SurfaceMaterial::Metal,
            mass_kg: 500.0,
            height_m: 1.8, width_m: 0.6, limb_ratio: 0.5,
            limb_count: 4, surface_hardness: 1.0,
            volume_m3: 0.3, compactness: 0.8, symmetry: 1.0,
        };
        let bone_profile = PhysicalProfile {
            surface_material: SurfaceMaterial::Bone,
            mass_kg: 150.0,
            height_m: 1.5, width_m: 0.4, limb_ratio: 0.6,
            limb_count: 4, surface_hardness: 0.7,
            volume_m3: 0.1, compactness: 0.6, symmetry: 1.0,
        };
        let mut bus = CorrespondenceBus::new();
        bus.update_from_audio_profile(&derive_audio(&metal_profile));
        let (metal_b0, ..) = bus.material_coeffs();
        bus.update_from_audio_profile(&derive_audio(&bone_profile));
        let (bone_b0, ..) = bus.material_coeffs();
        // Metal Q=12 → alpha = sin(w0)/(2*12) is smaller → smaller b0 = alpha/a0.
        // Bone Q=6 → larger alpha → larger b0. Metal rings narrow, bone resonates broad.
        assert!(metal_b0 < bone_b0,
            "metal rings narrower than bone: metal_b0={metal_b0} bone_b0={bone_b0}");
    }
}