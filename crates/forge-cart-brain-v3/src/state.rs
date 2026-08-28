//! State domain — the deterministic 120Hz tick kernel + arena entity state.
//!
//! A SLIM port of the quarry `forge-game-systems::tick_engine` +
//! `arena_core::state`: the entity + input + `state_hash` replay ring WITHOUT
//! the voxel-chunk coupling (`forge_physics` / `forge_zones`). The 2D arena cart
//! runs zero voxel chunks (`cartridge_arena` recorded `0, // no voxel chunks in
//! arena mode`), so dropping that coupling keeps this crate pure integer and
//! edge-portable. This IS "the deterministic tick kernel at 120Hz, run history
//! recorded as event data" (backtick.yaml).
//!
//! PORT RECEIPT (2026-08-16): ported verbatim from `F:\NewRepo\crates\
//! forge-cart-brain\src\state.rs`. Only the sink import path changed
//! (`forge_cart_sink` -> `forge_cart_sink_v3`, matching this crate's landed
//! dependency name), plus doc comments added on every public item the v2
//! crate left undocumented (this workspace's `missing_docs = "deny"` lint
//! forces it; v2's did not — same delta `forge-cart-sink-v3`'s own port
//! receipt already named).
//!
//! MERGE RECEIPT (2026-08-16): `EntityState` grew a `kind` byte (Sean's call —
//! fold `run_dev_run`'s `ArenaCart`-shape hazards, wolves included, into
//! `ArenaState`'s mob slots rather than keep two parallel game brains).
//! `ENTITY_HASH_BYTES` grew 22 -> 23 to match; no existing test asserts a
//! fixed magic hash (only cross-run equality), so this is a safe, non-breaking
//! widening. `kind: 0` on a player entity reads as "no hazard type" — the
//! player is never itself an `ENT_*` value. `spawn_mob` (kind 0, unchanged
//! callers) and the new `spawn_mob_kind` (tagged) both stay live — additive,
//! not a breaking rename.

use forge_cart_sink_v3::DeterminismSink;

/// 120 frames @ 120Hz = a 1-second rollback window (matches `TickEngine` `RING_SIZE`).
pub const RING_SIZE: usize = 120;
/// Max simultaneous entities per snapshot (matches `TickEngine` `MAX_ENTITIES`).
pub const MAX_ENTITIES: usize = 64;
/// Max simultaneous mobs (AI domain).
pub const MAX_MOBS: usize = 32;
/// Microseconds per tick @ 120Hz (matches `TickEngine` `TICK_DURATION_US`).
pub const TICK_DURATION_US: u64 = 8_333;

/// Bytes serialized per entity for the state hash: x(8) + y(8) + hp(4) + status(2) + kind(1).
const ENTITY_HASH_BYTES: usize = 23;

/// One entity's deterministic state. MilliUnit (mm) integer position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EntityState {
    /// World-space X, millimetres.
    pub x_mm: i64,
    /// World-space Y, millimetres.
    pub y_mm: i64,
    /// Hit points; `<= 0` means dead.
    pub hp: i32,
    /// Status bitfield (buff/debuff flags).
    pub status: u16,
    /// Entity type tag (0 = generic/player; `ENT_*` values from `run_dev_run`
    /// for hazard-kind mobs, e.g. wolves — a player entity is never tagged).
    pub kind: u8,
}

/// Compact, fixed-size per-tick snapshot. Copy, zero heap.
#[derive(Clone, Copy)]
pub struct EntitySnapshot {
    /// Fixed-capacity entity slots; only the first `count` are live.
    pub entities: [EntityState; MAX_ENTITIES],
    /// Number of live entities in `entities`.
    pub count: u8,
}

impl Default for EntitySnapshot {
    fn default() -> Self {
        Self { entities: [EntityState::default(); MAX_ENTITIES], count: 0 }
    }
}

/// One recorded tick — the replay frame. Copy, zero heap (slim `TickDiffFrame`).
#[derive(Clone, Copy)]
pub struct TickFrame {
    /// The tick number this frame was recorded at.
    pub tick: u64,
    /// The entity snapshot recorded this tick.
    pub snapshot: EntitySnapshot,
    /// The raw input button bitmask that drove this tick.
    pub input_bits: u16,
    /// Deterministic state hash for desync detection.
    pub state_hash: u64,
}

impl Default for TickFrame {
    fn default() -> Self {
        Self { tick: 0, snapshot: EntitySnapshot::default(), input_bits: 0, state_hash: 0 }
    }
}

/// The 120-frame deterministic ring — a slim port of `TickEngine`. Zero heap on
/// the hot path; `frames` is boxed once at `new()` (not per-tick), keeping the
/// ~188 KB frame buffer off the stack (safe on 1 MB main-thread stacks + WASM).
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
    /// Allocate a fresh, empty ring (frames boxed on the heap, not the stack).
    pub fn new() -> Self {
        // Allocate via vec to land frames on the heap — avoids putting ~188 KB
        // on the caller's stack (the main thread's 1 MB limit on Windows + WASM).
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

    /// The 64-bit state hash for a given tick (desync detection).
    pub fn state_hash(&self, tick: u64) -> Option<u64> {
        self.find_by_tick(tick).map(|f| f.state_hash)
    }
}

