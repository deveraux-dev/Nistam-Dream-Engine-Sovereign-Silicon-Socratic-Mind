//! Arena State — deterministic 120Hz tick kernel + entity state packing.
//!
//! Ported by translation from forge-cart-brain::state. Integer-deterministic:
//! same (seed, tick, entity updates) → bit-identical state every replay.
//! No voxel-chunk coupling (the mud is 2D only); pure integer, no floats.
//!
//! MERGE RECEIPT (this session): EntityState grew a kind byte, porting
//! forge-cart-brain-v3::state.rs's 2026-08-16 merge to close a two-crate
//! EntityState divergence found via lateral-criticality this session. Additive,
//! non-breaking: pack_into/unpack_from bijection still holds (see updated tests).
//!
//! **PACKING (L07 bijection)**: EntityState (23 bytes per entity) serializes as:
//! x_mm (8 LE) + y_mm (8 LE) + hp (4 LE) + status (2 LE) + kind (1).

/// 120 frames @ 120Hz = a 1-second rollback window.
pub const RING_SIZE: usize = 120;
/// Max simultaneous entities per arena (players + allies).
pub const MAX_ENTITIES: usize = 64;
/// Max simultaneous mobs (AI domain).
pub const MAX_MOBS: usize = 32;
/// Bytes serialized per entity: x(8) + y(8) + hp(4) + status(2) + kind(1).
pub const ENTITY_PACK_BYTES: usize = 23;

/// One entity's deterministic state. All integer; MilliUnit (mm) position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EntityState {
    /// X position in MilliUnits.
    pub x_mm: i64,
    /// Y position in MilliUnits.
    pub y_mm: i64,
    /// Hit points (health).
    pub hp: i32,
    /// Status/standing bits.
    pub status: u16,
    /// Entity type tag (0 = generic/player; nonzero = hazard/mob kind — mirrors
    /// forge-cart-brain-v3::state::EntityState.kind's 2026-08-16 merge, ported
    /// here to close the two-crate divergence).
    pub kind: u8,
}

impl EntityState {
    /// Pack this entity into a fixed 23-byte buffer at offset. Returns the new offset.
    /// **L07 bijection**: pack + unpack produces the identical EntityState.
    fn pack_into(&self, buf: &mut [u8], offset: usize) -> usize {
        let mut off = offset;
        buf[off..off + 8].copy_from_slice(&self.x_mm.to_le_bytes());
        off += 8;
        buf[off..off + 8].copy_from_slice(&self.y_mm.to_le_bytes());
        off += 8;
        buf[off..off + 4].copy_from_slice(&self.hp.to_le_bytes());
        off += 4;
        buf[off..off + 2].copy_from_slice(&self.status.to_le_bytes());
        off += 2;
        buf[off] = self.kind;
        off += 1;
        off
    }

    /// Unpack an entity from a buffer at offset. Returns the entity and new offset.
    /// **L07 bijection**: unpack + pack produces the identical buffer.
    fn unpack_from(buf: &[u8], offset: usize) -> (Self, usize) {
        let mut off = offset;
        let x_mm = i64::from_le_bytes([
            buf[off], buf[off + 1], buf[off + 2], buf[off + 3],
            buf[off + 4], buf[off + 5], buf[off + 6], buf[off + 7],
        ]);
        off += 8;
        let y_mm = i64::from_le_bytes([
            buf[off], buf[off + 1], buf[off + 2], buf[off + 3],
            buf[off + 4], buf[off + 5], buf[off + 6], buf[off + 7],
        ]);
        off += 8;
        let hp = i32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
        off += 4;
        let status = u16::from_le_bytes([buf[off], buf[off + 1]]);
        off += 2;
        let kind = buf[off];
        off += 1;
        (EntityState { x_mm, y_mm, hp, status, kind }, off)
    }
}

/// Compact, fixed-size per-tick snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntitySnapshot {
    /// The entity states.
    pub entities: [EntityState; MAX_ENTITIES],
    /// Number of active entities.
    pub count: u8,
}

impl Default for EntitySnapshot {
    fn default() -> Self {
        Self { entities: [EntityState::default(); MAX_ENTITIES], count: 0 }
    }
}

/// One recorded tick — the replay frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickFrame {
    /// Tick counter (0, 1, 2, ...).
    pub tick: u64,
    /// Entity snapshot at this tick.
    pub snapshot: EntitySnapshot,
    /// Input bits (button state).
    pub input_bits: u16,
    /// Deterministic state hash.
    pub state_hash: u64,
}

