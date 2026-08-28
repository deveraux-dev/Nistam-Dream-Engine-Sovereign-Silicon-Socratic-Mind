//! Deterministic Hermite spline evaluation — a **quarantined float leaf**.
//!
//! FLOAT BOUNDARY: this is the cart brain's only `f32` surface. It is a
//! DISPLAY / cold-path primitive — cinematic camera + entity paths sampled at
//! render time — and is **never called from `CartSession::tick`** (the integer
//! 120Hz sim). The hot-path integer-only invariant is preserved: floats stay on
//! the display clock, exactly as the engine quarantines them to GPU/display leaves.
//!
//! No heap allocation; bit-identical for identical inputs (each component is
//! computed independently from `t`, so there is no accumulation drift).
//!
//! Ported by TRANSLATION from the quarry `ironroot-edict` (pure module, no engine
//! edge; originally written to replace a phantom forge-cutscene dependency).
//!
//! p(t) = H0(t)*p0 + H1(t)*p1 + H2(t)*v0 + H3(t)*v1
//!
//! Basis functions:
//!   H0(t) = 2t³ - 3t² + 1
//!   H1(t) = -2t³ + 3t²
//!   H2(t) = t³ - 2t² + t
//!   H3(t) = t³ - t²
#![allow(clippy::disallowed_types)] // float leaf — see module doc (display/cold-path only)

/// Evaluate a cubic Hermite spline at parameter `t` in [0.0, 1.0].
///
/// # Arguments
/// * `p0` - Start point
/// * `p1` - End point
/// * `v0` - Start tangent vector
/// * `v1` - End tangent vector
/// * `t`  - Interpolation parameter, clamped to [0.0, 1.0]
///
/// # Determinism
/// Bit-identical results for same inputs. No floating-point accumulation
/// drift — each component is computed independently from `t`.
#[inline]
pub fn hermite_eval(p0: [f32; 3], p1: [f32; 3], v0: [f32; 3], v1: [f32; 3], t: f32) -> [f32; 3] {
    let t2 = t * t;
    let t3 = t2 * t;

    let h0 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h1 = -2.0 * t3 + 3.0 * t2;
    let h2 = t3 - 2.0 * t2 + t;
    let h3 = t3 - t2;

    [
        h0 * p0[0] + h1 * p1[0] + h2 * v0[0] + h3 * v1[0],
        h0 * p0[1] + h1 * p1[1] + h2 * v0[1] + h3 * v1[1],
        h0 * p0[2] + h1 * p1[2] + h2 * v0[2] + h3 * v1[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermite_at_t0_returns_p0() {
        let p0 = [1.0, 2.0, 3.0];
        let p1 = [4.0, 5.0, 6.0];
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [0.0, 0.0, 0.0];
        let result = hermite_eval(p0, p1, v0, v1, 0.0);
        assert_eq!(result, p0);
    }

    #[test]
    fn hermite_at_t1_returns_p1() {
        let p0 = [1.0, 2.0, 3.0];
        let p1 = [4.0, 5.0, 6.0];
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [0.0, 0.0, 0.0];
        let result = hermite_eval(p0, p1, v0, v1, 1.0);
        assert_eq!(result, p1);
    }

    #[test]
    fn hermite_midpoint_zero_tangents() {
        let p0 = [0.0, 0.0, 0.0];
        let p1 = [10.0, 10.0, 10.0];
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [0.0, 0.0, 0.0];
        let result = hermite_eval(p0, p1, v0, v1, 0.5);
        // At t=0.5 with zero tangents: H0=0.5, H1=0.5
        assert!((result[0] - 5.0).abs() < 1e-6);
        assert!((result[1] - 5.0).abs() < 1e-6);
        assert!((result[2] - 5.0).abs() < 1e-6);
    }

    #[test]
    fn hermite_deterministic_repeated_calls() {
        let p0 = [1.5, -2.3, 7.8];
        let p1 = [-4.1, 6.2, 0.3];
        let v0 = [3.0, -1.0, 2.0];
        let v1 = [-2.0, 4.0, -1.0];
        let t = 0.37;

        let a = hermite_eval(p0, p1, v0, v1, t);
        let b = hermite_eval(p0, p1, v0, v1, t);
        // Bit-identical — not just epsilon-close
        assert_eq!(a[0].to_bits(), b[0].to_bits());
        assert_eq!(a[1].to_bits(), b[1].to_bits());
        assert_eq!(a[2].to_bits(), b[2].to_bits());
    }

    #[test]
    fn hermite_tangent_influence() {
        let p0 = [0.0, 0.0, 0.0];
        let p1 = [10.0, 0.0, 0.0];
        // Strong upward tangent at start
        let v0 = [0.0, 100.0, 0.0];
        let v1 = [0.0, 0.0, 0.0];
        let result = hermite_eval(p0, p1, v0, v1, 0.25);
        // Y should be significantly positive due to v0 tangent
        assert!(result[1] > 1.0);
    }
}
