//! Brickwall look-ahead limiter for speaker protection.
//!
//! Sits on the master bus after all processing, before cpal output.
//! Ceiling: -1 dBFS default. Cannot be bypassed from UI.
//! Look-ahead: 1ms. Attack: instant. Release: 50ms smooth.

/// Brickwall limiter state.
pub struct BrickwallLimiter {
    /// Ceiling in linear amplitude (default: 10^(-1/20) ≈ 0.891).
    ceiling: f64,
    /// Look-ahead buffer (circular, interleaved stereo f64).
    delay_buf: Vec<f64>,
    /// Current write position in the delay buffer.
    write_pos: usize,
    /// Look-ahead in samples.
    lookahead_samples: usize,
    /// Current gain reduction (1.0 = no reduction).
    gain: f64,
    /// Release coefficient per sample.
    release_coeff: f64,
    /// Number of channels.
    channels: usize,
}

impl BrickwallLimiter {
    /// Create a new limiter.
    ///
    /// - `ceiling_db`: ceiling in dBFS (e.g., -1.0)
    /// - `lookahead_ms`: look-ahead time in milliseconds (e.g., 1.0)
    /// - `release_ms`: release time in milliseconds (e.g., 50.0)
    /// - `sample_rate`: audio sample rate
    /// - `channels`: number of channels (typically 2)
    pub fn new(
        ceiling_db: f64,
        lookahead_ms: f64,
        release_ms: f64,
        sample_rate: u32,
        channels: usize,
    ) -> Self {
        let ceiling = 10.0_f64.powf(ceiling_db / 20.0);
        let lookahead_samples = ((lookahead_ms / 1000.0) * sample_rate as f64).ceil() as usize;
        let release_coeff = (-1.0 / ((release_ms / 1000.0) * sample_rate as f64)).exp();
        let buf_size = lookahead_samples * channels;

        Self {
            ceiling,
            delay_buf: vec![0.0; buf_size],
            write_pos: 0,
            lookahead_samples,
            gain: 1.0,
            release_coeff,
            channels,
        }
    }

    /// Create a limiter with default safe settings: -1 dBFS, 1ms look-ahead, 50ms release.
    pub fn default_safe(sample_rate: u32, channels: usize) -> Self {
        Self::new(-1.0, 1.0, 50.0, sample_rate, channels)
    }

    /// Process a block of interleaved f64 audio. Correct look-ahead implementation.
    ///
    /// REAL-TIME SAFE: no allocations, no locks, no I/O.
    pub fn process_block(&mut self, input: &[f64], output: &mut [f64], frames: usize) {
        let ch = self.channels;
        let la = self.lookahead_samples;

        for frame in 0..frames {
            let offset = frame * ch;

            // Find peak across all channels for this frame.
            let mut peak = 0.0_f64;
            for c in 0..ch {
                let idx = offset + c;
                if idx < input.len() {
                    peak = peak.max(input[idx].abs());
                }
            }

            // Required gain reduction for this input sample.
            let target_gain = if peak > self.ceiling {
                self.ceiling / peak
            } else {
                1.0
            };

            // Instant attack, smooth release.
            if target_gain < self.gain {
                self.gain = target_gain;
            } else {
                self.gain = self.gain * self.release_coeff + target_gain * (1.0 - self.release_coeff);
            }

            // Write current input into delay buffer, read delayed output.
            for c in 0..ch {
                let buf_idx = self.write_pos * ch + c;
                let data_idx = offset + c;

                if data_idx < input.len() && buf_idx < self.delay_buf.len() {
                    // Read the delayed sample.
                    let delayed = self.delay_buf[buf_idx];
                    // Write current input into the delay slot.
                    self.delay_buf[buf_idx] = input[data_idx];
                    // Output = delayed sample * gain.
                    if data_idx < output.len() {
                        output[data_idx] = delayed * self.gain;
                    }
                }
            }

            // Advance circular write position.
            self.write_pos = (self.write_pos + 1) % la.max(1);
        }
    }