impl Default for TickFrame {
    fn default() -> Self {
        Self { tick: 0, snapshot: EntitySnapshot::default(), input_bits: 0, state_hash: 0 }
    }
}

/// The 120-frame deterministic ring — a slim port of `TickEngine`.
/// Zero heap on the hot path; `frames` is boxed once at `new()`.
pub struct TickRing {
    frames: Box<[TickFrame]>,
    write_cursor: u8,
    valid_count: u8,
}

impl Default for TickRing {
    fn default() -> Self {
        Self::new()
    }
}

impl TickRing {
    /// Create a new empty tick ring with 120-frame capacity.
    pub fn new() -> Self {
        let frames = vec![TickFrame::default(); RING_SIZE].into_boxed_slice();
        Self { frames, write_cursor: 0, valid_count: 0 }
    }

    /// Record a completed frame, evicting the oldest when the ring is full.
    pub fn record(&mut self, frame: TickFrame) {
        self.frames[self.write_cursor as usize] = frame;
        self.write_cursor = ((self.write_cursor as usize + 1) % RING_SIZE) as u8;
        if (self.valid_count as usize) < RING_SIZE {
            self.valid_count += 1;
        }
    }

    /// Look up a frame by tick number; `None` if evicted.
    pub fn find_by_tick(&self, tick: u64) -> Option<&TickFrame> {
        let count = self.valid_count as usize;
        for i in 0..count {
            let idx = (self.write_cursor as usize + RING_SIZE - 1 - i) % RING_SIZE;
            if self.frames[idx].tick == tick {
                return Some(&self.frames[idx]);
            }
        }
        None
    }

    /// Most recent recorded frame.
    pub fn latest(&self) -> Option<&TickFrame> {
        if self.valid_count == 0 {
            return None;
        }
        let idx = (self.write_cursor as usize + RING_SIZE - 1) % RING_SIZE;
        Some(&self.frames[idx])
    }

    /// Current valid frame count (0..=120).
    pub fn valid_count(&self) -> u8 {
        self.valid_count
    }
}

/// The arena's live deterministic state. Integer-only, seed-driven.
pub struct ArenaState {
    /// RNG seed for determinism.
    pub seed: u64,
    /// Current tick counter.
    pub tick: u64,
    /// Player/ally entities.
    pub entities: [EntityState; MAX_ENTITIES],
    /// Number of active entities.
    pub count: u8,
    /// AI mobs.
    pub mobs: [EntityState; MAX_MOBS],
    /// Number of active mobs.
    pub mob_count: u8,
}

impl ArenaState {
    /// Create a new arena state with the given seed and player count.
    /// Players are initialized with 100 HP.
    pub fn new(seed: u64, player_count: u8) -> Self {
        let mut entities = [EntityState::default(); MAX_ENTITIES];
        let count = player_count.min(MAX_ENTITIES as u8);
        for e in entities.iter_mut().take(count as usize) {
            e.hp = 100;
        }
        Self { seed, tick: 0, entities, count, mobs: [EntityState::default(); MAX_MOBS], mob_count: 0 }
    }

    /// Spawn a generic mob (AI domain, `kind` 0). Bounded at `MAX_MOBS`.
    pub fn spawn_mob(&mut self, x_mm: i64, y_mm: i64, hp: i32) {
        self.spawn_mob_kind(0, x_mm, y_mm, hp);
    }

    /// Spawn a mob tagged with an entity-type `kind` (nonzero for hazard/mob variants).
    /// Bounded at `MAX_MOBS`.
    pub fn spawn_mob_kind(&mut self, kind: u8, x_mm: i64, y_mm: i64, hp: i32) {
        if (self.mob_count as usize) < MAX_MOBS {
            self.mobs[self.mob_count as usize] = EntityState { x_mm, y_mm, hp, status: 0, kind };
            self.mob_count += 1;
        }
    }

    /// Advance one 120Hz tick: apply pre-computed displacements to the player.
    pub fn step_raw(&mut self, input_bits: u16, dx_mm: i64, dy_mm: i64) {
        if self.count > 0 {
            self.entities[0].x_mm += dx_mm;
            self.entities[0].y_mm += dy_mm;
        }
        let _ = input_bits;
        self.tick += 1;
    }

