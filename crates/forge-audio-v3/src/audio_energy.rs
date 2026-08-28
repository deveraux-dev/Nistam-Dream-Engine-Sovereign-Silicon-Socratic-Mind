//! AudioEnergy GPU uniform — bridges analyzer metrics to wgpu shaders.
//!
//! 32 bytes total (8 × f32) — aligned for wgpu uniform buffers.
//! Bound to ghost shader (@group(1) @binding(0)) and post-process shaders.

use crate::analyzer::AnalyzerSnapshot;

/// Structured audio energy data for GPU shaders.
///
/// 32 bytes total (8 × f32) — aligned for wgpu uniform buffers.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AudioEnergy {
    /// 20-80 Hz band energy (from spectrum_smoothed[0..4])
    pub sub_bass: f32,
    /// 250 Hz – 4 kHz band energy (from spectrum_smoothed[16..48])
    pub mid: f32,
    /// 4-20 kHz band energy (from spectrum_smoothed[48..64])
    pub treble: f32,
    /// Spectral sharpness / resonance (proxy: spectral_centroid)
    pub q_factor: f32,
    /// 0.0..1.0 transient pulse (from beat detection)
    pub beat_pulse: f32,
    /// Master RMS level
    pub rms: f32,
    /// Padding to align to 32 bytes
    pub _pad: [f32; 2],
}

impl AudioEnergy {
    // to_audio_uniforms: EXCLUDED — needs crate::bus (excluded, see lib.rs).


    /// Derive from current analyzer state. Called once per render frame.
    pub fn from_analyzer(
        spectrum: &[f32; 64],
        az: &AnalyzerSnapshot,
        beat_pulse: f32,
        rms: f32,
    ) -> Self {
        Self {
            sub_bass: spectrum[0..4].iter().sum::<f32>() / 4.0,
            mid: spectrum[16..48].iter().sum::<f32>() / 32.0,
            treble: spectrum[48..64].iter().sum::<f32>() / 16.0,
            q_factor: az.spectral_centroid,
            beat_pulse,
            rms,
            _pad: [0.0; 2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_snap() -> AnalyzerSnapshot {
        AnalyzerSnapshot::default()
    }

    #[test]
    fn from_analyzer_zero_spectrum() {
        let spectrum = [0.0f32; 64];
        let e = AudioEnergy::from_analyzer(&spectrum, &zero_snap(), 0.0, 0.0);
        assert_eq!(e.sub_bass, 0.0);
        assert_eq!(e.mid, 0.0);
        assert_eq!(e.treble, 0.0);
    }

    #[test]
    fn from_analyzer_uniform_spectrum() {
        let spectrum = [1.0f32; 64];
        let e = AudioEnergy::from_analyzer(&spectrum, &zero_snap(), 0.5, 0.8);
        assert!((e.sub_bass - 1.0).abs() < 1e-5);
        assert!((e.mid - 1.0).abs() < 1e-5);
        assert!((e.treble - 1.0).abs() < 1e-5);
        assert!((e.beat_pulse - 0.5).abs() < 1e-5);
        assert!((e.rms - 0.8).abs() < 1e-5);
    }

    #[test]
    fn struct_is_32_bytes() {
        assert_eq!(std::mem::size_of::<AudioEnergy>(), 32);
    }

    #[test]
    fn bytemuck_cast_roundtrip() {
        let e = AudioEnergy { sub_bass: 0.1, mid: 0.2, treble: 0.3, q_factor: 0.4, beat_pulse: 0.5, rms: 0.6, _pad: [0.0; 2] };
        let bytes: &[u8] = bytemuck::bytes_of(&e);
        assert_eq!(bytes.len(), 32);
        let e2: AudioEnergy = *bytemuck::from_bytes(bytes);
        assert!((e2.sub_bass - 0.1).abs() < 1e-6);
    }
}
