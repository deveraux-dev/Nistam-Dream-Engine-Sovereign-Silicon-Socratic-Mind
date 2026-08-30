//! Unified 120Hz rollback ring — the spine's memory.
//!
//! ONE HOME (L05): this file is the only definition of `TickEngine`,
//! `EntitySnapshot`, `TickDiffFrame`, `ChunkDiffRange`, `ChunkMap` trait.
//!
//! Manages a 120-frame ring buffer of `TickDiffFrame`s. Enforces PDC =
//! Rollback Equivalence: zero heap allocation on the hot path after
//! construction. All 120Hz tick operations use fixed-size arrays and
//! stack-allocated structures.


// ── Constants ────────────────────────────────────────────────────────────────

/// 120 frames at 120Hz = 1 second rollback window.
pub const RING_SIZE: usize = 120;

/// 3x3x3 chunk neighborhood — max active chunks per tick.
pub const MAX_ACTIVE_CHUNKS: usize = 27;

/// Max entities per snapshot. 64 × 28 bytes = 1,792 bytes per frame.
pub const MAX_ENTITIES: usize = 64;

/// Max concurrent compaction signals.
pub const MAX_PENDING_COMPACTIONS: usize = 8;

// ── Error Types ──────────────────────────────────────────────────────────────

/// Errors returned by rollback operations.
#[derive(Debug, PartialEq, Eq)]
pub enum RollbackError {
    /// Requested tick has been evicted from the ring buffer.
    TickEvicted {
        /// The tick number that was requested.
        requested: u64,
        /// The oldest tick still available in the ring.
        oldest_available: u64,
    },
    /// Tick range partially outside the available window.
    PartialRange {
        /// Oldest tick still available.
        available_from: u64,
        /// Most recent tick available.
        available_to: u64,
        /// Start of the requested range.
        requested_from: u64,
        /// End of the requested range.
        requested_to: u64,
    },
    /// Target tick is ahead of the current tick.
    TickAhead {
        /// The target tick requested.
        target: u64,
        /// The current latest tick.
        current: u64,
    },
}

// ── Entity Types ─────────────────────────────────────────────────────────────

/// Per-entity position + combat state. MilliUnit precision (i64).
#[derive(Copy, Clone, Debug, Default)]
pub struct EntityPosition {
    /// X coordinate in MilliUnits.
    pub x: i64,
    /// Y coordinate in MilliUnits.
    pub y: i64,
    /// Z coordinate in MilliUnits.
    pub z: i64,
    /// Health points.
    pub health: u16,
    /// Combat or status flags.
    pub status_bits: u16,
}

/// Compact per-frame entity state. Fixed-size, Copy, no heap.
#[derive(Copy, Clone, Debug)]
pub struct EntitySnapshot {
    /// Array of entity position + state structs.
    pub positions: [EntityPosition; MAX_ENTITIES],
    /// Number of valid entities in this snapshot.
    pub entity_count: u8,
}

impl Default for EntitySnapshot {
    fn default() -> Self {
        Self {
            positions: [EntityPosition::default(); MAX_ENTITIES],
            entity_count: 0,
        }
    }
}

// ── Snapshot Encoding (L07 bijection) ────────────────────────────────────────

/// Pack `EntitySnapshot` into a byte array.
/// Inverse of [`unpack_snapshot`].
#[inline]
pub fn pack_snapshot(snap: &EntitySnapshot) -> [u8; SNAPSHOT_PACKED_SIZE] {
    let mut buf = [0u8; SNAPSHOT_PACKED_SIZE];
    buf[0] = snap.entity_count;
    for i in 0..snap.entity_count as usize {
        let ep = &snap.positions[i];
        let base = 1 + i * 28;
        buf[base..base + 8].copy_from_slice(&ep.x.to_le_bytes());
        buf[base + 8..base + 16].copy_from_slice(&ep.y.to_le_bytes());
        buf[base + 16..base + 24].copy_from_slice(&ep.z.to_le_bytes());
        buf[base + 24..base + 26].copy_from_slice(&ep.health.to_le_bytes());
        buf[base + 26..base + 28].copy_from_slice(&ep.status_bits.to_le_bytes());
    }
    buf
}

