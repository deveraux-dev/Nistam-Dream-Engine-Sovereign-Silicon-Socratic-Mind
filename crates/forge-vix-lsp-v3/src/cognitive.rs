//! CognitiveLens — contextual tone adaptation and response scoring.
//! Standalone module: no Sieve trait imports, no game events.
//! Consumes `CognitiveSignal` streams; adapts persona to observed patterns.

use serde::{Deserialize, Serialize};

/// A single cognitive observation from a session tick.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CognitiveSignal {
    pub word_count: u16,
    pub question_count: u8,
    pub code_blocks: u8,
    pub error_rate_pmy: u16,   // 0-10000
    pub accept_rate_pmy: u16,  // 0-10000
    pub ticks_since_last: u16,
}

/// Observed cognitive state — what mode the user is in right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[derive(Default)]
pub enum CognitiveState {
    #[default]
    Neutral      = 0,
    Focused      = 1,
    Stressed     = 2,
    Exploring    = 3,
    Fatigued     = 4,
    /// Rapid-rerun error thrash: max error signal with no gap between runs.
    Frustrated   = 5,
    /// Deep flow: 10+ clean accepts in a code context with zero errors.
    Hyperfocused = 6,
}


/// A recommended response adaptation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[derive(Default)]
pub enum Adaptation { #[default]
None = 0, Shorten = 1, AddExamples = 2, SlowPace = 3, Clarify = 4 }


/// Trait for pluggable cognitive adaptation strategies.
pub trait CognitiveLens {
    fn ingest(&mut self, signal: &CognitiveSignal);
    fn state(&self) -> CognitiveState;
    fn adapt(&self) -> Adaptation;
    fn score_response(&self, response: &CognitiveSignal) -> u16; // Permyriad quality score
}

/// Configuration for ADHD-adapted sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdhdConfig {
    pub response_length_target: u16,   // Target word count for responses
    pub chunk_count: u8,               // How many chunks to break long content into
    pub example_density: u8,           // Examples per N paragraphs
    pub max_errors_before_pause: u8,   // Error threshold before recommending a break
    pub idle_timeout: u16,             // Ticks before assuming fatigue
}

impl Default for AdhdConfig {
    fn default() -> Self {
        Self {
            response_length_target: 80,
            chunk_count: 3,
            example_density: 2,
            max_errors_before_pause: 5,
            idle_timeout: 30,
        }
    }
}

/// ADHD-adapted cognitive lens: prefers short chunks, frequent examples,
/// detects stress/fatigue early, adapts pacing accordingly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdhdLens {
    pub config: AdhdConfig,
    pub recent_errors: u8,
    pub recent_accepts: u8,
    pub signals_observed: u32,
    pub last_word_count: u16,
    pub idle_ticks: u16,
    pub current_state: CognitiveState,
    pub last_adaptation: Adaptation,
}

impl AdhdLens {
    pub fn new(config: AdhdConfig) -> Self {
        Self {
            config,
            recent_errors: 0,
            recent_accepts: 0,
            signals_observed: 0,
            last_word_count: 0,
            idle_ticks: 0,
            current_state: CognitiveState::Neutral,
            last_adaptation: Adaptation::None,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(AdhdConfig::default())
    }
}

impl CognitiveLens for AdhdLens {
    fn ingest(&mut self, signal: &CognitiveSignal) {
        self.signals_observed += 1;
        self.last_word_count = signal.word_count;
        self.idle_ticks = if signal.ticks_since_last > 0 { signal.ticks_since_last } else { 0 };

        if signal.error_rate_pmy > 3000 {
            self.recent_errors = self.recent_errors.saturating_add(1);
        } else {
            self.recent_errors = self.recent_errors.saturating_sub(1);
        }
        if signal.accept_rate_pmy > 7000 {
            self.recent_accepts = self.recent_accepts.saturating_add(1);
        }

        self.current_state = if self.idle_ticks > self.config.idle_timeout {
            CognitiveState::Fatigued
        } else if signal.question_count >= 16 {
            // question_count repurposed as context-switch count in LSP telemetry; >= 16 = overload.
            CognitiveState::Fatigued
        } else if signal.error_rate_pmy >= 10000 && signal.ticks_since_last == 0 {
            // Rapid-rerun thrash: max error signal with no idle gap between runs.
            CognitiveState::Frustrated
        } else if self.recent_errors >= self.config.max_errors_before_pause {
            CognitiveState::Stressed
        } else if self.recent_accepts >= 10 && signal.code_blocks > 0 && signal.error_rate_pmy == 0 {
            // Deep flow: 10+ clean accepts in a code context with zero errors.
            CognitiveState::Hyperfocused
        } else if signal.question_count > 3 {
            CognitiveState::Exploring
        } else if signal.code_blocks > 0 && signal.word_count < self.config.response_length_target {
            CognitiveState::Focused
        } else {
            CognitiveState::Neutral
        };

        self.last_adaptation = match self.current_state {
            CognitiveState::Stressed | CognitiveState::Fatigued => Adaptation::SlowPace,
            CognitiveState::Frustrated => Adaptation::Clarify,
            CognitiveState::Hyperfocused | CognitiveState::Focused => Adaptation::Shorten,
            CognitiveState::Exploring => Adaptation::AddExamples,
            _ if signal.word_count > self.config.response_length_target * 2 => Adaptation::Shorten,
            _ => Adaptation::None,
        };
    }

