//! Verified story-to-PNG timing rules (dwell/blink/transition/arc). Port of the
//! adversarially-verified technique report — ratios+ms only, no engine coupling.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-export\src\pacing.rs` (2026-08-15,
//! first slice of `source-compiler`'s five-gate ladder — `forge_export::pacing` owns
//! dwell/blink/Section/Beat/arc_check per that skill's "Owners" table). Logic
//! byte-identical to the donor; the only additions are per-item doc comments this
//! workspace's `missing_docs = "deny"` lint requires and the donor never carried.
//!
//! This is authoring-time export math (offline video pacing), not the 120Hz
//! deterministic tick — `f64` here is the same "wall-and-f64" carve-out
//! `forge-pp-lore-v3` already uses: no `SimTick`/`MilliUnit`/`Permyriad` in any
//! signature, because this is not replay-deterministic simulation state.

/// Frames per second the export pipeline renders at.
pub const FPS: u32 = 20;
/// Dwell floor for a motion/in-between frame, milliseconds.
pub const DWELL_MS_MOTION: u32 = 100;
/// Absolute minimum dwell for any frame, milliseconds.
pub const DWELL_MS_MIN: u32 = 300;
/// Dwell floor for a plot-critical memory frame, milliseconds.
pub const DWELL_MS_MEMORY: u32 = 500;
/// Attentional-blink dead zone `(lo, hi)` in milliseconds — a 2nd key frame must
/// never land in this gap after the 1st.
pub const BLINK_DEAD_MS: (u32, u32) = (200, 500);
/// McCloud's action-pacing histogram target (65/20/15 soft band, hand-tallied
/// from one Kirby sample). Only meaningful for action mode, not contemplative.
pub const TRANSITION_TARGET: [(&str, f64); 3] =
    [("action-to-action", 0.65), ("subject-to-subject", 0.20), ("scene-to-scene", 0.15)];

/// Which act of the story arc a beat belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    /// The opening draw — Establisher/Initial in Cohn's arc grammar.
    Hook,
    /// States the tension — also Establisher/Initial.
    Problem,
    /// Rising action — Peak in Cohn's arc grammar.
    Build,
    /// The payoff — also Peak.
    Result,
    /// The close — Release in Cohn's arc grammar.
    Cta,
}

/// One timed beat in the export track.
#[derive(Clone, Debug)]
pub struct Beat {
    /// Ordinal position in the beat sequence.
    pub beat: u32,
    /// Start time, seconds.
    pub start_s: f64,
    /// End time, seconds.
    pub end_s: f64,
    /// Duration, seconds.
    pub duration_s: f64,
    /// Which arc section this beat belongs to.
    pub section: Section,
}

/// McCloud's panel-transition taxonomy, the three kinds this pacing model scores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    /// Same section, adjacent beat.
    ActionToAction,
    /// Same section, non-adjacent beat.
    SubjectToSubject,
    /// A change of section.
    SceneToScene,
}

/// Frames to hold a distinct image so it survives to the next one. `kind` selects
/// the floor: motion/in-between (100ms) vs plot-critical memory (500ms, 300ms
/// absolute min).
pub fn dwell_frames(ms: u32, fps: u32) -> u32 {
    ((ms as f64 / 1000.0) * fps as f64).round().max(1.0) as u32
}

/// Attentional-blink guard: a 2nd key frame must never land 200-500ms after the 1st.
pub fn blink_safe(gap_ms: f64) -> bool {
    let (lo, hi) = BLINK_DEAD_MS;
    !(gap_ms > lo as f64 && gap_ms < hi as f64)
}

/// McCloud-style pair classifier: same section + adjacent beat = action-to-action,
/// same section non-adjacent = subject-to-subject, section change = scene-to-scene.
pub fn classify_transition(prev: Option<&Beat>, cur: &Beat) -> Transition {
    match prev {
        None => Transition::SceneToScene,
        Some(p) if p.section != cur.section => Transition::SceneToScene,
        Some(p) if cur.beat.saturating_sub(p.beat) == 1 => Transition::ActionToAction,
        Some(_) => Transition::SubjectToSubject,
    }
}

/// Fraction of each transition type across a beat sequence.
pub fn transition_histogram(beats: &[Beat]) -> [(Transition, f64); 3] {
    let mut counts = [0u32; 3];
    let mut prev: Option<&Beat> = None;
    for b in beats {
        let idx = match classify_transition(prev, b) {
            Transition::ActionToAction => 0,
            Transition::SubjectToSubject => 1,
            Transition::SceneToScene => 2,
        };
        counts[idx] += 1;
        prev = Some(b);
    }
    let total = counts.iter().sum::<u32>().max(1) as f64;
    [
        (Transition::ActionToAction, counts[0] as f64 / total),
        (Transition::SubjectToSubject, counts[1] as f64 / total),
        (Transition::SceneToScene, counts[2] as f64 / total),
    ]
}

