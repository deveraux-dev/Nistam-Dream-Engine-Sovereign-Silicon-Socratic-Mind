#![allow(clippy::disallowed_types)] // @forge:allow_alloc — cold-path module, init-time allocations permitted
//! Audio Backend Bridge — connects PhotometricEngine AudioCommands to forge-audio.
//!
//! Synthesizes short PCM one-shot buffers from AudioMaterialProfile parameters,
//! loads them into the mixer's sampler slots, and triggers playback.
//! Uses the existing LoadSampler/TriggerSampler MixerCommand path.
//!
//! MVP Vertical Slice: Milestone 3 — "hear something"
//! Inventions: #8 (CE), #23 (Color→Audio Synesthesia), #34 (Procedural Audio)

use crate::bus::command_tx::AudioCommandTx;
use forge_photometric::audio_types::{AudioCommand, AudioMaterialProfile};
use forge_photometric::sound_consumer::AudioBackend;
use crate::recipe::RecipeEngine;
use crate::dsp::AudioBuffer;
use crate::bus::command::MixerCommand;

/// Sample rate for synthesized SFX.
const SAMPLE_RATE: u32 = 44100;

/// Duration of synthesized one-shot SFX in seconds.
const SFX_DURATION_SECS: f32 = 0.15;

/// Number of sampler slots available (mixer has 8).
const MAX_SAMPLER_SLOTS: usize = 8;

/// Bridge from PhotometricEngine AudioCommands to forge-audio mixer samplers.
///
/// For each AudioCommand:
/// 1. Synthesize a short PCM buffer from the AudioMaterialProfile
/// 2. Load it into a sampler slot via LoadSampler
/// 3. Trigger playback via TriggerSampler
///
/// Slots are used round-robin so concurrent SFX don't stomp each other.
pub struct ForgeAudioBackend {
    tx: AudioCommandTx,
    /// Next sampler slot to use (round-robin 0..7).
    next_slot: usize,
    /// Running count of dispatched commands.
    pub commands_dispatched: u64,
    /// Recipe engine for material-specific synthesis.
    recipe_engine: RecipeEngine,
    /// Seed counter for deterministic synthesis.
    seed_counter: u64,
}

impl ForgeAudioBackend {
    pub fn new(tx: AudioCommandTx) -> Self {
        println!("[AUDIO] ForgeAudioBackend initialized with RecipeEngine.");
        Self {
            tx,
            next_slot: 0,
            commands_dispatched: 0,
            recipe_engine: RecipeEngine::new(),
            seed_counter: 0,
        }
    }

    /// Update vibe modulation signals from weather bridge.
    pub fn update_vibe(&mut self, fog_density: f32, chromatic_aberration: f32, artifact_glow: f32, distortion: f32) {
        self.recipe_engine.update_vibe(fog_density, chromatic_aberration, artifact_glow, distortion);
    }

    /// Update era index and brand distortion level.
    pub fn update_era(&mut self, era_index: u8, brand_level: u8) {
        self.recipe_engine.update_era(era_index, brand_level);
    }
}

/// Synthesize a short one-shot PCM buffer from an AudioMaterialProfile.
///
/// Uses the RecipeEngine for material-specific synthesis when available,
/// falling back to basic sine+harmonics synthesis on failure.
fn synthesize_sfx(profile: &AudioMaterialProfile, intensity_db: f32, recipe_engine: &mut RecipeEngine, seed: u64) -> AudioBuffer {
    let engine_profile: crate::recipe::AudioMaterialProfile = profile.into();
    let pcm = recipe_engine.synthesize(
        &engine_profile,
        intensity_db,
        crate::recipe::SoundSource::Impact, // default source for basic dispatch
        None,                // no material bitmask in basic path
        seed,
    );

    AudioBuffer {
        samples: vec![pcm],
        sample_rate: SAMPLE_RATE,
    }
}