    fn state(&self) -> CognitiveState {
        self.current_state
    }

    fn adapt(&self) -> Adaptation {
        self.last_adaptation
    }

    fn score_response(&self, response: &CognitiveSignal) -> u16 {
        score_response(response, &self.config)
    }
}

/// Detect the dominant tone in a session: calm/focused/scattered/urgent.
pub fn detect_tone(signals: &[CognitiveSignal]) -> CognitiveState {
    if signals.is_empty() { return CognitiveState::Neutral; }

    let n = signals.len() as u32;
    let avg_errors: u32 = signals.iter().map(|s| s.error_rate_pmy as u32).sum::<u32>() / n;
    let avg_questions: u32 = signals.iter().map(|s| s.question_count as u32).sum::<u32>() / n;
    let avg_idle: u32 = signals.iter().map(|s| s.ticks_since_last as u32).sum::<u32>() / n;

    if avg_idle > 60 {
        CognitiveState::Fatigued
    } else if avg_errors > 5000 {
        CognitiveState::Stressed
    } else if avg_questions > 3 {
        CognitiveState::Exploring
    } else {
        CognitiveState::Focused
    }
}

/// Score a response quality based on config targets (Permyriad 0-10000).
pub fn score_response(response: &CognitiveSignal, config: &AdhdConfig) -> u16 {
    let mut score: i32 = 10000;

    let target = config.response_length_target as i32;
    if response.word_count as i32 > target * 2 {
        score -= 3000;
    } else if response.word_count as i32 > target {
        score -= 1000;
    }

    if response.accept_rate_pmy > 7000 {
        score += 1000;
    } else if response.accept_rate_pmy < 2000 {
        score -= 2000;
    }

    if response.error_rate_pmy > 5000 {
        score -= 2000;
    }

    score.clamp(0, 10000) as u16
}

/// Count unique decision branches in a signal stream.
pub fn count_decisions(signals: &[CognitiveSignal]) -> u32 {
    let mut decisions = 0u32;
    let mut prev_accepts: u16 = 0;
    for s in signals {
        if s.accept_rate_pmy != prev_accepts {
            decisions += 1;
        }
        prev_accepts = s.accept_rate_pmy;
    }
    decisions
}

/// Guidance intensity. The lens NEVER forces (Sean 2026-07-18): Off = pure
/// display; On(pmy) scales how hard adapt() may bite. intensity 0 == Off.
/// A slider you can always turn back, never a switch that traps you.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Guidance { Off, On(u16) } // u16 = intensity permyriad 0..=10000

impl Default for Guidance {
    fn default() -> Self { Guidance::Off } // safe default = never force
}

impl Guidance {
    pub fn intensity_pmy(self) -> u16 {
        match self { Guidance::Off => 0, Guidance::On(p) => p.min(10000) }
    }
    pub fn active(self) -> bool { self.intensity_pmy() > 0 }
}

/// Advisory adaptation: `None` when guidance is Off/0 — surface the state for
/// DISPLAY, apply nothing. Every consumer routes adapt() through here; a raw
/// adapt() application is a forcing bug.
pub fn guided_adapt(adapt: Adaptation, guidance: Guidance) -> Option<Adaptation> {
    if guidance.active() { Some(adapt) } else { None }
}

/// Focused-muzzle strength (permyriad): how hard to compress output for the
/// current state, scaled by guidance. 0 = no muzzle. Only flow states muzzle;
/// Stressed/Frustrated/Fatigued/Exploring want clarity or pacing, never a gag.
pub fn muzzle_pmy(state: CognitiveState, guidance: Guidance) -> u16 {
    let base: u32 = match state {
        CognitiveState::Hyperfocused => 10000,
        CognitiveState::Focused => 7000,
        CognitiveState::Neutral => 3000,
        _ => 0,
    };
    ((base * guidance.intensity_pmy() as u32) / 10000) as u16
}

/// Operator rung — a dial the operator sets, never inferred, no purchase tier
/// (Sean 07-20): Child/Curious ride the dynamic bar; Maker+ cull it for the
/// command palette; Master alone unlocks daemon QA tools. Entry rung = default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum CognitiveCeiling {
    #[default]
    Child = 0,
    Curious = 1,
    Maker = 2,
    Master = 3,
}

