// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! ERB-spaced auditory filterbank (Glasberg-Moore, 1990).
//!
//! Replacement for octave-band partitioning: maps pitch to 7 bands equally
//! spaced on the auditory (ERB-rate) scale, where hearing actually resolves.
//! Integer-only; all computation in millihertz, zero floating-point.

/// Number of ERB-spaced frequency bands.
pub const ERB_BAND_COUNT: usize = 7;

/// ERB-band centre frequencies (millihertz).
/// Precomputed: 7 centres equally spaced on the ERB-rate (ERBS) scale
/// across MIDI pitch range 8,175..12,543,850 mHz. ERB-rate equals
/// 21.4*log10(1 + 4.37*f_kHz), defining the auditory frequency scale.
/// Centres are at midpoints of equal ERBS intervals, inverted to Hz.
const ERB_BAND_CENTRES_MHZ: [u32; 7] = [
    86_264,
    328_100,
    755_547,
    1_511_060,
    2_846_432,
    5_206_703,
    9_378_486,
];

/// ERB-band upper edges (millihertz).
/// Boundaries between adjacent bands on the ERB-rate scale, inverted to Hz.
/// Six edges partition seven bands; first and last bands are open.
/// Band 0 edge at 190 Hz keeps low-frequency bulk from swallowing the partition.
const ERB_BAND_EDGES_MHZ: [u32; 6] = [
    190_080,
    511_595,
    1_079_875,
    2_084_310,
    3_859_652,
    6_997_572,
];

/// Partition mHz into its ERB-spaced band (0..ERB_BAND_COUNT-1).
///
/// Total partition: every u32 lands in exactly one band.
/// Band 0: [0, edges[0])
/// Band i (0 < i < 6): [edges[i-1], edges[i])
/// Band 6: [edges[5], ∞)
///
/// No panics, no unchecked indexing.
#[inline]
pub fn erb_band_index(mhz: u32) -> usize {
    let mut i = 0;
    while i < ERB_BAND_EDGES_MHZ.len() && mhz >= ERB_BAND_EDGES_MHZ[i] {
        i += 1;
    }
    i
}

/// Centres of each ERB band in millihertz.
#[inline]
pub const fn erb_band_centres_mhz() -> [u32; ERB_BAND_COUNT] {
    ERB_BAND_CENTRES_MHZ
}

/// Upper edges of bands 0..5 in millihertz; bands 0 and 6 are open.
#[inline]
pub const fn erb_band_edges_mhz() -> [u32; ERB_BAND_COUNT - 1] {
    ERB_BAND_EDGES_MHZ
}

/// Lowest and highest pitch `note_to_mhz` can produce (MIDI 0 and 127).
pub const PITCH_SPAN_MHZ: (u32, u32) = (8_175, 12_543_850);

