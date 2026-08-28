// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//! L0 Audio lane — the conductor's audio executor (ADR-013).
//!
//! Turns [`ScheduledEvent`]s (phrase_kind in `event.tag`) into actual sound via the
//! three composed instruments: [`Synth`] (voice), [`LoopSequencer`] (bed), [`Transport`] (grid).
//!
//! | Phrase              | Gesture                                          |
//! |---------------------|--------------------------------------------------|
//! | `MinorThirdDescent` | grave-bell: strike root + minor-third dyad (A3)  |
//! | `SilentHold`        | sustain: transport.play + raise loop intensity   |
//! | `RefusalRest`       | rest: all_notes_off (deliberate silence)         |
//!
//! Zero-heap: [`AudioLane::apply`] walks a borrowed slice, [`AudioLane::render`] calls only the
//! instruments' own no-alloc paths.

use crate::phrase::BardPhraseKind;
use crate::scheduled_event::ScheduledEvent;

use crate::effects::{reverb_preset, ZoneReverb, REVERB_PRESETS};
use crate::harmonic_brush::HarmonicBrushState;
use crate::loop_sequencer::LoopSequencer;
use crate::synth::Synth;
use crate::transport::Transport;

const BELL_VEL: u8 = 110;
const BELL_DUR_MS: u32 = 600;
const HOLD_INTENSITY: f32 = 1.0; // @forge:allow_float

/// The L0 Audio peer: the three conductor-driven instruments + phrase router.
pub struct AudioLane {
    pub synth: Synth,
    pub loops: LoopSequencer,
    pub transport: Transport,
    pub reverb: ZoneReverb,
    /// Live harmonic context: root note + scale mask + voice preset.
    /// Set by the host to change the brush key/timbre without restarting the lane.
    pub harmonics: HarmonicBrushState,
}

impl AudioLane {
    pub fn new(sample_rate: u32, bpm: f32) -> Self {
        Self {
            synth: Synth::new(sample_rate),
            loops: LoopSequencer::new(2.0, sample_rate),
            transport: Transport::new(bpm, sample_rate),
            reverb: ZoneReverb::new(
                sample_rate as f32,
                reverb_preset("medium_hall").unwrap_or(&REVERB_PRESETS[1]),
            ),
            harmonics: HarmonicBrushState::default(),
        }
    }

    // ── Facade (drive through AudioLane, not the inner structs) ──────────────

    pub fn trigger_note(&mut self, note: u8, velocity: u8, duration_ms: u32) {
        self.synth.note_on(note, velocity, duration_ms);
    }
    pub fn release_note(&mut self, note: u8) { self.synth.note_off(note); }
    pub fn silence(&mut self) { self.synth.all_notes_off(); }
    pub fn play(&mut self) { self.transport.play(); }
    pub fn stop(&mut self) { self.transport.stop_and_rewind(); }
    pub fn seek_beat(&mut self, beat: usize) { self.transport.seek_to_beat(beat); }
    pub fn set_bed_intensity(&mut self, intensity: f32) { self.loops.set_intensity(intensity); }
    pub fn add_loop_voice(&mut self, len_samples: u64, phase_offset: u64, activation: f32) -> Option<usize> {
        self.loops.add_voice(len_samples, phase_offset, activation)
    }
    pub fn sync(&mut self) { self.loops.sync_all(); }
    pub fn active_voices(&self) -> usize { self.synth.active_voices() }

    pub fn set_reverb_preset(&mut self, name: &str) -> bool {
        match reverb_preset(name) {
            Some(p) => { self.reverb.set_preset(p); true }
            None => false,
        }
    }
    pub fn set_reverb_wet(&mut self, wet: f32) { self.reverb.set_wet(wet); }

