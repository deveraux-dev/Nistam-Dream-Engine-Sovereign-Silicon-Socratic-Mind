//! Vocal-formant energy — LPC spectral envelope over the 300-3500Hz band.
//!
//! Levinson-Durbin core lands verbatim from Sean's donor implementation
//! (2026-08-24 session, scalar f32, zero-alloc, caller-owned scratch).
//! Autocorrelation + windowing + peak-picking wrap it to turn a signal
//! window into one Permyriad scalar for the vibe vector.

const LPC_ORDER: usize = 12;
const FORMANT_BAND_LO_HZ: f32 = 300.0;
const FORMANT_BAND_HI_HZ: f32 = 3500.0;
const ENVELOPE_BINS: usize = 128;
const NORM_BINS: usize = 256;

/// Solves Yule-Walker equations using Levinson-Durbin recursion.
///
/// # Arguments
/// * `autocorr` - Autocorrelation slice `[r_0, r_1, ..., r_p]`.
/// * `lpc_coeffs` - Output slice for prediction coefficients `[a_1, a_2, ..., a_p]`.
/// * `reflections` - Output slice for reflection coefficients `[k_1, k_2, ..., k_p]`.
/// * `scratch` - Scratchpad slice for internal state (minimum length = prediction order `p`).
///
/// Returns the residual prediction error power $E_p$.
pub fn levinson_durbin(
    autocorr: &[f32],
    lpc_coeffs: &mut [f32],
    reflections: &mut [f32],
    scratch: &mut [f32],
) -> Result<f32, &'static str> {
    let order = lpc_coeffs.len();

    if autocorr.len() <= order {
        return Err("Autocorrelation slice must be longer than prediction order");
    }
    if reflections.len() < order || scratch.len() < order {
        return Err("Output and scratch buffers must be at least as long as prediction order");
    }

    let r0 = autocorr[0];
    if r0 <= 0.0 {
        return Err("Signal energy r_0 must be strictly positive");
    }

    let mut error = r0;

    for i in 0..order {
        // 1. Calculate cross-correlation sum lambda
        let mut lambda = autocorr[i + 1];
        for j in 0..i {
            lambda += lpc_coeffs[j] * autocorr[i - j];
        }

        // 2. Compute reflection coefficient k_i
        let k = -lambda / error;
        reflections[i] = k;

        // 3. Verify filter stability (|k_i| < 1)
        if k.abs() >= 1.0 {
            return Err("Filter unstable: reflection coefficient magnitude >= 1.0");
        }

        // 4. Update LPC coefficients in-place using previous iteration state saved in scratch
        if i > 0 {
            scratch[..i].copy_from_slice(&lpc_coeffs[..i]);
            for j in 0..i {
                lpc_coeffs[j] = scratch[j] + k * scratch[i - 1 - j];
            }
        }
        lpc_coeffs[i] = k;

        // 5. Update prediction error energy E_i
        error *= 1.0 - k * k;
        if error <= 0.0 {
            return Err("Residual error power dropped to non-positive value");
        }
    }

    Ok(error)
}

/// Hamming-window a signal frame before autocorrelation (reduces spectral leakage).
fn apply_hamming(signal: &[f32]) -> Vec<f32> {
    let n = signal.len();
    if n == 0 {
        return Vec::new(); // @forge:allow_alloc worker-thread analysis, not RT callback
    }
    let nf = (n.max(2) - 1) as f32;
    signal
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let w = 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / nf).cos();
            s * w
        })
        .collect() // @forge:allow_alloc worker-thread analysis, not RT callback
}

/// Biased autocorrelation `[r_0, r_1, ..., r_max_lag]`.
fn autocorrelate(signal: &[f32], max_lag: usize) -> Vec<f32> {
    let n = signal.len();
    let mut r = vec![0.0f32; max_lag + 1]; // @forge:allow_alloc worker-thread analysis, not RT callback
    for (lag, slot) in r.iter_mut().enumerate() {
        let mut sum = 0.0f32;
        for i in 0..n.saturating_sub(lag) {
            sum += signal[i] * signal[i + lag];
        }
        *slot = sum;
    }
    r
}

/// LPC all-pole spectral envelope magnitude `|1 / A(e^jw)|` at one frequency.
fn spectral_envelope_bin(lpc_coeffs: &[f32], freq_hz: f32, sample_rate: f32) -> f32 {
    let w = 2.0 * std::f32::consts::PI * freq_hz / sample_rate;
    let mut re = 1.0f32;
    let mut im = 0.0f32;
    for (k, &a) in lpc_coeffs.iter().enumerate() {
        let kw = (k + 1) as f32 * w;
        re -= a * kw.cos();
        im += a * kw.sin();
    }
    let denom = (re * re + im * im).max(1e-12);
    1.0 / denom.sqrt()
}

