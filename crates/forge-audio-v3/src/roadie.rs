// @forge:allow_float — audio metering/perceptual DSP leaf; f32 is the correct type here.
//! RoadieBot — always-on perceptual sound-quality auditor.
//!
//! Consumes `MeterData` snapshots (pure analysis, no audio thread access) and
//! emits diagnostic events: clip / phase-cancel / mud / harsh / thin / pumping.
//! Advisory only — never issues `MixerCommand`s or touches the RT path.
//!
//! Severity tiers:
//!   Critical — immediate (clipping, phase inversion)
//!   Warning  — sustained 10 s (mud, harsh, thin, pumping, low phase)
//!   Info     — on-request only
//!
//! Cooldown: same diagnosis max 1× per 30 s.  3 dismissals → muted for session.
//! Chaos mode: `vibe_state > 0.8` raises the clip threshold to 0.995.
//!
//! Ported from `dead-drop/dead-drop-engine/src/roadie.rs` (quarry 2026-06-07).
//! Changes from quarry: `MeterData` defined here (was `crate::MeterData`);
//! `deck_playing` is `[bool; 4]` (no heap alloc) instead of `Vec<bool>`.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

const MAX_EVENTS: usize = 500;
const COOLDOWN_SECS: u64 = 30;
const SUSTAINED_SECS: u64 = 10;
const DISMISSAL_MUTE: u32 = 3;

const CLIP_THRESHOLD: f32 = 0.98;
const CLIP_THRESHOLD_CHAOS: f32 = 0.995;
const PHASE_CRITICAL: f32 = 0.0;
const PHASE_WARNING: f32 = 0.3;
const FREQ_IMBALANCE_RATIO: f32 = 2.0;
const THIN_LOW_FLOOR: f32 = 0.01;
const THIN_MID_FLOOR: f32 = 0.1;
const PUMPING_VARIANCE_RATIO: f32 = 0.1;
const RMS_HISTORY_LEN: usize = 60;

// ── MeterData ────────────────────────────────────────────────────────────────

