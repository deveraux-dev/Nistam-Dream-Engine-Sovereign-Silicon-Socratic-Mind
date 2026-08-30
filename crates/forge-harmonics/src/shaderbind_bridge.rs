// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Audio lane producer — maps live note state to forge_shaderbind::SignalValues.
//! Integer-only deterministic harmonics snapshot at a given conductor tick.

use crate::mask::MaskProfile;
use crate::synthxml::ScheduledNote;
use forge_shaderbind::SignalValues;

/// Sub-bass ceiling in millihertz. Overlaps band 0; its own lane, not a partition member.
const SUB_BASS_CEIL_MHZ: u32 = 80_000;

/// Conductor ticks per beat: the 120 Hz master tick at 120 BPM.
const TICKS_PER_BEAT: u32 = 60;

/// Full-scale velocity sum for the absolute loudness lane: eight voices at 127.
const POLYPHONY_FULL_SCALE: u32 = 8 * 127;

/// Integer-only snapshot of harmonics engine at a conductor tick.
///
/// Captures active note state (fire_tick <= tick < fire_tick + dur_ms),
/// then derives spectral properties for audio lane routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarmonicFrame {
    /// Sum of active note velocities (0..=127 each).
    total_velocity: u32,
    /// Amplitude-weighted sum of frequencies.
    centroid_weighted_mhz: u64,
    /// Energy in each frequency band (7 ISO-31 octave centres).
    band_energy: [u32; 7],
    /// Energy in sub-bass (below 80 Hz).
    sub_bass_energy: u32,
    /// Conductor tick (120 Hz) for beat phase calculation.
    tick: u32,
}

impl HarmonicFrame {
    /// Compute frame state at `tick` (120 Hz conductor time).
    /// Notes are live if `fire_tick <= tick < fire_tick + dur_ticks`.
    pub fn from_note_plan(plan: &[ScheduledNote], tick: u32) -> Self {
        let mut frame = HarmonicFrame {
            total_velocity: 0,
            centroid_weighted_mhz: 0,
            band_energy: [0; 7],
            sub_bass_energy: 0,
            tick,
        };

        let tick_u64 = tick as u64;
        for note in plan {
            let dur_ticks = (note.dur_ms as u64 * 120) / 1000;
            if tick_u64 < note.fire_tick || tick_u64 >= note.fire_tick + dur_ticks {
                continue;
            }

            let vel_u32 = note.vel as u32;
            let mhz = crate::note_to_mhz(note.note);

            frame.total_velocity = frame.total_velocity.saturating_add(vel_u32);
            frame.centroid_weighted_mhz = frame.centroid_weighted_mhz.saturating_add(
                (mhz as u64).saturating_mul(vel_u32 as u64),
            );

            if mhz < SUB_BASS_CEIL_MHZ {
                frame.sub_bass_energy = frame.sub_bass_energy.saturating_add(vel_u32);
            }

            let band = crate::gammatone::erb_band_index(mhz);
            frame.band_energy[band] = frame.band_energy[band].saturating_add(vel_u32);
        }

        frame
    }

    /// Convert to audio lane routing values (Permyriad 0..=10000).
    pub fn signal_values(&self) -> SignalValues {
        let total = self.total_velocity;
        let rms = (((total as u64) * 10000) / POLYPHONY_FULL_SCALE as u64).min(10000) as u16;

        let spectral_centroid = if total > 0 {
            let mean_mhz = self.centroid_weighted_mhz / (total as u64);
            frequency_to_permyriad(mean_mhz as u32)
        } else {
            0
        };

        let beat_phase =
            ((self.tick % TICKS_PER_BEAT) as u64 * 10000 / TICKS_PER_BEAT as u64) as u16;

        let b = self.band_energy;
        SignalValues {
            audio_rms: rms,
            audio_beat_phase: beat_phase,
            audio_spectral_centroid: spectral_centroid,
            audio_sub_bass: energy_ratio(self.sub_bass_energy, total),
            audio_spectrum_low: energy_ratio(b[0] + b[1], total),
            audio_spectrum_mid: energy_ratio(b[2] + b[3] + b[4], total),
            audio_spectrum_high: energy_ratio(b[5] + b[6], total),
            audio_spectrum_bands: [
                energy_ratio(b[0], total),
                energy_ratio(b[1], total),
                energy_ratio(b[2], total),
                energy_ratio(b[3], total),
                energy_ratio(b[4], total),
                energy_ratio(b[5], total),
                energy_ratio(b[6], total),
            ],
            ..SignalValues::default()
        }
    }

    /// As [`HarmonicFrame::signal_values`], with the Zwicker spreading mask
    /// applied to the band array first. Masked bands emit zero and their energy
    /// is discarded, so the band shares no longer sum to full scale.
    pub fn signal_values_masked(&self, profile: &MaskProfile) -> SignalValues {
        let mut sv = self.signal_values();
        crate::mask::apply_mask(&mut sv.audio_spectrum_bands, profile);
        sv
    }
}

