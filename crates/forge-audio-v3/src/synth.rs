// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//! 8-voice polyphonic synthesizer — oscillators + ADSR envelope + noise.
//!
//! Triangle waveform default (glassy bell-like tone). Zero-heap: fixed `[Voice; 8]`,
//! `render` and `note_on` allocate nothing.

/// Waveform type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
    Noise,
}

/// ADSR envelope parameters.
#[derive(Clone, Debug)]
pub struct AdsrParams {
    pub attack: f32,   // @forge:allow_float
    pub decay: f32,    // @forge:allow_float
    pub sustain: f32,  // @forge:allow_float
    pub release: f32,  // @forge:allow_float
}

impl Default for AdsrParams {
    fn default() -> Self {
        // Crisp 6 ms onset, bell-like decay to low sustain, 0.5 s release tail.
        Self { attack: 0.006, decay: 0.16, sustain: 0.32, release: 0.5 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdsrStage {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

#[derive(Clone, Debug)]
struct AdsrState {
    stage: AdsrStage,
    level: f32, // @forge:allow_float
    time: f32,  // @forge:allow_float
}

impl Default for AdsrState {
    fn default() -> Self {
        Self { stage: AdsrStage::Off, level: 0.0, time: 0.0 }
    }
}

#[derive(Clone, Debug)]
struct Voice {
    note: u8,
    frequency: f32,   // @forge:allow_float
    phase: f32,       // @forge:allow_float
    adsr: AdsrState,
    active: bool,
    duration_ms: f32, // @forge:allow_float
    elapsed_ms: f32,  // @forge:allow_float
    noise_state: u64,
    vel: f32,         // @forge:allow_float
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            note: 0,
            frequency: 440.0,
            phase: 0.0,
            adsr: AdsrState::default(),
            active: false,
            duration_ms: 0.0,
            elapsed_ms: 0.0,
            noise_state: 12345,
            vel: 1.0,
        }
    }
}

/// 8-voice polyphonic synthesizer.
pub struct Synth {
    pub waveform: Waveform,
    pub adsr: AdsrParams,
    pub volume: f32,      // @forge:allow_float
    voices: [Voice; 8],
    sample_rate: f32,     // @forge:allow_float
}

impl Synth {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            waveform: Waveform::Triangle,
            adsr: AdsrParams::default(),
            volume: 0.5,
            voices: Default::default(),
            sample_rate: sample_rate as f32,
        }
    }

    /// Trigger a note. `duration_ms == 0` = held until [`Self::note_off`]. Steals oldest voice if full.
    ///
    /// Same-note retrigger RE-EXCITES the live voice instead of stacking a duplicate
    /// (a re-struck string law, 2026-07-05): a sustained brush drag re-strikes one
    /// note ~10-14x/s, and duplicate same-frequency voices sum coherently until the
    /// master clamp clips — the "scaling feedback" screech. Phase is kept on the
    /// re-strike (click-free); different notes still stack (polyphony untouched).
    pub fn note_on(&mut self, note: u8, velocity: u8, duration_ms: u32) {
        let freq = 440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0);
        let vel = (velocity as f32 / 127.0).clamp(0.0, 1.0);

        let attack = self.adsr.attack.max(0.001);
        if let Some(v) = self.voices.iter_mut().find(|v| v.active && v.note == note) {
            // Resume the attack at the voice's current level (attack level is
            // absolute in time, so seed time accordingly) — click-free re-strike.
            let level = v.adsr.level.clamp(0.0, 1.0);
            v.adsr = AdsrState { stage: AdsrStage::Attack, level, time: level * attack };
            v.duration_ms = duration_ms as f32;
            v.elapsed_ms = 0.0;
            v.vel = vel;
            return;
        }

        let idx = self.voices.iter().position(|v| !v.active).unwrap_or_else(|| {
            let mut steal = 0usize;
            let mut max_elapsed = -1.0_f32;
            for (i, v) in self.voices.iter().enumerate() {
                if v.elapsed_ms > max_elapsed {
                    max_elapsed = v.elapsed_ms;
                    steal = i;
                }
            }
            steal
        });

