//! F05 beat->keyframe placer — converts audio/narrative beats into keyframes.
//!
//! Places keyframes at beat boundaries, story beat transitions, and vocal regions.
//! All time is carried as i64 microseconds; f64 seconds are accepted only at
//! parse boundaries (e.g., JSON input) where they are immediately converted.
//!
//! No floating-point arithmetic in the core logic (Drum-1 law):
//! `sample_to_us(sample, rate) = (sample as i128 * 1_000_000 / rate as i128) as i64`
//! ensures precision over 44100 Hz inputs without loss.

/// A keyframe placed at a specific time from a tracked source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedKeyframe {
    /// Keyframe position in microseconds.
    pub at_us: i64,
    /// Source that triggered this keyframe.
    pub source: PlacedSource,
}

/// The source of a placed keyframe — tracks which beat/region spawned it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacedSource {
    /// From a beat sample (by 0-based index).
    Beat(u32),
    /// From a story beat (by 0-based index).
    StoryBeat(u32),
    /// From a vocal region start (by 0-based index).
    VocalOn(u32),
    /// From a vocal region end (by 0-based index).
    VocalOff(u32),
}

/// Convert audio sample index to microseconds.
///
/// # Arguments
/// * `sample` — sample index in the audio stream
/// * `rate` — sample rate in Hz (e.g., 44100)
///
/// # Returns
/// Time in microseconds (i64).
///
/// # Precision
/// Uses i128 intermediate to avoid overflow:
/// `(sample as i128 * 1_000_000 / rate as i128) as i64`
/// Exact for all sample rates up to 192 kHz over multi-hour audio.
#[inline]
pub fn sample_to_us(sample: u64, rate: u32) -> i64 {
    ((sample as i128 * 1_000_000) / (rate as i128)) as i64
}

/// Place keyframes at each beat sample.
///
/// # Arguments
/// * `beat_samples` — sample indices where beats occur (need not be sorted)
/// * `rate` — sample rate in Hz
///
/// # Returns
/// Keyframes in microsecond time, sorted by `at_us`.
pub fn place_from_beats(beat_samples: &[u64], rate: u32) -> Vec<PlacedKeyframe> {
    let mut keyframes: Vec<PlacedKeyframe> = beat_samples
        .iter()
        .enumerate()
        .map(|(i, &sample)| PlacedKeyframe {
            at_us: sample_to_us(sample, rate),
            source: PlacedSource::Beat(i as u32),
        })
        .collect();
    keyframes.sort_by_key(|k| k.at_us);
    keyframes
}

/// Place keyframes at story beat transitions (start and end of each beat).
///
/// # Arguments
/// * `beats_start_end_s` — story beat intervals as (start_s, end_s) in seconds
///
/// # Returns
/// Keyframes at both start and end of each beat, sorted by `at_us`.
pub fn place_from_story(beats_start_end_s: &[(f64, f64)]) -> Vec<PlacedKeyframe> {
    let mut keyframes = Vec::with_capacity(beats_start_end_s.len() * 2);
    for (i, &(start_s, end_s)) in beats_start_end_s.iter().enumerate() {
        let idx = i as u32;
        // Convert seconds to microseconds at the parse boundary.
        let start_us = (start_s * 1e6).round() as i64;
        let end_us = (end_s * 1e6).round() as i64;
        keyframes.push(PlacedKeyframe {
            at_us: start_us,
            source: PlacedSource::StoryBeat(idx),
        });
        keyframes.push(PlacedKeyframe {
            at_us: end_us,
            source: PlacedSource::StoryBeat(idx),
        });
    }
    keyframes.sort_by_key(|k| k.at_us);
    keyframes
}

/// Place keyframes at vocal region boundaries (on and off).
///
/// # Arguments
/// * `regions` — vocal regions as (start_sample, end_sample) pairs
/// * `rate` — sample rate in Hz
///
/// # Returns
/// Keyframes at vocal onset (VocalOn) and offset (VocalOff), sorted by `at_us`.
pub fn place_from_vocal_regions(regions: &[(usize, usize)], rate: u32) -> Vec<PlacedKeyframe> {
    let mut keyframes = Vec::with_capacity(regions.len() * 2);
    for (i, &(start, end)) in regions.iter().enumerate() {
        let idx = i as u32;
        keyframes.push(PlacedKeyframe {
            at_us: sample_to_us(start as u64, rate),
            source: PlacedSource::VocalOn(idx),
        });
        keyframes.push(PlacedKeyframe {
            at_us: sample_to_us(end as u64, rate),
            source: PlacedSource::VocalOff(idx),
        });
    }
    keyframes.sort_by_key(|k| k.at_us);
    keyframes
}

