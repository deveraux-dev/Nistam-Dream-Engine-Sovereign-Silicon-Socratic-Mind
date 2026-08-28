//! TripleBuffer — the clock-isolation bridge for ONE producer and TWO consumers.
//!
//! This is the lock-free double-buffer generalised to the GPU.CPU hybrid dual-loop
//! compositor's THREE clocks aligning to three threads:
//!
//! * **Producer** — the audio DSP clock (~10 ms block).
//! * **Consumer A** — the 120 Hz CPU overlay clock.
//! * **Consumer B** — the uncapped GPU compositor clock.
//!
//! The hot path **NEVER blocks** — both consumers use `try_lock`; a miss returns
//! `None` and the caller reuses its last snapshot. Zero steady-state heap — `publish`
//! is a pure `mem::swap`; `try_take` copies into a caller-owned `dst` that reuses
//! its existing allocation.

use std::sync::Mutex;

/// A payload that can be copied across the clock bridge while **reusing** the
/// destination's existing allocation — the contract that keeps steady-state heap
/// traffic at zero.
///
/// Implementations must overwrite `dst` with `self`'s contents, reusing `dst`'s
/// buffer if it is large enough (no allocation in steady state).
pub trait ClockPlane {
    /// Overwrite `dst` with `self`'s contents, reusing `dst`'s buffer if large enough.
    fn copy_into(&self, dst: &mut Self);
}

impl ClockPlane for Vec<u8> {
    #[inline]
    fn copy_into(&self, dst: &mut Self) {
        if dst.len() != self.len() {
            dst.resize(self.len(), 0);
        }
        dst.copy_from_slice(self);
    }
}

impl ClockPlane for Vec<i32> {
    #[inline]
    fn copy_into(&self, dst: &mut Self) {
        if dst.len() != self.len() {
            dst.resize(self.len(), 0);
        }
        dst.copy_from_slice(self);
    }
}

/// ClockPlane impl for `Vec<f32>`, used by forge-audio-v3's mic_capture and audio DSP planes.
///
/// This is a render/DSP boundary payload, NOT SIM state. While forge-hal-clockspine's
/// fixed.rs integer law (SimTick, Permyriad) governs deterministic SIM state, the
/// TripleBuffer is a clock-isolation bridge that transfers data between the audio DSP
/// clock (~10 ms blocks of PCM samples) and the CPU/GPU consumer clocks. Audio samples
/// are naturally f32 in DSP processing, so this payload type is legal here.
impl ClockPlane for Vec<f32> {
    #[inline]
    fn copy_into(&self, dst: &mut Self) {
        if dst.len() != self.len() {
            dst.resize(self.len(), 0.0);
        }
        dst.copy_from_slice(self);
    }
}

struct Slot<T> {
    /// The published value.
    value: T,
    /// Generation counter; incremented on each publish.
    generation: u64,
}

/// Lock-free (`try_lock`) single-producer / multi-consumer clock bridge.
///
/// One producer [`publish`](Self::publish)es a fresh plane (a `mem::swap` under a
/// non-blocking `try_lock`, recycling the old plane back to the producer). Each
/// consumer [`try_take`](Self::try_take)s with `try_lock` — it NEVER blocks on the
/// producer or on another consumer; a miss just reuses its last front.
pub struct TripleBuffer<T> {
    /// Slot holding the published value, generation, and the lock guarding swaps.
    inner: Mutex<Slot<T>>,
}

impl<T> TripleBuffer<T> {
    /// Create a bridge seeded with `initial` as the first published plane (generation
    /// 0). The producer's first `publish` recycles this initial buffer.
    pub fn new(initial: T) -> Self {
        Self { inner: Mutex::new(Slot { value: initial, generation: 0 }) }
    }

    /// **Producer side.** TRY to swap `fresh` into the published slot, returning the
    /// OLD plane for the producer to refill — two (or more) planes ping-pong, so
    /// steady state is alloc-free. Bumps the generation so consumers can tell new
    /// from unchanged.
    ///
    /// Uses `try_lock`, never a blocking `lock()`. If a consumer momentarily holds
    /// the slot, the producer does NOT block: it returns `fresh` UNCHANGED and
    /// re-publishes next frame.
    pub fn publish(&self, mut fresh: T) -> T {
        let mut slot = match self.inner.try_lock() {
            Ok(slot) => slot,
            Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return fresh,
        };
        std::mem::swap(&mut slot.value, &mut fresh);
        slot.generation = slot.generation.wrapping_add(1);
        fresh
    }

    /// Current published generation (cheap state probe). Returns `None` if the lock
    /// is momentarily contended — never blocks.
    pub fn try_generation(&self) -> Option<u64> {
        self.inner.try_lock().ok().map(|s| s.generation)
    }
}

