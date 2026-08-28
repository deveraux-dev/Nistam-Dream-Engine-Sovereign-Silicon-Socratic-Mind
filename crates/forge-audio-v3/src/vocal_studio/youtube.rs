//! YouTube production bridge — VocalFrame stream → scene-map + duck trigger.
// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//!
//! Replaces youtube-forge's Python `scene_map.py` beat detection with the same
//! energy-envelope analysis that drives gameplay (`speech_clip`) and rhythm
//! (`rhythm_judge`). A single stream of VocalFrames produces:
//!
//! 1. Scene-map beats (onset-delimited segments for video editing)
//! 2. Duck triggers (frames where voice is active → music ducks)
//! 3. Emotional arc (for colour/accent selection in the renderer)
//!
//! Output: `SceneMap` struct serializable to the same `scene-map.json` format
//! that `render.py` consumes.

use crate::vocal_frame::{VocalFrame, Permyriad};

/// One beat/scene in the video timeline.
#[derive(Debug, Clone)]
pub struct SceneBeat {
    /// Beat index (0-based).
    pub beat: usize,
    /// Start time in 120 Hz ticks.
    pub start_tick: u64,
    /// End time in 120 Hz ticks.
    pub end_tick: u64,
    /// Duration in seconds.
    pub duration_secs: f32,
    /// Dominant emotion of this beat (index into valence/arousal/tension/release).
    pub mood: u8,
    /// Average loudness (Permyriad) — for emphasis detection.
    pub avg_rms_q: Permyriad,
}

/// The complete scene map produced from a VocalFrame stream.
#[derive(Debug, Clone)]
pub struct SceneMap {
    pub beats: Vec<SceneBeat>,
    /// Ticks where the duck trigger is active (voice present → music ducks).
    pub duck_active: Vec<(u64, u64)>, // (start_tick, end_tick) ranges
    /// Emotional arc: one mood value per beat for colour selection.
    pub emotional_arc: Vec<u8>,
}

/// Configuration for scene-map generation.
#[derive(Debug, Clone)]
pub struct SceneMapConfig {
    /// Minimum beat duration in ticks (120 Hz). Default: 240 (2 seconds).
    pub min_beat_ticks: u64,
    /// RMS threshold (Permyriad) for duck activation. Default: 300.
    pub duck_threshold_q: Permyriad,
    /// Minimum silence gap (ticks) to force a new beat. Default: 60 (0.5s).
    pub silence_gap_ticks: u64,
}

impl Default for SceneMapConfig {
    fn default() -> Self {
        Self {
            min_beat_ticks: 240,    // 2 seconds
            duck_threshold_q: 300,  // ~-30 dBFS
            silence_gap_ticks: 60,  // 0.5 seconds
        }
    }
}

