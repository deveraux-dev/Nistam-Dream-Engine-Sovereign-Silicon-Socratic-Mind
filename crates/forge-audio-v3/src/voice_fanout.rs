//! voice_fanout — dispatch one live mic `VocalFrame` to ALL THREE consumers in
//! one call: gameplay (voice_bridge), HEAR→SEE (viz_buffer), and the conductor
//! Drum-2 presence pulse (conductor_audio).
//!
//! This is the ONE fan-out seam for the mic capillary. It carries the
//! push-to-talk gate: `mic_active` (spacebar held) — when false, every sink
//! receives `VocalFrame::SILENT` (the mic-off "X" state), so releasing the key
//! cleanly quiets gameplay reactivity, the viz level, and the conductor bed.

use crate::conductor_audio::AudioLane;
use crate::viz_buffer::AudioVizBuffer;
use crate::vocal_frame::VocalFrame;
use forge_harmonics::voice_bridge::{process_voice_tick, VoiceTickResult};

/// Stateful fan-out. Holds the latest gameplay result + liveness counters so a
/// caller (studio loop / proof gate) can read back what the mic drove.
#[derive(Debug, Default)]
pub struct VoiceFanout {
    /// Latest gameplay bridge result (aura/rhythm/combat).
    pub last_gameplay: Option<VoiceTickResult>,
    pub total_frames: u64,
    pub voiced_frames: u64,
}

impl VoiceFanout {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fan one frame to gameplay + viz + conductor.
    ///
    /// `tick`: current 120 Hz tick · `mic_active`: push-to-talk gate (spacebar
    /// held). When `mic_active` is false the frame is replaced by
    /// `VocalFrame::SILENT` so all three sinks quiet to the mic-off state.
    pub fn dispatch(
        &mut self,
        frame: &VocalFrame,
        tick: u64,
        mic_active: bool,
        viz: &AudioVizBuffer,
        conductor: &mut AudioLane,
    ) -> &VoiceTickResult {
        let f = if mic_active { *frame } else { VocalFrame::SILENT };
        self.total_frames += 1;
        if f.is_voiced() {
            self.voiced_frames += 1;
        }

        // 1. gameplay bridge — neutral chart/target (studio supplies real ones).
        let result = process_voice_tick(&f, tick, &[], 0);

        // 2. HEAR→SEE viz: rms (Permyriad 0..10000) → f32 level 0..1, mono→L+R.
        let level = (f.rms_q as f32 / 10_000.0).clamp(0.0, 1.0);
        viz.rms_left.store(level);
        viz.rms_right.store(level);

        // 3. conductor Drum-2 presence pulse: bed intensity tracks mic loudness.
        conductor.set_bed_intensity(level);

        self.last_gameplay = Some(result);
        self.last_gameplay.as_ref().unwrap()
    }
}