    /// Apply one tick's worth of L0 events. Zero-heap.
    pub fn apply(&mut self, events: &[ScheduledEvent], _now: u64) {
        for ev in events {
            match BardPhraseKind::from_u8(ev.tag as u8) {
                Some(BardPhraseKind::MinorThirdDescent) => self.strike_grave_bell(),
                Some(BardPhraseKind::SilentHold) => self.sustain_hold(),
                Some(BardPhraseKind::RefusalRest) => self.refusal_rest(),
                None => {}
            }
        }
    }

    fn strike_grave_bell(&mut self) {
        for note in self.harmonics.phrase_notes(BardPhraseKind::MinorThirdDescent).into_iter().flatten() {
            self.synth.note_on(note, BELL_VEL, BELL_DUR_MS);
        }
    }
    fn sustain_hold(&mut self) {
        self.transport.play();
        self.loops.set_intensity(HOLD_INTENSITY);
    }
    fn refusal_rest(&mut self) { self.synth.all_notes_off(); }

    /// Render one block: synth + loop bed + hall tail, additively into `out`. Zero-heap.
    pub fn render(&mut self, out: &mut [f32], loop_buffers: &[&[f32]]) {
        self.synth.render(out);
        self.loops.mix_into(out, loop_buffers);
        self.reverb.process_block(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn ev(phrase: BardPhraseKind) -> ScheduledEvent {
        ScheduledEvent { fire_tick: 0, tag: phrase.as_u8() as u32, ..Default::default() }
    }

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
    }

    #[test]
    fn minor_third_descent_strikes_two_voices() {
        let mut lane = AudioLane::new(SR, 120.0);
        lane.apply(&[ev(BardPhraseKind::MinorThirdDescent)], 0);
        assert_eq!(lane.synth.active_voices(), 2, "bell should strike root + minor third");
        let mut out = vec![0.0f32; 4800];
        lane.render(&mut out, &[]);
        assert!(rms(&out) > 0.01, "bell should be audible, rms={}", rms(&out));
    }

    #[test]
    fn silent_hold_runs_transport_no_note() {
        let mut lane = AudioLane::new(SR, 120.0);
        assert!(!lane.transport.is_playing());
        lane.apply(&[ev(BardPhraseKind::SilentHold)], 0);
        assert!(lane.transport.is_playing(), "SilentHold → transport plays");
        assert_eq!(lane.synth.active_voices(), 0, "SilentHold rings no synth note");
    }

    #[test]
    fn refusal_rest_releases_all_voices() {
        let mut lane = AudioLane::new(SR, 120.0);
        lane.apply(&[ev(BardPhraseKind::MinorThirdDescent)], 0);
        assert_eq!(lane.synth.active_voices(), 2);
        lane.apply(&[ev(BardPhraseKind::RefusalRest)], 1);
        let mut out = vec![0.0f32; SR as usize];
        lane.render(&mut out, &[]);
        assert_eq!(lane.synth.active_voices(), 0, "RefusalRest releases every voice");
    }

    #[test]
    fn unknown_phrase_is_a_noop() {
        let mut lane = AudioLane::new(SR, 120.0);
        let bogus = ScheduledEvent { fire_tick: 0, tag: 222, ..Default::default() };
        lane.apply(&[bogus], 0);
        assert_eq!(lane.synth.active_voices(), 0);
        assert!(!lane.transport.is_playing());
    }

    #[test]
    fn facade_drives_voice_transport_and_bed() {
        let mut lane = AudioLane::new(SR, 120.0);
        lane.trigger_note(60, 100, 500);
        assert_eq!(lane.active_voices(), 1);
        lane.silence();
        lane.play();
        assert!(lane.transport.is_playing());
        lane.seek_beat(2);
        assert_eq!(lane.transport.playhead, 2 * lane.transport.grid.beat_interval as u64);
        lane.stop();
        assert!(!lane.transport.is_playing());
        let v = lane.add_loop_voice(29, 0, 0.0);
        assert_eq!(v, Some(0));
        lane.set_bed_intensity(1.0);
        lane.sync();
    }

    #[test]
    fn batch_of_events_applies_in_order() {
        let mut lane = AudioLane::new(SR, 120.0);
        lane.apply(&[ev(BardPhraseKind::SilentHold), ev(BardPhraseKind::MinorThirdDescent)], 0);
        assert!(lane.transport.is_playing(), "hold ran");
        assert_eq!(lane.synth.active_voices(), 2, "bell struck after hold");
    }

    #[test]
    fn daw_surface_artifact_is_signed_and_readback_proves_the_rendered_buffer() {
        // V1 WIDE-PARITY: the `daw` surface artifact. This AudioLane IS the live
        // conductor AUDIO EXECUTOR (the L0 leg of the Conductor spine): apply a
        // scheduled phrase -> render a 48kHz f32 buffer, the exact apply->render the
        // quarry ConductorFeeder drives. That buffer is the surface artifact ->
        // shared provenance seam -> a signed, user-owned file; an ADR-0008 readback
        // proves it is AUDIBLE (the grave bell rang) vs a silent control. Negative
        // control = planted fault. SoT: ROADMAP velocity_milestones V0/V1 ("daw").
        // (The full tick-SCHEDULER port — forge-semantic Conductor/ExecLane/dispatch
        // -> forge-sieve/forge-consequence — is the V7 lane, NOT faked here.)
        use forge_evidence::provenance::{ProvenanceCompiler, AssetType};

        // REAL render: schedule the grave-bell phrase on the executor, render 0.1s.
        let mut lane = AudioLane::new(SR, 120.0);
        lane.apply(&[ev(BardPhraseKind::MinorThirdDescent)], 0);
        let mut buf = vec![0.0f32; 4800];
        lane.render(&mut buf, &[]);

        // SILENT control: nothing scheduled -> the same render yields ~silence.
        let mut silent_lane = AudioLane::new(SR, 120.0);
        let mut silent = vec![0.0f32; 4800];
        silent_lane.render(&mut silent, &[]);

        let mut bytes = Vec::with_capacity(buf.len() * 4);
        for &s in &buf { bytes.extend_from_slice(&s.to_le_bytes()); }

        let chain = std::env::temp_dir()
            .join(format!("forgeaudio_daw_parity_chain_{}.jsonl", std::process::id()));
        let art = std::path::PathBuf::from("F:/output/parity/daw/conductor-buffer.bin");
        let mut compiler = ProvenanceCompiler::new([23u8; 32], &chain).unwrap();
        let vk = compiler.verifying_key();
        let (written, receipt) = compiler
            .compile_bytes(&bytes, &art, AssetType::Audio, "13forge-daw-v0", 1715600002)
            .unwrap();

        // ADR-0008 READBACK DISCRIMINATOR: reload the buffer off disk; it must be
        // AUDIBLE (the bell rang) and louder than the silent control. Fails RED if
        // the executor rendered silence / no voice struck.
        let raw = std::fs::read(&written).unwrap();
        let reloaded: Vec<f32> = raw.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(reloaded, buf, "audio buffer round-trips byte-identical off disk");
        let rms_active = rms(&reloaded);
        let rms_silent = rms(&silent);
        assert!(rms_active > 0.01, "conductor rendered an audible bell, rms={rms_active}");
        assert!(rms_active > rms_silent,
            "active ({rms_active}) must exceed the silent control ({rms_silent})");

        // PROVENANCE + NEGATIVE CONTROL.
        assert!(ProvenanceCompiler::verify_receipt(&vk, &receipt, &written).unwrap(),
            "daw artifact carries a verifying Ed25519 receipt");
        let tamper = std::env::temp_dir()
            .join(format!("forgeaudio_daw_parity_tamper_{}.bin", std::process::id()));
        let mut bad = bytes.clone();
        bad[0] ^= 0x01;
        std::fs::write(&tamper, &bad).unwrap();
        assert!(!ProvenanceCompiler::verify_receipt(&vk, &receipt, &tamper).unwrap(),
            "tampered audio buffer must fail verify");

        std::fs::remove_file(&tamper).ok();
        std::fs::remove_file(&chain).ok();
    }
}
