//! Hypercube math floor — the deterministic state space for the Sevenfold duel.
//!
//! 14-bit Boolean hypercube: `16384 = 2^14` vertices, each a `u16` in `0..16384`.
//! Metric = Hamming distance. Routing = Mersenne prime M13 (8191) over the
//! **lower 13 bits** (the coordinate); the **14th bit** is a separate flare /
//! Cataclysm flag set by the World Consequence Engine, never by the modulo.
//! Integer-only: no float, no `%`, no division — GPU-portable by construction
//! (CPU==GPU parity holds; proved by the forge-kv-math harness).

/// Mersenne prime M13 = 2^13 - 1. The routing modulus over the coordinate bits.
pub const M13: u16 = 8191;
/// Lower-13-bit coordinate mask (== M13).
pub const COORD_MASK: u16 = 0x1FFF;
/// The 14th bit — the flare / Cataclysm flag. Splits the 16384 space in half.
pub const FLARE_BIT: u16 = 1 << 13; // 8192
/// Hypercube dimension.
pub const DIM: u32 = 14;
/// Total vertex count (`2^DIM`).
pub const STATES: u32 = 1 << DIM; // 16384

/// A vertex of the 14-bit hypercube.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Node(pub u16);

impl Node {
    /// The Void node — the origin all life-distance is measured from.
    pub const VOID: Node = Node(0);

    /// The coordinate (lower 13 bits), flare flag stripped.
    #[inline]
    pub const fn coord(self) -> u16 {
        self.0 & COORD_MASK
    }

    /// Is the flare (14th) bit set — i.e. is this a reactive Cataclysm state?
    #[inline]
    pub const fn is_flare(self) -> bool {
        self.0 & FLARE_BIT != 0
    }
}

/// Hamming distance — the discrete metric. `d(x, y) = popcount(x XOR y)`.
#[inline]
pub const fn hamming(a: Node, b: Node) -> u32 {
    (a.0 ^ b.0).count_ones()
}

/// Exact reduction mod M13 (8191) by shift-fold — no `%`, no division.
/// Deterministic and GPU-portable (integer shift / add / mask only).
#[inline]
pub fn m13_reduce(x: u32) -> u16 {
    let mut v = x;
    // Fold the high bits into the low 13 until within one 14-bit window.
    while v >= FLARE_BIT as u32 {
        v = (v & M13 as u32) + (v >> 13);
    }
    // v is now 0..=8191; 8191 ≡ 0 (mod 8191).
    if v == M13 as u32 { 0 } else { v as u16 }
}

/// Route a state by a card: `(coord(s) XOR coord(c)) mod M13`.
/// Result is `0..=8190` with the flare bit **clear** — the WCE stamps flare
/// separately via [`set_flare`]. Routing always starts from the coordinate,
/// so a prior flare flag does not carry into the next transition.
#[inline]
pub fn route(s: Node, c: Node) -> Node {
    Node(m13_reduce((s.coord() ^ c.coord()) as u32))
}

/// Stamp the flare / Cataclysm flag onto a routed coordinate. WCE-only:
/// this is the *only* path into the upper half `8192..16384`.
#[inline]
pub const fn set_flare(routed: Node) -> Node {
    Node(routed.0 | FLARE_BIT)
}

/// A player's life = Hamming distance from their core node to the Void.
/// Range `0..=14`; `0` = dead. Fits `u8`.
#[inline]
pub const fn life_as_distance(core: Node, void: Node) -> u8 {
    hamming(core, void) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trial-division primality, for the build-time M13 check below.
    fn is_prime(n: u32) -> bool {
        if n < 2 {
            return false;
        }
        let mut d = 2u32;
        while d * d <= n {
            if n % d == 0 {
                return false;
            }
            d += 1;
        }
        true
    }

    #[test]
    fn m13_matches_modulo_exhaustive_u16() {
        for x in 0u32..=u16::MAX as u32 {
            assert_eq!(m13_reduce(x) as u32, x % 8191, "x={x}");
        }
    }

    #[test]
    fn m13_matches_modulo_u32_samples() {
        for &x in &[0u32, 8190, 8191, 8192, 16382, 16383, 65_535, 1_000_000, u32::MAX] {
            assert_eq!(m13_reduce(x) as u32, x % 8191, "x={x}");
        }
    }

    #[test]
    fn constants_are_the_mersenne_split() {
        assert_eq!(M13, (1u16 << 13) - 1); // M13 = 2^13 - 1
        assert_eq!(FLARE_BIT, 8192);
        assert_eq!(COORD_MASK, 8191);
        assert_eq!(STATES, 16_384);
        assert!(is_prime(M13 as u32)); // M13 is a Mersenne *prime*
        assert_eq!(FLARE_BIT as u32 * 2, STATES); // clean half-split
    }

    #[test]
    fn hamming_is_a_metric() {
        let a = Node(0b0000_0000_0000);
        let b = Node(0b1010_1010_1010);
        let c = Node(0b1111_0000_1111);
        assert_eq!(hamming(a, a), 0); // identity
        assert_eq!(hamming(a, b), hamming(b, a)); // symmetry
        assert_eq!(hamming(a, b), 6);
        assert!(hamming(a, c) <= hamming(a, b) + hamming(b, c)); // triangle
        assert_eq!(hamming(Node(0), Node(0x3FFF)), 14); // antipode = max
    }

    #[test]
    fn route_stays_in_play_half_and_is_flare_free() {
        for s in (0u16..16384).step_by(97) {
            for c in (0u16..16384).step_by(101) {
                let r = route(Node(s), Node(c));
                assert!(r.0 <= 8190, "route escaped play half: {}", r.0);
                assert!(!r.is_flare());
            }
        }
    }

    #[test]
    fn flare_reachable_only_via_set_flare() {
        let r = route(Node(1234), Node(5678));
        assert!(!r.is_flare());
        let f = set_flare(r);
        assert!(f.is_flare());
        assert!(f.0 >= FLARE_BIT && (f.0 as u32) < STATES);
        assert_eq!(f.coord(), r.0); // coordinate preserved under the flag
    }

    #[test]
    fn route_is_deterministic_and_xor_involutive() {
        // Same inputs → same output.
        assert_eq!(route(Node(42), Node(99)), route(Node(42), Node(99)));
        // Playing card c twice returns home, except across the single M13 remap.
        for s in (0u16..8191).step_by(53) {
            for c in (0u16..8191).step_by(59) {
                if (s ^ c) == M13 {
                    continue; // the one value the modulo folds to 0
                }
                let once = route(Node(s), Node(c));
                let twice = route(once, Node(c));
                assert_eq!(twice.0, s & COORD_MASK, "s={s} c={c}");
            }
        }
    }

    #[test]
    fn life_is_bounded_by_dimension() {
        assert_eq!(life_as_distance(Node::VOID, Node::VOID), 0);
        assert_eq!(life_as_distance(Node(0x3FFF), Node::VOID), 14);
        for core in (0u16..16384).step_by(37) {
            assert!(life_as_distance(Node(core), Node::VOID) <= 14);
        }
    }
}