/// Band energy as a Permyriad share of total live velocity. Zero when silent.
#[inline]
fn energy_ratio(energy: u32, total: u32) -> u16 {
    if total == 0 {
        return 0;
    }
    (((energy as u64) * 10000) / total as u64).min(10000) as u16
}

/// Map frequency to Permyriad along the ERB-rate scale, not linear frequency.
/// A linear map crushes the whole musical register into the bottom few percent;
/// this places a pitch by its auditory distance, interpolating within its band.
#[inline]
fn frequency_to_permyriad(mhz: u32) -> u16 {
    let (span_lo, span_hi) = crate::gammatone::PITCH_SPAN_MHZ;
    let edges = crate::gammatone::erb_band_edges_mhz();
    let band = crate::gammatone::erb_band_index(mhz);

    let lo = if band == 0 { span_lo } else { edges[band - 1] };
    let hi = if band == edges.len() { span_hi } else { edges[band] };
    let clamped = mhz.clamp(lo, hi);

    let within = ((clamped - lo) as u64 * 10000) / ((hi - lo).max(1) as u64);
    (((band as u64 * 10000 + within) / crate::gammatone::ERB_BAND_COUNT as u64).min(10000)) as u16
}

/// Convenience: compute audio signal values for a plan at a tick.
#[inline]
pub fn signals_from_note_plan(plan: &[ScheduledNote], tick: u32) -> SignalValues {
    HarmonicFrame::from_note_plan(plan, tick).signal_values()
}

