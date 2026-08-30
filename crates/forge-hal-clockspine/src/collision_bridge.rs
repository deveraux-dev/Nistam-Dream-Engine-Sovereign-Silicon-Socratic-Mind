//! CollisionBridge — lock-free bidirectional domain reconciliation.
//!
//! Two orthogonal producers/consumers exchange state through independent `TripleBuffer`
//! lanes without ever blocking. Per ARCH-009 §4, this is a domain-agnostic mechanism:
//! neither producer nor consumer has a privileged name here; Alpha and Beta are assigned
//! by the caller's orchestration, not by the bridge itself.

use crate::fixed::Permyriad;
use crate::triple_buffer::{ClockPlane, TripleBuffer};

/// Payload crossing the collision bridge: index (domain-specific meaning),
/// magnitude (integer permyriad), and lane selector.
///
/// `idx` is deliberately NOT a tick; the receiver decides what it indexes
/// in its own domain. `mag_pmy` is "the one-way float valve": integer
/// crosses the bridge, float does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResonanceImpulse {
    /// Domain-specific index (not a tick).
    pub idx: u64,
    /// Integer magnitude in Permyriad range [0, 10_000].
    pub mag_pmy: Permyriad,
    /// Lane selector (typically 0 or 1, but generalized to u8).
    pub lane: u8,
}

impl ClockPlane for ResonanceImpulse {
    #[inline]
    fn copy_into(&self, dst: &mut Self) {
        *dst = *self;
    }
}

/// Collision bridge: two independent `TripleBuffer` lanes for bidirectional
/// lock-free exchange. Daemon-owned (neither Alpha nor Beta is privileged).
/// Each side touches only its own emit and its own take.
pub struct CollisionBridge {
    /// Alpha → Beta lane.
    alpha_to_beta: TripleBuffer<ResonanceImpulse>,
    /// Beta → Alpha lane.
    beta_to_alpha: TripleBuffer<ResonanceImpulse>,
}

impl CollisionBridge {
    /// Create a bridge seeded with default impulses on both lanes.
    pub fn new() -> Self {
        let default = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        Self {
            alpha_to_beta: TripleBuffer::new(default),
            beta_to_alpha: TripleBuffer::new(default),
        }
    }

    /// Alpha side: publish an impulse into the alpha→beta lane.
    pub fn alpha_publish(&self, impulse: ResonanceImpulse) -> ResonanceImpulse {
        self.alpha_to_beta.publish(impulse)
    }

    /// Alpha side: try to take the latest impulse from the beta→alpha lane.
    /// Returns `Some(impulse)` if fresh, `None` if no change or contended —
    /// caller reuses its last front.
    pub fn alpha_take(&self, last_gen: u64, dst: &mut ResonanceImpulse) -> Option<u64> {
        self.beta_to_alpha.try_take(last_gen, dst)
    }

    /// Beta side: publish an impulse into the beta→alpha lane.
    pub fn beta_publish(&self, impulse: ResonanceImpulse) -> ResonanceImpulse {
        self.beta_to_alpha.publish(impulse)
    }

    /// Beta side: try to take the latest impulse from the alpha→beta lane.
    /// Returns `Some(impulse)` if fresh, `None` if no change or contended —
    /// caller reuses its last front.
    pub fn beta_take(&self, last_gen: u64, dst: &mut ResonanceImpulse) -> Option<u64> {
        self.alpha_to_beta.try_take(last_gen, dst)
    }
}

impl Default for CollisionBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_publish_and_beta_take() {
        let bridge = CollisionBridge::new();
        let impulse = ResonanceImpulse { idx: 42, mag_pmy: Permyriad(5000), lane: 1 };

        let recycled = bridge.alpha_publish(impulse);
        assert_eq!(recycled.idx, 0, "publisher recycles old impulse");

