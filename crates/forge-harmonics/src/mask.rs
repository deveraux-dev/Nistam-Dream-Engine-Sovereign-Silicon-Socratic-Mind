// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Zwicker spreading-function masking over the 7-band spectrum, Permyriad.
//! A masker raises its neighbours' thresholds asymmetrically: steeply toward
//! lower frequencies, shallowly toward higher, so masking spreads upward.

/// Psychoacoustic masking profile: spreading slopes and absolute floor.
///
/// All values in Permyriad (0..=10000). Slopes express as integer ratios
/// per neighbouring band — e.g. `lower_slope_ratio=5` means 5% of the
/// masker's energy sets the threshold one band downward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskProfile {
    /// Absolute threshold of hearing floor (ISO 226 in spirit).
    /// Bands below this are zeroed regardless of neighbours.
    pub absolute_floor: u16,

    /// Lower-slope multiplier (Permyriad %) per band step towards lower freq.
    /// E.g. 5 → threshold at band i-1 is 5% of band i masker energy.
    pub lower_slope_ratio_percent: u16,

    /// Upper-slope multiplier (Permyriad %) per band step towards higher freq.
    /// E.g. 30 → threshold at band i+1 is 30% of band i masker energy.
    pub upper_slope_ratio_percent: u16,
}

impl MaskProfile {
    /// Zwicker spreading function default (published psychoacoustic data).
    ///
    /// Based on Zwicker & Fastl, "Psychoacoustics" (Springer, 1999):
    /// - Lower slope: ~27 dB/Bark downward → ~5% (linear 10^(-27/20)).
    /// - Upper slope: ~10 dB/Bark upward → ~30% (linear 10^(-10/20)).
    /// - Absolute floor: ISO 226 ~4 Phons ≈ 50 Permyriad (0.5% full scale).
    pub fn zwicker() -> Self {
        MaskProfile {
            absolute_floor: 50,
            lower_slope_ratio_percent: 5,
            upper_slope_ratio_percent: 30,
        }
    }
}

/// Compute masking thresholds for each band given a profile and current bands.
///
/// Returns thresholds (in Permyriad) below which each band should be zeroed.
/// Thresholds are raised by maskers in neighbouring bands and enforced by
/// the absolute floor.
pub fn mask_thresholds(bands: &[u16; 7], profile: &MaskProfile) -> [u16; 7] {
    let mut thresholds = [profile.absolute_floor; 7];

    for (i, &energy) in bands.iter().enumerate() {
        if energy == 0 {
            continue;
        }

        let lower = (energy as u32 * profile.lower_slope_ratio_percent as u32) / 100;
        let lower_clamped = (lower.min(10000)) as u16;

        let upper = (energy as u32 * profile.upper_slope_ratio_percent as u32) / 100;
        let upper_clamped = (upper.min(10000)) as u16;

        if i > 0 {
            thresholds[i - 1] = thresholds[i - 1].max(lower_clamped);
        }
        if i < 6 {
            thresholds[i + 1] = thresholds[i + 1].max(upper_clamped);
        }
    }

    thresholds
}

/// Apply masking profile to spectrum bands in-place.
///
/// Any band at or below its computed masking threshold is zeroed.
/// **Critical**: masked output will sum to less than 10000 — this is correct
/// and intended (we remove inaudible energy, not resurrect it).
pub fn apply_mask(bands: &mut [u16; 7], profile: &MaskProfile) {
    let thresholds = mask_thresholds(bands, profile);
    for (band, &threshold) in bands.iter_mut().zip(thresholds.iter()) {
        if *band <= threshold {
            *band = 0;
        }
    }
}

/// Check whether band at `index` is masked below its threshold.
///
/// Returns `false` (not masked) if index >= 7 — a defined, safe result
/// that avoids panic on out-of-range access.
pub fn is_masked(bands: &[u16; 7], index: usize, profile: &MaskProfile) -> bool {
    if index >= 7 {
        return false;
    }
    let thresholds = mask_thresholds(bands, profile);
    bands[index] <= thresholds[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_loud_band_masks_quiet_neighbours() {
        let profile = MaskProfile::zwicker();
        let mut bands = [0u16, 0, 200, 5000, 200, 0, 0];

        apply_mask(&mut bands, &profile);

        assert_eq!(bands[3], 5000, "masker itself stays intact");
        assert_eq!(bands[2], 0, "band below the masker, via the steep lower slope");
        assert_eq!(bands[4], 0, "band above the masker, via the shallow upper slope");
    }

    #[test]
    fn a_quiet_band_above_the_masker_is_masked_while_the_same_band_below_survives() {
        let profile = MaskProfile::zwicker();
        let quiet = 400u16;

        let mut above = [0u16, 0, 0, 5000, quiet, 0, 0];
        let mut below = [0u16, 0, quiet, 5000, 0, 0, 0];
        apply_mask(&mut above, &profile);
        apply_mask(&mut below, &profile);

        assert_eq!(above[4], 0, "upper slope is shallow: {quiet} falls under 5000*30%");
        assert_eq!(below[2], quiet, "lower slope is steep: {quiet} clears 5000*5%");
    }

    #[test]
    fn two_adjacent_equal_bands_mask_neither() {
        let profile = MaskProfile::zwicker();
        let bands = [0u16, 0, 1000, 1000, 0, 0, 0];

        let thresholds = mask_thresholds(&bands, &profile);

        assert!(
            bands[2] > thresholds[2],
            "band 2 should not be masked (equal neighbour)"
        );
        assert!(
            bands[3] > thresholds[3],
            "band 3 should not be masked (equal neighbour)"
        );
    }

    #[test]
    fn all_zero_stays_zero() {
        let profile = MaskProfile::zwicker();
        let mut bands = [0u16; 7];

        apply_mask(&mut bands, &profile);

        assert_eq!(bands, [0u16; 7]);
    }

    #[test]
    fn all_equal_input_unchanged_by_spreading() {
        let profile = MaskProfile::zwicker();
        let original = [1000u16; 7];
        let mut bands = original;

        apply_mask(&mut bands, &profile);

        assert_eq!(
            bands, original,
            "uniform energy clears its own spread thresholds; only the floor may act"
        );
    }

    #[test]
    fn no_renormalize_masked_sum_less_than_10000() {
        let profile = MaskProfile::zwicker();
        let original = [0u16, 0, 100, 9000, 100, 0, 0];
        let original_sum: u32 = original.iter().map(|&b| b as u32).sum();
        let mut bands = original;

        apply_mask(&mut bands, &profile);

        let masked_sum: u32 = bands.iter().map(|&b| b as u32).sum();

        assert_eq!(
            original_sum, 9200,
            "original sum is 9200 (sanity check)"
        );
        assert!(
            masked_sum < original_sum,
            "masked sum {} is strictly less than original {}",
            masked_sum,
            original_sum
        );
    }

    #[test]
    fn is_masked_out_of_range_returns_false_no_panic() {
        let profile = MaskProfile::zwicker();
        let bands = [1000u16; 7];

        let result_7 = is_masked(&bands, 7, &profile);
        let result_99 = is_masked(&bands, 99, &profile);

        assert!(!result_7);
        assert!(!result_99);
    }
}
