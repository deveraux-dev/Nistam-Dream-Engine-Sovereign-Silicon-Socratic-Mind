//! YIN — fundamental frequency (pitch) estimation via autocorrelation.

/// Track pitch through a mono signal, returning F0 in Hz per frame.
/// Returns 0.0 for unvoiced/silent frames.
pub fn yin_track(
    signal: &[f32],
    sample_rate: u32,
    window_size: usize,
    hop: usize,
    threshold: f32,
) -> Vec<f32> {
    let half_w = window_size / 2;
    let mut pitches = Vec::new();

    let mut pos = 0;
    while pos + window_size <= signal.len() {
        let frame = &signal[pos..pos + window_size];
        let pitch = yin_frame(frame, sample_rate, half_w, threshold);
        pitches.push(pitch);
        pos += hop;
    }
    pitches
}

fn yin_frame(frame: &[f32], sample_rate: u32, max_lag: usize, threshold: f32) -> f32 {
    // Step 1: Squared difference function
    let mut d = vec![0.0f32; max_lag];
    for tau in 1..max_lag {
        let mut sum = 0.0f32;
        for j in 0..max_lag {
            let diff = frame[j] - frame[j + tau];
            sum += diff * diff;
        }
        d[tau] = sum;
    }

    // Step 2: Cumulative mean normalized difference function (CMNDF)
    let mut d_prime = vec![0.0f32; max_lag];
    d_prime[0] = 1.0;
    let mut running_sum = 0.0f32;
    for tau in 1..max_lag {
        running_sum += d[tau];
        if running_sum > 0.0 {
            d_prime[tau] = d[tau] * tau as f32 / running_sum;
        } else {
            d_prime[tau] = 1.0;
        }
    }

    // Step 3: Absolute threshold — find first dip below threshold,
    // then walk to the local minimum of that valley.
    let min_lag = (sample_rate as f32 / 2000.0).max(2.0) as usize;
    let mut best_tau = 0;
    for tau in min_lag..max_lag {
        if d_prime[tau] < threshold {
            // Walk forward to find the bottom of this valley
            best_tau = tau;
            while best_tau + 1 < max_lag && d_prime[best_tau + 1] < d_prime[best_tau] {
                best_tau += 1;
            }
            break;
        }
    }

    if best_tau == 0 {
        return 0.0;
    }

    // Step 4: Parabolic interpolation
    let tau = best_tau;
    let refined = if tau > 0 && tau + 1 < max_lag {
        let a = d_prime[tau - 1];
        let b = d_prime[tau];
        let c = d_prime[tau + 1];
        let offset = (a - c) / (2.0 * (a - 2.0 * b + c));
        tau as f32 + offset
    } else {
        tau as f32
    };

    if refined > 0.0 {
        sample_rate as f32 / refined
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yin_detects_a440() {
        let sr = 44100;
        let n = 44100;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let pitches = yin_track(&signal, sr as u32, 2048, 512, 0.15);
        let valid: Vec<&f32> = pitches.iter().filter(|&&p| p > 0.0).collect();
        assert!(!valid.is_empty(), "Should detect pitch");
        let avg: f32 = valid.iter().copied().sum::<f32>() / valid.len() as f32;
        assert!(
            (avg - 440.0).abs() < 5.0,
            "Average pitch {} should be ~440Hz",
            avg
        );
    }

    #[test]
    fn test_yin_silence_returns_zero() {
        let signal = vec![0.0f32; 8192];
        let pitches = yin_track(&signal, 44100, 2048, 512, 0.15);
        for &p in &pitches {
            assert_eq!(p, 0.0, "Silence should yield 0Hz");
        }
    }

    #[test]
    fn test_yin_different_pitches() {
        let sr = 44100;
        let n = 22050;
        let mut signal = Vec::with_capacity(n * 2);
        for i in 0..n {
            signal.push((2.0 * std::f32::consts::PI * 220.0 * i as f32 / sr as f32).sin());
        }
        for i in 0..n {
            signal.push((2.0 * std::f32::consts::PI * 880.0 * i as f32 / sr as f32).sin());
        }
        let pitches = yin_track(&signal, sr as u32, 2048, 512, 0.15);
        let mid = pitches.len() / 2;
        let first_half: Vec<f32> = pitches[..mid]
            .iter()
            .filter(|&&p| p > 0.0)
            .copied()
            .collect();
        let second_half: Vec<f32> = pitches[mid..]
            .iter()
            .filter(|&&p| p > 0.0)
            .copied()
            .collect();
        if !first_half.is_empty() && !second_half.is_empty() {
            let avg1: f32 = first_half.iter().sum::<f32>() / first_half.len() as f32;
            let avg2: f32 = second_half.iter().sum::<f32>() / second_half.len() as f32;
            assert!(avg1 < 300.0, "First half avg {} should be ~220Hz", avg1);
            assert!(avg2 > 700.0, "Second half avg {} should be ~880Hz", avg2);
        }
    }
}