impl CognitiveCeiling {
    pub fn palette_active(self) -> bool { self >= CognitiveCeiling::Maker }
    pub fn bar_active(self) -> bool { !self.palette_active() }
    pub fn daemon_unlocked(self) -> bool { self == CognitiveCeiling::Master }
}

/// The cross-process cognitive bus: what the lens publishes for the neuro-hud +
/// heal synth to READ. Written by the producer (forge-vix-lsp), read by the
/// studio. Guidance rides along so a reader never forces past the user's slider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveBus {
    pub state: CognitiveState,
    pub adaptation: Adaptation,
    pub guidance: Guidance,
    /// Pre-ceiling bus files parse as Child (entry rung) — never a hard MISSING.
    #[serde(default)]
    pub ceiling: CognitiveCeiling,
}

/// The one on-disk cognitive-bus file, relative to the SoT root. BOTH the producer
/// (forge-vix-lsp editor telemetry) and the consumer (forge-studio neuro-hud) MUST
/// resolve the bus through [`cognitive_bus_path`] so a write and a read can never
/// disagree on where the bus lives (C5 path-match law). forge-studio's neuro-hud reads
/// exactly `<root>/.forge/cognitive.json` — this constant is that path's authority.
pub const COGNITIVE_BUS_REL: &str = ".forge/cognitive.json";

/// Resolve the cognitive-bus file under a source-of-truth root.
pub fn cognitive_bus_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(COGNITIVE_BUS_REL)
}

/// The SoT root for the bus, for producers that cannot depend on the heavy
/// forge-daemon (the LSP). MIRRORS `forge_daemon::platform::sot_root`'s precedence
/// exactly — `FORGE_FLOOR`, then `FORGE_REPO_MAP_DIR`'s parent, then the clean-slate
/// `F:\v3` — so the LSP's write path and the studio's read path (which resolves
/// via `forge_daemon::sot_root`) land on the identical file. Kept in lock-step by the
/// C5 board test; a future consolidation could collapse the two into one resolver.
pub fn bus_root() -> std::path::PathBuf {
    bus_root_from_env(
        std::env::var("FORGE_FLOOR").ok(),
        std::env::var("FORGE_REPO_MAP_DIR").ok(),
    )
}

fn bus_root_from_env(
    forge_floor: Option<String>,
    repo_map_dir: Option<String>,
) -> std::path::PathBuf {
    use std::path::{Path, PathBuf};
    if let Some(p) = forge_floor {
        return PathBuf::from(p);
    }
    if let Some(dir) = repo_map_dir {
        if let Some(parent) = Path::new(&dir).parent() {
            return parent.to_path_buf();
        }
    }
    PathBuf::from(r"F:\v3")
}

