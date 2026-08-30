//! F06 droplaw adapter — turns timestamped cuts into Drop Law frames.
//!
//! Converts a list of timed cuts (microsecond timestamps paired with frame types)
//! into Drop Law `Frame` structs ready for Drop Law analysis. Each cut's duration
//! is determined by the interval to the next cut; the final cut holds its type's
//! dwell floor.
//!
//! Time is carried as i64 microseconds throughout; conversion to tenths-of-a-millisecond
//! uses integer math only (delta_us / 100).

use crate::droplaw::{Analysis, DropLawCompiler, Frame, FrameType, Transition, CohnRole};

/// Convert a sorted list of cuts into Drop Law frames.
///
/// # Arguments
/// * `cuts` — sorted by `at_us`, each `(at_us, frame_type)` pair
/// * `fps` — frames per second for the Drop Law compiler
///
/// # Returns
/// A vector of `Frame` structs ready for Drop Law analysis.
///
/// # Panics
/// Panics if the input is empty.
pub fn frames_from_cuts(cuts: &[(i64, FrameType)], fps: u32) -> Vec<Frame> {
    assert!(!cuts.is_empty(), "frames_from_cuts requires at least one cut");

    let mut frames = Vec::with_capacity(cuts.len());

    for (i, (at_us, frame_type)) in cuts.iter().enumerate() {
        let duration_us = if i < cuts.len() - 1 {
            // Duration = next timestamp - this timestamp
            cuts[i + 1].0 - at_us
        } else {
            // Last cut: use the dwell floor for its frame type
            frame_type.dwell_x10_ms() as i64 * 100
        };

        // Convert microseconds to tenths-of-a-millisecond (integer math only)
        let duration_x10_ms = (duration_us / 100) as u32;

        // Deduce role per droplaw's frame-type defaults
        let role = match frame_type {
            FrameType::Establish => CohnRole::Establisher,
            FrameType::Dialogue | FrameType::Motion => CohnRole::Initial,
            FrameType::Pillow => CohnRole::Release,
            FrameType::Key => CohnRole::Peak,
        };

        // Round: dur_ms * fps / 1000 done in tenths: round(dur_x10*fps/10000)
        let frames_count = round_div(duration_x10_ms as u64 * fps as u64, 10_000);

        frames.push(Frame {
            line_num: (i + 1) as u32,
            section: "Cuts".to_string(),
            frame_type: *frame_type,
            role,
            transition: Transition::ActionToAction,
            description: String::new(),
            dialogue: String::new(),
            text: String::new(),
            duration_x10_ms,
            frames: frames_count as u32,
            stakes_x10: 10,
        });
    }

    frames
}

/// Analyze a cut list for Drop Law pacing violations.
///
/// # Arguments
/// * `cuts` — sorted by `at_us`
/// * `fps` — frames per second for the compiler
///
/// # Returns
/// The full Drop Law `Analysis` report including critical hazards and warnings.
pub fn analyze_cuts(cuts: &[(i64, FrameType)], fps: u32) -> Analysis {
    let frames = frames_from_cuts(cuts, fps);
    let compiler = DropLawCompiler::from_frames(fps, frames);
    compiler.analyze()
}

/// Helper: round division (a + b/2) / b.
fn round_div(numer: u64, denom: u64) -> u64 {
    (numer + denom / 2) / denom
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droplaw::attention_trit;

    #[test]
    fn blink_violation_with_tight_key_spacing() {
        // Establish, then Key 300ms later (violates blink window of 200-500ms).
        // Cuts at 0µs and 300_000µs (300ms).
        let cuts = vec![
            (0i64, FrameType::Establish),
            (300_000i64, FrameType::Key),
        ];
        let analysis = analyze_cuts(&cuts, 24);
        // Should have a critical mentioning the blink hazard.
        assert!(!analysis.criticals.is_empty(), "Expected a critical for tight blink spacing");
        assert!(analysis.criticals[0].contains("Attentional Blink") || analysis.criticals[0].contains("Blink Hazard"),
                "Critical should mention blink: {:?}", analysis.criticals[0]);
        // attention_trit should be +1 (supercritical) when there's a hazard.
        assert_eq!(attention_trit(&analysis), 1);
    }

    #[test]
    fn clean_schedule_no_blink_violation() {
        // Three frames: Establish, Key (600ms apart), Pillow.
        // Gap of 600ms is safe (outside 200-500ms blink window, which is 200-500ms).
        let cuts = vec![
            (0i64, FrameType::Establish),
            (600_000i64, FrameType::Key),
            (1_200_000i64, FrameType::Pillow),
        ];
        let analysis = analyze_cuts(&cuts, 24);
        // Should not have a blink-related critical.
        let has_blink_critical = analysis.criticals
            .iter()
            .any(|c| c.contains("Attentional Blink") || c.contains("Blink Hazard"));
        assert!(!has_blink_critical, "Clean schedule should not trigger blink hazard");
        // attention_trit may be 0 (soft warnings) or -1 (no issues), but never +1
        // (supercritical hazard) for a properly-spaced schedule.
        let trit = attention_trit(&analysis);
        assert!(trit <= 0, "Safe spacing should not trigger supercritical (trit=+1), got {}", trit);
    }

    #[test]
    fn frames_duration_computed_from_cut_intervals() {
        // Cuts at 0µs, 1_000_000µs (1000ms), 2_500_000µs (2500ms).
        let cuts = vec![
            (0i64, FrameType::Dialogue),
            (1_000_000i64, FrameType::Key),
            (2_500_000i64, FrameType::Motion),
        ];
        let frames = frames_from_cuts(&cuts, 24);
        assert_eq!(frames.len(), 3);
        // First frame: duration 1000ms (1_000_000µs / 100 = 10_000 x10_ms).
        assert_eq!(frames[0].duration_x10_ms, 10_000);
        // Second frame: duration 1500ms (1_500_000µs / 100 = 15_000 x10_ms).
        assert_eq!(frames[1].duration_x10_ms, 15_000);
        // Third frame: duration is Motion's dwell floor (100ms = 1_000 x10_ms).
        assert_eq!(frames[2].duration_x10_ms, 1_000);
    }

    #[test]
    fn roles_deduced_per_frame_type() {
        let cuts = vec![
            (0i64, FrameType::Establish),
            (500_000i64, FrameType::Dialogue),
            (1_000_000i64, FrameType::Pillow),
            (1_500_000i64, FrameType::Key),
            (2_000_000i64, FrameType::Motion),
        ];
        let frames = frames_from_cuts(&cuts, 24);
        assert_eq!(frames[0].role, CohnRole::Establisher);
        assert_eq!(frames[1].role, CohnRole::Initial);
        assert_eq!(frames[2].role, CohnRole::Release);
        assert_eq!(frames[3].role, CohnRole::Peak);
        assert_eq!(frames[4].role, CohnRole::Initial);
    }
}
