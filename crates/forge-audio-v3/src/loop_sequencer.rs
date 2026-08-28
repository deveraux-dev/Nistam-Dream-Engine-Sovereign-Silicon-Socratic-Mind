// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//! Layered coprime-loop sequencer.
//!
//! Coprime lengths \[29,43,61,97,113\] mirror `forge-harmonics::harmonic_threads`
//! without a cargo dep — cited, not shared, to keep the forge-audio firewall intact.
//!
//! Two axes:
//! - **Horizontal (coprime phases):** N voices loop at pairwise-coprime lengths.
//! - **Vertical (intensity crossfade):** each voice has an activation threshold + slewed gain.

pub const MAX_LAYERS: usize = 8;

/// Coprime loop lengths in seconds. All prime → pairwise coprime.
/// Mirror of `forge-harmonics::harmonic_threads::RECOMMENDED_LOOP_SECS`.
pub const COPRIME_LOOP_SECS: [u32; 5] = [29, 43, 61, 97, 113];

#[derive(Clone, Copy)]
pub struct LoopVoice {
    pub len_samples: u64,
    pub phase: u64,
    pub phase_offset: u64,
    pub activation: f32,    // @forge:allow_float
    pub target_gain: f32,   // @forge:allow_float
    pub current_gain: f32,  // @forge:allow_float
}

impl LoopVoice {
    fn silent() -> Self {
        Self {
            len_samples: 1,
            phase: 0,
            phase_offset: 0,
            activation: 0.0,
            target_gain: 0.0,
            current_gain: 0.0,
        }
    }
}

/// Deterministic position within a voice's loop at a global sample tick.
/// Mirror of `forge-harmonics::harmonic_threads::loop_phase`.
#[inline]
pub fn loop_phase(voice: &LoopVoice, tick: u64) -> u64 {
    let len = voice.len_samples.max(1);
    tick.wrapping_add(voice.phase_offset) % len
}

pub struct LoopSequencer {
    voices: [LoopVoice; MAX_LAYERS],
    count: usize,
    slew_per_sample: f32, // @forge:allow_float
}

impl LoopSequencer {
    /// `crossfade_secs` = time to fade a layer fully in/out (MusicSequencer default 2s).
    pub fn new(crossfade_secs: f32, sample_rate: u32) -> Self {
        let secs = crossfade_secs.max(0.001);
        Self {
            voices: [LoopVoice::silent(); MAX_LAYERS],
            count: 0,
            slew_per_sample: 1.0 / (secs * sample_rate as f32),
        }
    }

    /// Add a voice. Returns its index, or `None` if full.
    pub fn add_voice(&mut self, len_samples: u64, phase_offset: u64, activation: f32) -> Option<usize> {
        if self.count >= MAX_LAYERS {
            return None;
        }
        let len = len_samples.max(1);
        let idx = self.count;
        self.voices[idx] = LoopVoice {
            len_samples: len,
            phase: phase_offset % len,
            phase_offset,
            activation: activation.clamp(0.0, 1.0),
            target_gain: 0.0,
            current_gain: 0.0,
        };
        self.count += 1;
        Some(idx)
    }

    /// Coprime loop length in samples for layer `slot` (cycles through the 5 primes).
    pub fn coprime_len(slot: usize, sample_rate: u32) -> u64 {
        COPRIME_LOOP_SECS[slot % COPRIME_LOOP_SECS.len()] as u64 * sample_rate as u64
    }

    /// Vertical crossfade: voices above `intensity` activation fade toward full gain.
    pub fn set_intensity(&mut self, intensity: f32) {
        let intensity = intensity.clamp(0.0, 1.0);
        for v in self.voices[..self.count].iter_mut() {
            v.target_gain = if intensity >= v.activation { 1.0 } else { 0.0 };
        }
    }

    /// Hard-sync: snap every voice phase back to its offset (ritual anchor / downbeat).
    pub fn sync_all(&mut self) {
        for v in self.voices[..self.count].iter_mut() {
            v.phase = v.phase_offset % v.len_samples.max(1);
        }
    }

    pub fn len(&self) -> usize { self.count }
    pub fn is_empty(&self) -> bool { self.count == 0 }

