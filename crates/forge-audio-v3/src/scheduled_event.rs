//! TickSchedule's event record — ported from
//! `F:\NewRepo\crates\forge-core\src\tick_schedule.rs` (v2 Crate Zero).
//! Only `ScheduledEvent` itself lands here (forge-audio's real need, in
//! `device.rs`/`conductor_audio.rs`) — the scheduler/`drain_due` machinery
//! around it is a separate, not-yet-needed port.

use serde::{Deserialize, Serialize};

/// Maximum concurrent scheduled events. 256 × 32 bytes = 8 KB — L2-resident.
pub const MAX_SCHEDULED_EVENTS: usize = 256;

/// A single scheduled event. `#[repr(C)]`, 32 bytes — multiple of 16, packed.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledEvent {
    pub fire_tick: u64,
    pub repeat_interval: u64,
    pub entity: u64,
    pub tag: u32,
    pub active: bool,
    pub _pad: [u8; 3],
}
