//! forge-contour-v3 — T5 of the forge-vision drain. The quantized contour
//! profile word (`ContourTrit8`, 8 bytes, exact) and its one bridge onto the
//! normalized `(f32, f32)` profile point `contour.rs:18-20` and
//! `extrude.rs:7` consume. Extrusion, stereo caching, SDF volumes, depth
//! fusion, and scan curves arrive in later tranches as functions over this
//! type; nothing here is a second home for it.
//!
//! Excluded from this weld: `SdfCell8` and `mesh_reconstruct.rs`'s
//! `Quadric [f64; 10]` (brief T5, plan-2-welds.md:21, 07-geometry.md:62) —
//! a separate L-effort re-founding, never bundled here.

mod trit;

pub use trit::ContourTrit8;

/// Denormalize a quantized coordinate back to the `[0.0, 1.0]` float range
/// `contour.rs:18-20` and `extrude.rs:7` operate on. A projection, not a
/// bijection over floats — 65_536 quantized steps spread onto the
/// continuous `[0.0, 1.0]` range, and the perceptual/geometric meaning of
/// the result belongs to the consumer, not here.
#[inline(always)]
pub const fn to_normalized(t: ContourTrit8) -> (f32, f32) {
    (t.u as f32 / u16::MAX as f32, t.v as f32 / u16::MAX as f32)
}

#[cfg(test)]
mod bridge_tests {
    use super::*;

    /// The origin lands exactly on `(0.0, 0.0)`.
    #[test]
    fn origin_lands_on_zero() {
        assert_eq!(to_normalized(ContourTrit8::ORIGIN), (0.0, 0.0));
    }

    /// The corner lands exactly on `(1.0, 1.0)`.
    #[test]
    fn corner_lands_on_one() {
        assert_eq!(to_normalized(ContourTrit8::new(u16::MAX, u16::MAX)), (1.0, 1.0));
    }

    /// Denormalization is monotonic in each axis independently.
    #[test]
    fn denormalize_is_monotonic() {
        let mut last = None;
        for u in [0u16, 1, 16_384, 32_768, 49_151, 65_534, u16::MAX] {
            let (x, _) = to_normalized(ContourTrit8::new(u, 0));
            if let Some(prev) = last {
                assert!(x > prev, "u={u} did not raise x");
            }
            last = Some(x);
        }
    }
}
