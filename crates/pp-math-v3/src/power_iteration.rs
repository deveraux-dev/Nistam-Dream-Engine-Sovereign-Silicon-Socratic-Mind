//! DRAINED 2026-08-02 -> [`crate::spectral`].
//!
//! This module was written the same session as `spectral` and implemented the same
//! mechanism: integer-native power iteration, max-norm scaling, resolvent pole. Two
//! lanes landed one job under two names. `spectral` won on shape — flat row-major
//! `&[i64]` instead of `Vec<Vec<i64>>`, `Option` instead of a zero sentinel, and a
//! `Permyriad`-typed API — so the logic lives there and this is the forwarding face
//! (root#revascularize: fold the primitive into the canonical home, keep the
//! surface reachable, never leave two homes for one name).

pub use crate::spectral::{critical_lambda, deflate, principal, Eigenpair, MAX_ITERS, SCALE};

/// The convergence bar, under this module's original name.
pub use crate::spectral::SETTLE_PMY as STABLE_DELTA_PMY;

#[cfg(test)]
mod tests {
    /// The seam itself: the drained name must still reach the live mechanism, or
    /// the fold left a dangling surface.
    #[test]
    fn the_drained_name_still_reaches_the_canonical_home() {
        let k = [30_000i64, 70_000, 70_000, 30_000];
        let via_shim = super::principal(&k, 2);
        let via_home = crate::spectral::principal(&k, 2);
        assert_eq!(via_shim, via_home, "the shim must forward, never reimplement");
        assert!(via_shim.is_some(), "a coupled kernel has a dominant mode");
    }
}
