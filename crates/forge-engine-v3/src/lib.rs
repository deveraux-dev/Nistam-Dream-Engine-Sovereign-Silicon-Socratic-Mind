//! forge-engine-v3 — the ONE engine home (L05): the spine every engine
//! folds into or adapts onto (ENGINE-SPINE-BRIEF.md, wave `engine-spine`).
//! The tick word (`EngineTick8`, 8 bytes: 120Hz carrier frame + pulse phase
//! mod 30), the spine state word (`SpineState64`, 64 bytes: tick + Morton8
//! position + input digest + mechanism), the `EngineSpine` contract, and
//! its reference implementation `PulseSpine`. Later engines adapt onto this
//! spine; nothing here is a second home for any of it.

mod rollback;
mod spine;
mod state;
mod tick;

pub use rollback::{
    pack_snapshot, unpack_snapshot, ChunkCoord, ChunkDiffRange, EntityPosition, EntitySnapshot,
    RollbackError, RollbackRing, TickDiffFrame, SNAPSHOT_PACKED_SIZE, MAX_ACTIVE_CHUNKS,
    MAX_ENTITIES, MAX_PENDING_COMPACTIONS, RING_SIZE,
};
pub use spine::{EngineSpine, PulseSpine};
pub use state::{fnv1a, pack_mech, SpineState64, FNV_OFFSET_BASIS, FNV_PRIME, SCROLL_PMY_MAX};
pub use tick::{
    EngineTick8, MODE_BYTE_MAX, PULSE_PERIOD, REGISTER_INFERNO, REGISTER_PARADISO,
    REGISTER_PURGATORIO, RUN_STATE_HALT, RUN_STATE_REPLAY, RUN_STATE_RUN,
};