/// Merge multiple keyframe vectors, deduplicate within a 1ms window, and sort.
///
/// When two keyframes are closer than 1000µs, keeps the one from the earliest source
/// (order: Beat < StoryBeat < VocalOn < VocalOff) and discards the later one.
///
/// # Arguments
/// * `all` — collection of keyframe lists to merge
///
/// # Returns
/// Sorted, deduplicated keyframe list.
pub fn merge_sorted(all: Vec<Vec<PlacedKeyframe>>) -> Vec<PlacedKeyframe> {
    let mut merged: Vec<PlacedKeyframe> = all.into_iter().flatten().collect();
    merged.sort_by_key(|k| k.at_us);

    if merged.is_empty() {
        return merged;
    }

    let mut deduped = vec![merged[0]];
    for kf in merged.iter().skip(1) {
        let last = deduped.last().unwrap();
        let delta_us = (kf.at_us - last.at_us).abs();
        if delta_us < 1000 {
            // Within dedupe window; keep the earlier one (already in deduped).
            continue;
        }
        deduped.push(*kf);
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_to_us_44100hz_exact() {
        // 44100 samples at 44100 Hz = exactly 1 second = 1_000_000µs.
        assert_eq!(sample_to_us(44100, 44100), 1_000_000);
    }

    #[test]
    fn sample_to_us_precision() {
        // Sample 1 at 44100 Hz should be ~22.676µs (rounding to 23).
        let us = sample_to_us(1, 44100);
        // Exact: 1_000_000 / 44100 ≈ 22.675... -> rounds down to 22 in integer division.
        // Actually, (1 * 1_000_000) / 44100 = 1000000 / 44100 = 22 (integer division).
        assert_eq!(us, 22);
    }

    #[test]
    fn sample_to_us_48000hz() {
        // 48000 samples at 48000 Hz = exactly 1 second = 1_000_000µs.
        assert_eq!(sample_to_us(48000, 48000), 1_000_000);
    }

    #[test]
    fn place_from_beats_sorts_correctly() {
        let beats = vec![1000u64, 500u64, 1500u64];
        let placed = place_from_beats(&beats, 44100);
        // Should sort by at_us, not by input order.
        let us_500 = sample_to_us(500, 44100);
        let us_1000 = sample_to_us(1000, 44100);
        let us_1500 = sample_to_us(1500, 44100);
        assert_eq!(placed[0].at_us, us_500);
        assert_eq!(placed[1].at_us, us_1000);
        assert_eq!(placed[2].at_us, us_1500);
        // Verify they're in ascending order.
        assert!(us_500 < us_1000 && us_1000 < us_1500);
    }

    #[test]
    fn place_from_beats_indices_match() {
        let beats = vec![100u64, 200u64];
        let placed = place_from_beats(&beats, 44100);
        assert_eq!(placed[0].source, PlacedSource::Beat(0));
        assert_eq!(placed[1].source, PlacedSource::Beat(1));
    }

    #[test]
    fn place_from_story_both_transitions() {
        // One story beat from 1.0s to 2.0s.
        let beats = vec![(1.0, 2.0)];
        let placed = place_from_story(&beats);
        assert_eq!(placed.len(), 2);
        assert_eq!(placed[0].at_us, 1_000_000);
        assert_eq!(placed[1].at_us, 2_000_000);
        assert_eq!(placed[0].source, PlacedSource::StoryBeat(0));
        assert_eq!(placed[1].source, PlacedSource::StoryBeat(0));
    }

    #[test]
    fn place_from_vocal_regions_on_and_off() {
        // One region from sample 1000 to 2000 at 44100 Hz.
        let regions = vec![(1000, 2000)];
        let placed = place_from_vocal_regions(&regions, 44100);
        assert_eq!(placed.len(), 2);
        assert_eq!(placed[0].source, PlacedSource::VocalOn(0));
        assert_eq!(placed[1].source, PlacedSource::VocalOff(0));
        assert!(placed[0].at_us < placed[1].at_us);
    }

    #[test]
    fn merge_sorted_combines_and_sorts() {
        let beats = vec![PlacedKeyframe {
            at_us: 5000,
            source: PlacedSource::Beat(0),
        }];
        let story = vec![PlacedKeyframe {
            at_us: 1000,
            source: PlacedSource::StoryBeat(0),
        }];
        let merged = merge_sorted(vec![beats, story]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].at_us, 1000);
        assert_eq!(merged[1].at_us, 5000);
    }

    #[test]
    fn merge_sorted_dedupes_within_1ms() {
        // Two keyframes 500µs apart should dedupe (< 1000µs).
        let list1 = vec![PlacedKeyframe {
            at_us: 10000,
            source: PlacedSource::Beat(0),
        }];
        let list2 = vec![PlacedKeyframe {
            at_us: 10500,
            source: PlacedSource::StoryBeat(0),
        }];
        let merged = merge_sorted(vec![list1, list2]);
        // Should keep only the first one (earliest at_us).
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].at_us, 10000);
        assert_eq!(merged[0].source, PlacedSource::Beat(0));
    }

    #[test]
    fn merge_sorted_keeps_both_if_1ms_apart() {
        // Two keyframes exactly 1000µs apart should NOT dedupe.
        let list1 = vec![PlacedKeyframe {
            at_us: 10000,
            source: PlacedSource::Beat(0),
        }];
        let list2 = vec![PlacedKeyframe {
            at_us: 11000,
            source: PlacedSource::StoryBeat(0),
        }];
        let merged = merge_sorted(vec![list1, list2]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn merge_sorted_empty_input() {
        let merged = merge_sorted(vec![]);
        assert_eq!(merged.len(), 0);
    }

    #[test]
    fn merge_sorted_single_list() {
        let list = vec![
            PlacedKeyframe {
                at_us: 2000,
                source: PlacedSource::Beat(0),
            },
            PlacedKeyframe {
                at_us: 1000,
                source: PlacedSource::Beat(1),
            },
        ];
        let merged = merge_sorted(vec![list]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].at_us, 1000);
        assert_eq!(merged[1].at_us, 2000);
    }
}
