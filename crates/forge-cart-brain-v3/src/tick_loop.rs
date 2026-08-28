//! Tick loop — the actual live caller of [`crate::ArenaCart`]'s [`CartSession`]
//! impl. Prior to this file, `ArenaCart` had zero callers: it compiled, it
//! passed unit tests, but nothing ever drove it repeatedly the way a real
//! host would. This is that driver, folded in at the crate-local level.
//!
//! SCOPE (named, not built): wiring `TickLoop`'s per-tick output into
//! `forge-core-v3`'s `UmpEventBundle`/`Pose5D` publisher is a cross-crate
//! follow-on, out of this pass — this file makes the LOOP real, it does not
//! make the loop's output visible to other crates yet (C16 diff-floor).

use crate::ArenaCart;
use forge_cart_sink_v3::{CartInput, CartSession, CartSinks};

/// Drives an [`ArenaCart`] for real, tick after tick — the smallest possible
/// "something calls `tick()` in a loop" that still leaves room for a host to
/// plug in real input later (`input_fn` is called once per tick, not baked in).
pub struct TickLoop {
    /// The cart being driven.
    pub cart: ArenaCart,
    /// Ticks advanced so far by this loop instance.
    pub tick_count: u64,
}

impl TickLoop {
    /// Wrap a cart in a fresh tick loop (0 ticks advanced yet).
    pub fn new(cart: ArenaCart) -> Self {
        Self { cart, tick_count: 0 }
    }

    /// Advance exactly one tick with the given input.
    pub fn advance(&mut self, input: &CartInput, sinks: &CartSinks) {
        self.cart.tick(input, sinks);
        self.tick_count += 1;
    }

    /// Advance `ticks` times, pulling input from `input_fn(tick_number)` each
    /// call — `tick_number` is 1-based, matching `CartInput::tick`'s own
    /// convention elsewhere in this crate's tests.
    pub fn run_for(&mut self, ticks: u64, sinks: &CartSinks, mut input_fn: impl FnMut(u64) -> CartInput) {
        for _ in 0..ticks {
            let t = self.tick_count + 1;
            let input = input_fn(t);
            self.advance(&input, sinks);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_cart_sink_v3::{NullDeterminism, NullEvidence, NullHarmonics, NullMotion, NullVfx};

    #[test]
    fn tick_loop_actually_drives_the_cart() {
        // The discriminator this file exists for: current_tick() must move,
        // proving something outside the cart itself called tick() repeatedly.
        let rng = NullDeterminism::new(1);
        let motion = NullMotion;
        let harmonics = NullHarmonics::default();
        let evidence = NullEvidence;
        let vfx = NullVfx::default();
        let sinks = CartSinks { rng: &rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };

        let mut loop_ = TickLoop::new(ArenaCart::new(1, 1));
        assert_eq!(loop_.tick_count, 0);
        assert_eq!(loop_.cart.current_tick(), 0);

        loop_.run_for(100, &sinks, |t| CartInput { tick: t, buttons: 0, x_vel: 1, y_vel: 0 });

        assert_eq!(loop_.tick_count, 100);
        assert_eq!(loop_.cart.current_tick(), 100, "the cart's own tick counter must track the loop");
    }
}
