//! Zero-copy byte router: the `UmpPacket64` wire format + a fixed-capacity,
//! lock-free SPSC ring buffer for the Android<->desktop link edge.
//!
//! Same lock-free head/tail atomics idiom as `forge-input::spsc::SpscRing`
//! (Relaxed load / Release store, wrapping indices), reimplemented on a
//! const-generic array -- zero heap allocation at any point, not just after
//! `new()` -- so this cdylib doesn't pull the engine workspace into the
//! `aarch64-linux-android` target.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// One 64-bit (8-byte) Universal-MIDI-Packet-style event crossing the link:
/// `header` tags the source/kind, `timestamp` is a local delta tick (not
/// wall-clock -- [`crate::governor::RestGate`] phase-aligns against it),
/// `payload` carries up to 4 raw bytes (a text chunk or a control word).
#[repr(C, packed)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UmpPacket64 {
    /// Tags the packet's source/kind (e.g. [`crate::link_bridge::TAG_TEXT`]).
    pub header: u16,
    /// Local delta tick, not wall-clock -- the governor phase-aligns against it.
    pub timestamp: u16,
    /// Up to 4 raw bytes: a text chunk or a control word.
    pub payload: u32,
}

impl UmpPacket64 {
    /// Wire size in bytes: always 8.
    pub const WIRE_SIZE: usize = core::mem::size_of::<Self>();

    /// Construct one packet from its three wire fields.
    #[inline]
    pub const fn new(header: u16, timestamp: u16, payload: u32) -> Self {
        Self { header, timestamp, payload }
    }

    /// Zero-copy parse of one packet off an 8-byte wire slice.
    #[inline]
    pub fn from_wire(bytes: &[u8]) -> Option<Self> {
        Self::read_from_bytes(bytes).ok()
    }

    /// Zero-copy encode into an 8-byte wire slice. `false` if `out` is short.
    #[inline]
    pub fn write_wire(&self, out: &mut [u8]) -> bool {
        self.write_to(out).is_ok()
    }
}

/// Fixed-capacity lock-free SPSC ring of [`UmpPacket64`]. `N` is the slot
/// count. Backed by a const-generic array: no heap allocation at any point
/// in its lifetime, so an instance can live in static/shared memory bridging
/// the link-ingest thread and the drain-and-dispatch thread in `lib.rs`.
pub struct ByteRouter<const N: usize> {
    slots: [UnsafeCell<MaybeUninit<UmpPacket64>>; N],
    head: AtomicUsize, // producer-owned
    tail: AtomicUsize, // consumer-owned
}

// SAFETY: single-producer single-consumer only; every slot access is guarded
// by the head/tail atomics below, so the `UnsafeCell`s never alias across
// the producer/consumer boundary. `UmpPacket64` is `Copy + Send`.
unsafe impl<const N: usize> Send for ByteRouter<N> {}
unsafe impl<const N: usize> Sync for ByteRouter<N> {}

impl<const N: usize> ByteRouter<N> {
    /// `N` must be > 0 -- a zero-capacity ring cannot exist.
    pub fn new() -> Self {
        assert!(N > 0, "ByteRouter capacity must be > 0");
        Self {
            slots: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Producer: push one packet. `Err(pkt)` if the ring is full -- caller
    /// decides whether to [`Self::force_push`] (drop oldest) or back off.
    pub fn try_push(&self, pkt: UmpPacket64) -> Result<(), UmpPacket64> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        if head.wrapping_sub(tail) >= N {
            return Err(pkt);
        }
        let idx = head % N;
        // SAFETY: producer is the sole writer to slots[idx] while head is
        // ahead of tail; the consumer cannot be reading it (not yet published).
        unsafe { (*self.slots[idx].get()).write(pkt) };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Producer: push, overwriting the oldest slot when full. Returns `true`
    /// if an overwrite occurred (packet loss the caller may want to count).
    pub fn force_push(&self, pkt: UmpPacket64) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        let full = head.wrapping_sub(tail) >= N;
        if full {
            self.tail.store(tail.wrapping_add(1), Ordering::Release);
        }
        let idx = head % N;
        // SAFETY: same as try_push -- producer owns this slot.
        unsafe { (*self.slots[idx].get()).write(pkt) };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        full
    }

    /// Consumer: pop the oldest packet. `None` if empty.
    pub fn try_pop(&self) -> Option<UmpPacket64> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Relaxed);
        if tail == head {
            return None;
        }
        let idx = tail % N;
        // SAFETY: consumer is the sole reader of slots[idx] while tail is
        // behind head; the producer published it via Release before head advanced.
        let pkt = unsafe { (*self.slots[idx].get()).assume_init() };
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(pkt)
    }

    /// Packets currently queued.
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }

    /// `true` when no packets are queued.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fixed slot count `N`.
    pub const fn capacity(&self) -> usize {
        N
    }
}

impl<const N: usize> Default for ByteRouter<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_is_8_bytes_and_round_trips_the_wire() {
        assert_eq!(UmpPacket64::WIRE_SIZE, 8);
        let pkt = UmpPacket64::new(0x5458, 42, 0xdead_beef);
        let mut wire = [0u8; 8];
        assert!(pkt.write_wire(&mut wire));
        assert_eq!(UmpPacket64::from_wire(&wire), Some(pkt));
    }

    #[test]
    fn short_wire_slice_fails_to_parse() {
        let short = [0u8; 4];
        assert_eq!(UmpPacket64::from_wire(&short), None);
    }

    #[test]
    fn push_pop_round_trip_is_fifo() {
        let ring: ByteRouter<4> = ByteRouter::new();
        ring.try_push(UmpPacket64::new(1, 1, 1)).unwrap();
        ring.try_push(UmpPacket64::new(1, 2, 2)).unwrap();
        let first = ring.try_pop().unwrap();
        let second = ring.try_pop().unwrap();
        let first_ts = first.timestamp;
        let second_ts = second.timestamp;
        assert_eq!(first_ts, 1);
        assert_eq!(second_ts, 2);
        assert!(ring.try_pop().is_none());
    }

    #[test]
    fn try_push_to_full_ring_returns_err() {
        let ring: ByteRouter<2> = ByteRouter::new();
        ring.try_push(UmpPacket64::default()).unwrap();
        ring.try_push(UmpPacket64::default()).unwrap();
        assert!(ring.try_push(UmpPacket64::default()).is_err());
    }

    #[test]
    fn force_push_overwrites_oldest_when_full() {
        let ring: ByteRouter<2> = ByteRouter::new();
        ring.try_push(UmpPacket64::new(0, 10, 0)).unwrap();
        ring.try_push(UmpPacket64::new(0, 20, 0)).unwrap();
        assert!(ring.force_push(UmpPacket64::new(0, 30, 0)));
        let a = ring.try_pop().unwrap();
        let b = ring.try_pop().unwrap();
        let a_ts = a.timestamp;
        let b_ts = b.timestamp;
        assert_eq!(a_ts, 20);
        assert_eq!(b_ts, 30);
    }

    #[test]
    fn len_and_is_empty_track_the_queue() {
        let ring: ByteRouter<4> = ByteRouter::new();
        assert!(ring.is_empty());
        ring.try_push(UmpPacket64::default()).unwrap();
        assert_eq!(ring.len(), 1);
        ring.try_pop().unwrap();
        assert!(ring.is_empty());
    }
}