/// Synthesize using the basic fallback (no recipe engine).
fn synthesize_sfx_basic(profile: &AudioMaterialProfile, intensity_db: f32) -> AudioBuffer {
    let num_samples = (SAMPLE_RATE as f32 * SFX_DURATION_SECS) as usize;
    let mut samples = vec![0.0f32; num_samples];

    let freq = profile.ring_frequency_hz;
    let attack = profile.attack_sharpness;
    let harmonics = profile.harmonic_content;
    let decay = profile.decay_secs.max(0.01);

    // Linear volume from dB, clamped
    let volume = (10.0f32).powf(intensity_db / 20.0).clamp(0.0, 1.0);

    let inv_sr = 1.0 / SAMPLE_RATE as f32;

    for i in 0..num_samples {
        let t = i as f32 * inv_sr;

        // Envelope: sharp attack + exponential decay
        // attack_sharpness controls how fast the onset is (1.0 = instant, 0.0 = slow fade-in)
        let attack_time = (1.0 - attack) * 0.02 + 0.001; // 1ms to 21ms
        let attack_env = (t / attack_time).min(1.0);
        let decay_env = (-t / decay).exp();
        let envelope = attack_env * decay_env;

        // Fundamental sine
        let phase = 2.0 * std::f32::consts::PI * freq * t;
        let mut signal = phase.sin();

        // Add odd harmonics (3rd, 5th, 7th) scaled by harmonic_content
        if harmonics > 0.01 {
            signal += (phase * 3.0).sin() * harmonics * 0.5;
            signal += (phase * 5.0).sin() * harmonics * 0.25;
            signal += (phase * 7.0).sin() * harmonics * 0.125;
        }

        // Normalize the harmonic sum
        let max_amp = 1.0 + harmonics * (0.5 + 0.25 + 0.125);
        signal /= max_amp;

        samples[i] = signal * envelope * volume;
    }

    AudioBuffer {
        samples: vec![samples], // mono
        sample_rate: SAMPLE_RATE,
    }
}

impl AudioBackend for ForgeAudioBackend {
    fn dispatch(&mut self, commands: &[AudioCommand]) -> usize {
        let mut dispatched = 0;

        for cmd in commands {
            let slot = self.next_slot;
            self.next_slot = (self.next_slot + 1) % MAX_SAMPLER_SLOTS;

            // Advance deterministic seed
            self.seed_counter = self.seed_counter.wrapping_add(1);

            // 1. Synthesize PCM using RecipeEngine (material-specific)
            let buffer = synthesize_sfx(&cmd.profile, cmd.intensity_db, &mut self.recipe_engine, self.seed_counter);

            // 2. Load into sampler slot
            let _ = self.tx.send(MixerCommand::LoadSampler {
                slot,
                buffer,
            });

            // 3. Trigger playback
            let _ = self.tx.send(MixerCommand::TriggerSampler { slot });

            dispatched += 1;
        }

        self.commands_dispatched += dispatched as u64;
        dispatched
    }

    fn is_real(&self) -> bool {
        true
    }

    fn update_vibe(&mut self, fog: f32, aberration: f32, glow: f32, distortion: f32) {
        self.recipe_engine.update_vibe(fog, aberration, glow, distortion);
    }