impl<T: ClockPlane> TripleBuffer<T> {
    /// **Consumer side.** TRY to copy the latest plane into `dst`. `try_lock` means
    /// the consumer NEVER stalls.
    ///
    /// Returns `Some(generation)` on a fresh copy (and `dst` now holds it), or `None`
    /// when the caller should reuse its last front:
    /// * `None` — lock contended (producer publishing, or the other consumer copying), OR
    /// * `None` — generation unchanged since `last_gen` (nothing new to take).
    ///
    /// The consumer passes its OWN `last_gen` and OWN `dst`, so two consumers drain
    /// the same bridge independently.
    pub fn try_take(&self, last_gen: u64, dst: &mut T) -> Option<u64> {
        let slot = self.inner.try_lock().ok()?;
        if slot.generation == last_gen {
            return None;
        }
        slot.value.copy_into(dst);
        Some(slot.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane_u8(fill: u8) -> Vec<u8> {
        vec![fill; 4]
    }

    fn plane_i32(fill: i32) -> Vec<i32> {
        vec![fill; 4]
    }

    #[test]
    fn publish_recycles_old_buffer_zero_alloc() {
        let bridge = TripleBuffer::new(plane_u8(0));
        let spare = plane_u8(0);
        let cap_before = spare.capacity();
        let recycled = bridge.publish(spare);
        assert_eq!(recycled.len(), 4);
        assert_eq!(recycled.capacity(), cap_before);
    }

    #[test]
    fn consumer_takes_fresh_then_reuses_last_on_unchanged() {
        let bridge = TripleBuffer::new(plane_u8(0));
        let _ = bridge.publish(plane_u8(0xAA));
        let mut dst = plane_u8(0);
        let mut last = 0u64;

        let g = bridge.try_take(last, &mut dst).expect("fresh plane");
        assert_eq!(g, 1);
        assert_eq!(dst, plane_u8(0xAA));
        last = g;

        assert!(bridge.try_take(last, &mut dst).is_none(), "must NOT falsely report fresh");
        assert_eq!(dst, plane_u8(0xAA));
    }

    #[test]
    fn two_consumers_drain_independently() {
        let bridge = TripleBuffer::new(plane_u8(0));
        let _ = bridge.publish(plane_u8(0x11));

        let (mut a_dst, mut a_gen) = (plane_u8(0), 0u64);
        let (mut b_dst, mut b_gen) = (plane_u8(0), 0u64);

        a_gen = bridge.try_take(a_gen, &mut a_dst).expect("A fresh");
        b_gen = bridge.try_take(b_gen, &mut b_dst).expect("B still fresh after A");
        assert_eq!(a_gen, 1);
        assert_eq!(b_gen, 1);
        assert_eq!(a_dst, plane_u8(0x11));
        assert_eq!(b_dst, plane_u8(0x11));

        let _ = bridge.publish(plane_u8(0x22));
        a_gen = bridge.try_take(a_gen, &mut a_dst).expect("A sees gen 2");
        assert_eq!(a_gen, 2);
        assert_eq!(a_dst, plane_u8(0x22));
        b_gen = bridge.try_take(b_gen, &mut b_dst).expect("B catches latest");
        assert_eq!(b_gen, 2);
        assert_eq!(b_dst, plane_u8(0x22));
    }

    #[test]
    fn resize_grows_dst_then_reuses() {
        let bridge = TripleBuffer::new(vec![0u8; 0]);
        let _ = bridge.publish(vec![0x7F; 8]);
        let mut dst: Vec<u8> = Vec::new();
        let g = bridge.try_take(0, &mut dst).expect("fresh");
        assert_eq!(g, 1);
        assert_eq!(dst, vec![0x7F; 8]);
    }

    #[test]
    fn i32_vec_clockplane_works() {
        let bridge = TripleBuffer::new(plane_i32(0));
        let _ = bridge.publish(plane_i32(42));
        let mut dst = plane_i32(0);
        let g = bridge.try_take(0, &mut dst).expect("fresh");
        assert_eq!(g, 1);
        assert_eq!(dst, plane_i32(42));
    }

    fn plane_f32(fill: f32) -> Vec<f32> {
        vec![fill; 4]
    }

    #[test]
    fn f32_vec_clockplane_works() {
        let bridge = TripleBuffer::new(plane_f32(0.0));
        let _ = bridge.publish(plane_f32(3.14));
        let mut dst = plane_f32(0.0);
        let g = bridge.try_take(0, &mut dst).expect("fresh");
        assert_eq!(g, 1);
        assert_eq!(dst, plane_f32(3.14));
    }

    #[test]
    fn try_take_under_live_contention_never_blocks() {
        use std::sync::Arc;
        use std::thread;

        let bridge = Arc::new(TripleBuffer::new(plane_u8(0)));
        let producer = {
            let b = Arc::clone(&bridge);
            thread::spawn(move || {
                let mut spare = plane_u8(0);
                for i in 0..10_000u32 {
                    spare.iter_mut().for_each(|b| *b = (i & 0xFF) as u8);
                    spare = b.publish(spare);
                }
            })
        };

        let mut dst = plane_u8(0);
        let mut last = 0u64;
        let mut fresh_hits = 0u32;
        for _ in 0..50_000 {
            if let Some(g) = bridge.try_take(last, &mut dst) {
                assert!(g >= last || last == u64::MAX);
                last = g;
                fresh_hits += 1;
            }
        }
        producer.join().unwrap();
        assert!(fresh_hits > 0, "consumer should have caught fresh planes");
    }
}
