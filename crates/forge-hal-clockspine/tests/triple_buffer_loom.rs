//! [BOARD: TRIPLEBUF-LOOM] Loom model of the TripleBuffer publish/try_take swap —
//! proves the producer/consumer handoff is race-free and NEVER blocks across
//! EXHAUSTIVE thread interleavings (loom's model checker), not a sampled stress
//! test.
//!
//! ## Live v3 seam
//! This validates `forge_hal_clockspine::triple_buffer::TripleBuffer<T>`:
//! a `Mutex<Slot<T>>` with try_lock-only publish/take and a monotonic u64 generation.
//!
//! ## Why a MIRROR, not the runtime type
//! The runtime type's `std::sync::Mutex` cannot be cfg-swapped for `loom::sync::Mutex` in place
//! without affecting the zero-dependency / standard build. This file re-implements
//! the IDENTICAL algorithm (Slot<T>{value,generation}, try_lock swap, wrapping
//! generation bump, try_take compare-then-copy) against loom primitives, and
//! drift-checks that mirror against the REAL `forge_hal_clockspine` type below.
//!
//! Whole file is `#![cfg(loom)]`: plain `cargo test -p forge-hal-clockspine` compiles this
//! to zero tests (no runtime code touched); the model runs only under the host's
//! `--cfg loom` gate.
#![cfg(loom)]

use loom::sync::{Arc, Mutex};
use loom::thread;

/// Mirror of `forge_hal_clockspine::triple_buffer::Slot<T>` (private in the real crate).
struct Slot<T> {
    value: T,
    generation: u64,
}

/// Mirror of `forge_hal_clockspine::triple_buffer::TripleBuffer<T>` -- same algorithm,
/// loom-instrumented `Mutex` so the model checker explores every interleaving of
/// one producer + one consumer instead of sampling one (the real crate's own
/// `try_take_under_live_contention_never_blocks` test samples 50_000 iterations;
/// this proves the same property exhaustively).
struct LoomTripleBuffer<T> {
    inner: Mutex<Slot<T>>,
}

impl<T> LoomTripleBuffer<T> {
    fn new(initial: T) -> Self {
        Self { inner: Mutex::new(Slot { value: initial, generation: 0 }) }
    }

    /// Mirrors `TripleBuffer::publish`: try_lock, mem::swap, bump generation; on
    /// contention return `fresh` UNCHANGED (drop this publish, never block).
    fn publish(&self, mut fresh: T) -> T {
        let mut slot = match self.inner.try_lock() {
            Ok(slot) => slot,
            Err(_) => return fresh, // contended (or poisoned) -- never blocks
        };
        std::mem::swap(&mut slot.value, &mut fresh);
        slot.generation = slot.generation.wrapping_add(1);
        fresh
    }
}

impl<T: Clone> LoomTripleBuffer<T> {
    /// Mirrors `TripleBuffer::try_take`: try_lock, compare generation, clone on new.
    fn try_take(&self, last_gen: u64) -> Option<(u64, T)> {
        let slot = self.inner.try_lock().ok()?;
        if slot.generation == last_gen {
            return None;
        }
        Some((slot.generation, slot.value.clone()))
    }
}

/// SEAM step 2: producer publishes TWICE while the consumer reads ONCE, across
/// every interleaving loom can schedule. Assert the consumer, whenever it lands a
/// `Some`, always sees ONE WHOLE published value (never a torn mix of the two
/// swaps) -- and that every call (publish x2, try_take x1) RETURNS, proving the
/// try_lock-only handoff never deadlocks under genuine cross-thread contention.
#[test]
fn publish_twice_consume_once_never_torn_never_blocks() {
    loom::model(|| {
        let bridge = Arc::new(LoomTripleBuffer::new(0u32));

        let producer = {
            let b = Arc::clone(&bridge);
            thread::spawn(move || {
                let _ = b.publish(1);
                let _ = b.publish(2);
            })
        };

        // Consumer side, running concurrently with both publishes above.
        let taken = bridge.try_take(0);

        producer.join().unwrap();

        if let Some((gen, val)) = taken {
            // Generation only advances on a completed swap -> never 0 once Some.
            assert!(gen >= 1, "torn/impossible generation: {gen}");
            // Whole-value proof: never a value other than one the producer
            // actually swapped in -- a torn mem::swap would surface as some
            // other pattern, which cannot happen since the swap+bump executes
            // entirely inside one try_lock guard.
            assert!(val == 1 || val == 2, "torn state: saw {val} at gen {gen}");
        }
    });
}

/// Drift check (SEAM step 3): run the IDENTICAL op sequence -- publish(1),
/// publish(2), try_take(0) -- against the REAL `forge_hal_clockspine::TripleBuffer` and this
/// file's mirror, and assert identical observable transitions (recycled values +
/// generation ladder + final payload). An edit to
/// `forge_hal_clockspine::triple_buffer::TripleBuffer` that changes its transition table
/// without a matching edit here fails THIS test, not a silent model-drift.
#[test]
fn mirror_matches_forge_hal_clockspine_on_same_op_sequence() {
    loom::model(|| {
        let plane = |b: u8| vec![b; 4];

        let real = forge_hal_clockspine::TripleBuffer::new(plane(0));
        let real_recycled_1 = real.publish(plane(1));
        let real_recycled_2 = real.publish(plane(2));
        let mut real_dst = plane(0);
        let real_take = real.try_take(0, &mut real_dst);

        let mirror = LoomTripleBuffer::new(plane(0));
        let mirror_recycled_1 = mirror.publish(plane(1));
        let mirror_recycled_2 = mirror.publish(plane(2));
        let mirror_take = mirror.try_take(0);

        assert_eq!(real_recycled_1, mirror_recycled_1, "publish() #1 recycled-value drift");
        assert_eq!(real_recycled_2, mirror_recycled_2, "publish() #2 recycled-value drift");
        assert_eq!(real_take, Some(2), "forge_hal_clockspine generation-ladder drift");
        match mirror_take {
            Some((gen, val)) => {
                assert_eq!(gen, 2, "mirror generation-ladder drift vs forge_hal_clockspine");
                assert_eq!(val, real_dst, "mirror payload drift vs forge_hal_clockspine dst");
            }
            None => panic!("mirror try_take drift: forge_hal_clockspine returned Some(2)"),
        }
    });
}