/// Cohn arc grammar: a Peak must never open an arc, an Initial must never close one.
/// HOOK/PROBLEM read as Establisher/Initial, BUILD/RESULT as Peak, CTA as Release.
pub fn arc_check(beats: &[Beat]) -> Vec<&'static str> {
    let mut warn = Vec::new();
    let is_peak = |s: Section| matches!(s, Section::Build | Section::Result);
    let is_initial = |s: Section| matches!(s, Section::Problem);
    if let Some(first) = beats.first() {
        if is_peak(first.section) {
            warn.push("Peak-at-arc-start");
        }
    }
    if let Some(last) = beats.last() {
        if is_initial(last.section) {
            warn.push("Initial-at-arc-end");
        }
    }
    warn
}

/// Compare a transition histogram against `TRANSITION_TARGET` with McCloud's
/// stated soft-band tolerance (±10 percentage points — "plus or minus 5-10
/// points; it is McCloud's hand tally of one Kirby sample, generalized").
/// Only meaningful for action-paced content; contemplative mode intentionally
/// injects moment-to-moment/aspect-to-aspect runs this band does not cover.
pub fn histogram_verdict(hist: &[(Transition, f64); 3]) -> Vec<String> {
    const TOLERANCE: f64 = 0.10;
    hist.iter()
        .map(|(t, frac)| {
            let key = match t {
                Transition::ActionToAction => "action-to-action",
                Transition::SubjectToSubject => "subject-to-subject",
                Transition::SceneToScene => "scene-to-scene",
            };
            let target = TRANSITION_TARGET.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).unwrap();
            let ok = (frac - target).abs() <= TOLERANCE;
            format!("{key}: {:.0}% (target {:.0}% ±10) {}", frac * 100.0, target * 100.0, if ok { "PASS" } else { "WARN" })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beat(n: u32, section: Section) -> Beat {
        Beat { beat: n, start_s: 0.0, end_s: 0.0, duration_s: 0.0, section }
    }

    #[test]
    fn dwell_frames_hits_the_documented_floors() {
        assert_eq!(dwell_frames(DWELL_MS_MOTION, FPS), 2);
        assert_eq!(dwell_frames(DWELL_MS_MIN, FPS), 6);
        assert_eq!(dwell_frames(DWELL_MS_MEMORY, FPS), 10);
    }

    #[test]
    fn blink_dead_zone_rejects_200_to_500ms() {
        assert!(blink_safe(199.0));
        assert!(!blink_safe(200.001));
        assert!(!blink_safe(499.999));
        assert!(blink_safe(500.0));
        assert!(blink_safe(0.0)); // lag-1 chain is legal
    }

    #[test]
    fn classify_transition_matches_mccloud_rules() {
        let a = beat(1, Section::Hook);
        let b = beat(2, Section::Hook);
        let c = beat(5, Section::Hook);
        let d = beat(6, Section::Build);
        assert_eq!(classify_transition(None, &a), Transition::SceneToScene);
        assert_eq!(classify_transition(Some(&a), &b), Transition::ActionToAction);
        assert_eq!(classify_transition(Some(&b), &c), Transition::SubjectToSubject);
        assert_eq!(classify_transition(Some(&c), &d), Transition::SceneToScene);
    }

    #[test]
    fn transition_histogram_sums_to_one() {
        let beats: Vec<Beat> = (1..=10).map(|i| beat(i, Section::Hook)).collect();
        let hist = transition_histogram(&beats);
        let total: f64 = hist.iter().map(|(_, f)| f).sum();
        assert!((total - 1.0).abs() < 1e-9);
    }

    #[test]
    fn arc_check_flags_peak_first_and_initial_last() {
        let beats = vec![beat(1, Section::Build), beat(2, Section::Problem)];
        let warn = arc_check(&beats);
        assert_eq!(warn, vec!["Peak-at-arc-start", "Initial-at-arc-end"]);
    }

    #[test]
    fn arc_check_clean_hook_to_cta_has_no_warnings() {
        let beats = vec![beat(1, Section::Hook), beat(2, Section::Build), beat(3, Section::Cta)];
        assert!(arc_check(&beats).is_empty());
    }

    #[test]
    fn histogram_verdict_passes_when_on_target() {
        let hist = [(Transition::ActionToAction, 0.65), (Transition::SubjectToSubject, 0.20), (Transition::SceneToScene, 0.15)];
        let lines = histogram_verdict(&hist);
        assert!(lines.iter().all(|l| l.ends_with("PASS")), "{lines:?}");
    }

    #[test]
    fn histogram_verdict_warns_outside_the_tolerance_band() {
        let hist = [(Transition::ActionToAction, 0.0), (Transition::SubjectToSubject, 0.0), (Transition::SceneToScene, 1.0)];
        let lines = histogram_verdict(&hist);
        assert!(lines[0].ends_with("WARN"), "{lines:?}"); // 0% vs 65% target, action-to-action
        assert!(lines[2].ends_with("WARN"), "{lines:?}"); // 100% vs 15% target, scene-to-scene
    }
}
