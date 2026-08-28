//! Color→frequency binding.
//!
//! Maps an RGBA colour to an audio frequency using the Cree moon cycle as root key.
//! Port of `13moons/scripts/audio/chromabeat_synth.gd` (via dead-drop-private).
//!
//! Encoding:
//!   R  — base frequency: lerp(220 Hz, 880 Hz, R/255)  [A3 → A5]
//!   G  — amplitude:      G / 255  [0.0 → 1.0]
//!   B  — wave type:      0–84 = sine, 85–169 = square, 170–255 = saw
//!   A  — moon index:     A % 13  → root key offset in semitones
//!
//! The moon root key shifts the frequency chromatically (2^(n/12) scaling),
//! binding each of the 13 moons to a different tonal center.

/// Returns the bound frequency in Hz for the given color.
pub fn bind(color: [u8; 4]) -> f32 {
    let r        = color[0] as f32 / 255.0;
    let moon_idx = (color[3] % 13) as f32;

    // Base frequency: lerp A3 (220 Hz) → A5 (880 Hz) on red channel
    let base_freq = 220.0_f32 + r * (880.0 - 220.0);

    // Chromatic root-key shift: moon % 13 semitones above base
    base_freq * 2.0_f32.powf(moon_idx / 12.0)
}

/// Returns the amplitude (0.0–1.0) encoded in the green channel.
pub fn amplitude(color: [u8; 4]) -> f32 {
    color[1] as f32 / 255.0
}

/// Returns the wave type index (0 = sine, 1 = square, 2 = saw) from blue channel.
pub fn wave_type(color: [u8; 4]) -> u8 {
    match color[2] {
        0..=84   => 0, // sine
        85..=169 => 1, // square
        _        => 2, // saw
    }
}

/// Synthesize one sample for the given color at the current phase.
/// `phase` must be accumulated by the caller: phase += 2π·freq/sr per sample.
pub fn synth_sample(color: [u8; 4], phase: f32) -> f32 {
    let amp = amplitude(color);
    let s = match wave_type(color) {
        0 => phase.sin(),
        1 => if phase.sin() >= 0.0 { 1.0 } else { -1.0 },
        _ => 1.0 - (phase / std::f32::consts::PI).fract() * 2.0,
    };
    s * amp
}
</content>
