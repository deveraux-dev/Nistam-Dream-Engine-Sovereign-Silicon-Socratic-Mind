//! forge-semantic-quadlane: quad-lane conductor and effect dispatcher for phase scheduling.
//!
//! Provides the mechanical baton (ADR-013) that fans scheduled phrases to four independent
//! execution lanes (L0 audio, L1 physics, L2 render, L3 inference) based on effect masks.
//! All tick logic operates on u64 integers; no floating-point arithmetic. No unsafe code.

#![deny(missing_docs)]

pub mod dispatch;
pub mod quad_lane;
pub mod schedule;

pub use dispatch::{BindingEntry, EffectDispatcher, SieveAction, SieveEvent};
pub use quad_lane::{Conductor, ExecLane, LaneFanout};
pub use schedule::{ScheduleError, ScheduledEvent, TickSchedule};
