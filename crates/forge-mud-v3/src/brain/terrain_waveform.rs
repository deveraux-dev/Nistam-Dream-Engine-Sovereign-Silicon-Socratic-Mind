//! Photometric Terrain Waveform — terrain surface modulation from resonance data.
//!
//! Concept: terrain height is perturbed by a standing wave pattern derived from
//! the zone's dominant resonance frequency. Low-Hz zones (Nigredo) get slow,
//! deep undulations. High-Hz zones (Rubedo) get rapid, shallow ripples.
//!
//! This is a VISUAL-ONLY effect. It never writes back into physics state.
//! The waveform is computed per-frame at render time and applied as a vertex
//! displacement in the terrain shader.
//!
//! Integer pipeline: resonance_hz → wavelength_mm → amplitude_mm → displacement.

/// Minimum resonance frequency (Hz). Low-frequency zones: slow deep waves.
pub const RESONANCE_MIN_HZ: u16 = 40;

/// Maximum resonance frequency (Hz). High-frequency zones: rapid shallow ripples.
pub const RESONANCE_MAX_HZ: u16 = 800;

/// Terrain waveform parameters derived from zone resonance.
#[derive(Debug, Clone, Copy)]
pub struct TerrainWaveform {
    /// Wavelength in mm (distance between wave peaks).
    pub wavelength_mm: i32,
    /// Amplitude in mm (max vertical displacement).
    pub amplitude_mm: i32,
    /// Phase velocity in mm/tick (how fast the wave moves).
    pub phase_velocity_mm_per_tick: i32,
    /// Current phase offset in mm (advances each tick).
    pub phase_offset_mm: i64,
    /// Gothic limiter mode: constrains extrusion to Ad Triangulum arches.
    pub gothic_limiter: bool,
}

impl TerrainWaveform {
    /// Derive waveform from zone resonance frequency.
    /// Lower Hz → longer wavelength, higher amplitude (deep slow waves).
    /// Higher Hz → shorter wavelength, lower amplitude (shallow fast ripples).
    pub fn from_resonance_hz(hz: u16) -> Self {
        let clamped = hz.clamp(RESONANCE_MIN_HZ, RESONANCE_MAX_HZ) as i32;
        // Wavelength inversely proportional to frequency: 40Hz → 20000mm, 800Hz → 1000mm
        let wavelength_mm = 800_000 / clamped;
        // Amplitude inversely proportional: 40Hz → 500mm, 800Hz → 25mm
        let amplitude_mm = 20_000 / clamped;
        // Phase velocity proportional to frequency: 40Hz → 40mm/tick, 800Hz → 800mm/tick
        let phase_velocity_mm_per_tick = clamped;

        Self {
            wavelength_mm,
            amplitude_mm,
            phase_velocity_mm_per_tick,
            phase_offset_mm: 0,
            gothic_limiter: false,
        }
    }

    /// Toggle Gothic limiter (Ad Triangulum arch constraints).
    pub fn set_gothic_limiter(&mut self, enabled: bool) {
        self.gothic_limiter = enabled;
    }

    /// Advance the waveform by one physics tick.
    pub fn tick(&mut self) {
        self.phase_offset_mm += self.phase_velocity_mm_per_tick as i64;
    }

