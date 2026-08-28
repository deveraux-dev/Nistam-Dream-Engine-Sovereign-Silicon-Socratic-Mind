//! AudioUniforms — GPU uniform block driven by real-time audio energy.

use super::audio_context::AudioContext;
use super::snapshot::LiveMixerState;

/// GPU-ready uniform block populated from a [`LiveMixerState`] each frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AudioUniforms {
    pub time: f32,
    pub beat_phase: f32,
    pub rms: f32,
    pub sub_bass_ratio: f32,
    pub energy_multiplier: f32,
    pub spectrum_low: f32,
    pub spectrum_mid: f32,
    pub spectrum_high: f32,
    pub bpm: f32,
    pub amber_intensity: f32,
    pub bloom_intensity: f32,
    pub void_pinch: f32,
    pub resolution: [f32; 2],
    pub constellation: [f32; 8],
    pub _padding_end: [f32; 2],
}

/// Where this frame's spectrum points, as a Gerzon energy vector over the
/// three bands the struct above actually carries.
///
/// Deliberately a FUNCTION rather than three more fields: `AudioUniforms` is a
/// GPU uniform block with explicit tail padding (`_padding_end`), so widening
/// it silently breaks every shader bound to the current layout. A caller that
/// wants the vector on the GPU has to grow the block and its shaders together,
/// on purpose — this seam gives CPU-side callers the value today without that.
pub fn localisation(u: &AudioUniforms) -> crate::gerzon::GerzonVector {
    crate::gerzon::three_band_energy_vector(u.spectrum_low, u.spectrum_mid, u.spectrum_high)
}

#[inline]
fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn compute_rms(waveform: &[f32]) -> f32 {
    if waveform.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = waveform.iter().map(|s| s * s).sum();
    (sum_sq / waveform.len() as f32).sqrt()
}

fn spectrum_bands(spectrum: &[f32]) -> (f32, f32, f32) {
    if spectrum.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let n = spectrum.len();
    let low_end = n / 3;
    let mid_end = 2 * n / 3;

    let avg = |slice: &[f32]| -> f32 {
        if slice.is_empty() {
            0.0
        } else {
            slice.iter().sum::<f32>() / slice.len() as f32
        }
    };

    (
        avg(&spectrum[..low_end]),
        avg(&spectrum[low_end..mid_end]),
        avg(&spectrum[mid_end..]),
    )
}

#[deprecated(note = "Use build_audio_uniforms_from_ctx instead")]
pub fn build_audio_uniforms(
    snapshot: &LiveMixerState,
    time: f32,
    resolution: [f32; 2],
) -> AudioUniforms {
    let rms = finite_or(compute_rms(&snapshot.waveform_buffer), 0.0);
    let (spectrum_low, spectrum_mid, spectrum_high) = spectrum_bands(&snapshot.spectrum);

    let sub_bass_ratio = if snapshot.spectrum.is_empty() {
        0.0
    } else {
        let sub_end = (snapshot.spectrum.len() / 10).max(1);
        let sub_avg = snapshot.spectrum[..sub_end].iter().sum::<f32>() / sub_end as f32;
        finite_or(sub_avg, 0.0).clamp(0.0, 1.0)
    };

    let bpm = finite_or(snapshot.bpm, 0.0).max(0.0);
    let beat_phase = if bpm > 0.0 {
        let beats_per_sec = bpm / 60.0;
        (finite_or(time, 0.0) * beats_per_sec).fract()
    } else {
        0.0
    };

    let playing_count = snapshot
        .decks
        .iter()
        .filter(|d| d.state == super::snapshot::DeckState::Playing)
        .count() as f32;
    let energy_multiplier = finite_or(playing_count, 0.0);

    let amber_intensity = finite_or(rms * 1.5, 0.0).clamp(0.0, 1.0);

    let beat_spike = if beat_phase < 0.05 { 0.3 } else { 0.0 };
    let bloom_intensity = finite_or((rms * 0.7 + beat_spike).min(1.0), 0.0);

    let void_pinch = finite_or(sub_bass_ratio * 0.8, 0.0).clamp(0.0, 1.0);

    let res = [
        finite_or(resolution[0], 1.0).max(1.0),
        finite_or(resolution[1], 1.0).max(1.0),
    ];

    AudioUniforms {
        time: finite_or(time, 0.0),
        beat_phase: finite_or(beat_phase, 0.0),
        rms,
        sub_bass_ratio,
        energy_multiplier,
        spectrum_low: finite_or(spectrum_low, 0.0),
        spectrum_mid: finite_or(spectrum_mid, 0.0),
        spectrum_high: finite_or(spectrum_high, 0.0),
        bpm,
        amber_intensity,
        bloom_intensity,
        void_pinch,
        resolution: res,
        constellation: [0.0; 8],
        _padding_end: [0.0; 2],
    }
}

