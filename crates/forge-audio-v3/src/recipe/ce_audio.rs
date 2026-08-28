//! CE Scan → Audio Profile Derivation.
//!
//! Derives AudioMaterialProfile from CE MaterialScan pixel analysis.
//! Blends weapon + sigil scans for crafting-driven audio.

use super::AudioMaterialProfile;
use crate::correspondence::MaterialScan;
use crate::gesture_brush::BrushOp;

// ---------------------------------------------------------------------------
// derive_audio_profile_from_ce
// ---------------------------------------------------------------------------

/// Derive AudioMaterialProfile from CE MaterialScan.
///
/// Material distribution: counts\[0\]=Void, \[1\]=Shadow, \[2\]=Iron, \[3\]=Stone,
/// \[4\]=Bone, \[5\]=Ash, \[6\]=None.
pub fn derive_audio_profile_from_ce(scan: &MaterialScan) -> AudioMaterialProfile {
    if scan.total_pixels == 0 {
        return AudioMaterialProfile {
            ring_frequency_hz: 200.0,
            attack_sharpness: 0.2,
            harmonic_content: 0.1,
            decay_secs: 0.3,
            reverb_amount: 0.1,
        };
    }

    let total = scan.total_pixels as f32;
    let void_r = scan.counts[0] as f32 / total;
    let shadow_r = scan.counts[1] as f32 / total;
    let iron_r = scan.counts[2] as f32 / total;
    let _stone_r = scan.counts[3] as f32 / total;
    let bone_r = scan.counts[4] as f32 / total;
    let ash_r = scan.counts[5] as f32 / total;

    let ring_frequency_hz = 20.0 + iron_r * 19980.0;
    let attack_sharpness = (iron_r + shadow_r * 0.5).clamp(0.0, 1.0);
    let harmonic_content = (iron_r * 0.8 + ash_r * 0.3 + bone_r * 0.2).clamp(0.0, 1.0);
    let decay_secs = 0.01 + (1.0 - void_r) * 2.0;
    let reverb_amount = void_r.clamp(0.0, 1.0);

    AudioMaterialProfile {
        ring_frequency_hz,
        attack_sharpness,
        harmonic_content,
        decay_secs,
        reverb_amount,
    }
}

// ---------------------------------------------------------------------------
// blend_sigil_audio
// ---------------------------------------------------------------------------

fn dominant_material(scan: &MaterialScan) -> usize {
    if scan.total_pixels == 0 { return 6; }
    scan.counts.iter().enumerate().max_by_key(|&(_, c)| *c).map(|(i, _)| i).unwrap_or(6)
}

/// Blend weapon + sigil MaterialScans into a single AudioMaterialProfile.
pub fn blend_sigil_audio(
    weapon_scan: &MaterialScan,
    sigil_scans: &[&MaterialScan],
) -> AudioMaterialProfile {
    if sigil_scans.is_empty() {
        return derive_audio_profile_from_ce(weapon_scan);
    }

    let weapon_dominant = dominant_material(weapon_scan);
    let num_sources = (1 + sigil_scans.len()) as f32;
    let weight = 1.0 / num_sources;

    let weapon_total = weapon_scan.total_pixels.max(1) as f32;
    let mut blended_counts = [0.0f32; 7];
    for i in 0..7 {
        blended_counts[i] += weapon_scan.counts[i] as f32 / weapon_total * weight;
    }

    let mut matching_boost = false;
    for sigil in sigil_scans {
        let sigil_total = sigil.total_pixels.max(1) as f32;
        let sigil_dominant = dominant_material(sigil);
        if sigil_dominant == weapon_dominant {
            matching_boost = true;
        }
        for i in 0..7 {
            blended_counts[i] += sigil.counts[i] as f32 / sigil_total * weight;
        }
    }

    let iron_r = blended_counts[2];
    let void_r = blended_counts[0];
    let shadow_r = blended_counts[1];
    let ash_r = blended_counts[5];
    let bone_r = blended_counts[4];

    let mut ring_frequency_hz = 20.0 + iron_r * 19980.0;
    let mut attack_sharpness = (iron_r + shadow_r * 0.5).clamp(0.0, 1.0);
    let mut harmonic_content = (iron_r * 0.8 + ash_r * 0.3 + bone_r * 0.2).clamp(0.0, 1.0);
    let decay_secs = 0.01 + (1.0 - void_r) * 2.0;
    let reverb_amount = void_r.clamp(0.0, 1.0);

    if matching_boost {
        ring_frequency_hz *= 1.25;
        harmonic_content = (harmonic_content * 1.15).min(1.0);
        attack_sharpness = (attack_sharpness * 1.1).min(1.0);
    }

    AudioMaterialProfile {
        ring_frequency_hz: ring_frequency_hz.clamp(20.0, 25000.0),
        attack_sharpness,
        harmonic_content,
        decay_secs,
        reverb_amount,
    }
}