/// The arena's live deterministic state — a slim `ArenaState`. Seed-driven;
/// steps entities by quantized input velocity (the `cartridge_arena` PackedInput
/// physics path, integer-only).
pub struct ArenaState {
    /// Root determinism seed for this run.
    pub seed: u64,
    /// Current tick count.
    pub tick: u64,
    /// Fixed-capacity player/entity slots; only the first `count` are live.
    pub entities: [EntityState; MAX_ENTITIES],
    /// Number of live entities in `entities`.
    pub count: u8,
    /// Fixed-capacity mob slots; only the first `mob_count` are live.
    pub mobs: [EntityState; MAX_MOBS],
    /// Number of live mobs in `mobs`.
    pub mob_count: u8,
}

impl ArenaState {
    /// A fresh arena, seeded, with `player_count` entities spawned at full HP.
    pub fn new(seed: u64, player_count: u8) -> Self {
        let mut entities = [EntityState::default(); MAX_ENTITIES];
        let count = player_count.min(MAX_ENTITIES as u8);
        for e in entities.iter_mut().take(count as usize) {
            e.hp = 100;
        }
        Self {
            seed,
            tick: 0,
            entities,
            count,
            mobs: [EntityState::default(); MAX_MOBS],
            mob_count: 0,
        }
    }

    /// Spawn a generic mob (AI domain, `kind` 0). Bounded at `MAX_MOBS`.
    pub fn spawn_mob(&mut self, x_mm: i64, y_mm: i64, hp: i32) {
        self.spawn_mob_kind(0, x_mm, y_mm, hp);
    }

    /// Spawn a mob tagged with an entity-type `kind` (`ENT_*` from
    /// `run_dev_run`, e.g. `ENT_WOLF`) — hazard-specific collision/consequence
    /// logic keys off this tag. Bounded at `MAX_MOBS`.
    pub fn spawn_mob_kind(&mut self, kind: u8, x_mm: i64, y_mm: i64, hp: i32) {
        if (self.mob_count as usize) < MAX_MOBS {
            self.mobs[self.mob_count as usize] = EntityState { x_mm, y_mm, hp, status: 0, kind };
            self.mob_count += 1;
        }
    }

    /// Advance one 120Hz tick: apply pre-computed displacements to the player.
    ///
    /// Speed + tier scaling + haunt drag are computed by the `movement` domain
    /// before this call — `state` is a pure "apply and record" kernel.
    pub fn step_raw(&mut self, input_bits: u16, dx_mm: i64, dy_mm: i64) {
        if self.count > 0 {
            self.entities[0].x_mm += dx_mm;
            self.entities[0].y_mm += dy_mm;
        }
        let _ = input_bits;
        self.tick += 1;
    }

    /// Snapshot the active entities (Copy, zero heap).
    pub fn snapshot(&self) -> EntitySnapshot {
        let mut snap = EntitySnapshot::default();
        snap.count = self.count;
        let n = self.count as usize;
        snap.entities[..n].copy_from_slice(&self.entities[..n]);
        snap
    }

    /// Deterministic state hash via the live `BrutalHash` (through the sink).
    /// Serializes active entity fields into a bounded stack buffer, then hashes —
    /// zero heap, so this is the same proof-hash on native, browser, and WASI.
    pub fn state_hash(&self, rng: &dyn DeterminismSink) -> u64 {
        let mut buf = [0u8; (MAX_ENTITIES + MAX_MOBS) * ENTITY_HASH_BYTES];
        let mut n = 0;
        for e in self.entities.iter().take(self.count as usize) {
            n = write_entity_bytes(&mut buf, n, e);
        }
        for m in self.mobs.iter().take(self.mob_count as usize) {
            n = write_entity_bytes(&mut buf, n, m);
        }
        rng.hash_state(&buf[..n])
    }
}

/// Serialize one entity's fields (LE) into `buf` at `n`; returns the new offset.
fn write_entity_bytes(buf: &mut [u8], mut n: usize, e: &EntityState) -> usize {
    buf[n..n + 8].copy_from_slice(&e.x_mm.to_le_bytes());
    n += 8;
    buf[n..n + 8].copy_from_slice(&e.y_mm.to_le_bytes());
    n += 8;
    buf[n..n + 4].copy_from_slice(&e.hp.to_le_bytes());
    n += 4;
    buf[n..n + 2].copy_from_slice(&e.status.to_le_bytes());
    n += 2;
    buf[n] = e.kind;
    n += 1;
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_write_entity_bytes_kind_placement(kind in 0u8..=255u8) {
            let mut buf = [0u8; 23];
            let entity = EntityState {
                x_mm: 1000,
                y_mm: 2000,
                hp: 50,
                status: 0xFF,
                kind,
            };

            let offset = write_entity_bytes(&mut buf, 0, &entity);

            // Assert returned offset is exactly 23
            prop_assert_eq!(offset, 23, "write_entity_bytes should return exactly 23");

            // Assert kind byte lands at buf[22] for all u8 values,
            // including reserved zone (243..=255)
            prop_assert_eq!(
                buf[22], kind,
                "kind byte should land at buf[22]"
            );
        }
    }
}