pub fn build_audio_uniforms_from_ctx(
    ctx: &AudioContext,
    time: f32,
    resolution: [f32; 2],
) -> AudioUniforms {
    let rms = finite_or(ctx.rms, 0.0).clamp(0.0, 1.0);
    let spectrum_low = finite_or(ctx.spectrum[0], 0.0).clamp(0.0, 1.0);
    let spectrum_mid = finite_or(ctx.spectrum[1], 0.0).clamp(0.0, 1.0);
    let spectrum_high = finite_or(ctx.spectrum[2], 0.0).clamp(0.0, 1.0);
    let beat_phase = finite_or(ctx.beat_phase, 0.0).clamp(0.0, 1.0);
    let sub_bass_ratio = finite_or(ctx.sub_bass_ratio, 0.0).clamp(0.0, 1.0);
    let energy_multiplier = finite_or(ctx.energy_multiplier, 0.0).clamp(0.0, 1.0);
    let bpm = finite_or(ctx.bpm, 0.0).max(0.0);

    let amber_intensity = finite_or(rms * 1.5, 0.0).clamp(0.0, 1.0);

    let (bloom_intensity, void_pinch) = if rms == 0.0 {
        (0.0, 0.0)
    } else {
        let beat_spike = if beat_phase < 0.05 { 0.3 } else { 0.0 };
        let bloom = finite_or((rms * 0.7 + beat_spike).min(1.0), 0.0);
        let pinch = finite_or(sub_bass_ratio * 0.8, 0.0).clamp(0.0, 1.0);
        (bloom, pinch)
    };

    let res = [
        finite_or(resolution[0], 1.0).max(1.0),
        finite_or(resolution[1], 1.0).max(1.0),
    ];

    AudioUniforms {
        time: finite_or(time, 0.0),
        beat_phase,
        rms,
        sub_bass_ratio,
        energy_multiplier,
        spectrum_low,
        spectrum_mid,
        spectrum_high,
        bpm,
        amber_intensity,
        bloom_intensity,
        void_pinch,
        resolution: res,
        constellation: [0.0; 8],
        _padding_end: [0.0; 2],
    }
}

/// Reduce a full magnitude spectrum into the 8 `constellation` bands the
/// audio-reactive shader passes (and the Magic Window visualizer) read.
///
/// Mirrors the dreadpirateradio visualizer's `band_energy`: each band is the
/// mean magnitude of its eighth of the spectrum, clamped to `0.0..=1.0`. For
/// the canonical 64-bin FFT this is exactly `fft[b*8 .. b*8+8].sum() / 8`.
/// Empty spectrum → eight zeros (the silent floor). Fixed `[f32; 8]`, no heap.
///
/// This is the feed-wire that lets the visualizer's per-band detail reach the
/// GPU through the existing `AudioUniforms` sink — no parallel uniform struct.
pub fn fill_constellation_from_spectrum(uniforms: &mut AudioUniforms, spectrum: &[f32]) {
    let mut bands = [0.0f32; 8];
    let n = spectrum.len();
    if n > 0 {
        for (b, slot) in bands.iter_mut().enumerate() {
            let lo = b * n / 8;
            let hi = (((b + 1) * n / 8).max(lo + 1)).min(n);
            let mut sum = 0.0f32;
            for &v in &spectrum[lo..hi] {
                sum += finite_or(v, 0.0);
            }
            *slot = (sum / (hi - lo) as f32).clamp(0.0, 1.0);
        }
    }
    uniforms.constellation = bands;
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::snapshot::LiveMixerState;
    use bytemuck::Zeroable;

    #[allow(deprecated)]
    #[test]
    fn default_snapshot_produces_finite_uniforms() {
        let snap = LiveMixerState::default();
        let u = build_audio_uniforms(&snap, 0.0, [1920.0, 1080.0]);
        assert!(u.time.is_finite());
        assert!(u.rms.is_finite());
        assert!(u.resolution[0] > 0.0);
        assert!(u.resolution[1] > 0.0);
    }

    #[allow(deprecated)]
    #[test]
    fn nan_time_is_sanitized() {
        let snap = LiveMixerState::default();
        let u = build_audio_uniforms(&snap, f32::NAN, [1920.0, 1080.0]);
        assert!(u.time.is_finite());
    }

    #[allow(deprecated)]
    #[test]
    fn zero_resolution_is_clamped() {
        let snap = LiveMixerState::default();
        let u = build_audio_uniforms(&snap, 1.0, [0.0, -5.0]);
        assert!(u.resolution[0] >= 1.0);
        assert!(u.resolution[1] >= 1.0);
    }

    #[test]
    fn struct_is_96_bytes() {
        assert_eq!(std::mem::size_of::<AudioUniforms>(), 96);
    }

    #[test]
    fn constellation_empty_spectrum_is_silent_floor() {
        let mut u = AudioUniforms::zeroed();
        fill_constellation_from_spectrum(&mut u, &[]);
        assert_eq!(u.constellation, [0.0; 8]);
    }

    #[test]
    fn constellation_isolates_the_lit_band() {
        // 64-bin spectrum with energy only in the first eighth (bins 0..8).
        let mut spectrum = [0.0f32; 64];
        for s in spectrum.iter_mut().take(8) {
            *s = 0.5;
        }
        let mut u = AudioUniforms::zeroed();
        fill_constellation_from_spectrum(&mut u, &spectrum);
        assert!((u.constellation[0] - 0.5).abs() < 1e-6, "band 0 should hold the energy");
        for &b in &u.constellation[1..] {
            assert_eq!(b, 0.0, "silent bands stay dark");
        }
    }

    #[test]
    fn constellation_clamps_hot_bins() {
        let spectrum = [9.0f32; 64];
        let mut u = AudioUniforms::zeroed();
        fill_constellation_from_spectrum(&mut u, &spectrum);
        for &b in &u.constellation {
            assert_eq!(b, 1.0, "energy above 1.0 saturates, never overflows");
        }
    }

    #[test]
    fn constellation_handles_non_multiple_of_eight() {
        // 10-bin spectrum must not panic or drop the tail.
        let spectrum = [0.2f32; 10];
        let mut u = AudioUniforms::zeroed();
        fill_constellation_from_spectrum(&mut u, &spectrum);
        for &b in &u.constellation {
            assert!((b - 0.2).abs() < 1e-6);
        }
    }
}