/// Unpack a byte array into `EntitySnapshot`.
/// Inverse of [`pack_snapshot`].
#[inline]
pub fn unpack_snapshot(buf: &[u8; SNAPSHOT_PACKED_SIZE]) -> EntitySnapshot {
    let entity_count = buf[0];
    let mut snap = EntitySnapshot::default();
    snap.entity_count = entity_count;
    for i in 0..entity_count as usize {
        let base = 1 + i * 28;
        let x_bytes: [u8; 8] = [
            buf[base], buf[base + 1], buf[base + 2], buf[base + 3], buf[base + 4], buf[base + 5],
            buf[base + 6], buf[base + 7],
        ];
        let y_bytes: [u8; 8] = [
            buf[base + 8], buf[base + 9], buf[base + 10], buf[base + 11], buf[base + 12],
            buf[base + 13], buf[base + 14], buf[base + 15],
        ];
        let z_bytes: [u8; 8] = [
            buf[base + 16], buf[base + 17], buf[base + 18], buf[base + 19], buf[base + 20],
            buf[base + 21], buf[base + 22], buf[base + 23],
        ];
        let health_bytes: [u8; 2] = [buf[base + 24], buf[base + 25]];
        let status_bytes: [u8; 2] = [buf[base + 26], buf[base + 27]];

        let x = i64::from_le_bytes(x_bytes);
        let y = i64::from_le_bytes(y_bytes);
        let z = i64::from_le_bytes(z_bytes);
        let health = u16::from_le_bytes(health_bytes);
        let status_bits = u16::from_le_bytes(status_bytes);

        snap.positions[i] = EntityPosition { x, y, z, health, status_bits };
    }
    snap
}

/// Size of packed snapshot: 1 byte (entity_count) + 64 * 28 bytes (positions).
pub const SNAPSHOT_PACKED_SIZE: usize = 1 + MAX_ENTITIES * 28;

// ── Diff Range Types ────────────────────────────────────────────────────────

/// Reference into a chunk diff ledger for one tick's worth of diffs.
/// Zero-copy: stores indices, not data.
#[derive(Copy, Clone, Debug, Default)]
pub struct ChunkDiffRange {
    /// Chunk X coordinate.
    pub coord_x: i32,
    /// Chunk Y coordinate.
    pub coord_y: i32,
    /// Chunk Z coordinate.
    pub coord_z: i32,
    /// Starting index into the ChunkLedger's diff array.
    pub start_idx: u16,
    /// Count of consecutive diffs from start_idx.
    pub count: u16,
}

impl ChunkDiffRange {
    /// Encode to 16 bytes for storage/transport.
    #[inline]
    pub fn encode(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(&self.coord_x.to_le_bytes());
        buf[4..8].copy_from_slice(&self.coord_y.to_le_bytes());
        buf[8..12].copy_from_slice(&self.coord_z.to_le_bytes());
        buf[12..14].copy_from_slice(&self.start_idx.to_le_bytes());
        buf[14..16].copy_from_slice(&self.count.to_le_bytes());
        buf
    }

    /// Decode from 16 bytes.
    #[inline]
    pub fn decode(buf: &[u8; 16]) -> Self {
        let coord_x = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let coord_y = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let coord_z = i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let start_idx = u16::from_le_bytes([buf[12], buf[13]]);
        let count = u16::from_le_bytes([buf[14], buf[15]]);
        Self { coord_x, coord_y, coord_z, start_idx, count }
    }
}

/// Per-tick container: voxel diff ranges + entity snapshot + metadata.
/// Fixed size, no heap allocation.
#[derive(Copy, Clone)]
pub struct TickDiffFrame {
    /// The tick index this frame records.
    pub tick: u64,
    /// Array of chunk diff range references.
    pub chunk_ranges: [ChunkDiffRange; MAX_ACTIVE_CHUNKS],
    /// Number of valid chunk ranges in this frame.
    pub chunk_range_count: u8,
    /// Snapshot of all active entities at this tick.
    pub entity_snapshot: EntitySnapshot,
    /// Input buttons/sticks packed into 16 bits.
    pub input_bits: u16,
    /// State hash for desync detection (CRC or similar).
    pub state_hash: u64,
}