/// Peak-pick the LPC spectral envelope in the vocal-formant band, normalized
/// against the envelope's overall peak, averaged across detected peaks, and
/// converted to Permyriad at the boundary.
fn formant_band_energy_pmy(lpc_coeffs: &[f32], sample_rate: f32) -> i32 {
    let nyquist = sample_rate * 0.5;
    let lo = FORMANT_BAND_LO_HZ.min(nyquist);
    let hi = FORMANT_BAND_HI_HZ.min(nyquist);
    if hi <= lo {
        return 0;
    }

    let mut global_max = 1e-6f32;
    for i in 0..NORM_BINS {
        let freq = nyquist * i as f32 / NORM_BINS as f32;
        let mag = spectral_envelope_bin(lpc_coeffs, freq, sample_rate);
        if mag > global_max {
            global_max = mag;
        }
    }

    let mut band_vals = [0.0f32; ENVELOPE_BINS];
    for (i, slot) in band_vals.iter_mut().enumerate() {
        let freq = lo + (hi - lo) * i as f32 / (ENVELOPE_BINS - 1) as f32;
        *slot = spectral_envelope_bin(lpc_coeffs, freq, sample_rate);
    }

    let mut peak_sum = 0.0f32;
    let mut peak_count = 0u32;
    for i in 1..band_vals.len() - 1 {
        if band_vals[i] > band_vals[i - 1] && band_vals[i] >= band_vals[i + 1] {
            peak_sum += band_vals[i] / global_max;
            peak_count += 1;
        }
    }
    if peak_count == 0 {
        return 0;
    }
    let avg_peak = (peak_sum / peak_count as f32).clamp(0.0, 1.0);
    (avg_peak * 10_000.0) as i32
}

/// Estimate vocal-formant energy in a signal window as Permyriad (0..10000).
/// Returns 0 for silence, unvoiced, or too-short input rather than erroring —
/// this feeds a continuous visual signal, not a hard DSP boundary.
pub fn formant_energy_pmy(signal: &[f32], sample_rate: u32) -> i32 {
    if signal.len() <= LPC_ORDER + 1 {
        return 0;
    }
    let windowed = apply_hamming(signal);
    let autocorr = autocorrelate(&windowed, LPC_ORDER);

    let mut lpc_coeffs = [0.0f32; LPC_ORDER];
    let mut reflections = [0.0f32; LPC_ORDER];
    let mut scratch = [0.0f32; LPC_ORDER];

    match levinson_durbin(&autocorr, &mut lpc_coeffs, &mut reflections, &mut scratch) {
        Ok(_residual) => formant_band_energy_pmy(&lpc_coeffs, sample_rate as f32),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sean's smoke test (2026-08-24), ported from the donor `main()`.
    #[test]
    fn levinson_durbin_smoke_test() {
        let autocorr = [100.0, 80.0, 50.0, 20.0, 5.0];
        let mut lpc = [0.0f32; 4];
        let mut reflections = [0.0f32; 4];
        let mut scratch = [0.0f32; 4];

        let final_error =
            levinson_durbin(&autocorr, &mut lpc, &mut reflections, &mut scratch).unwrap();

        assert!(final_error > 0.0, "residual power must stay positive: {final_error}");
        assert!(
            reflections.iter().all(|k| k.abs() < 1.0),
            "reflection coefficients must be stable: {reflections:?}"
        );
        assert!(final_error < autocorr[0], "prediction must reduce residual energy below r_0");
    }

    #[test]
    fn levinson_durbin_rejects_non_positive_energy() {
        let autocorr = [0.0, 80.0, 50.0, 20.0, 5.0];
        let mut lpc = [0.0f32; 4];
        let mut reflections = [0.0f32; 4];
        let mut scratch = [0.0f32; 4];
        assert!(levinson_durbin(&autocorr, &mut lpc, &mut reflections, &mut scratch).is_err());
    }

    #[test]
    fn levinson_durbin_rejects_undersized_autocorr() {
        let autocorr = [100.0, 80.0];
        let mut lpc = [0.0f32; 4];
        let mut reflections = [0.0f32; 4];
        let mut scratch = [0.0f32; 4];
        assert!(levinson_durbin(&autocorr, &mut lpc, &mut reflections, &mut scratch).is_err());
    }

    fn synth_vowel(sample_rate: u32, n: usize, formants_hz: &[f32]) -> Vec<f32> {
        let sr = sample_rate as f32;
        let mut state: u32 = 0xC0FF_EE11;
        (0..n)
            .map(|i| {
                let t = i as f32 / sr;
                let tone: f32 = formants_hz
                    .iter()
                    .map(|&f| (2.0 * std::f32::consts::PI * f * t).sin())
                    .sum::<f32>()
                    / formants_hz.len() as f32;
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let dither = ((state >> 20) as f32 / 4096.0 - 0.5) * 0.01;
                0.6 * tone + dither
            })
            .collect()
    }

    #[test]
    fn formant_energy_detects_vowel_band_over_silence() {
        let sr = 44_100u32;
        // Classic /a/-vowel formant triplet (approximate F1/F2/F3, Hz).
        let vowel = synth_vowel(sr, 2048, &[700.0, 1220.0, 2600.0]);
        let silence = vec![0.0f32; 2048];

        let vowel_pmy = formant_energy_pmy(&vowel, sr);
        let silence_pmy = formant_energy_pmy(&silence, sr);

        assert!(vowel_pmy > 0, "vowel signal should score nonzero formant energy: {vowel_pmy}");
        assert_eq!(silence_pmy, 0, "silence must score zero (r_0 <= 0 rejects the recursion)");
        assert!(
            vowel_pmy > silence_pmy,
            "vowel {vowel_pmy} should exceed silence {silence_pmy}"
        );
    }

    #[test]
    fn formant_energy_short_signal_is_zero() {
        let sr = 44_100u32;
        let short = vec![0.1f32; LPC_ORDER];
        assert_eq!(formant_energy_pmy(&short, sr), 0);
    }
}