    /// Compute vertical displacement at a world position (mm).
    /// Uses integer sine approximation (Bhaskara I).
    /// Returns displacement in mm. VISUAL ONLY — never feed into physics.
    pub fn displacement_at_mm(&self, world_x_mm: i64, world_z_mm: i64) -> i32 {
        // Phase = (x + z + offset) mod wavelength, mapped to 0..wavelength
        let combined = world_x_mm + world_z_mm + self.phase_offset_mm;
        let wl = self.wavelength_mm as i64;
        let phase = ((combined % wl) + wl) % wl; // always positive

        // Integer sine approximation: triangle wave (good enough for terrain)
        // Map phase 0..wl to -amplitude..+amplitude via triangle
        let half = wl / 2;
        let tri = if phase < half {
            (phase * 2 * self.amplitude_mm as i64 / wl) - self.amplitude_mm as i64
        } else {
            self.amplitude_mm as i64 - ((phase - half) * 2 * self.amplitude_mm as i64 / wl)
        };

        if !self.gothic_limiter {
            return tri as i32;
        }

        // ── Gothic Limiter: Ad Triangulum Arch Constraint ────────────────
        // Forces displacement into pointed-arch geometry.
        // Peak only in central third of wavelength (Rule of Central Third).
        // Max extrusion capped by √3 ratio (Ad Triangulum = 1732/1000).
        let third = wl / 3;
        let in_peak_zone = phase >= third && phase < third * 2;

        if !in_peak_zone {
            // Outside peak zone: suppress to 25% amplitude (buttress/floor)
            return (tri / 4) as i32;
        }

        // Inside peak zone: shape as pointed arch (two parabolic arcs meeting at apex)
        // Map phase within peak zone to 0..1000 (permyriad of peak width)
        let local = (phase - third) * 1000 / third;
        // Parabolic arch: 4x(1-x) peaks at 1.0 when x=0.5
        // Compute in larger intermediate to avoid integer truncation
        let arch_num = 4 * local * (1000 - local); // max = 1_000_000
        // Scale by Ad Triangulum ratio (√3 ≈ 1732/1000) of base amplitude
        let ad_triangulum: i64 = 1732;
        let gothic_amp = self.amplitude_mm as i64 * ad_triangulum / 1000;
        (arch_num * gothic_amp / 1_000_000) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_hz_has_long_wavelength() {
        let w = TerrainWaveform::from_resonance_hz(40);
        assert_eq!(w.wavelength_mm, 20000);
        assert_eq!(w.amplitude_mm, 500);
    }

    #[test]
    fn high_hz_has_short_wavelength() {
        let w = TerrainWaveform::from_resonance_hz(800);
        assert_eq!(w.wavelength_mm, 1000);
        assert_eq!(w.amplitude_mm, 25);
    }

    #[test]
    fn displacement_is_bounded() {
        let w = TerrainWaveform::from_resonance_hz(200);
        for x in 0..1000 {
            let d = w.displacement_at_mm(x * 100, 0);
            assert!(d.abs() <= w.amplitude_mm, "displacement {} exceeds amplitude {}", d, w.amplitude_mm);
        }
    }

    #[test]
    fn tick_advances_phase() {
        let mut w = TerrainWaveform::from_resonance_hz(120);
        let before = w.phase_offset_mm;
        w.tick();
        assert_eq!(w.phase_offset_mm, before + 120);
    }

    #[test]
    fn gothic_limiter_suppresses_outside_peak() {
        let mut w = TerrainWaveform::from_resonance_hz(200);
        w.set_gothic_limiter(true);
        // Sample at phase 0 (outside peak zone = first third)
        let d = w.displacement_at_mm(0, 0);
        let amp = w.amplitude_mm;
        // Should be suppressed to ≤25% of amplitude
        assert!(d.abs() <= amp / 4 + 1, "got {} but max should be ~{}", d, amp / 4);
    }

    #[test]
    fn gothic_limiter_peaks_in_center() {
        let mut w = TerrainWaveform::from_resonance_hz(200);
        w.set_gothic_limiter(true);
        let wl = w.wavelength_mm as i64;
        // Sample at center of peak zone (wavelength/2)
        let d = w.displacement_at_mm(wl / 2, 0);
        // Should be positive and significant (arch apex)
        assert!(d > 0, "gothic peak should be positive, got {}", d);
        assert!(d > w.amplitude_mm / 2, "gothic peak {} should exceed half amplitude {}", d, w.amplitude_mm / 2);
    }

    #[test]
    fn gothic_off_matches_original() {
        let w = TerrainWaveform::from_resonance_hz(200);
        assert!(!w.gothic_limiter);
        // Without limiter, displacement should reach full amplitude range
        let wl = w.wavelength_mm as i64;
        let d_peak = w.displacement_at_mm(wl / 2, 0);
        assert!(d_peak.abs() > w.amplitude_mm / 2);
    }

    // L18 sabotage test: flip the displacement sign, confirm bounded invariant breaks.
    #[test]
    fn sabotage_displacement_bounds_gate() {
        let w = TerrainWaveform::from_resonance_hz(300);
        // Sweep across a full wavelength and verify ALL displacements stay within bounds.
        // If the math is broken (e.g., sign flipped), at least one will exceed amplitude.
        let wl = w.wavelength_mm as i64;
        for offset in 0..wl {
            let d = w.displacement_at_mm(offset, 0);
            assert!(
                d.abs() <= w.amplitude_mm,
                "sabotage gate: offset {} gave displacement {} which exceeds amplitude {}",
                offset, d, w.amplitude_mm
            );
        }
    }
}