        let mut dst = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        let gen = bridge.beta_take(0, &mut dst).expect("beta should see fresh impulse");
        assert_eq!(gen, 1);
        assert_eq!(dst, impulse);
    }

    #[test]
    fn beta_publish_and_alpha_take() {
        let bridge = CollisionBridge::new();
        let impulse = ResonanceImpulse { idx: 99, mag_pmy: Permyriad(9999), lane: 0 };

        let recycled = bridge.beta_publish(impulse);
        assert_eq!(recycled.idx, 0);

        let mut dst = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        let gen = bridge.alpha_take(0, &mut dst).expect("alpha should see fresh impulse");
        assert_eq!(gen, 1);
        assert_eq!(dst, impulse);
    }

    #[test]
    fn miss_reuses_last_front() {
        let bridge = CollisionBridge::new();
        let impulse1 = ResonanceImpulse { idx: 10, mag_pmy: Permyriad(1000), lane: 0 };
        let impulse2 = ResonanceImpulse { idx: 20, mag_pmy: Permyriad(2000), lane: 0 };

        bridge.alpha_publish(impulse1);
        let mut dst = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        let gen1 = bridge.beta_take(0, &mut dst).expect("first take should be fresh");
        assert_eq!(dst, impulse1);

        assert!(bridge.beta_take(gen1, &mut dst).is_none(), "second take should miss (no update)");
        assert_eq!(dst, impulse1, "dst must remain unchanged on miss");

        bridge.alpha_publish(impulse2);
        let gen2 = bridge.beta_take(gen1, &mut dst).expect("third take should see new impulse");
        assert_eq!(gen2, 2);
        assert_eq!(dst, impulse2);
    }

    #[test]
    fn two_directions_independent() {
        let bridge = CollisionBridge::new();
        let alpha_impulse = ResonanceImpulse { idx: 111, mag_pmy: Permyriad(1111), lane: 0 };
        let beta_impulse = ResonanceImpulse { idx: 222, mag_pmy: Permyriad(2222), lane: 1 };

        bridge.alpha_publish(alpha_impulse);
        bridge.beta_publish(beta_impulse);

        let mut alpha_dst = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        let mut beta_dst = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };

        let alpha_gen = bridge.alpha_take(0, &mut alpha_dst).expect("alpha sees from beta");
        let beta_gen = bridge.beta_take(0, &mut beta_dst).expect("beta sees from alpha");

        assert_eq!(alpha_gen, 1);
        assert_eq!(beta_gen, 1);
        assert_eq!(alpha_dst, beta_impulse, "alpha receives beta's direction");
        assert_eq!(beta_dst, alpha_impulse, "beta receives alpha's direction");
    }

    #[test]
    fn alpha_solo_beta_absent() {
        let bridge = CollisionBridge::new();
        let impulse1 = ResonanceImpulse { idx: 1, mag_pmy: Permyriad(1000), lane: 0 };
        let impulse2 = ResonanceImpulse { idx: 2, mag_pmy: Permyriad(2000), lane: 0 };
        let impulse3 = ResonanceImpulse { idx: 3, mag_pmy: Permyriad(3000), lane: 0 };

        let recycled1 = bridge.alpha_publish(impulse1);
        assert_eq!(recycled1.idx, 0, "first publish returns the initial impulse");

        let recycled2 = bridge.alpha_publish(impulse2);
        assert_eq!(recycled2.idx, 1, "second publish returns the first published impulse");

        let recycled3 = bridge.alpha_publish(impulse3);
        assert_eq!(recycled3.idx, 2, "third publish returns the second published impulse");
    }

    #[test]
    fn beta_solo_alpha_absent() {
        let bridge = CollisionBridge::new();
        let mut recycled = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        let impulses = [
            ResonanceImpulse { idx: 5, mag_pmy: Permyriad(5000), lane: 1 },
            ResonanceImpulse { idx: 6, mag_pmy: Permyriad(6000), lane: 1 },
        ];

        for imp in &impulses {
            recycled = bridge.beta_publish(*imp);
        }
        assert_eq!(
            recycled.idx, 5,
            "beta's second publish should recycle the first impulse (TripleBuffer uses 2+ buffers)"
        );
    }
}