    /// Mix every voice's loop window additively into `out`. Zero-heap.
    pub fn mix_into(&mut self, out: &mut [f32], buffers: &[&[f32]]) {
        for sample in out.iter_mut() {
            let mut mix = 0.0_f32;
            for i in 0..self.count {
                let buf = match buffers.get(i) {
                    Some(b) if !b.is_empty() => *b,
                    _ => continue,
                };
                let v = &mut self.voices[i];

                if v.current_gain < v.target_gain {
                    v.current_gain = (v.current_gain + self.slew_per_sample).min(v.target_gain);
                } else if v.current_gain > v.target_gain {
                    v.current_gain = (v.current_gain - self.slew_per_sample).max(v.target_gain);
                }

                if v.current_gain > 0.0001 {
                    let idx = (v.phase % buf.len() as u64) as usize;
                    mix += buf[idx] * v.current_gain;
                }

                v.phase += 1;
                if v.phase >= v.len_samples {
                    v.phase -= v.len_samples;
                }
            }
            *sample += mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn gcd(mut a: u64, mut b: u64) -> u64 {
        while b != 0 { let t = b; b = a % b; a = t; }
        a
    }

    #[test]
    fn coprime_lengths_are_pairwise_coprime() {
        for i in 0..COPRIME_LOOP_SECS.len() {
            for j in (i + 1)..COPRIME_LOOP_SECS.len() {
                assert_eq!(
                    gcd(COPRIME_LOOP_SECS[i] as u64, COPRIME_LOOP_SECS[j] as u64),
                    1,
                    "secs[{i}] and secs[{j}] must be coprime"
                );
            }
        }
    }

    #[test]
    fn coprime_voices_realign_only_at_lcm() {
        let a = LoopVoice { len_samples: 29, phase_offset: 0, ..silent_voice() };
        let b = LoopVoice { len_samples: 43, phase_offset: 0, ..silent_voice() };
        let lcm = 29 * 43;
        for tick in 1..lcm {
            let both_zero = loop_phase(&a, tick) == 0 && loop_phase(&b, tick) == 0;
            assert!(!both_zero, "voices realigned early at tick {tick}");
        }
        assert_eq!(loop_phase(&a, lcm), 0);
        assert_eq!(loop_phase(&b, lcm), 0);
    }

    #[test]
    fn intensity_crossfade_gates_layers_and_mixes() {
        let sr = 48_000u32;
        let mut seq = LoopSequencer::new(0.01, sr);
        seq.add_voice(29, 0, 0.0).unwrap();
        seq.add_voice(43, 0, 0.8).unwrap();

        let buf0 = vec![1.0f32; 64];
        let buf1 = vec![1.0f32; 64];
        let bufs: [&[f32]; 2] = [&buf0, &buf1];

        seq.set_intensity(0.3);
        let mut out = vec![0.0f32; sr as usize / 4];
        seq.mix_into(&mut out, &bufs);
        let tail = out[out.len() - 1];
        assert!((tail - 1.0).abs() < 0.05, "only layer 0 audible (~1.0), got {tail}");

        seq.set_intensity(1.0);
        let mut out2 = vec![0.0f32; sr as usize / 4];
        seq.mix_into(&mut out2, &bufs);
        let tail2 = out2[out2.len() - 1];
        assert!(tail2 > 1.8, "both layers audible (~2.0), got {tail2}");
    }

    #[test]
    fn sync_all_resets_phase_to_offset() {
        let sr = 48_000u32;
        let mut seq = LoopSequencer::new(1.0, sr);
        seq.add_voice(LoopSequencer::coprime_len(0, sr), 5, 0.0).unwrap();
        let buf = vec![0.0f32; 128];
        let bufs: [&[f32]; 1] = [&buf];
        let mut out = vec![0.0f32; 1000];
        seq.mix_into(&mut out, &bufs);
        assert_ne!(seq.voices[0].phase, 5);
        seq.sync_all();
        assert_eq!(seq.voices[0].phase, 5, "sync snaps phase back to offset");
    }

    fn silent_voice() -> LoopVoice {
        LoopVoice { len_samples: 1, phase: 0, phase_offset: 0, activation: 0.0, target_gain: 0.0, current_gain: 0.0 }
    }
}