        self.voices[idx] = Voice {
            note,
            frequency: freq,
            phase: 0.0,
            adsr: AdsrState { stage: AdsrStage::Attack, level: 0.0, time: 0.0 },
            active: true,
            duration_ms: duration_ms as f32,
            elapsed_ms: 0.0,
            noise_state: note as u64 * 31337 + 1,
            vel,
        };
    }

    /// Release any active voice sounding `note` (gate-off).
    pub fn note_off(&mut self, note: u8) {
        for v in &mut self.voices {
            if v.active
                && v.note == note
                && !matches!(v.adsr.stage, AdsrStage::Release | AdsrStage::Off)
            {
                v.adsr.stage = AdsrStage::Release;
                v.adsr.time = 0.0;
            }
        }
    }

    /// Release every voice (all-notes-off).
    pub fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            if v.active && !matches!(v.adsr.stage, AdsrStage::Off) {
                v.adsr.stage = AdsrStage::Release;
                v.adsr.time = 0.0;
            }
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Render mono samples, mixed **additively** into `output`. Zero-heap.
    pub fn render(&mut self, output: &mut [f32]) {
        let dt = 1.0 / self.sample_rate;
        let ms_per_sample = 1000.0 / self.sample_rate;

        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }
            let vel = voice.vel;

            for sample_out in output.iter_mut() {
                let sample = match self.waveform {
                    Waveform::Sine => (2.0 * core::f32::consts::PI * voice.phase).sin(),
                    Waveform::Square => {
                        if voice.phase.fract() < 0.5 { 1.0 } else { -1.0 }
                    }
                    Waveform::Saw => 2.0 * voice.phase.fract() - 1.0,
                    Waveform::Triangle => {
                        let t = voice.phase.fract();
                        if t < 0.5 { 4.0 * t - 1.0 } else { 3.0 - 4.0 * t }
                    }
                    Waveform::Noise => {
                        voice.noise_state ^= voice.noise_state << 13;
                        voice.noise_state ^= voice.noise_state >> 7;
                        voice.noise_state ^= voice.noise_state << 17;
                        (voice.noise_state & 0xFFFF) as f32 / 32768.0 - 1.0
                    }
                };

                let env = advance_adsr(&mut voice.adsr, &self.adsr, dt);
                *sample_out += sample * env * vel * self.volume;

                voice.phase += voice.frequency / self.sample_rate;

                voice.elapsed_ms += ms_per_sample;
                if voice.duration_ms > 0.0
                    && voice.elapsed_ms >= voice.duration_ms
                    && !matches!(voice.adsr.stage, AdsrStage::Release | AdsrStage::Off)
                {
                    voice.adsr.stage = AdsrStage::Release;
                    voice.adsr.time = 0.0;
                }
            }

            if matches!(voice.adsr.stage, AdsrStage::Off) {
                voice.active = false;
            }
        }
    }
}

fn advance_adsr(state: &mut AdsrState, params: &AdsrParams, dt: f32) -> f32 {
    match state.stage {
        AdsrStage::Attack => {
            state.time += dt;
            state.level = (state.time / params.attack.max(0.001)).min(1.0);
            if state.level >= 1.0 {
                state.stage = AdsrStage::Decay;
                state.time = 0.0;
            }
        }
        AdsrStage::Decay => {
            state.time += dt;
            let t = (state.time / params.decay.max(0.001)).min(1.0);
            state.level = 1.0 + (params.sustain - 1.0) * t;
            if t >= 1.0 {
                state.stage = AdsrStage::Sustain;
            }
        }
        AdsrStage::Sustain => {
            state.level = params.sustain;
        }
        AdsrStage::Release => {
            state.time += dt;
            let t = (state.time / params.release.max(0.001)).min(1.0);
            state.level = params.sustain * (1.0 - t);
            if t >= 1.0 {
                state.stage = AdsrStage::Off;
                state.level = 0.0;
            }
        }
        AdsrStage::Off => {
            state.level = 0.0;
        }
    }
    state.level
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
    }

    #[test]
    fn same_note_retrigger_reuses_the_voice_and_bounds_energy() {
        // The 2026-07-05 brush-drag law: N re-strikes of one note = ONE voice,
        // so sustained drawing can never stack coherent duplicates into the
        // clamp (the "scaling feedback" screech regression).
        let mut s = Synth::new(SR);
        for _ in 0..40 {
            s.note_on(64, 110, 150); // ~10-14x/s drag, compressed in time
        }
        assert_eq!(s.active_voices(), 1, "same-note retriggers must reuse one voice");

        // Different notes still stack — polyphony is untouched.
        s.note_on(67, 110, 150);
        s.note_on(72, 110, 150);
        assert_eq!(s.active_voices(), 3, "distinct notes keep full polyphony");

        // And the reused voice still makes sound.
        let mut out = vec![0.0f32; 4800];
        s.render(&mut out);
        assert!(rms(&out) > 0.01, "re-struck voice must produce energy");
    }

    #[test]
    fn note_on_produces_energy() {
        let mut s = Synth::new(SR);
        let mut out = vec![0.0f32; 4800];
        s.note_on(69, 127, 1000);
        s.render(&mut out);
        assert!(rms(&out) > 0.01, "silent after note_on, rms={}", rms(&out));
    }

    #[test]
    fn duration_zero_holds_until_note_off() {
        let mut s = Synth::new(SR);
        s.adsr = AdsrParams { attack: 0.001, decay: 0.001, sustain: 0.8, release: 0.01 };
        s.note_on(60, 127, 0);
        let mut out = vec![0.0f32; 2400];
        s.render(&mut out);
        assert_eq!(s.active_voices(), 1, "held note should still sound");
        s.note_off(60);
        let mut tail = vec![0.0f32; 4800];
        s.render(&mut tail);
        assert_eq!(s.active_voices(), 0, "note_off should release the held voice");
    }

    #[test]
    fn polyphony_up_to_eight() {
        let mut s = Synth::new(SR);
        for n in 60..68 {
            s.note_on(n, 100, 1000);
        }
        assert_eq!(s.active_voices(), 8);
        s.note_on(68, 100, 1000);
        assert_eq!(s.active_voices(), 8, "voice stealing caps at 8");
    }

    #[test]
    fn velocity_scales_output() {
        let mut soft = Synth::new(SR);
        let mut loud = Synth::new(SR);
        soft.adsr = AdsrParams { attack: 0.001, decay: 0.001, sustain: 1.0, release: 0.5 };
        loud.adsr = soft.adsr.clone();
        soft.note_on(69, 32, 1000);
        loud.note_on(69, 127, 1000);
        let mut a = vec![0.0f32; 2400];
        let mut b = vec![0.0f32; 2400];
        soft.render(&mut a);
        loud.render(&mut b);
        assert!(rms(&b) > rms(&a) * 2.0, "velocity 127 should be much louder than 32");
    }

    #[test]
    fn release_decays_to_silence() {
        let mut s = Synth::new(SR);
        s.adsr = AdsrParams { attack: 0.001, decay: 0.001, sustain: 0.9, release: 0.02 };
        s.note_on(72, 127, 5);
        let mut out = vec![0.0f32; SR as usize / 10];
        s.render(&mut out);
        assert_eq!(s.active_voices(), 0, "voice should have freed after release");
        let tail = &out[out.len() - 480..];
        assert!(rms(tail) < 1e-4, "tail not silent: {}", rms(tail));
    }

    #[test]
    fn instrument_rack_is_total_and_named() {
        for (ask, want) in [("pad", "pad"), ("guitar", "pluck"), ("bell", "glass"), ("808", "drums"), ("", "piano"), ("???", "piano")] {
            assert_eq!(instrument(ask).name, want);
        }
    }
}

