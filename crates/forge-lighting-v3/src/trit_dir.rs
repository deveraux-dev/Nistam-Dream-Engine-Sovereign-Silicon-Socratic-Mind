//! Direction quantized to three balanced trits — 26 cube directions plus a
//! refused origin. Green field: neither v2 nor v3 had any direction or normal
//! quantization.

/// Trits per direction word. `3^3 = 27` states: 26 directions + the origin.
pub const TRITS_PER_DIR: usize = 3;

/// Total packed states, `3^3`.
pub const DIR_STATES: u8 = 27;

/// The packed byte of the degenerate all-zero direction. Refused, never a
/// direction — the same discipline `TritCell5D` applies to its sentinels.
pub const DIR_ORIGIN: u8 = 13;

/// Component magnitude above which an axis reads as live rather than zero.
/// `0.5` after max-normalization is the cube rule: it splits the sphere into
/// 6 faces, 12 edges and 8 corners.
const LIVE: f32 = 0.5;

/// A direction on the 26-point cube lattice, radix-3 packed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TritDir(/// The packed direction byte, `0..27`.
pub u8);

impl TritDir {
    /// Quantize a direction. `None` for a zero-length vector — a direction that
    /// points nowhere is refused, not silently snapped to an axis.
    pub fn quantize(v: [f32; 3]) -> Option<Self> {
        let m = v[0].abs().max(v[1].abs()).max(v[2].abs());
        if !(m > 0.0) || !m.is_finite() {
            return None;
        }
        let q = |c: f32| -> i8 {
            let n = c / m;
            if n > LIVE {
                1
            } else if n < -LIVE {
                -1
            } else {
                0
            }
        };
        let t = [q(v[0]), q(v[1]), q(v[2])];
        if t == [0, 0, 0] {
            return None;
        }
        Some(Self::from_trits(t))
    }

    /// Pack three balanced trits, Horner radix-3 — the same order
    /// `TritCell5D::from_trits` uses.
    #[inline]
    pub const fn from_trits(t: [i8; 3]) -> Self {
        Self((t[0] + 1) as u8 + (t[1] + 1) as u8 * 3 + (t[2] + 1) as u8 * 9)
    }

    /// Unpack to three balanced trits.
    #[inline]
    pub const fn trits(self) -> [i8; 3] {
        [
            (self.0 % 3) as i8 - 1,
            (self.0 / 3 % 3) as i8 - 1,
            (self.0 / 9 % 3) as i8 - 1,
        ]
    }

    /// True for the all-zero word, which is not a direction.
    #[inline]
    pub const fn is_origin(self) -> bool {
        self.0 == DIR_ORIGIN
    }

    /// The unit vector this direction represents.
    pub fn to_unit(self) -> [f32; 3] {
        let t = self.trits();
        let (x, y, z) = (t[0] as f32, t[1] as f32, t[2] as f32);
        let m = (x * x + y * y + z * z).sqrt();
        if m == 0.0 {
            return [0.0; 3];
        }
        [x / m, y / m, z / m]
    }

    /// Negate all three trits — the antipode.
    #[inline]
    pub const fn flip(self) -> Self {
        let t = self.trits();
        Self::from_trits([-t[0], -t[1], -t[2]])
    }

    /// How many axes are live: 1 = face, 2 = edge, 3 = corner, 0 = origin.
    #[inline]
    pub const fn order(self) -> u8 {
        let t = self.trits();
        (t[0] != 0) as u8 + (t[1] != 0) as u8 + (t[2] != 0) as u8
    }
}

/// Every real direction, ascending by packed byte. The origin is excluded, so
/// this is exactly 26 long.
pub fn all_directions() -> impl Iterator<Item = TritDir> {
    (0..DIR_STATES).map(TritDir).filter(|d| !d.is_origin())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lattice_is_twenty_six_directions_and_one_refusal() {
        assert_eq!(all_directions().count(), 26);
        assert!(TritDir(DIR_ORIGIN).is_origin());
        assert_eq!(TritDir::from_trits([0, 0, 0]).0, DIR_ORIGIN);
    }

    #[test]
    fn trits_round_trip_over_every_state() {
        for b in 0..DIR_STATES {
            let d = TritDir(b);
            assert_eq!(TritDir::from_trits(d.trits()), d, "state {b}");
        }
    }

    #[test]
    fn flip_is_an_involution_fixed_only_at_the_origin() {
        for b in 0..DIR_STATES {
            let d = TritDir(b);
            assert_eq!(d.flip().flip(), d, "f(f(x)) = x at {b}");
            if d.flip() == d {
                assert!(d.is_origin(), "only the origin may be its own antipode");
            }
        }
    }

    #[test]
    fn axes_quantize_to_faces() {
        assert_eq!(TritDir::quantize([1.0, 0.0, 0.0]).unwrap().trits(), [1, 0, 0]);
        assert_eq!(TritDir::quantize([0.0, -3.0, 0.0]).unwrap().trits(), [0, -1, 0]);
        assert_eq!(TritDir::quantize([0.0, 0.0, 0.2]).unwrap().trits(), [0, 0, 1]);
        for d in [
            TritDir::quantize([1.0, 0.0, 0.0]),
            TritDir::quantize([0.0, 1.0, 0.0]),
            TritDir::quantize([0.0, 0.0, 1.0]),
        ] {
            assert_eq!(d.unwrap().order(), 1, "an axis is a face, not an edge");
        }
    }

    #[test]
    fn diagonals_quantize_to_edges_and_corners() {
        assert_eq!(TritDir::quantize([1.0, 1.0, 0.0]).unwrap().order(), 2);
        assert_eq!(TritDir::quantize([1.0, 1.0, 1.0]).unwrap().order(), 3);
        assert_eq!(TritDir::quantize([-1.0, 1.0, -1.0]).unwrap().trits(), [-1, 1, -1]);
    }

    #[test]
    fn a_direction_that_points_nowhere_is_refused() {
        assert!(TritDir::quantize([0.0, 0.0, 0.0]).is_none());
        assert!(TritDir::quantize([f32::NAN, 0.0, 0.0]).is_none());
        assert!(TritDir::quantize([f32::INFINITY, 0.0, 0.0]).is_none());
    }

    #[test]
    fn scale_does_not_change_the_direction() {
        let a = TritDir::quantize([0.3, 0.9, -0.2]).unwrap();
        let b = TritDir::quantize([30.0, 90.0, -20.0]).unwrap();
        assert_eq!(a, b, "quantization is scale-invariant");
    }

    #[test]
    fn to_unit_is_unit_length_for_every_direction() {
        for d in all_directions() {
            let u = d.to_unit();
            let m = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
            assert!((m - 1.0).abs() < 1e-5, "dir {} magnitude {m}", d.0);
        }
    }

    #[test]
    fn quantizing_a_directions_own_unit_vector_returns_it() {
        // The lattice is a fixed point of its own quantizer — the property that
        // makes a baked per-direction table safe to look up.
        for d in all_directions() {
            assert_eq!(TritDir::quantize(d.to_unit()), Some(d), "dir {} moved", d.0);
        }
    }
}
