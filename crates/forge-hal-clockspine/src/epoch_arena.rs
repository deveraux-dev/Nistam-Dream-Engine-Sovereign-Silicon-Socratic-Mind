//! epoch_arena.rs — Epoch-bump allocator aligned to the T1 tick boundary.
//! Ported verbatim from `F:\NewRepo\crates\forge-hal\src\epoch_arena.rs` 2026-08-14.
//!
//! Unreal Engine's frame allocator pattern adapted for forge's TWO-CLOCKS law:
//! a pre-sized slab that hands out bump-pointer slices during a T1 tick, then
//! resets to zero with a single pointer assignment at the tick boundary.
//!
//! **Invariants:**
//! - Zero individual frees — amortised to ONE pointer reset per 120Hz tick.
//! - Pre-allocated at init (cold path, `@forge:allow_alloc`). Never grows.
//! - No `unsafe` in the public API. Uses index math, not raw pointers.
//! - Thread-local by design: T1 owns its arena, T3 owns its own (if needed).
//!
//! **Sound Gate compliance:**
//! The slab is pre-allocated once. `alloc()` is a pointer bump (addition +
//! comparison). `reset()` is a single `self.cursor = 0`. Both are sub-nanosecond
//! and heap-free at steady state.

/// A fixed-capacity bump allocator that resets every epoch (T1 tick boundary).
///
/// Generic over nothing — stores raw bytes. Callers slice out `&mut [u8]` and
/// reinterpret as needed (typically via `bytemuck::from_bytes_mut` or similar).
///
/// # Example
/// ```
/// use forge_hal_clockspine::epoch_arena::EpochArena;
///
/// let mut arena = EpochArena::new(4096); // 4KB per tick
/// let slot = arena.alloc(64).expect("fits");
/// slot.fill(0xAA);
/// assert_eq!(arena.used(), 64);
///
/// arena.reset(); // tick boundary — one pointer reset
/// assert_eq!(arena.used(), 0);
/// ```
pub struct EpochArena {
    /// Pre-allocated slab. Size is fixed at init; never reallocated.
    slab: Vec<u8>, // @forge:allow_alloc — cold init only
    /// Bump cursor: next free byte index. Reset to 0 at tick boundary.
    cursor: usize,
    /// High-water mark across all epochs (diagnostic, not functional).
    high_water: usize,
}

impl EpochArena {
    /// Allocate a new arena with `capacity` bytes. Cold path only.
    ///
    /// # Panics
    /// Panics if `capacity == 0`.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "EpochArena capacity must be > 0");
        Self {
            slab: vec![0u8; capacity], // @forge:allow_alloc — one-time init
            cursor: 0,
            high_water: 0,
        }
    }

    /// Bump-allocate `size` bytes from the arena. Returns `None` if the arena
    /// is exhausted for this epoch (caller must handle gracefully — typically
    /// by skipping optional work or logging a LOUD budget warning).
    ///
    /// Alignment: returned slice starts at `self.cursor` (byte-aligned).
    /// For alignment > 1, use [`alloc_aligned`](Self::alloc_aligned).
    #[inline]
    pub fn alloc(&mut self, size: usize) -> Option<&mut [u8]> {
        let end = self.cursor.checked_add(size)?;
        if end > self.slab.len() {
            return None;
        }
        let start = self.cursor;
        self.cursor = end;
        if end > self.high_water {
            self.high_water = end;
        }
        Some(&mut self.slab[start..end])
    }

    /// Bump-allocate `size` bytes with the given power-of-two alignment.
    /// Wastes up to `align - 1` bytes of padding. Returns `None` on exhaustion.
    #[inline]
    pub fn alloc_aligned(&mut self, size: usize, align: usize) -> Option<&mut [u8]> {
        debug_assert!(align.is_power_of_two(), "align must be power of two");
        let aligned_cursor = (self.cursor + align - 1) & !(align - 1);
        let end = aligned_cursor.checked_add(size)?;
        if end > self.slab.len() {
            return None;
        }
        self.cursor = end;
        if end > self.high_water {
            self.high_water = end;
        }
        Some(&mut self.slab[aligned_cursor..end])
    }

    /// Reset the arena for a new epoch. ONE pointer assignment — the entire
    /// tick's allocations are "freed" in a single operation.
    ///
    /// Data in the slab is NOT zeroed (callers must not rely on zeroed memory
    /// from a previous epoch). This is intentional: zero-fill would cost
    /// O(capacity) per tick; bump-allocators don't guarantee initialisation.
    #[inline]
    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    /// Bytes currently allocated in this epoch.
    #[inline]
    pub fn used(&self) -> usize {
        self.cursor
    }

    /// Total capacity of the arena (fixed at init).
    #[inline]
    pub fn capacity(&self) -> usize {
        self.slab.len()
    }

    /// High-water mark: maximum bytes used in any single epoch since creation.
    /// Useful for right-sizing the arena after profiling.
    #[inline]
    pub fn high_water(&self) -> usize {
        self.high_water
    }

    /// Remaining bytes available in this epoch.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.slab.len() - self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_alloc_and_reset() {
        let mut arena = EpochArena::new(256);
        assert_eq!(arena.used(), 0);
        assert_eq!(arena.capacity(), 256);

        let slot = arena.alloc(64).unwrap();
        assert_eq!(slot.len(), 64);
        assert_eq!(arena.used(), 64);

        let slot2 = arena.alloc(128).unwrap();
        assert_eq!(slot2.len(), 128);
        assert_eq!(arena.used(), 192);

        // Reset = one pointer assignment
        arena.reset();
        assert_eq!(arena.used(), 0);
        assert_eq!(arena.high_water(), 192);

        // Can allocate again from the start
        let slot3 = arena.alloc(256).unwrap();
        assert_eq!(slot3.len(), 256);
        assert_eq!(arena.used(), 256);
    }

    #[test]
    fn exhaustion_returns_none() {
        let mut arena = EpochArena::new(64);
        assert!(arena.alloc(64).is_some());
        assert!(arena.alloc(1).is_none()); // exhausted
        arena.reset();
        assert!(arena.alloc(1).is_some()); // alive again
    }

    #[test]
    fn aligned_alloc() {
        let mut arena = EpochArena::new(256);
        // Start at 0, alloc 1 byte to misalign
        arena.alloc(1).unwrap();
        assert_eq!(arena.used(), 1);

        // Align to 16 — should skip to offset 16
        let slot = arena.alloc_aligned(32, 16).unwrap();
        assert_eq!(slot.len(), 32);
        // cursor should be at 16 + 32 = 48
        assert_eq!(arena.used(), 48);
    }

    #[test]
    fn high_water_tracks_max() {
        let mut arena = EpochArena::new(1024);
        arena.alloc(100).unwrap();
        arena.reset();
        arena.alloc(200).unwrap();
        arena.reset();
        arena.alloc(50).unwrap();
        assert_eq!(arena.high_water(), 200);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn zero_capacity_panics() {
        EpochArena::new(0);
    }
}