// ── The instrument rack — name → (Waveform ⊕ AdsrParams ⊕ trim ⊕ strike length).
// ONE home (fold 07-11): was mirrored verbatim in termi/src/lib.rs and
// technothesia/src/synth.rs. Add packs here.

/// A struck-note voice: waveform + ADSR + trim + auto-release duration.
pub struct Instrument {
    pub name: &'static str,
    pub wave: Waveform,
    pub adsr: AdsrParams,
    pub gain: f32,
    pub dur_ms: u32,
}

/// Map a name to a synth voice. Same table the pen-instrument and
/// step-sequencer drive — not a parallel synth.
pub fn instrument(name: &str) -> Instrument {
    match name.trim().to_ascii_lowercase().as_str() {
        // Soft, lush, slow-blooming pad.
        "pad" | "soft" => Instrument {
            name: "pad",
            wave: Waveform::Saw,
            adsr: AdsrParams { attack: 0.08, decay: 0.30, sustain: 0.70, release: 1.20 },
            gain: 0.55,
            dur_ms: 600,
        },
        // Short percussive attack, no sustain — a plucked string.
        "pluck" | "guitar" => Instrument {
            name: "pluck",
            wave: Waveform::Triangle,
            adsr: AdsrParams { attack: 0.002, decay: 0.18, sustain: 0.0, release: 0.25 },
            gain: 0.90,
            dur_ms: 220,
        },
        // Pure sine, long shimmering decay — bell / glass.
        "glass" | "bell" => Instrument {
            name: "glass",
            wave: Waveform::Sine,
            adsr: AdsrParams { attack: 0.004, decay: 0.50, sustain: 0.10, release: 0.80 },
            gain: 0.80,
            dur_ms: 500,
        },
        // Noise burst, instant decay — a percussive tick.
        "drums" | "808" => Instrument {
            name: "drums",
            wave: Waveform::Noise,
            adsr: AdsrParams { attack: 0.001, decay: 0.08, sustain: 0.0, release: 0.05 },
            gain: 0.70,
            dur_ms: 90,
        },
        // Default: triangle + instrument-responsive ADSR — piano-ish.
        _ => Instrument {
            name: "piano",
            wave: Waveform::Triangle,
            adsr: AdsrParams::default(),
            gain: 0.85,
            dur_ms: 360,
        },
    }
}
