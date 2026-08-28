//! Archival Alchemy — deterministic DSP pipeline for pre-1925 recordings.

pub mod stft;
pub mod restoration;
pub mod hpss;
pub mod formant;
pub mod pitch;
pub mod dtw;
pub mod vocoder;
pub mod ducking;
pub mod ghost_speak;
pub mod wave_phase;

/// Result of the full alchemy pipeline.
pub struct AlchemyResult {
    /// The final mixed mono signal.
    pub mixed: Vec<f32>,
    /// Harmonic component of Track A (for visualization).
    pub harmonic_a: Vec<f32>,
    /// Percussive component of Track A (for visualization).
    pub percussive_a: Vec<f32>,
    /// YIN pitch contour of Track A's harmonic (Hz per frame, 0=unvoiced).
    pub pitch_a: Vec<f32>,
    /// YIN pitch contour of Track B's harmonic.
    pub pitch_b: Vec<f32>,
    /// DTW warping path: (frame_a, frame_b) pairs.
    pub warp_path: Vec<(usize, usize)>,
    /// Instantaneous tempo map (stretch ratio per frame of A).
    pub tempo_map: Vec<f32>,
}

/// Run the full Archival Alchemy pipeline on two mono signals.
///
/// Pipeline:
/// 1. Restore both tracks (spectral subtraction)
/// 2. HPSS separate both tracks into harmonic + percussive
/// 3. YIN pitch track both harmonic signals
/// 4. DTW align percussive envelopes
/// 5. Phase vocoder stretch Track B's harmonic to match Track A's tempo
/// 6. Spectral duck the aligned Track B against Track A
/// 7. Sum the final mix
pub fn run_alchemy(track_a: &[f32], track_b: &[f32], sample_rate: u32) -> AlchemyResult {
    // 1. Restoration
    let clean_a = restoration::restore_spectral(track_a, sample_rate);
    let clean_b = restoration::restore_spectral(track_b, sample_rate);

    // 2. HPSS separation
    let (harmonic_a, percussive_a) = hpss::hpss_separate(&clean_a, sample_rate);
    let (harmonic_b, _percussive_b) = hpss::hpss_separate(&clean_b, sample_rate);

    // 3. Pitch tracking
    let pitch_a = pitch::yin_track(&harmonic_a, sample_rate, 2048, 512, 0.15);
    let pitch_b = pitch::yin_track(&harmonic_b, sample_rate, 2048, 512, 0.15);

    // 4. DTW alignment on percussive onset envelopes
    let env_a = onset_envelope(&percussive_a, sample_rate);
    let env_b = onset_envelope(&clean_b, sample_rate);
    let radius = (env_a.len() / 10).max(10).min(500);
    let warp_path = dtw::dtw_align(&env_a, &env_b, radius);
    let tempo_map = dtw::path_to_stretch_map(&warp_path, env_a.len());

    // 5. Phase vocoder — stretch Track B to match Track A's timing
    let avg_stretch = if tempo_map.is_empty() {
        1.0
    } else {
        tempo_map.iter().sum::<f32>() / tempo_map.len() as f32
    };
    let stretched_b = vocoder::phase_vocoder(&harmonic_b, sample_rate, avg_stretch);

    // 6. Spectral ducking
    let min_len = clean_a.len().min(stretched_b.len());
    let ducked_b = ducking::spectral_duck(
        &clean_a[..min_len],
        &stretched_b[..min_len],
        sample_rate,
        0.6,
    );

    // 7. Mix: sum A + ducked B, normalize
    let mut mixed = vec![0.0f32; min_len];
    for i in 0..min_len {
        mixed[i] = clean_a[i] + ducked_b[i];
    }
    let peak = mixed.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
    if peak > 1.0 {
        let scale = 0.95 / peak;
        for s in &mut mixed {
            *s *= scale;
        }
    }

    AlchemyResult {
        mixed,
        harmonic_a,
        percussive_a,
        pitch_a,
        pitch_b,
        warp_path,
        tempo_map,
    }
}

/// Simple onset envelope for DTW: energy per frame.
fn onset_envelope(signal: &[f32], _sample_rate: u32) -> Vec<f32> {
    let hop = 512;
    let window = 1024;
    let n_frames = signal.len().saturating_sub(window) / hop + 1;
    let mut env = Vec::with_capacity(n_frames);
    for i in 0..n_frames {
        let start = i * hop;
        let end = (start + window).min(signal.len());
        let energy: f32 = signal[start..end].iter().map(|s| s * s).sum();
        env.push(energy.sqrt());
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alchemy_pipeline_runs() {
        let sr = 44100u32;
        let n = 44100; // 1 second
        let track_a: Vec<f32> = (0..n)
            .map(|i| 0.4 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let track_b: Vec<f32> = (0..n)
            .map(|i| 0.3 * (2.0 * std::f32::consts::PI * 330.0 * i as f32 / sr as f32).sin())
            .collect();

        let result = run_alchemy(&track_a, &track_b, sr);
        assert!(!result.mixed.is_empty(), "Mixed output should not be empty");
        assert!(!result.pitch_a.is_empty(), "Pitch contour A should not be empty");
        assert!(!result.tempo_map.is_empty(), "Tempo map should not be empty");
    }

    #[test]
    fn test_alchemy_result_not_silent() {
        let sr = 44100u32;
        let n = 22050;
        let track_a: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let track_b: Vec<f32> = (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 660.0 * i as f32 / sr as f32).sin())
            .collect();
        let result = run_alchemy(&track_a, &track_b, sr);
        let energy: f32 = result.mixed.iter().map(|s| s * s).sum();
        assert!(energy > 0.1, "Mixed output energy {} should be non-zero", energy);
    }
}