/// Audio lanes plus the harmonic colour lanes: the Tonnetz angle drives
/// `vibe_hue` and the tonal-focus radius drives `vibe_intensity`.
pub fn signals_with_colour(plan: &[ScheduledNote], tick: u32) -> SignalValues {
    let mut sv = signals_from_note_plan(plan, tick);
    let chroma = crate::tonnetz::chroma_from_notes(plan, tick);
    let (hue, saturation) = crate::tonnetz::hue_saturation(&crate::tonnetz::tonnetz_position(&chroma));
    sv.vibe_hue = hue;
    sv.vibe_intensity = saturation;
    sv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silent_plan_yields_default_audio_lanes() {
        let plan: Vec<ScheduledNote> = vec![];
        let sv = signals_from_note_plan(&plan, 0);
        assert_eq!(sv.audio_rms, 0);
        assert_eq!(sv.audio_spectral_centroid, 0);
        assert_eq!(sv.audio_sub_bass, 0);
        for band in sv.audio_spectrum_bands {
            assert_eq!(band, 0);
        }
    }

    #[test]
    fn loud_plan_yields_nonzero_audio_lanes() {
        let plan = vec![ScheduledNote {
            fire_tick: 0,
            note: 60,
            vel: 127,
            dur_ms: 1000,
        }];
        let sv = signals_from_note_plan(&plan, 30);
        assert!(sv.audio_rms > 0);
        assert!(sv.audio_spectral_centroid > 0);
        assert_eq!(sv.audio_sub_bass, 0);
        let any_band_nonzero = sv.audio_spectrum_bands.iter().any(|&b| b > 0);
        assert!(any_band_nonzero);
    }

    #[test]
    fn audio_not_identity_multiple_lanes_differ() {
        let plan = vec![
            ScheduledNote {
                fire_tick: 0,
                note: 40,
                vel: 100,
                dur_ms: 1000,
            },
            ScheduledNote {
                fire_tick: 0,
                note: 60,
                vel: 100,
                dur_ms: 1000,
            },
        ];
        let sv = signals_from_note_plan(&plan, 30);
        let lanes = [
            sv.audio_rms,
            sv.audio_spectral_centroid,
            sv.audio_sub_bass,
            sv.audio_spectrum_low,
            sv.audio_spectrum_mid,
            sv.audio_spectrum_high,
        ];
        let unique_count = lanes.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(unique_count >= 2, "at least two audio lanes must differ");
    }

    #[test]
    fn saturation_clamps_to_permyriad_max() {
        let plan: Vec<ScheduledNote> = (0..100)
            .map(|i| ScheduledNote {
                fire_tick: 0,
                note: (36 + i % 12) as u8,
                vel: 127,
                dur_ms: 1000,
            })
            .collect();
        let sv = signals_from_note_plan(&plan, 50);
        assert!(sv.audio_rms <= 10000);
        assert!(sv.audio_spectrum_low <= 10000);
        assert!(sv.audio_spectrum_mid <= 10000);
        assert!(sv.audio_spectrum_high <= 10000);
        for band in sv.audio_spectrum_bands {
            assert!(band <= 10000);
        }
    }

    #[test]
    fn deterministic_same_plan_tick_yields_same_output() {
        let plan = vec![
            ScheduledNote {
                fire_tick: 0,
                note: 60,
                vel: 100,
                dur_ms: 500,
            },
            ScheduledNote {
                fire_tick: 60,
                note: 67,
                vel: 80,
                dur_ms: 400,
            },
        ];
        let sv1 = signals_from_note_plan(&plan, 100);
        let sv2 = signals_from_note_plan(&plan, 100);
        assert_eq!(sv1, sv2);
    }

    #[test]
    fn every_midi_pitch_lands_in_exactly_one_band() {
        for note in 0u8..=127 {
            let plan = vec![ScheduledNote { fire_tick: 0, note, vel: 127, dur_ms: 1000 }];
            let sv = signals_from_note_plan(&plan, 0);
            let occupied = sv.audio_spectrum_bands.iter().filter(|&&b| b > 0).count();
            assert_eq!(occupied, 1, "note {note} occupied {occupied} bands, expected 1");
            let sum: u32 = sv.audio_spectrum_bands.iter().map(|&b| b as u32).sum();
            assert_eq!(sum, 10000, "note {note} lost energy: bands sum to {sum}");
        }
    }

    #[test]
    fn band_shares_sum_to_full_scale_across_the_pitch_range() {
        let plan: Vec<ScheduledNote> = [24u8, 48, 72, 96, 120]
            .iter()
            .map(|&note| ScheduledNote { fire_tick: 0, note, vel: 100, dur_ms: 1000 })
            .collect();
        let sv = signals_from_note_plan(&plan, 0);
        let sum: u32 = sv.audio_spectrum_bands.iter().map(|&b| b as u32).sum();
        assert!((9993..=10000).contains(&sum), "one integer floor loss per band at most: {sum}");
        assert!(sv.audio_spectrum_high > 0, "the top bands must carry energy");
    }

    #[test]
    fn the_musical_register_spreads_across_the_centroid_lane() {
        let centroid = |note: u8| {
            let plan = vec![ScheduledNote { fire_tick: 0, note, vel: 100, dur_ms: 1000 }];
            signals_from_note_plan(&plan, 0).audio_spectral_centroid
        };
        let (c2, c4, c6) = (centroid(36), centroid(60), centroid(84));
        assert!(c2 < c4 && c4 < c6, "centroid must rise with pitch: {c2} {c4} {c6}");
        assert!(
            c4 - c2 > 1000 && c6 - c4 > 1000,
            "octaves must be far apart, not crushed into the floor: {c2} {c4} {c6}"
        );
        assert!(c6 > 4000, "C6 sits well up the lane, not at 2% of it: {c6}");
    }

    #[test]
    fn the_erb_partition_is_the_one_in_use() {
        let plan = vec![ScheduledNote { fire_tick: 0, note: 83, vel: 127, dur_ms: 1000 }];
        let sv = signals_from_note_plan(&plan, 0);
        assert_eq!(sv.audio_spectrum_bands[2], 10000, "~988 Hz is ERB band 2");
        assert_eq!(sv.audio_spectrum_bands[4], 0, "the old octave partition put it in band 4");
    }

    #[test]
    fn masking_discards_energy_rather_than_redistributing_it() {
        let plan: Vec<ScheduledNote> = [36u8, 40, 90]
            .iter()
            .map(|&note| ScheduledNote { fire_tick: 0, note, vel: 100, dur_ms: 1000 })
            .collect();
        let frame = HarmonicFrame::from_note_plan(&plan, 0);
        let plain: u32 = frame.signal_values().audio_spectrum_bands.iter().map(|&b| b as u32).sum();
        let masked: u32 = frame
            .signal_values_masked(&MaskProfile::zwicker())
            .audio_spectrum_bands
            .iter()
            .map(|&b| b as u32)
            .sum();
        assert!((9993..=10000).contains(&plain), "shares fill the scale, less floor loss: {plain}");
        assert!(masked <= plain, "masking may only remove energy: {masked} vs {plain}");
    }

    #[test]
    fn colour_lanes_carry_harmony_and_silence_is_defined() {
        let plan = vec![ScheduledNote { fire_tick: 0, note: 60, vel: 127, dur_ms: 1000 }];
        let voiced = signals_with_colour(&plan, 0);
        assert!(voiced.vibe_intensity > 0, "a single sustained note is tonally focused");

        let silent = signals_with_colour(&[], 0);
        assert_eq!(silent.vibe_hue, 0);
        assert_eq!(silent.vibe_intensity, 0);
    }

    #[test]
    fn note_outside_duration_not_active() {
        let plan = vec![ScheduledNote {
            fire_tick: 10,
            note: 60,
            vel: 127,
            dur_ms: 100,
        }];
        let sv_before = signals_from_note_plan(&plan, 5);
        let sv_after = signals_from_note_plan(&plan, 30);
        assert_eq!(sv_before.audio_rms, 0);
        assert_eq!(sv_after.audio_rms, 0);
    }
}