/// Write the cognitive bus to `<root>/.forge/cognitive.json` (producer side), creating
/// `.forge` if missing. Runs on the ~1/min cognitive lane, never the audio callback.
pub fn write_cognitive_bus(root: &std::path::Path, bus: &CognitiveBus) -> std::io::Result<()> {
    let path = cognitive_bus_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(bus)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Read the cognitive bus back (consumer side). `None` = no producer has written yet
/// (lens idle) or the file is unparseable — the neuro-hud shows a LOUD "lens idle".
pub fn read_cognitive_bus(root: &std::path::Path) -> Option<CognitiveBus> {
    let bytes = std::fs::read(cognitive_bus_path(root)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(words: u16, questions: u8, code: u8, errors: u16, accepts: u16, idle: u16) -> CognitiveSignal {
        CognitiveSignal { word_count: words, question_count: questions, code_blocks: code, error_rate_pmy: errors, accept_rate_pmy: accepts, ticks_since_last: idle }
    }

    #[test]
    fn lens_detects_stressed_state() {
        let mut lens = AdhdLens::with_defaults();
        for _ in 0..6 {
            lens.ingest(&sig(100, 0, 0, 8000, 1000, 5));
        }
        assert_eq!(lens.state(), CognitiveState::Stressed);
        assert_eq!(lens.adapt(), Adaptation::SlowPace);
    }

    #[test]
    fn lens_detects_fatigued_after_idle() {
        let mut lens = AdhdLens::with_defaults();
        lens.ingest(&sig(50, 0, 0, 0, 9000, 100)); // idle > 30 threshold
        assert_eq!(lens.state(), CognitiveState::Fatigued);
    }

    #[test]
    fn lens_detects_exploring_on_questions() {
        let mut lens = AdhdLens::with_defaults();
        lens.ingest(&sig(50, 5, 0, 0, 8000, 1));
        assert_eq!(lens.state(), CognitiveState::Exploring);
        assert_eq!(lens.adapt(), Adaptation::AddExamples);
    }

    #[test]
    fn lens_detects_focused() {
        let mut lens = AdhdLens::with_defaults();
        lens.ingest(&sig(30, 0, 2, 0, 9000, 1)); // code + short = focused
        assert_eq!(lens.state(), CognitiveState::Focused);
        assert_eq!(lens.adapt(), Adaptation::Shorten);
    }

    #[test]
    fn guidance_off_never_forces() {
        assert_eq!(guided_adapt(Adaptation::Shorten, Guidance::Off), None);
        assert_eq!(guided_adapt(Adaptation::Shorten, Guidance::On(0)), None);
        assert_eq!(muzzle_pmy(CognitiveState::Hyperfocused, Guidance::Off), 0);
        assert_eq!(Guidance::default(), Guidance::Off);
    }

    #[test]
    fn guidance_on_is_advisory_and_scaled() {
        assert_eq!(guided_adapt(Adaptation::Shorten, Guidance::On(5000)), Some(Adaptation::Shorten));
        assert_eq!(muzzle_pmy(CognitiveState::Hyperfocused, Guidance::On(10000)), 10000);
        assert_eq!(muzzle_pmy(CognitiveState::Focused, Guidance::On(5000)), 3500);
        // clarity states are never muzzled, at any intensity
        assert_eq!(muzzle_pmy(CognitiveState::Frustrated, Guidance::On(10000)), 0);
        assert_eq!(muzzle_pmy(CognitiveState::Fatigued, Guidance::On(10000)), 0);
    }

    #[test]
    fn score_response_penalizes_long() {
        let config = AdhdConfig::default(); // target = 80 words
        let long = sig(300, 0, 0, 0, 9000, 0);
        let short = sig(60, 0, 0, 0, 9000, 0);
        assert!(score_response(&short, &config) > score_response(&long, &config));
    }

    #[test]
    fn score_response_penalizes_errors() {
        let config = AdhdConfig::default();
        let clean = sig(80, 0, 0, 0, 9000, 0);
        let buggy = sig(80, 0, 0, 8000, 9000, 0);
        assert!(score_response(&clean, &config) > score_response(&buggy, &config));
    }

    #[test]
    fn detect_tone_from_high_error_signals() {
        let signals: Vec<_> = (0..5).map(|_| sig(100, 0, 0, 8000, 0, 5)).collect();
        let tone = detect_tone(&signals);
        assert_eq!(tone, CognitiveState::Stressed);
    }

    #[test]
    fn detect_tone_fatigued_on_idle() {
        let signals = vec![sig(50, 0, 0, 0, 9000, 120)];
        assert_eq!(detect_tone(&signals), CognitiveState::Fatigued);
    }

    #[test]
    fn count_decisions_zero_on_flat_stream() {
        let signals: Vec<_> = (0..5).map(|_| sig(100, 0, 0, 0, 5000, 1)).collect();
        // Only the first change from 0 → 5000 counts
        assert_eq!(count_decisions(&signals), 1);
    }

    #[test]
    fn count_decisions_counts_accept_rate_changes() {
        let signals = vec![
            sig(100, 0, 0, 0, 3000, 1),
            sig(100, 0, 0, 0, 5000, 1),
            sig(100, 0, 0, 0, 5000, 1),
            sig(100, 0, 0, 0, 8000, 1),
        ];
        assert_eq!(count_decisions(&signals), 3); // 0→3000, 3000→5000, 5000→8000
    }

    #[test]
    fn adhd_lens_score_response_via_trait() {
        let lens = AdhdLens::with_defaults();
        let good = sig(80, 0, 1, 0, 9000, 1);
        let score = lens.score_response(&good);
        assert!(score >= 9000, "clean short response should score high, got {}", score);
    }

    #[test]
    fn ceiling_rungs_gate_disclosure_and_daemon() {
        use CognitiveCeiling::*;
        assert!(Child < Curious && Curious < Maker && Maker < Master);
        for c in [Child, Curious] {
            assert!(c.bar_active() && !c.palette_active() && !c.daemon_unlocked());
        }
        for c in [Maker, Master] {
            assert!(c.palette_active() && !c.bar_active());
        }
        assert!(!Maker.daemon_unlocked() && Master.daemon_unlocked());
        assert_eq!(CognitiveCeiling::default(), Child);
    }

    #[test]
    fn ceiling_serde_back_compat_defaults_child() {
        // the exact on-disk bus shape from before the ceiling field existed
        let old = r#"{"state":"Focused","adaptation":"Shorten","guidance":{"On":5000}}"#;
        let bus: CognitiveBus = serde_json::from_str(old).expect("pre-ceiling bus json parses");
        assert_eq!(bus.ceiling, CognitiveCeiling::Child);
    }

    #[test]
    fn cognitive_bus_path_matches_producer_and_consumer() {
        use std::path::PathBuf;

        // The consumer (forge-studio neuro-hud) reads exactly `<root>/.forge/cognitive.json`.
        // Pin that the shared helper produces that identical path — so a producer write and a
        // consumer read can never land on different files. This IS the path-match proof.
        let root = PathBuf::from(r"X:\some\root");
        assert_eq!(cognitive_bus_path(&root), root.join(".forge/cognitive.json"));

        // bus_root (the LSP producer's root) mirrors forge_daemon::sot_root's precedence.
        // Pin every branch so the producer root can never silently drift from the daemon's.
        assert_eq!(bus_root_from_env(Some(r"D:\floor".into()), None), PathBuf::from(r"D:\floor"));
        assert_eq!(
            bus_root_from_env(None, Some(r"D:\repo\.forge".into())),
            PathBuf::from(r"D:\repo")
        );
        assert_eq!(bus_root_from_env(None, None), PathBuf::from(r"F:\v3"));

        // Round-trip a bus through the on-disk file at the shared path (producer -> consumer).
        let tmp = std::env::temp_dir().join(format!("forge-sieve-c5-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let bus = CognitiveBus {
            state: CognitiveState::Fatigued,
            adaptation: Adaptation::SlowPace,
            guidance: Guidance::On(5000),
            ceiling: CognitiveCeiling::Master,
        };
        write_cognitive_bus(&tmp, &bus).expect("producer writes the bus");
        assert!(cognitive_bus_path(&tmp).exists(), "producer wrote to the shared path");
        assert_eq!(
            read_cognitive_bus(&tmp),
            Some(bus),
            "consumer reads the identical bus back from the same path"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