/// Per-frame metering snapshot consumed by `RoadieBot` and the telemetry seam.
///
/// Written by `mix_block`'s metering path; read on the analysis thread and by
/// the telemetry WS. Not on the RT hot-path — cloned/sent cold.
#[derive(Debug, Clone, Default)]
pub struct MeterData {
    /// True-peak amplitude L and R for the master output (linear, 0..1+).
    pub true_peak: [f32; 2],
    /// Stereo phase correlation (Pearson): −1 = inverted, +1 = mono.
    pub phase_correlation: f32,
    /// RMS energy per band: `[0]` = low (<250 Hz), `[1]` = mid, `[2]` = high.
    pub rms: [f32; 3],
    /// Playing state for each of the 4 decks.
    pub deck_playing: [bool; 4],
    /// Normalised vibe scalar 0..1; > 0.8 = chaos mode (raises clip threshold).
    pub vibe_state: f32,
}

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DiagnosisType {
    Clipping,
    PhaseCancel,
    Mud,
    Harsh,
    Thin,
    Pumping,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RoadieEvent {
    pub severity: Severity,
    pub diagnosis: DiagnosisType,
    pub message: String,
    /// Milliseconds since engine start.
    pub timestamp_ms: u64,
}

// ── RoadieBot ────────────────────────────────────────────────────────────────

pub struct RoadieBot {
    events: VecDeque<RoadieEvent>,
    cooldowns: HashMap<DiagnosisType, Instant>,
    dismissals: HashMap<DiagnosisType, u32>,
    sustained: HashMap<DiagnosisType, Instant>,
    start_time: Instant,
    pro_mode: bool,
    rms_history: VecDeque<f32>,
}

impl RoadieBot {
    pub fn new(pro_mode: bool) -> Self {
        Self {
            events: VecDeque::with_capacity(MAX_EVENTS),
            cooldowns: HashMap::new(),
            dismissals: HashMap::new(),
            sustained: HashMap::new(),
            start_time: Instant::now(),
            pro_mode,
            rms_history: VecDeque::with_capacity(RMS_HISTORY_LEN),
        }
    }

    /// Analyze one meter snapshot. Returns new events to deliver to the UI.
    /// Advisory only — never touches the audio thread or issues MixerCommands.
    pub fn analyze(&mut self, meter: &MeterData) -> Vec<RoadieEvent> {
        let mut out = Vec::new();
        let now = Instant::now();
        let ts = now.duration_since(self.start_time).as_millis() as u64;

        // Nothing playing → clear sustained timers (avoids false positives on silence).
        if !meter.deck_playing.iter().any(|&p| p) {
            self.sustained.clear();
            self.rms_history.clear();
            return out;
        }

        let chaos = meter.vibe_state > 0.8;

        self.check_clipping(meter, &mut out, now, ts, chaos);
        self.check_phase(meter, &mut out, now, ts);
        self.check_mud(meter, &mut out, now, ts);
        self.check_harsh(meter, &mut out, now, ts);
        self.check_thin(meter, &mut out, now, ts);
        self.check_pumping(meter, &mut out, now, ts);

        out
    }

    /// Dismiss a diagnosis. After 3 dismissals it is muted for the session.
    pub fn dismiss(&mut self, diagnosis: &DiagnosisType) {
        *self.dismissals.entry(diagnosis.clone()).or_insert(0) += 1;
    }

    /// Recent events, newest first.
    pub fn recent_events(&self, limit: usize) -> Vec<&RoadieEvent> {
        self.events.iter().rev().take(limit).collect()
    }

    /// Total events recorded this session.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn set_pro_mode(&mut self, pro: bool) {
        self.pro_mode = pro;
    }

    // ── Detectors ────────────────────────────────────────────────────────

    fn check_clipping(
        &mut self, m: &MeterData, out: &mut Vec<RoadieEvent>,
        _now: Instant, ts: u64, chaos: bool,
    ) {
        let thresh = if chaos { CLIP_THRESHOLD_CHAOS } else { CLIP_THRESHOLD };
        let (l, r) = (m.true_peak[0], m.true_peak[1]);
        if l > thresh || r > thresh {
            let msg = if self.pro_mode {
                format!(
                    "Clipping: {:.1}dBFS peak (L:{:.1} R:{:.1})",
                    to_db(l.max(r)), to_db(l), to_db(r)
                )
            } else {
                "Signal clipping — pull the gain down".into()
            };
            self.emit(out, Severity::Critical, DiagnosisType::Clipping, msg, ts);
        }
    }

    fn check_phase(&mut self, m: &MeterData, out: &mut Vec<RoadieEvent>, now: Instant, ts: u64) {
        if m.phase_correlation < PHASE_CRITICAL {
            let msg = if self.pro_mode {
                format!(
                    "Phase inversion: correlation {:.2} — check stereo routing",
                    m.phase_correlation
                )
            } else {
                "Stereo phase inverted — something's wired backwards".into()
            };
            self.emit(out, Severity::Critical, DiagnosisType::PhaseCancel, msg, ts);
        } else if m.phase_correlation < PHASE_WARNING {
            let msg = if self.pro_mode {
                format!(
                    "Phase correlation low: {:.2} — possible cancellation",
                    m.phase_correlation
                )
            } else {
                "Stereo image is thin — check phase".into()
            };
            self.check_sustained(out, DiagnosisType::PhaseCancel, now, ts, msg);
        } else {
            self.sustained.remove(&DiagnosisType::PhaseCancel);
        }
    }

    fn check_mud(&mut self, m: &MeterData, out: &mut Vec<RoadieEvent>, now: Instant, ts: u64) {
        if m.rms[1] > 0.01 && m.rms[0] > m.rms[1] * FREQ_IMBALANCE_RATIO {
            let msg = if self.pro_mode {
                format!(
                    "Mud: low {:.1}dB over mid — cut 200-400Hz",
                    to_db(m.rms[0] / m.rms[1])
                )
            } else {
                "Low end is muddy — try cutting some bass".into()
            };
            self.check_sustained(out, DiagnosisType::Mud, now, ts, msg);
        } else {
            self.sustained.remove(&DiagnosisType::Mud);
        }
    }

    fn check_harsh(&mut self, m: &MeterData, out: &mut Vec<RoadieEvent>, now: Instant, ts: u64) {
        if m.rms[1] > 0.01 && m.rms[2] > m.rms[1] * FREQ_IMBALANCE_RATIO {
            let msg = if self.pro_mode {
                format!(
                    "Harsh: high {:.1}dB over mid — tame 2-6kHz",
                    to_db(m.rms[2] / m.rms[1])
                )
            } else {
                "Highs are harsh — ease off the treble".into()
            };
            self.check_sustained(out, DiagnosisType::Harsh, now, ts, msg);
        } else {
            self.sustained.remove(&DiagnosisType::Harsh);
        }
    }

    fn check_thin(&mut self, m: &MeterData, out: &mut Vec<RoadieEvent>, now: Instant, ts: u64) {
        if m.rms[0] < THIN_LOW_FLOOR && m.rms[1] > THIN_MID_FLOOR {
            let msg = if self.pro_mode {
                format!(
                    "Thin: low {:.1}dB, mid {:.1}dB — missing low end",
                    to_db(m.rms[0].max(1e-10)),
                    to_db(m.rms[1])
                )
            } else {
                "Mix sounds thin — where's the bass?".into()
            };
            self.check_sustained(out, DiagnosisType::Thin, now, ts, msg);
        } else {
            self.sustained.remove(&DiagnosisType::Thin);
        }
    }

    fn check_pumping(&mut self, m: &MeterData, out: &mut Vec<RoadieEvent>, now: Instant, ts: u64) {
        let avg = (m.rms[0] + m.rms[1] + m.rms[2]) / 3.0;
        self.rms_history.push_back(avg);
        if self.rms_history.len() > RMS_HISTORY_LEN {
            self.rms_history.pop_front();
        }
        if self.rms_history.len() < 30 {
            return;
        }
        let mean: f32 = self.rms_history.iter().sum::<f32>() / self.rms_history.len() as f32;
        if mean < 0.01 {
            // Mean too low → ratio undefined; clear to avoid spurious sustained timer.
            self.sustained.remove(&DiagnosisType::Pumping);
            return;
        }
        let variance: f32 = self.rms_history
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f32>()
            / self.rms_history.len() as f32;

        if variance / mean > PUMPING_VARIANCE_RATIO {
            let msg = if self.pro_mode {
                format!(
                    "Pumping: RMS variance {:.3} (mean {:.3}) — check compressor",
                    variance, mean
                )
            } else {
                "Mix is pumping — ease off the compressor".into()
            };
            self.check_sustained(out, DiagnosisType::Pumping, now, ts, msg);
        } else {
            self.sustained.remove(&DiagnosisType::Pumping);
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn emit(
        &mut self,
        out: &mut Vec<RoadieEvent>,
        severity: Severity,
        diagnosis: DiagnosisType,
        message: String,
        timestamp_ms: u64,
    ) {
        if *self.dismissals.get(&diagnosis).unwrap_or(&0) >= DISMISSAL_MUTE {
            return;
        }
        let now = Instant::now();
        if let Some(last) = self.cooldowns.get(&diagnosis) {
            if now.duration_since(*last) < Duration::from_secs(COOLDOWN_SECS) {
                return;
            }
        }
        let event = RoadieEvent {
            severity,
            diagnosis: diagnosis.clone(),
            message,
            timestamp_ms,
        };
        self.cooldowns.insert(diagnosis, now);
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event.clone());
        out.push(event);
    }

    fn check_sustained(
        &mut self,
        out: &mut Vec<RoadieEvent>,
        diagnosis: DiagnosisType,
        now: Instant,
        timestamp_ms: u64,
        message: String,
    ) {
        // `or_insert(now)` is load-bearing: keeps the FIRST detection time, not the latest.
        let first_seen = *self.sustained.entry(diagnosis.clone()).or_insert(now);
        if now.duration_since(first_seen) >= Duration::from_secs(SUSTAINED_SECS) {
            self.emit(out, Severity::Warning, diagnosis, message, timestamp_ms);
        }
    }

    // ── Test-only helpers ────────────────────────────────────────────────

    /// Pre-populate the sustained timer as if `diagnosis` was first seen
    /// `backdated_secs` ago. Lets tests drive sustained detectors without sleeping.
    #[cfg(test)]
    pub fn inject_sustained(&mut self, diagnosis: DiagnosisType, backdated_secs: u64) {
        self.sustained
            .insert(diagnosis, Instant::now() - Duration::from_secs(backdated_secs));
    }

    /// Remove a cooldown entry so tests can re-fire an event on demand.
    #[cfg(test)]
    pub fn reset_cooldown(&mut self, diagnosis: &DiagnosisType) {
        self.cooldowns.remove(diagnosis);
    }
}

fn to_db(linear: f32) -> f32 {
    20.0 * linear.max(1e-10).log10()
}

// ── Tests ─────────────────────────────────────────────────────────────────────
//
// Offline + deterministic: synthetic MeterData, no audio device, no sleep.
// Phase-0 RED gate for RoadieBot — each test has a discriminator that FAILS
// if the detector is absent or the logic is broken.

#[cfg(test)]
mod tests {
    use super::*;

    /// A MeterData with one deck playing, clean stereo, balanced bands.
    fn playing(deck: usize) -> MeterData {
        let mut m = MeterData::default();
        m.deck_playing[deck] = true;
        m.phase_correlation = 1.0;
        m.rms = [0.1, 0.1, 0.1];
        m
    }

    // ── Silence ───────────────────────────────────────────────────────────

    #[test]
    fn silence_produces_no_events() {
        let mut bot = RoadieBot::new(false);
        // Even clipping peaks and inverted phase must be ignored when silent.
        let mut m = MeterData::default(); // all deck_playing = false
        m.true_peak = [0.99, 0.99];
        m.phase_correlation = -0.5;
        let events = bot.analyze(&m);
        assert!(events.is_empty(), "silence must never emit events (got {events:?})");
    }

    // ── Clipping ─────────────────────────────────────────────────────────

    #[test]
    fn clipping_fires_critical_immediately() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.true_peak = [0.99, 0.5]; // L over 0.98 threshold
        let events = bot.analyze(&m);
        assert_eq!(events.len(), 1, "expected one clip event, got {}", events.len());
        assert_eq!(events[0].diagnosis, DiagnosisType::Clipping);
        assert_eq!(events[0].severity, Severity::Critical);
    }

    #[test]
    fn clipping_chaos_mode_raises_threshold() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        // 0.99 exceeds normal (0.98) but NOT chaos threshold (0.995).
        m.true_peak = [0.99, 0.5];
        m.vibe_state = 0.9;
        let events = bot.analyze(&m);
        assert!(
            events.is_empty(),
            "chaos mode must suppress clip at 0.99 (threshold 0.995), got {events:?}"
        );
    }

    #[test]
    fn clipping_chaos_still_fires_above_995() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.true_peak = [0.999, 0.5]; // above chaos threshold
        m.vibe_state = 0.9;
        let events = bot.analyze(&m);
        assert_eq!(events.len(), 1, "0.999 must still clip in chaos mode");
        assert_eq!(events[0].diagnosis, DiagnosisType::Clipping);
    }

    // ── Phase cancel ──────────────────────────────────────────────────────

    #[test]
    fn phase_inversion_fires_critical_immediately() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.phase_correlation = -0.5;
        let events = bot.analyze(&m);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].diagnosis, DiagnosisType::PhaseCancel);
        assert_eq!(events[0].severity, Severity::Critical);
    }

    #[test]
    fn healthy_phase_produces_no_event() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.phase_correlation = 0.9;
        let events = bot.analyze(&m);
        assert!(events.is_empty());
    }

    // ── Pro mode ─────────────────────────────────────────────────────────

    #[test]
    fn pro_mode_includes_db_readout() {
        let mut bot = RoadieBot::new(true);
        let mut m = playing(0);
        m.true_peak = [0.99, 0.5];
        let events = bot.analyze(&m);
        assert_eq!(events.len(), 1);
        assert!(
            events[0].message.contains("dBFS"),
            "pro mode must include dBFS readout: {}", events[0].message
        );
    }

    // ── Cooldown ─────────────────────────────────────────────────────────

    #[test]
    fn cooldown_suppresses_repeat_within_30s() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.true_peak = [0.99, 0.5];
        let first = bot.analyze(&m);
        assert_eq!(first.len(), 1, "first clip must fire");
        let second = bot.analyze(&m);
        assert!(second.is_empty(), "repeat within 30 s must be suppressed");
    }

    // ── Dismiss ───────────────────────────────────────────────────────────

    #[test]
    fn dismiss_mutes_after_three_dismissals() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.phase_correlation = -0.5;

        // Fire first event (sets cooldown + dismissal counter starts at 0).
        let e1 = bot.analyze(&m);
        assert_eq!(e1.len(), 1, "first analysis must fire PhaseCancel");

        // Three dismissals → muted.
        bot.dismiss(&DiagnosisType::PhaseCancel);
        bot.dismiss(&DiagnosisType::PhaseCancel);
        bot.dismiss(&DiagnosisType::PhaseCancel);

        // Clear the cooldown so the mute check is not masked by it.
        bot.reset_cooldown(&DiagnosisType::PhaseCancel);

        let e2 = bot.analyze(&m);
        assert!(e2.is_empty(), "3 dismissals must mute PhaseCancel; got {e2:?}");
    }

    // ── Sustained detectors (inject_sustained avoids wall-clock sleep) ────

    #[test]
    fn mud_fires_warning_after_sustained_period() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.rms = [0.5, 0.1, 0.1]; // low >> mid (6 dB+ imbalance)
        // First call starts the timer — must NOT fire yet.
        let immediate = bot.analyze(&m);
        assert!(
            immediate.iter().all(|e| e.diagnosis != DiagnosisType::Mud),
            "mud must not fire immediately"
        );
        // Backdate past the 10-second gate and re-analyze.
        bot.inject_sustained(DiagnosisType::Mud, SUSTAINED_SECS + 1);
        let sustained = bot.analyze(&m);
        let mud = sustained.iter().find(|e| e.diagnosis == DiagnosisType::Mud);
        assert!(mud.is_some(), "mud must fire as Warning after sustained period; events: {sustained:?}");
        assert_eq!(mud.unwrap().severity, Severity::Warning);
    }

    #[test]
    fn harsh_fires_warning_after_sustained_period() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.rms = [0.1, 0.1, 0.5]; // high >> mid
        let _ = bot.analyze(&m);
        bot.inject_sustained(DiagnosisType::Harsh, SUSTAINED_SECS + 1);
        let events = bot.analyze(&m);
        assert!(
            events.iter().any(|e| e.diagnosis == DiagnosisType::Harsh),
            "harsh must fire after sustained period; events: {events:?}"
        );
    }

    #[test]
    fn thin_fires_warning_after_sustained_period() {
        let mut bot = RoadieBot::new(false);
        let mut m = playing(0);
        m.rms = [0.001, 0.2, 0.1]; // low < 0.01, mid > 0.1
        let _ = bot.analyze(&m);
        bot.inject_sustained(DiagnosisType::Thin, SUSTAINED_SECS + 1);
        let events = bot.analyze(&m);
        assert!(
            events.iter().any(|e| e.diagnosis == DiagnosisType::Thin),
            "thin must fire after sustained period; events: {events:?}"
        );
    }

    #[test]
    fn pumping_fires_when_rms_variance_exceeds_threshold() {
        let mut bot = RoadieBot::new(false);
        // Feed 60 alternating high/low frames to build variance history.
        // Alternating 0.8 / 0.1: mean ≈ 0.45, variance ≈ 0.12, ratio ≈ 0.27 > 0.1.
        for i in 0..60usize {
            let mut m = playing(0);
            let level = if i % 2 == 0 { 0.8f32 } else { 0.1f32 };
            m.rms = [level, level, level];
            bot.analyze(&m);
        }
        // Backdate the sustained timer past the gate.
        bot.inject_sustained(DiagnosisType::Pumping, SUSTAINED_SECS + 1);
        let mut m = playing(0);
        m.rms = [0.8, 0.8, 0.8]; // keep variance high
        let events = bot.analyze(&m);
        assert!(
            events.iter().any(|e| e.diagnosis == DiagnosisType::Pumping),
            "high RMS variance must trigger Pumping warning; events: {events:?}"
        );
    }

    // ── Tracking ──────────────────────────────────────────────────────────

    #[test]
    fn event_count_and_recent_events_track_correctly() {
        let mut bot = RoadieBot::new(false);
        assert_eq!(bot.event_count(), 0);
        let mut m = playing(0);
        m.true_peak = [0.99, 0.5];
        bot.analyze(&m);
        assert_eq!(bot.event_count(), 1);
        let recent = bot.recent_events(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].diagnosis, DiagnosisType::Clipping);
    }
}
