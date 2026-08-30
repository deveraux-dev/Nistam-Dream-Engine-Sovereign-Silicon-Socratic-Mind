//! ASR segment -> Drop Law [`Frame`] beat merger.
//! The repeatable half of "transcript in, quiet deck out": takes raw
//! `{start_ms, end_ms, text}` segments (whisper's `--output_format json`
//! shape, read externally -- no JSON dep in this crate, see module doc on
//! why) and merges them into >=`min_duration_ms` beats, then buckets those
//! beats into a Kishotenketsu 60/30/10 Ki-Sho/Ten/Ketsu split by total
//! spoken duration -- the same three-act law [`crate::droplaw`] already
//! enforces for authored scripts, reused here rather than inventing a
//! second structure grammar for ASR-derived content (C06 revascularize).
//!
//! `[ASSUMED]` no per-beat `Stakes`/`Role` signal exists in raw ASR the way
//! it does in an authored Drop Law script, so every beat gets uniform
//! stakes (1.0) and a flat `Peak` role except the very first (`Establisher`)
//! and very last (`Release`) beat of the whole reel -- one arc, not one
//! per section, since a continuous monologue has no natural cut points a
//! merge-only pass can see. A future pass that scores stakes from words
//! per second or pause length would replace this, not this module's job
//! today (L15: name the blocker, don't silently fake precision).

use crate::droplaw::{CohnRole, Frame, FrameType, Transition};

/// One ASR segment as whisper's JSON emits it (caller parses the JSON;
/// this module only merges and buckets already-extracted segments).
#[derive(Debug, Clone)]
pub struct RawSegment {
    /// Segment start, milliseconds from reel start.
    pub start_ms: u32,
    /// Segment end, milliseconds from reel start.
    pub end_ms: u32,
    /// Spoken text for this segment.
    pub text: String,
}

/// A merged, section-assigned beat: a [`Frame`] (for reuse with
/// [`crate::droplaw::DropLawCompiler::analyze`]) plus the real wall-clock
/// span it covers, since `Frame::duration_x10_ms` alone can't reconstruct
/// an absolute reel position once beats have been merged out of order-1
/// dwell floors.
#[derive(Debug, Clone)]
pub struct Beat {
    /// The Drop Law frame -- text, section, role, duration.
    pub frame: Frame,
    /// Real transcript start, milliseconds.
    pub start_ms: u32,
    /// Real transcript end, milliseconds.
    pub end_ms: u32,
}

/// Merges consecutive segments until each beat's span is at least
/// `min_duration_ms`, then buckets beats into Ki-Sho/Ten/Ketsu by
/// cumulative duration (60/30/10) and builds `Frame`s. `fps` sets
/// `Frame::frames` the same way [`crate::droplaw`] does.
pub fn compile_beats(segments: &[RawSegment], min_duration_ms: u32, fps: u32) -> Vec<Beat> {
    let merged = merge(segments, min_duration_ms);
    let total_ms: u32 = merged.iter().map(|m| m.end_ms - m.start_ms).sum();
    let ki_sho_cutoff_ms = (total_ms as u64 * 60 / 100) as u32;
    let ten_cutoff_ms = (total_ms as u64 * 90 / 100) as u32;

    let mut beats = Vec::with_capacity(merged.len());
    let mut elapsed_ms = 0u32;
    for (i, m) in merged.iter().enumerate() {
        let section = if elapsed_ms < ki_sho_cutoff_ms {
            "Ki-Sho"
        } else if elapsed_ms < ten_cutoff_ms {
            "Ten"
        } else {
            "Ketsu"
        };
        let role = if i == 0 {
            CohnRole::Establisher
        } else if i == merged.len() - 1 {
            CohnRole::Release
        } else {
            CohnRole::Peak
        };
        let duration_ms = m.end_ms - m.start_ms;
        let duration_x10_ms = duration_ms * 10;
        let frames_count = ((duration_x10_ms as u64 * fps as u64) + 5_000) / 10_000;

        beats.push(Beat {
            frame: Frame {
                line_num: (i + 1) as u32,
                section: section.to_string(),
                frame_type: FrameType::Key,
                role,
                transition: Transition::Other("aspect_to_aspect".to_string()),
                description: m.text.trim().to_string(),
                dialogue: m.text.trim().to_string(),
                text: String::new(),
                duration_x10_ms,
                frames: frames_count as u32,
                stakes_x10: 10,
            },
            start_ms: m.start_ms,
            end_ms: m.end_ms,
        });
        elapsed_ms += duration_ms;
    }
    beats
}

struct MergedSpan {
    start_ms: u32,
    end_ms: u32,
    text: String,
}

fn merge(segments: &[RawSegment], min_duration_ms: u32) -> Vec<MergedSpan> {
    let mut out: Vec<MergedSpan> = Vec::new();
    let mut current: Option<MergedSpan> = None;

    for seg in segments {
        current = Some(match current.take() {
            None => MergedSpan {
                start_ms: seg.start_ms,
                end_ms: seg.end_ms,
                text: seg.text.trim().to_string(),
            },
            Some(mut span) => {
                span.end_ms = seg.end_ms;
                span.text.push(' ');
                span.text.push_str(seg.text.trim());
                span
            }
        });
        if current.as_ref().unwrap().end_ms - current.as_ref().unwrap().start_ms >= min_duration_ms {
            out.push(current.take().unwrap());
        }
    }
    if let Some(mut leftover) = current {
        // Trailing beat under the floor: fold into the previous beat
        // rather than ship a too-short final one.
        if let Some(last) = out.last_mut() {
            last.end_ms = leftover.end_ms;
            last.text.push(' ');
            last.text.push_str(leftover.text.trim());
        } else {
            leftover.text = leftover.text.trim().to_string();
            out.push(leftover);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start_ms: u32, end_ms: u32, text: &str) -> RawSegment {
        RawSegment { start_ms, end_ms, text: text.to_string() }
    }

    #[test]
    fn merges_short_segments_up_to_the_floor() {
        let segments = vec![
            seg(0, 3_000, "one"),
            seg(3_000, 6_000, "two"),
            seg(6_000, 9_000, "three"),
        ];
        let beats = compile_beats(&segments, 8_000, 24);
        // 0-3, 3-6 merge to 6s (< 8s floor), pull in 6-9 to reach 9s.
        assert_eq!(beats.len(), 1);
        assert_eq!(beats[0].start_ms, 0);
        assert_eq!(beats[0].end_ms, 9_000);
        assert_eq!(beats[0].frame.dialogue, "one two three");
    }

    #[test]
    fn first_beat_is_establisher_last_is_release() {
        let segments = vec![
            seg(0, 10_000, "a"),
            seg(10_000, 20_000, "b"),
            seg(20_000, 30_000, "c"),
        ];
        let beats = compile_beats(&segments, 8_000, 24);
        assert_eq!(beats.len(), 3);
        assert_eq!(beats[0].frame.role, CohnRole::Establisher);
        assert_eq!(beats[2].frame.role, CohnRole::Release);
        assert_eq!(beats[1].frame.role, CohnRole::Peak);
    }

    #[test]
    fn a_short_trailing_beat_folds_into_the_previous_one() {
        let segments = vec![
            seg(0, 9_000, "long enough"),
            seg(9_000, 10_500, "short tail"),
        ];
        let beats = compile_beats(&segments, 8_000, 24);
        assert_eq!(beats.len(), 1);
        assert_eq!(beats[0].end_ms, 10_500);
        assert!(beats[0].frame.dialogue.contains("short tail"));
    }
}
