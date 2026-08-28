//! Radial deadzone + float-to-Permyriad quantization — the analog-stick
//! quantization firewall boundary.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-input\src\deadzone.rs`.
//! `f32` is admitted here deliberately: this is the one boundary where raw
//! analog hardware state is filtered and quantized before it crosses into
//! the deterministic integer domain (`PadQuantizer`, `QuantizedPadFrame`) —
//! the float never travels past this module.

/// Default radial deadzone threshold, in normalized stick magnitude `0.0..1.0`.
pub const DEFAULT_DEADZONE: f32 = 0.15;

/// Apply radial deadzone to raw analog stick values.
///
/// - If `threshold >= 1.0`: returns `[0.0, 0.0]` (entire range is dead).
/// - If magnitude `< threshold`: returns `[0.0, 0.0]` (inside deadzone).
/// - If magnitude `>= threshold`: remaps `[threshold, 1.0]` to `[0.0, 1.0]`,
///   preserving the directional angle. Output magnitude is clamped to `[0.0, 1.0]`.
///
/// Runs in `f32` on the producer (OS poll) thread; the result must be
/// quantized to Permyriad before crossing into deterministic state.
pub fn apply_radial_deadzone(x: f32, y: f32, threshold: f32) -> [f32; 2] {
    if threshold >= 1.0 {
        return [0.0, 0.0];
    }

    let magnitude = (x * x + y * y).sqrt();

    if magnitude < threshold {
        return [0.0, 0.0];
    }

    let scale = (magnitude - threshold) / (1.0 - threshold);
    let clamped = if scale > 1.0 { 1.0 } else { scale };

    let nx = x / magnitude * clamped;
    let ny = y / magnitude * clamped;

    [nx, ny]
}

/// Quantize filtered `f32` stick values to Permyriad scale (`i32`).
///
/// Multiplies each component by `10000.0`, truncates to `i32`, and clamps
/// to `[-10000, 10000]`. The final `f32 -> i32` conversion before deadzoned
/// stick values enter deterministic state.
pub fn quantize_stick(filtered: [f32; 2]) -> [i32; 2] {
    let raw_x = (filtered[0] * 10000.0) as i32;
    let raw_y = (filtered[1] * 10000.0) as i32;

    [raw_x.clamp(-10000, 10000), raw_y.clamp(-10000, 10000)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_input_returns_zero() {
        let result = apply_radial_deadzone(0.0, 0.0, DEFAULT_DEADZONE);
        assert_eq!(result, [0.0, 0.0]);
    }

    #[test]
    fn full_deflection_returns_full() {
        let result = apply_radial_deadzone(1.0, 0.0, DEFAULT_DEADZONE);
        assert!((result[0] - 1.0).abs() < 1e-6, "expected ~1.0, got {}", result[0]);
        assert!((result[1] - 0.0).abs() < 1e-6, "expected ~0.0, got {}", result[1]);
    }

    #[test]
    fn magnitude_exactly_at_threshold_returns_zero() {
        let result = apply_radial_deadzone(0.15, 0.0, DEFAULT_DEADZONE);
        assert!((result[0]).abs() < 1e-6, "expected ~0.0, got {}", result[0]);
        assert!((result[1]).abs() < 1e-6, "expected ~0.0, got {}", result[1]);
    }

    #[test]
    fn threshold_at_ceiling_kills_everything() {
        let result = apply_radial_deadzone(1.0, 1.0, 1.0);
        assert_eq!(result, [0.0, 0.0]);
    }

    #[test]
    fn quantize_stick_full_deflection() {
        let result = quantize_stick([1.0, 0.0]);
        assert_eq!(result, [10000, 0]);
    }

    #[test]
    fn quantize_stick_negative_full_deflection() {
        let result = quantize_stick([-1.0, -1.0]);
        assert_eq!(result, [-10000, -10000]);
    }

    #[test]
    fn quantize_stick_zero() {
        let result = quantize_stick([0.0, 0.0]);
        assert_eq!(result, [0, 0]);
    }

    #[test]
    fn quantize_stick_clamps_out_of_range() {
        // Deadzone output is always <= 1.0 in magnitude, but quantize_stick's
        // own clamp is tested directly against an out-of-contract input —
        // the clamp is the last line of defence, not just deadzone's promise.
        assert_eq!(quantize_stick([1.5, -1.5]), [10000, -10000]);
    }
}