impl Default for TickDiffFrame {
    fn default() -> Self {
        Self {
            tick: 0,
            chunk_ranges: [ChunkDiffRange::default(); MAX_ACTIVE_CHUNKS],
            chunk_range_count: 0,
            entity_snapshot: EntitySnapshot::default(),
            input_bits: 0,
            state_hash: 0,
        }
    }
}

// Size check: TickDiffFrame is fixed-size, aligned for cache coherence.
// Layout: tick(8) + chunk_ranges(432) + chunk_range_count(1) + entity_snapshot(1800+) + input_bits(2) + state_hash(8) ~ 2250 bytes
// No heap allocation — fixed stack footprint per frame.

// ── Rollback Ring ────────────────────────────────────────────────────────────

/// Unified 120Hz simulation driver.
/// Manages a 120-frame ring buffer of TickDiffFrames.
/// Zero heap allocation on the hot path after construction.
#[derive(Clone)]
pub struct RollbackRing {
    frames: [TickDiffFrame; RING_SIZE],
    write_cursor: u8,
    valid_count: u8,
    confirmed_tick: u64,
    pending_compactions: [Option<ChunkCoord>; MAX_PENDING_COMPACTIONS],
    pending_compaction_count: u8,
}

/// A chunk coordinate: `(x, y, z)` in chunk-space.
///
/// **Twin definition (L05 one-home): Also defined in
/// `forge-physics-v3/src/types.rs:40` (identical struct). No existing dependency
/// edge exists to enable re-export. This is a scoped local definition per
/// BLUEPRINT-SUBSTRATE-CENSUS-2026-08-11; both homes are noted and tracked in
/// DEAD-LEDGER (2026-08-17).**
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    /// X chunk coordinate.
    pub x: i32,
    /// Y chunk coordinate.
    pub y: i32,
    /// Z chunk coordinate.
    pub z: i32,
}

impl RollbackRing {
    /// Construct with zeroed ring buffer. Single allocation point.
    #[inline]
    pub fn new() -> Self {
        Self {
            frames: [TickDiffFrame::default(); RING_SIZE],
            write_cursor: 0,
            valid_count: 0,
            confirmed_tick: 0,
            pending_compactions: [None; MAX_PENDING_COMPACTIONS],
            pending_compaction_count: 0,
        }
    }

    /// Record a completed tick into the ring buffer.
    #[inline]
    pub fn record_tick(
        &mut self,
        tick: u64,
        chunk_ranges: [ChunkDiffRange; MAX_ACTIVE_CHUNKS],
        chunk_range_count: u8,
        entity_snapshot: EntitySnapshot,
        input_bits: u16,
        state_hash: u64,
    ) {
        let frame = &mut self.frames[self.write_cursor as usize];
        frame.tick = tick;
        frame.chunk_ranges = chunk_ranges;
        frame.chunk_range_count = chunk_range_count;
        frame.entity_snapshot = entity_snapshot;
        frame.input_bits = input_bits;
        frame.state_hash = state_hash;

        self.write_cursor = ((self.write_cursor as usize + 1) % RING_SIZE) as u8;
        if (self.valid_count as usize) < RING_SIZE {
            self.valid_count += 1;
        }
    }

    /// Look up a frame by tick number. Returns `None` if evicted.
    #[inline]
    pub fn find_by_tick(&self, tick: u64) -> Option<&TickDiffFrame> {
        let count = self.valid_count as usize;
        if count == 0 {
            return None;
        }
        // Scan valid frames: the most recent `valid_count` entries
        // ending at write_cursor - 1.
        for i in 0..count {
            let idx = (self.write_cursor as usize + RING_SIZE - 1 - i) % RING_SIZE;
            if self.frames[idx].tick == tick {
                return Some(&self.frames[idx]);
            }
        }
        None
    }

    /// Most recent recorded frame.
    #[inline]
    pub fn latest(&self) -> Option<&TickDiffFrame> {
        if self.valid_count == 0 {
            return None;
        }
        let idx = (self.write_cursor as usize + RING_SIZE - 1) % RING_SIZE;
        Some(&self.frames[idx])
    }

    /// Current valid frame count (0..=120).
    #[inline]
    pub fn valid_count(&self) -> u8 {
        self.valid_count
    }