// ---------------------------------------------------------------------------
// L6 LATERAL: Effort Tokens -> Material Sound
// Design ref: docs/design-bible/LATERAL-CONNECTIONS-WIRING-LEDGER.md §L6.
// classify_effort() (forge-core::gesture_brush) already turns a paint stroke
// into a BrushOp; this is the missing link — the SAME BrushOp modulating the
// material's own impact voice, so a Press sounds different from a Flick on
// the SAME material. Painting IS sound (the seehear thesis).
// ---------------------------------------------------------------------------

/// Modulate `base` (a material's resting `AudioMaterialProfile`) by the
/// gesture's classified [`BrushOp`] — the impact voice this specific stroke
/// would strike. Bounded multipliers only; `base` itself is never mutated.
///
/// - `Press` (deep, sustained push): softer onset, longer ring, more reverb —
///   a mallet strike, not a tap.
/// - `Flick` (light, quick, jittery): sharp transient, short decay, brighter
///   harmonics — a flick of a fingernail against glass.
/// - `Wring` (strong, sustained, twisting): moderate onset, the LONGEST ring,
///   boosted harmonics — a bell set spinning, not struck once.
pub fn effort_to_impact_profile(op: BrushOp, base: &AudioMaterialProfile) -> AudioMaterialProfile {
    let (attack_mul, decay_mul, harmonic_mul, reverb_mul) = match op {
        BrushOp::Press => (0.7, 1.4, 1.0, 1.3),
        BrushOp::Flick => (1.5, 0.5, 1.2, 0.8),
        BrushOp::Wring => (0.9, 1.6, 1.35, 1.1),
        // The five completing the octet, 2026-08-26. `[AUTHORED]`, but not
        // freely: each follows the axes the original three already exhibit —
        // Quick raises attack and shortens decay, Sustained does the reverse,
        // Flexible adds harmonic content, Direct keeps it plain.
        BrushOp::Punch => (1.6, 0.45, 1.0, 0.9),
        BrushOp::Slash => (1.55, 0.55, 1.3, 1.0),
        BrushOp::Dab => (1.4, 0.4, 0.95, 0.75),
        BrushOp::Glide => (0.6, 1.5, 0.95, 1.25),
        BrushOp::Float => (0.5, 1.7, 1.25, 1.4),
    };
    AudioMaterialProfile {
        ring_frequency_hz: base.ring_frequency_hz,
        attack_sharpness: (base.attack_sharpness * attack_mul).clamp(0.0, 1.0),
        harmonic_content: (base.harmonic_content * harmonic_mul).clamp(0.0, 1.0),
        decay_secs: (base.decay_secs * decay_mul).clamp(0.01, 10.0),
        reverb_amount: (base.reverb_amount * reverb_mul).clamp(0.0, 1.0),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn base_profile() -> AudioMaterialProfile {
        AudioMaterialProfile {
            ring_frequency_hz: 440.0,
            attack_sharpness: 0.5,
            harmonic_content: 0.5,
            decay_secs: 0.5,
            reverb_amount: 0.5,
        }
    }

    #[test]
    fn press_softens_attack_and_lengthens_decay() {
        let base = base_profile();
        let out = effort_to_impact_profile(BrushOp::Press, &base);
        assert!(out.attack_sharpness < base.attack_sharpness, "Press must soften the onset");
        assert!(out.decay_secs > base.decay_secs, "Press must ring longer");
        assert_eq!(out.ring_frequency_hz, base.ring_frequency_hz, "effort never retunes the material");
    }

    #[test]
    fn flick_sharpens_attack_and_shortens_decay() {
        let base = base_profile();
        let out = effort_to_impact_profile(BrushOp::Flick, &base);
        assert!(out.attack_sharpness > base.attack_sharpness, "Flick must sharpen the onset");
        assert!(out.decay_secs < base.decay_secs, "Flick must ring shorter");
    }

    #[test]
    fn wring_rings_the_longest_of_the_three() {
        let base = base_profile();
        let press = effort_to_impact_profile(BrushOp::Press, &base);
        let flick = effort_to_impact_profile(BrushOp::Flick, &base);
        let wring = effort_to_impact_profile(BrushOp::Wring, &base);
        assert!(wring.decay_secs > press.decay_secs && wring.decay_secs > flick.decay_secs);
    }

    #[test]
    fn every_field_stays_in_its_physical_bound() {
        let hot = AudioMaterialProfile {
            ring_frequency_hz: 20000.0,
            attack_sharpness: 0.95,
            harmonic_content: 0.95,
            decay_secs: 8.0,
            reverb_amount: 0.95,
        };
        for op in [BrushOp::Press, BrushOp::Flick, BrushOp::Wring] {
            let out = effort_to_impact_profile(op, &hot);
            assert!((0.0..=1.0).contains(&out.attack_sharpness));
            assert!((0.0..=1.0).contains(&out.harmonic_content));
            assert!((0.0..=1.0).contains(&out.reverb_amount));
            assert!((0.01..=10.0).contains(&out.decay_secs));
        }
    }

    fn make_scan(counts: [u32; 7], total: u32) -> MaterialScan {
        MaterialScan {
            total_pixels: total,
            counts,
            centroids: [(0.5, 0.5); 6],
            width: 8,
            height: 8,
        }
    }

    #[test]
    fn iron_dominant_high_frequency() {
        let scan = make_scan([0, 0, 90, 5, 5, 0, 0], 100);
        let profile = derive_audio_profile_from_ce(&scan);
        assert!(profile.ring_frequency_hz > 15000.0);
        assert!(profile.attack_sharpness > 0.5);
    }

    #[test]
    fn void_dominant_high_reverb() {
        let scan = make_scan([90, 0, 0, 5, 5, 0, 0], 100);
        let profile = derive_audio_profile_from_ce(&scan);
        assert!(profile.reverb_amount > 0.5);
        assert!(profile.decay_secs < 0.5);
    }

    #[test]
    fn empty_scan_fallback() {
        let scan = make_scan([0; 7], 0);
        let profile = derive_audio_profile_from_ce(&scan);
        assert!((profile.ring_frequency_hz - 200.0).abs() < 0.1);
    }

    #[test]
    fn matching_sigil_boosts_resonance() {
        let weapon = make_scan([0, 0, 80, 10, 10, 0, 0], 100);
        let weapon_only = derive_audio_profile_from_ce(&weapon);
        let sigil = make_scan([0, 0, 70, 15, 15, 0, 0], 100);
        let blended = blend_sigil_audio(&weapon, &[&sigil]);
        assert!(blended.ring_frequency_hz > weapon_only.ring_frequency_hz);
    }

    #[test]
    fn dual_sigils_equal_weight() {
        let weapon = make_scan([0, 0, 60, 20, 20, 0, 0], 100);
        let sigil1 = make_scan([50, 0, 0, 25, 25, 0, 0], 100);
        let sigil2 = make_scan([0, 50, 0, 25, 25, 0, 0], 100);
        let blend_12 = blend_sigil_audio(&weapon, &[&sigil1, &sigil2]);
        let blend_21 = blend_sigil_audio(&weapon, &[&sigil2, &sigil1]);
        assert!((blend_12.ring_frequency_hz - blend_21.ring_frequency_hz).abs() < 0.01);
        assert!((blend_12.reverb_amount - blend_21.reverb_amount).abs() < 0.01);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_material_scan() -> impl Strategy<Value = MaterialScan> {
        proptest::collection::vec(0u32..200, 7).prop_map(|counts| {
            let mut c = [0u32; 7];
            for (i, &v) in counts.iter().enumerate().take(7) { c[i] = v; }
            let total: u32 = c.iter().sum();
            MaterialScan {
                total_pixels: total.max(1),
                counts: c,
                centroids: [(0.5, 0.5); 6],
                width: 8,
                height: 8,
            }
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p12_ce_blend_proportionality(iron_lo in 0u32..50, iron_hi in 50u32..100) {
            let other = 50u32;
            let scan_lo = MaterialScan {
                total_pixels: 100,
                counts: [10, 10, iron_lo, 10, 10, 10, 100u32.saturating_sub(other + iron_lo)],
                centroids: [(0.5, 0.5); 6], width: 8, height: 8,
            };
            let scan_hi = MaterialScan {
                total_pixels: 100,
                counts: [10, 10, iron_hi, 10, 10, 10, 100u32.saturating_sub(other + iron_hi)],
                centroids: [(0.5, 0.5); 6], width: 8, height: 8,
            };
            let profile_lo = derive_audio_profile_from_ce(&scan_lo);
            let profile_hi = derive_audio_profile_from_ce(&scan_hi);
            prop_assert!(profile_hi.ring_frequency_hz >= profile_lo.ring_frequency_hz);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p13_sigil_matching_boost(iron_weapon in 50u32..90, iron_sigil in 50u32..90) {
            let weapon = MaterialScan {
                total_pixels: 100,
                counts: [5, 5, iron_weapon, 5, 5, 5, 100u32.saturating_sub(25 + iron_weapon)],
                centroids: [(0.5, 0.5); 6], width: 8, height: 8,
            };
            let sigil = MaterialScan {
                total_pixels: 100,
                counts: [5, 5, iron_sigil, 5, 5, 5, 100u32.saturating_sub(25 + iron_sigil)],
                centroids: [(0.5, 0.5); 6], width: 8, height: 8,
            };
            let blended = blend_sigil_audio(&weapon, &[&sigil]);
            let avg_iron_r = (iron_weapon as f32 / 100.0 + iron_sigil as f32 / 100.0) / 2.0;
            let unboosted_freq = 20.0 + avg_iron_r * 19980.0;
            prop_assert!(blended.ring_frequency_hz >= unboosted_freq);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p14_sigil_equal_weighting(
            weapon in arb_material_scan(),
            sigil1 in arb_material_scan(),
            sigil2 in arb_material_scan(),
        ) {
            let blend_12 = blend_sigil_audio(&weapon, &[&sigil1, &sigil2]);
            let blend_21 = blend_sigil_audio(&weapon, &[&sigil2, &sigil1]);
            prop_assert!((blend_12.ring_frequency_hz - blend_21.ring_frequency_hz).abs() < 0.01);
            prop_assert!((blend_12.reverb_amount - blend_21.reverb_amount).abs() < 0.01);
        }
    }
}
