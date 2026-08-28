//! CodeVoice — the sound of your code. Healthy hums the full 13moons prairie;
//! a break notches the expected bands (the hole you hear); Silent stops the hum.
//! Pure: (outcome, tick, moon) -> deterministic fauna params. Same code -> same sound.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeVoice {
    Healthy,
    Warn,
    Broke,
    Silent,
}

impl CodeVoice {
    pub fn to_params(self, tick_id: u64, moon: u64) -> Value {
        let (fauna_gain, walker_active, temperature_c, fauna_density) = match self {
            CodeVoice::Healthy => (
                0.80,
                false,
                20.0,
                json!({
                    "cricket": 0.8,
                    "frog": 0.6,
                    "bird": 0.7,
                    "insect": 0.5,
                    "owl": 0.4,
                    "mosquito": 0.3
                }),
            ),
            CodeVoice::Warn => (
                0.55,
                false,
                18.0,
                json!({
                    "cricket": 0.4,
                    "frog": 0.3,
                    "bird": 0.35,
                    "insect": 0.25
                }),
            ),
            CodeVoice::Broke => (
                0.70,
                true,
                20.0,
                json!({
                    "cricket": 0.15,
                    "frog": 0.5,
                    "bird": 0.15,
                    "insect": 0.4,
                    "owl": 0.4
                }),
            ),
            CodeVoice::Silent => (0.0, false, 15.0, json!({})),
        };

        json!({
            "fauna_gain": fauna_gain,
            "walker_active": walker_active,
            "temperature_c": temperature_c,
            "fauna_density": fauna_density,
            "vibe_position": 0.0,
            "listener_age_bracket": 2,
            "tick_id": tick_id,
            "moon": moon
        })
    }

    /// Classify live system signals into a voice. Pure, total, deterministic.
    /// Priority: brain/door down (Silent) > build broke (Broke) > stale beat (Warn) > Healthy.
    /// `heartbeat_age_secs > 90` = the valve going cold (road-scan corpse threshold).
    pub fn from_signals(build_ok: bool, daemon_ok: bool, heartbeat_age_secs: u64) -> CodeVoice {
        if !daemon_ok {
            CodeVoice::Silent
        } else if !build_ok {
            CodeVoice::Broke
        } else if heartbeat_age_secs > 90 {
            CodeVoice::Warn
        } else {
            CodeVoice::Healthy
        }
    }
}

/// VoiceBank — the alloc-free, lock-free bridge to the audio thread. Four voices
/// precomputed at load time; the realtime callback only ever *borrows* one.
pub struct VoiceBank {
    params: [Value; 4], // 0=Healthy 1=Warn 2=Broke 3=Silent
    current: AtomicU8,
}

impl VoiceBank {
    /// Precompute all four voices once (heap here is fine — load-time, off the audio thread).
    pub fn new(tick_id: u64, moon: u64) -> Self {
        Self {
            params: [
                CodeVoice::Healthy.to_params(tick_id, moon),
                CodeVoice::Warn.to_params(tick_id, moon),
                CodeVoice::Broke.to_params(tick_id, moon),
                CodeVoice::Silent.to_params(tick_id, moon),
            ],
            current: AtomicU8::new(0),
        }
    }

    /// Control thread: swap the live voice. Lock-free store.
    pub fn set(&self, v: CodeVoice) {
        self.current.store(voice_to_idx(v), Ordering::Relaxed);
    }

    /// Audio thread: borrow the current precomputed params. Zero alloc, lock-free load.
    pub fn current_params(&self) -> &Value {
        let i = (self.current.load(Ordering::Relaxed) & 0b11) as usize;
        &self.params[i]
    }

    /// The live voice.
    pub fn current(&self) -> CodeVoice {
        idx_to_voice(self.current.load(Ordering::Relaxed))
    }
}

fn voice_to_idx(v: CodeVoice) -> u8 {
    match v {
        CodeVoice::Healthy => 0,
        CodeVoice::Warn => 1,
        CodeVoice::Broke => 2,
        CodeVoice::Silent => 3,
    }
}