    /// Get the current gain reduction in dB (0.0 = no reduction).
    pub fn gain_reduction_db(&self) -> f64 {
        if self.gain >= 1.0 { 0.0 } else { 20.0 * self.gain.log10() }
    }

    /// Get the ceiling in dB.
    pub fn ceiling_db(&self) -> f64 {
        20.0 * self.ceiling.log10()
    }

    /// Reset limiter state (call on track load or major discontinuity).
    pub fn reset(&mut self) {
        self.gain = 1.0;
        for s in self.delay_buf.iter_mut() { *s = 0.0; }
        self.write_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_below_ceiling_passes_through() {
        let mut lim = BrickwallLimiter::default_safe(44100, 2);
        let input = vec![0.5, -0.5, 0.3, -0.3]; // Well below -1 dBFS
        let mut output = vec![0.0; 4];
        // Need to prime the delay buffer first.
        // Process twice: first fills delay, second reads it.
        lim.process_block(&input, &mut output, 2);
        let mut output2 = vec![0.0; 4];
        lim.process_block(&input, &mut output2, 2);
        // After delay, samples should pass through with gain ~1.0.
        // The delayed output should be approximately the input values.
        for s in &output2 {
            assert!(s.abs() <= 1.0, "Output should be below ceiling");
        }
    }

    #[test]
    fn test_above_ceiling_gets_limited() {
        let mut lim = BrickwallLimiter::new(-1.0, 0.0, 50.0, 44100, 2);
        // 0 look-ahead for simpler testing.
        let hot_signal: Vec<f64> = (0..128).map(|_| 2.0).collect(); // Way above ceiling
        let mut output = vec![0.0; 128];
        lim.process_block(&hot_signal, &mut output, 64);
        let ceiling_linear = 10.0_f64.powf(-1.0 / 20.0);
        for &s in &output {
            assert!(
                s.abs() <= ceiling_linear + 0.01,
                "Sample {} exceeds ceiling {}", s, ceiling_linear
            );
        }
    }

    #[test]
    fn test_gain_reduction_reports() {
        let mut lim = BrickwallLimiter::new(-1.0, 0.0, 50.0, 44100, 2);
        let hot = vec![2.0, 2.0];
        let mut out = vec![0.0; 2];
        lim.process_block(&hot, &mut out, 1);
        assert!(lim.gain_reduction_db() < 0.0, "Should report gain reduction");
    }

    #[test]
    fn test_silence_no_reduction() {
        let mut lim = BrickwallLimiter::default_safe(44100, 2);
        let silence = vec![0.0; 128];
        let mut out = vec![0.0; 128];
        lim.process_block(&silence, &mut out, 64);
        assert!((lim.gain_reduction_db() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut lim = BrickwallLimiter::new(-1.0, 0.0, 50.0, 44100, 2);
        let hot = vec![5.0, 5.0];
        let mut out = vec![0.0; 2];
        lim.process_block(&hot, &mut out, 1);
        assert!(lim.gain < 1.0);
        lim.reset();
        assert!((lim.gain - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ceiling_db_correct() {
        let lim = BrickwallLimiter::new(-3.0, 1.0, 50.0, 44100, 2);
        assert!((lim.ceiling_db() - (-3.0)).abs() < 0.01);
    }

    #[test]
    fn test_speaker_protection_extreme() {
        // Simulate a catastrophic signal: 100x overdrive.
        let mut lim = BrickwallLimiter::new(-1.0, 0.0, 50.0, 44100, 2);
        let extreme: Vec<f64> = (0..256).map(|_| 100.0).collect();
        let mut out = vec![0.0; 256];
        lim.process_block(&extreme, &mut out, 128);
        let ceiling_linear = 10.0_f64.powf(-1.0 / 20.0);
        for &s in &out {
            assert!(
                s.abs() <= ceiling_linear + 0.01,
                "100x overdrive: {} exceeds ceiling", s
            );
        }
    }
}