    fn update_era(&mut self, era_index: u8, brand_level: u8) {
        self.recipe_engine.update_era(era_index, brand_level);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use forge_photometric::sound_consumer::DEFAULT_ORGANIC_PROFILE;
    use forge_photometric::audio_types::SoundSource;

    #[test]
    fn synthesize_produces_correct_length() {
        let buf = synthesize_sfx_basic(&DEFAULT_ORGANIC_PROFILE, -10.0);
        let expected = (SAMPLE_RATE as f32 * SFX_DURATION_SECS) as usize;
        assert_eq!(buf.samples.len(), 1); // mono
        assert_eq!(buf.samples[0].len(), expected);
        assert_eq!(buf.sample_rate, SAMPLE_RATE);
    }

    #[test]
    fn synthesize_iron_has_higher_frequency_content() {
        let iron = AudioMaterialProfile {
            ring_frequency_hz: 15000.0,
            attack_sharpness: 0.9,
            harmonic_content: 0.8,
            decay_secs: 0.5,
            reverb_amount: 0.1,
        };
        let void = AudioMaterialProfile {
            ring_frequency_hz: 60.0,
            attack_sharpness: 0.1,
            harmonic_content: 0.1,
            decay_secs: 0.05,
            reverb_amount: 0.9,
        };

        let iron_buf = synthesize_sfx_basic(&iron, -10.0);
        let void_buf = synthesize_sfx_basic(&void, -10.0);

        // Iron should have more zero-crossings (higher frequency)
        let iron_crossings = count_zero_crossings(&iron_buf.samples[0]);
        let void_crossings = count_zero_crossings(&void_buf.samples[0]);
        assert!(
            iron_crossings > void_crossings,
            "Iron ({}) should have more zero-crossings than void ({})",
            iron_crossings, void_crossings
        );
    }

    #[test]
    fn synthesize_envelope_decays() {
        // Use a profile with sharp attack so the envelope clearly decays
        let sharp_profile = AudioMaterialProfile {
            ring_frequency_hz: 200.0,
            attack_sharpness: 0.9, // sharp attack = fast onset
            harmonic_content: 0.1,
            decay_secs: 0.1, // short decay makes the difference more obvious
            reverb_amount: 0.1,
        };
        let buf = synthesize_sfx_basic(&sharp_profile, -10.0);
        let samples = &buf.samples[0];
        let n = samples.len();

        // RMS of first 20% should be higher than RMS of last 20%
        let first_rms = rms(&samples[..n / 5]);
        let last_rms = rms(&samples[n * 4 / 5..]);
        assert!(
            first_rms > last_rms,
            "First 20% RMS ({:.4}) should be louder than last 20% ({:.4})",
            first_rms, last_rms
        );
    }

    #[test]
    fn synthesize_silent_at_minus_infinity() {
        let buf = synthesize_sfx_basic(&DEFAULT_ORGANIC_PROFILE, -100.0);
        let peak = buf.samples[0].iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(peak < 0.001, "At -100dB, peak should be near zero, got {}", peak);
    }

    #[test]
    fn dispatch_sends_load_then_trigger() {
        let (tx_cmd, rx) = crossbeam_channel::unbounded();
        let tx = AudioCommandTx::new(tx_cmd);
        let mut backend = ForgeAudioBackend::new(tx);

        let cmd = AudioCommand {
            position: [0.0; 3],
            profile: DEFAULT_ORGANIC_PROFILE,
            intensity_db: -10.0,
            source: SoundSource::Impact,
        };

        let count = backend.dispatch(&[cmd]);
        assert_eq!(count, 1);

        // Should have sent LoadSampler then TriggerSampler
        let first = rx.try_recv().unwrap();
        assert!(matches!(first, MixerCommand::LoadSampler { slot: 0, .. }));

        let second = rx.try_recv().unwrap();
        assert!(matches!(second, MixerCommand::TriggerSampler { slot: 0 }));
    }

    #[test]
    fn dispatch_round_robins_slots() {
        let (tx_cmd, rx) = crossbeam_channel::unbounded();
        let tx = AudioCommandTx::new(tx_cmd);
        let mut backend = ForgeAudioBackend::new(tx);

        let cmd = AudioCommand {
            position: [0.0; 3],
            profile: DEFAULT_ORGANIC_PROFILE,
            intensity_db: -10.0,
            source: SoundSource::Impact,
        };

        // Dispatch 3 commands — should use slots 0, 1, 2
        backend.dispatch(&[cmd.clone(), cmd.clone(), cmd.clone()]);

        let mut slots = Vec::new();
        while let Ok(c) = rx.try_recv() {
            if let MixerCommand::LoadSampler { slot, .. } = c {
                slots.push(slot);
            }
        }
        assert_eq!(slots, vec![0, 1, 2]);
    }

    #[test]
    fn is_real_returns_true() {
        let (tx_cmd, _rx) = crossbeam_channel::unbounded();
        let tx = AudioCommandTx::new(tx_cmd);
        let backend = ForgeAudioBackend::new(tx);
        assert!(backend.is_real());
    }

    // Helpers
    fn count_zero_crossings(samples: &[f32]) -> usize {
        samples.windows(2).filter(|w| w[0].signum() != w[1].signum()).count()
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() { return 0.0; }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }
}