fn idx_to_voice(i: u8) -> CodeVoice {
    match i & 0b11 {
        0 => CodeVoice::Healthy,
        1 => CodeVoice::Warn,
        2 => CodeVoice::Broke,
        _ => CodeVoice::Silent,
    }
}

static VOICE: OnceLock<VoiceBank> = OnceLock::new();
static ENABLED: OnceLock<bool> = OnceLock::new();

/// Process-global voice bridge. First touch precomputes the four voices ONCE
/// (not per-buffer). Control threads call `global().set(..)`; the audio thread borrows.
pub fn global() -> &'static VoiceBank {
    VOICE.get_or_init(|| VoiceBank::new(0, 0))
}

/// Is the code-voice audio layer live? OFF unless `FORGE_CODE_VOICE=1`. Read once
/// (env is process-static). The hum stays silent until you opt in — no surprise blast.
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var("FORGE_CODE_VOICE").as_deref() == Ok("1"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determinism() {
        assert_eq!(
            CodeVoice::Broke.to_params(7, 3),
            CodeVoice::Broke.to_params(7, 3)
        );
    }

    #[test]
    fn distinct() {
        assert_ne!(
            CodeVoice::Healthy.to_params(0, 0),
            CodeVoice::Broke.to_params(0, 0)
        );
    }

    #[test]
    fn broke_walker_active() {
        let p = CodeVoice::Broke.to_params(0, 0);
        assert_eq!(p["walker_active"], true);
    }

    #[test]
    fn silent_gain_zero() {
        let p = CodeVoice::Silent.to_params(0, 0);
        assert_eq!(p["fauna_gain"], 0.0);
    }

    #[test]
    fn healthy_gain_and_walker() {
        let p = CodeVoice::Healthy.to_params(0, 0);
        assert!(p["fauna_gain"].as_f64().unwrap() > 0.0);
        assert_eq!(p["walker_active"], false);
    }

    #[test]
    fn arg_passthrough() {
        let p = CodeVoice::Healthy.to_params(42, 9);
        assert_eq!(p["tick_id"], 42);
        assert_eq!(p["moon"], 9);
    }

    #[test]
    fn signals_healthy() {
        assert_eq!(CodeVoice::from_signals(true, true, 0), CodeVoice::Healthy);
    }

    #[test]
    fn signals_daemon_down_is_silent() {
        // brain/door down beats a build failure — the hum stops.
        assert_eq!(CodeVoice::from_signals(false, false, 999), CodeVoice::Silent);
    }

    #[test]
    fn signals_build_broke() {
        assert_eq!(CodeVoice::from_signals(false, true, 0), CodeVoice::Broke);
    }

    #[test]
    fn signals_stale_beat_is_warn() {
        assert_eq!(CodeVoice::from_signals(true, true, 91), CodeVoice::Warn);
        // boundary: 90 is still healthy, 91 tips to warn.
        assert_eq!(CodeVoice::from_signals(true, true, 90), CodeVoice::Healthy);
    }

    #[test]
    fn bank_default_is_healthy() {
        let b = VoiceBank::new(0, 0);
        assert_eq!(b.current(), CodeVoice::Healthy);
        assert_eq!(b.current_params(), &CodeVoice::Healthy.to_params(0, 0));
    }

    #[test]
    fn bank_set_roundtrip() {
        let b = VoiceBank::new(0, 0);
        b.set(CodeVoice::Broke);
        assert_eq!(b.current(), CodeVoice::Broke);
        assert_eq!(b.current_params()["walker_active"], true);
    }

    #[test]
    fn bank_all_four_reachable() {
        let b = VoiceBank::new(0, 0);
        for v in [
            CodeVoice::Healthy,
            CodeVoice::Warn,
            CodeVoice::Broke,
            CodeVoice::Silent,
        ] {
            b.set(v);
            assert_eq!(b.current(), v);
        }
    }

    #[test]
    fn bank_silent_stops_hum() {
        let b = VoiceBank::new(0, 0);
        b.set(CodeVoice::Silent);
        assert_eq!(b.current_params()["fauna_gain"], 0.0);
    }
}
</content>