/// Glasberg-Moore ERB bandwidth at a given centre frequency (millihertz).
///
/// Formula: ERB_mHz = (247 * (437 * f_mHz + 100_000_000)) / 1_000_000
/// At 1 kHz: 132.639 Hz (reference 132.64 Hz, error < 0.1 Hz).
#[inline]
pub const fn erb_bandwidth_mhz(centre_mhz: u32) -> u32 {
    let numerator = 247u64 * (437u64 * (centre_mhz as u64) + 100_000_000u64);
    (numerator / 1_000_000u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale_voice::note_to_mhz;

    #[test]
    fn midi_range_total_partition_coverage() {
        for note in 0u8..=127 {
            let mhz = note_to_mhz(note);
            let band = erb_band_index(mhz);
            assert!(
                band < ERB_BAND_COUNT,
                "MIDI note {} ({}mHz) band {} out of range",
                note,
                mhz,
                band
            );
        }
    }

    #[test]
    fn boundary_values_land_in_partition() {
        let test_values = [0u32, 1u32, u32::MAX];
        for mhz in test_values {
            let band = erb_band_index(mhz);
            assert!(band < ERB_BAND_COUNT, "value {} landed in band {}", mhz, band);
        }
    }

    #[test]
    fn band_index_monotonic_non_decreasing() {
        let mut last_band = 0usize;
        for freq in (0u32..=12_544_000).step_by(1000) {
            let band = erb_band_index(freq);
            assert!(
                band >= last_band,
                "monotonicity violated at {}mHz: band {} < previous {}",
                freq,
                band,
                last_band
            );
            last_band = band;
        }
    }

    #[test]
    fn all_seven_bands_reachable_from_midi() {
        let mut bands_seen = [false; 7];
        for note in 0u8..=127 {
            let mhz = note_to_mhz(note);
            let band = erb_band_index(mhz);
            bands_seen[band] = true;
        }
        for (band, seen) in bands_seen.iter().enumerate() {
            assert!(*seen, "band {} unreachable from MIDI 0..127", band);
        }
    }

    #[test]
    fn erb_bandwidth_monotone_increasing() {
        let centres = erb_band_centres_mhz();
        for i in 1..centres.len() {
            let bw_prev = erb_bandwidth_mhz(centres[i - 1]);
            let bw_curr = erb_bandwidth_mhz(centres[i]);
            assert!(
                bw_curr > bw_prev,
                "bandwidth not monotone: band {} ({}mHz, bw {}mHz) <= band {} ({}mHz, bw {}mHz)",
                i - 1,
                centres[i - 1],
                bw_prev,
                i,
                centres[i],
                bw_curr
            );
        }
    }

    #[test]
    fn erb_bandwidth_1khz_matches_glasberg_moore() {
        let bw = erb_bandwidth_mhz(1_000_000);
        let bw_hz = bw as f64 / 1000.0;
        const REFERENCE_HZ: f64 = 132.64;
        const TOLERANCE_HZ: f64 = 0.5;
        assert!(
            (bw_hz - REFERENCE_HZ).abs() < TOLERANCE_HZ,
            "1 kHz ERB bandwidth {} Hz outside tolerance (ref {} Hz)",
            bw_hz,
            REFERENCE_HZ
        );
    }

    #[test]
    fn band_centres_ordered() {
        let centres = erb_band_centres_mhz();
        for i in 1..centres.len() {
            assert!(
                centres[i] > centres[i - 1],
                "band centres not ordered: {} >= {}",
                centres[i - 1],
                centres[i]
            );
        }
    }

    #[test]
    fn band_edges_ordered() {
        for i in 1..ERB_BAND_EDGES_MHZ.len() {
            assert!(
                ERB_BAND_EDGES_MHZ[i] > ERB_BAND_EDGES_MHZ[i - 1],
                "band edges not ordered at index {}",
                i
            );
        }
    }

    #[test]
    fn band_edges_interleave_centres() {
        let centres = erb_band_centres_mhz();
        assert!(centres[0] < ERB_BAND_EDGES_MHZ[0]);
        for i in 1..ERB_BAND_EDGES_MHZ.len() {
            assert!(
                centres[i] < ERB_BAND_EDGES_MHZ[i],
                "centre[{}] ({}) not < edge[{}] ({})",
                i,
                centres[i],
                i,
                ERB_BAND_EDGES_MHZ[i]
            );
            assert!(
                centres[i] > ERB_BAND_EDGES_MHZ[i - 1],
                "centre[{}] ({}) not > edge[{}] ({})",
                i,
                centres[i],
                i - 1,
                ERB_BAND_EDGES_MHZ[i - 1]
            );
        }
        assert!(centres[6] > ERB_BAND_EDGES_MHZ[5]);
    }

    #[test]
    fn erb_centres_are_not_linearly_spaced_in_hz() {
        let centres = erb_band_centres_mhz();
        let mut diffs = Vec::new();
        for i in 1..centres.len() {
            diffs.push(centres[i] - centres[i - 1]);
        }
        let first_diff = diffs[0];
        let mut all_equal = true;
        for &d in &diffs {
            if d != first_diff {
                all_equal = false;
                break;
            }
        }
        assert!(
            !all_equal,
            "band centres are linearly spaced in Hz (diffs: {:?}), not ERB-rate spaced",
            diffs
        );
    }

    #[test]
    fn erb_centres_equally_spaced_on_erb_rate_scale() {
        let centres = erb_band_centres_mhz();
        let mut erbs_gaps = Vec::new();
        for i in 1..centres.len() {
            let bw_curr = erb_bandwidth_mhz(centres[i]) as f64;
            let hz_diff = (centres[i] - centres[i - 1]) as f64;
            let erbs_gap = hz_diff / bw_curr.max(1.0);
            erbs_gaps.push(erbs_gap);
        }
        let mean_gap = erbs_gaps.iter().sum::<f64>() / erbs_gaps.len() as f64;
        const TOLERANCE_PERCENT: f64 = 10.0;
        for (i, &gap) in erbs_gaps.iter().enumerate() {
            let error_pct = 100.0 * (gap - mean_gap).abs() / mean_gap;
            assert!(
                error_pct < TOLERANCE_PERCENT,
                "ERB gap {}: {:.2} ERBs, mean {:.2} ERBs, error {:.1}%",
                i,
                gap,
                mean_gap,
                error_pct
            );
        }
    }

    #[test]
    fn band_0_edge_below_500hz() {
        const EDGE_500HZ_MHZ: u32 = 500_000;
        assert!(
            ERB_BAND_EDGES_MHZ[0] < EDGE_500HZ_MHZ,
            "band 0 upper edge {} mHz = {} Hz must be below 500 Hz to preserve MIDI pitch distribution",
            ERB_BAND_EDGES_MHZ[0],
            ERB_BAND_EDGES_MHZ[0] / 1000
        );
    }
}