    /// Advance the confirmed tick watermark. Monotonically increasing.
    /// Stale calls (tick <= current confirmed) are ignored.
    /// Returns the current confirmed tick after the operation.
    #[inline]
    pub fn confirm_tick(&mut self, tick: u64) -> u64 {
        if tick > self.confirmed_tick {
            self.confirmed_tick = tick;
        }
        self.confirmed_tick
    }

    /// Current confirmed tick watermark.
    #[inline]
    pub fn confirmed_tick(&self) -> u64 {
        self.confirmed_tick
    }

    /// Retrieve the 64-bit state hash for a given tick (desync detection).
    #[inline]
    pub fn state_hash(&self, tick: u64) -> Option<u64> {
        self.find_by_tick(tick).map(|f| f.state_hash)
    }

    /// Signal that a chunk needs compaction (its ChunkLedger hit capacity).
    /// Non-blocking — just records the coord in the pending list.
    #[inline]
    pub fn signal_compaction(&mut self, coord: ChunkCoord) {
        if (self.pending_compaction_count as usize) < MAX_PENDING_COMPACTIONS {
            self.pending_compactions[self.pending_compaction_count as usize] = Some(coord);
            self.pending_compaction_count += 1;
        }
        // If list is full, silently drop (caller should drain first)
    }

    /// Return the slice of chunks needing compaction.
    #[inline]
    pub fn pending_compactions(&self) -> &[Option<ChunkCoord>] {
        &self.pending_compactions[..self.pending_compaction_count as usize]
    }

    /// Notify that compaction is complete for a chunk. Removes it from the pending list.
    #[inline]
    pub fn compaction_complete(&mut self, coord: ChunkCoord) {
        // Find and remove by swapping with last
        for i in 0..self.pending_compaction_count as usize {
            if self.pending_compactions[i] == Some(coord) {
                let last = self.pending_compaction_count as usize - 1;
                self.pending_compactions[i] = self.pending_compactions[last];
                self.pending_compactions[last] = None;
                self.pending_compaction_count -= 1;
                return;
            }
        }
    }

    /// Depth of rollback ring: how many ticks are valid.
    #[inline]
    pub fn depth(&self) -> usize {
        self.valid_count as usize
    }
}

impl Default for RollbackRing {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_bijection_empty() {
        let snap = EntitySnapshot::default();
        let packed = pack_snapshot(&snap);
        let unpacked = unpack_snapshot(&packed);
        assert_eq!(unpacked.entity_count, 0);
    }

    #[test]
    fn test_snapshot_bijection_one_entity() {
        let mut snap = EntitySnapshot::default();
        snap.entity_count = 1;
        snap.positions[0] = EntityPosition { x: 1000, y: 2000, z: 3000, health: 42, status_bits: 7 };

        let packed = pack_snapshot(&snap);
        let unpacked = unpack_snapshot(&packed);

        assert_eq!(unpacked.entity_count, 1);
        assert_eq!(unpacked.positions[0].x, 1000);
        assert_eq!(unpacked.positions[0].y, 2000);
        assert_eq!(unpacked.positions[0].z, 3000);
        assert_eq!(unpacked.positions[0].health, 42);
        assert_eq!(unpacked.positions[0].status_bits, 7);
    }

    #[test]
    fn test_snapshot_bijection_max_entities() {
        let mut snap = EntitySnapshot::default();
        snap.entity_count = MAX_ENTITIES as u8;
        for i in 0..MAX_ENTITIES {
            snap.positions[i] = EntityPosition {
                x: (i as i64) * 1000,
                y: (i as i64) * 2000,
                z: (i as i64) * 3000,
                health: (i as u16) * 10,
                status_bits: (i as u16) * 7,
            };
        }

        let packed = pack_snapshot(&snap);
        let unpacked = unpack_snapshot(&packed);

        assert_eq!(unpacked.entity_count, MAX_ENTITIES as u8);
        for i in 0..MAX_ENTITIES {
            assert_eq!(unpacked.positions[i].x, (i as i64) * 1000);
            assert_eq!(unpacked.positions[i].y, (i as i64) * 2000);
            assert_eq!(unpacked.positions[i].z, (i as i64) * 3000);
            assert_eq!(unpacked.positions[i].health, (i as u16) * 10);
            assert_eq!(unpacked.positions[i].status_bits, (i as u16) * 7);
        }
    }