/// Build a SceneMap from a stream of VocalFrames.
///
/// Each frame represents one 120 Hz tick. Onsets mark potential beat
/// boundaries; silence gaps force boundaries; the minimum beat duration
/// prevents over-segmentation.
pub fn build_scene_map(frames: &[VocalFrame], config: &SceneMapConfig) -> SceneMap {
    let mut beats = Vec::new();
    let mut duck_active: Vec<(u64, u64)> = Vec::new();

    if frames.is_empty() {
        return SceneMap { beats, duck_active, emotional_arc: vec![] };
    }

    // Pass 1: Find beat boundaries (onsets + silence gaps)
    let mut boundaries: Vec<u64> = vec![0];
    let mut silence_run: u64 = 0;
    let last_boundary = |b: &[u64]| *b.last().unwrap_or(&0);

    for (i, frame) in frames.iter().enumerate() {
        let tick = i as u64;

        if frame.rms_q < config.duck_threshold_q {
            silence_run += 1;
        } else {
            silence_run = 0;
        }

        // Beat boundary on onset (if far enough from last)
        if frame.onset && tick - last_boundary(&boundaries) >= config.min_beat_ticks {
            boundaries.push(tick);
        }
        // Or on silence gap
        else if silence_run == config.silence_gap_ticks
            && tick - last_boundary(&boundaries) >= config.min_beat_ticks
        {
            boundaries.push(tick - config.silence_gap_ticks); // boundary at silence start
        }
    }

    // Final boundary
    let total_ticks = frames.len() as u64;
    if *boundaries.last().unwrap_or(&0) < total_ticks {
        boundaries.push(total_ticks);
    }

    // Pass 2: Build beats from boundaries
    for w in boundaries.windows(2) {
        let start = w[0];
        let end = w[1];
        let segment = &frames[start as usize..end as usize];

        if segment.is_empty() {
            continue;
        }

        // Average RMS
        let avg_rms: i64 = segment.iter().map(|f| f.rms_q as i64).sum::<i64>()
            / segment.len() as i64;

        // Dominant emotion across the beat
        let mut emotion_sum = [0i64; 4];
        for f in segment {
            for (i, &e) in f.emotion.iter().enumerate() {
                emotion_sum[i] += e as i64;
            }
        }
        let n = segment.len() as i64;
        let emotion_avg: Vec<i64> = emotion_sum.iter().map(|s| s / n).collect();
        let mood = emotion_avg.iter().enumerate()
            .max_by_key(|(_, &v)| (v - 5000).abs())
            .map(|(i, _)| i as u8)
            .unwrap_or(0);

        beats.push(SceneBeat {
            beat: beats.len(),
            start_tick: start,
            end_tick: end,
            duration_secs: (end - start) as f32 / 120.0,
            mood,
            avg_rms_q: avg_rms as Permyriad,
        });
    }

    // Pass 3: Duck trigger ranges (contiguous active-voice regions)
    let mut duck_start: Option<u64> = None;
    for (i, frame) in frames.iter().enumerate() {
        let tick = i as u64;
        if frame.rms_q >= config.duck_threshold_q {
            if duck_start.is_none() {
                duck_start = Some(tick);
            }
        } else if let Some(start) = duck_start {
            duck_active.push((start, tick));
            duck_start = None;
        }
    }
    if let Some(start) = duck_start {
        duck_active.push((start, total_ticks));
    }

    let emotional_arc: Vec<u8> = beats.iter().map(|b| b.mood).collect();

    SceneMap { beats, duck_active, emotional_arc }
}

/// Serialize a SceneMap to JSON (compatible with youtube-forge render.py).
pub fn scene_map_to_json(map: &SceneMap) -> String {
    let mut json = String::from("[\n");
    for (i, beat) in map.beats.iter().enumerate() {
        let section = match beat.mood {
            0 => "BUILD",   // valence-dominant = constructive
            1 => "HOOK",    // arousal-dominant = exciting
            2 => "PROBLEM", // tension-dominant = conflict
            3 => "RESULT",  // release-dominant = resolution
            _ => "BUILD",
        };
        if i > 0 { json.push_str(",\n"); }
        json.push_str(&format!(
            "  {{\"beat\": {}, \"start\": {:.3}, \"end\": {:.3}, \"duration\": {:.3}, \"section\": \"{}\", \"energy\": {}}}",
            beat.beat,
            beat.start_tick as f64 / 120.0,
            beat.end_tick as f64 / 120.0,
            beat.duration_secs,
            section,
            beat.avg_rms_q,
        ));
    }
    json.push_str("\n]");
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_stream() {
        let map = build_scene_map(&[], &SceneMapConfig::default());
        assert!(map.beats.is_empty());
        assert!(map.duck_active.is_empty());
    }

    #[test]
    fn test_single_voiced_segment() {
        // 600 ticks (5 seconds) of voiced audio with one onset at tick 300
        let mut frames = vec![VocalFrame::SILENT; 600];
        for f in &mut frames {
            f.rms_q = 2000; // active voice
            f.f0_mhz = 440_000;
        }
        frames[0].onset = true;
        frames[300].onset = true;

        let map = build_scene_map(&frames, &SceneMapConfig::default());
        // Should have at least 2 beats (onset at 0 and 300, both > min_beat_ticks=240)
        assert!(map.beats.len() >= 2);
        assert!(!map.duck_active.is_empty());
    }
}