    /// Snapshot the active entities.
    pub fn snapshot(&self) -> EntitySnapshot {
        let mut snap = EntitySnapshot::default();
        snap.count = self.count;
        let n = self.count as usize;
        snap.entities[..n].copy_from_slice(&self.entities[..n]);
        snap
    }

    /// **L07 bijection**: Pack all active entities + mobs into a buffer.
    /// Returns the number of bytes written. Buffer must be at least
    /// (count + mob_count) * 23 bytes.
    pub fn pack_to_buffer(&self, buf: &mut [u8]) -> usize {
        let mut offset = 0;
        for e in self.entities.iter().take(self.count as usize) {
            offset = e.pack_into(buf, offset);
        }
        for m in self.mobs.iter().take(self.mob_count as usize) {
            offset = m.pack_into(buf, offset);
        }
        offset
    }

    /// **L07 bijection**: Unpack entities + mobs from a buffer.
    /// Returns the number of bytes consumed.
    pub fn unpack_from_buffer(&mut self, buf: &[u8], count: u8, mob_count: u8) -> usize {
        self.count = count.min(MAX_ENTITIES as u8);
        self.mob_count = mob_count.min(MAX_MOBS as u8);
        let mut offset = 0;
        for e in self.entities.iter_mut().take(self.count as usize) {
            let (entity, new_off) = EntityState::unpack_from(buf, offset);
            *e = entity;
            offset = new_off;
        }
        for m in self.mobs.iter_mut().take(self.mob_count as usize) {
            let (mob, new_off) = EntityState::unpack_from(buf, offset);
            *m = mob;
            offset = new_off;
        }
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L07 bijection test: EntityState pack/unpack round-trip.
    #[test]
    fn entity_state_pack_unpack_bijection() {
        let original = EntityState { x_mm: 12345, y_mm: -67890, hp: 42, status: 0xABCD, kind: 0 };
        let mut buf = [0u8; 23];
        original.pack_into(&mut buf, 0);
        let (recovered, _) = EntityState::unpack_from(&buf, 0);
        assert_eq!(original, recovered, "EntityState pack/unpack is a bijection");
    }

    /// L07 bijection test: ArenaState pack/unpack round-trip over interior values.
    #[test]
    fn arena_state_pack_unpack_interior_bijection() {
        let mut arena = ArenaState::new(42, 3);
        arena.entities[0] = EntityState { x_mm: 1000, y_mm: 2000, hp: 100, status: 0, kind: 0 };
        arena.entities[1] = EntityState { x_mm: -5000, y_mm: 8000, hp: 75, status: 1, kind: 0 };
        arena.entities[2] = EntityState { x_mm: 0, y_mm: 0, hp: 50, status: 2, kind: 0 };
        arena.spawn_mob(999, -888, 60);

        let mut buf = [0u8; (MAX_ENTITIES + MAX_MOBS) * ENTITY_PACK_BYTES];
        let bytes_written = arena.pack_to_buffer(&mut buf);

        let mut recovered = ArenaState::new(42, 0);
        let bytes_read = recovered.unpack_from_buffer(&buf, 3, 1);

        assert_eq!(bytes_written, bytes_read, "pack/unpack consume same bytes");
        assert_eq!(arena.count, recovered.count, "entity count matches");
        assert_eq!(arena.mob_count, recovered.mob_count, "mob count matches");
        for i in 0..(arena.count as usize) {
            assert_eq!(
                arena.entities[i], recovered.entities[i],
                "entity {} matches after pack/unpack",
                i
            );
        }
        for i in 0..(arena.mob_count as usize) {
            assert_eq!(
                arena.mobs[i], recovered.mobs[i],
                "mob {} matches after pack/unpack",
                i
            );
        }
    }

    /// L07 bijection test: edge case — all zeros (sentinel).
    #[test]
    fn arena_state_pack_unpack_zeros() {
        let mut arena = ArenaState::new(0, 1);
        arena.entities[0] = EntityState { x_mm: 0, y_mm: 0, hp: 0, status: 0, kind: 0 };

        let mut buf = [0u8; (MAX_ENTITIES + MAX_MOBS) * ENTITY_PACK_BYTES];
        arena.pack_to_buffer(&mut buf);

        let mut recovered = ArenaState::new(0, 0);
        recovered.unpack_from_buffer(&buf, 1, 0);

        assert_eq!(arena.entities[0], recovered.entities[0], "zero sentinel packs/unpacks correctly");
    }

    /// L07 bijection test: edge case — extreme values (i64 min/max, i32 min/max).
    #[test]
    fn arena_state_pack_unpack_extremes() {
        let mut arena = ArenaState::new(0, 2);
        arena.entities[0] = EntityState {
            x_mm: i64::MAX,
            y_mm: i64::MIN,
            hp: i32::MAX,
            status: u16::MAX,
            kind: 0,
        };
        arena.entities[1] = EntityState {
            x_mm: i64::MIN,
            y_mm: i64::MAX,
            hp: i32::MIN,
            status: 0,
            kind: 0,
        };

        let mut buf = [0u8; (MAX_ENTITIES + MAX_MOBS) * ENTITY_PACK_BYTES];
        arena.pack_to_buffer(&mut buf);

        let mut recovered = ArenaState::new(0, 0);
        recovered.unpack_from_buffer(&buf, 2, 0);

        assert_eq!(arena.entities[0], recovered.entities[0], "i64::MAX/MIN round-trip");
        assert_eq!(arena.entities[1], recovered.entities[1], "i32::MAX/MIN round-trip");
    }

    /// L18 sabotage test: corrupt a byte in the packed buffer, confirm unpack differs.
    #[test]
    fn l18_sabotage_arena_state_pack_gate() {
        let mut arena = ArenaState::new(42, 1);
        arena.entities[0] = EntityState { x_mm: 12345, y_mm: 67890, hp: 99, status: 0xBEEF, kind: 0 };

        let mut buf = [0u8; (MAX_ENTITIES + MAX_MOBS) * ENTITY_PACK_BYTES];
        arena.pack_to_buffer(&mut buf);

        // Sabotage: flip bit in first byte of x_mm
        buf[0] ^= 0x01;

        let mut recovered = ArenaState::new(42, 0);
        recovered.unpack_from_buffer(&buf, 1, 0);

        // This gate confirms: corruption in the buffer produces an incorrect entity.
        assert_ne!(
            arena.entities[0].x_mm, recovered.entities[0].x_mm,
            "L18 sabotage: corrupting a byte changes the unpacked value (confirming gate was live)"
        );
    }

    /// Ticket test: TickRing records and retrieves frames.
    #[test]
    fn tick_ring_records_and_finds_frames() {
        let mut ring = TickRing::new();

        let frame0 = TickFrame { tick: 0, snapshot: EntitySnapshot::default(), input_bits: 0, state_hash: 0 };
        let frame1 = TickFrame { tick: 1, snapshot: EntitySnapshot::default(), input_bits: 1, state_hash: 100 };

        ring.record(frame0);
        ring.record(frame1);

        assert_eq!(ring.valid_count(), 2, "ring has 2 frames");
        assert_eq!(ring.find_by_tick(0).map(|f| f.tick), Some(0), "frame 0 found");
        assert_eq!(ring.find_by_tick(1).map(|f| f.state_hash), Some(100), "frame 1 found");
        assert_eq!(ring.find_by_tick(2), None, "frame 2 not found");
    }

    /// Ticket test: spawn_mob increments mob_count and doesn't exceed MAX_MOBS.
    #[test]
    fn spawn_mob_respects_limit() {
        let mut arena = ArenaState::new(0, 0);
        for i in 0..MAX_MOBS {
            arena.spawn_mob(i as i64 * 1000, 0, 50);
        }
        assert_eq!(arena.mob_count as usize, MAX_MOBS, "mob_count at limit");
        arena.spawn_mob(999_000, 0, 50);
        assert_eq!(arena.mob_count as usize, MAX_MOBS, "extra spawn refused");
    }

    /// Ticket test: step_raw advances tick and applies displacement.
    #[test]
    fn step_raw_advances_state() {
        let mut arena = ArenaState::new(0, 1);
        arena.entities[0] = EntityState { x_mm: 0, y_mm: 0, hp: 100, status: 0, kind: 0 };
        let before_tick = arena.tick;
        arena.step_raw(0, 100, -50);
        assert_eq!(arena.tick, before_tick + 1, "tick incremented");
        assert_eq!(arena.entities[0].x_mm, 100, "x displacement applied");
        assert_eq!(arena.entities[0].y_mm, -50, "y displacement applied");
    }
}