    #[test]
    fn test_chunk_diff_range_bijection() {
        let range = ChunkDiffRange {
            coord_x: -5,
            coord_y: 10,
            coord_z: -3,
            start_idx: 100,
            count: 42,
        };

        let encoded = range.encode();
        let decoded = ChunkDiffRange::decode(&encoded);

        assert_eq!(decoded.coord_x, -5);
        assert_eq!(decoded.coord_y, 10);
        assert_eq!(decoded.coord_z, -3);
        assert_eq!(decoded.start_idx, 100);
        assert_eq!(decoded.count, 42);
    }

    #[test]
    fn test_rollback_ring_recording() {
        let mut ring = RollbackRing::new();
        assert_eq!(ring.valid_count(), 0);
        assert!(ring.latest().is_none());

        let mut snap = EntitySnapshot::default();
        snap.entity_count = 1;

        ring.record_tick(
            0,
            [ChunkDiffRange::default(); MAX_ACTIVE_CHUNKS],
            0,
            snap,
            0,
            0,
        );

        assert_eq!(ring.valid_count(), 1);
        assert_eq!(ring.latest().unwrap().tick, 0);
        assert_eq!(ring.find_by_tick(0).unwrap().entity_snapshot.entity_count, 1);
    }

    #[test]
    fn test_rollback_ring_eviction() {
        let mut ring = RollbackRing::new();

        // Record 150 ticks (exceeds RING_SIZE = 120)
        for t in 0..150 {
            ring.record_tick(
                t,
                [ChunkDiffRange::default(); MAX_ACTIVE_CHUNKS],
                0,
                EntitySnapshot::default(),
                0,
                t,
            );
        }

        // Only 120 should be valid
        assert_eq!(ring.valid_count(), 120);

        // Latest should be tick 149
        assert_eq!(ring.latest().unwrap().tick, 149);

        // Early ticks should be evicted
        assert!(ring.find_by_tick(0).is_none());
        assert!(ring.find_by_tick(29).is_none());

        // Recent 120 should be findable
        assert!(ring.find_by_tick(30).is_some());
        assert!(ring.find_by_tick(149).is_some());
    }

    #[test]
    fn test_confirmed_tick_monotonic() {
        let mut ring = RollbackRing::new();

        let r1 = ring.confirm_tick(100);
        assert_eq!(r1, 100);

        let r2 = ring.confirm_tick(50); // Stale
        assert_eq!(r2, 100);

        let r3 = ring.confirm_tick(200);
        assert_eq!(r3, 200);

        assert_eq!(ring.confirmed_tick(), 200);
    }

    #[test]
    fn test_compaction_signaling() {
        let mut ring = RollbackRing::new();

        let coord1 = ChunkCoord { x: 0, y: 0, z: 0 };
        let coord2 = ChunkCoord { x: 1, y: 1, z: 1 };

        ring.signal_compaction(coord1);
        ring.signal_compaction(coord2);

        let pending = ring.pending_compactions();
        assert_eq!(pending.len(), 2);

        ring.compaction_complete(coord1);
        let pending = ring.pending_compactions();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0], Some(coord2));
    }

    // L18 SABOTAGE TEST: verify ring bounds gate
    #[test]
    #[should_panic(expected = "SABOTAGE")]
    fn test_sabotage_ring_bounds() {
        let mut ring = RollbackRing::new();

        // Record exactly RING_SIZE frames
        for t in 0..RING_SIZE as u64 {
            ring.record_tick(
                t,
                [ChunkDiffRange::default(); MAX_ACTIVE_CHUNKS],
                0,
                EntitySnapshot::default(),
                0,
                0,
            );
        }

        // Record one more (wraps over the first)
        ring.record_tick(
            RING_SIZE as u64,
            [ChunkDiffRange::default(); MAX_ACTIVE_CHUNKS],
            0,
            EntitySnapshot::default(),
            0,
            0,
        );

        // Sabotage: assert that tick 0 is still available (it should NOT be)
        assert!(
            ring.find_by_tick(0).is_some(),
            "SABOTAGE: tick 0 should have been evicted after recording {} ticks into a {}-frame ring",
            RING_SIZE + 1,
            RING_SIZE
        );
    }
}
