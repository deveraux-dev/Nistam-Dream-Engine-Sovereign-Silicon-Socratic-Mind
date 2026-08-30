#![allow(missing_docs)]
//! ironroot-signal: generic signal-routed creation and ambience primitives.
//!
//! Design law:
//! - Live signals are noisy and non-authoritative.
//! - Only quantized fixed-point summaries may cross into deterministic state.
//! - Gameplay-critical effects must be bounded, ledgered, and replayable.

pub mod events;
pub mod filter;
pub mod ids;
pub mod metronome;
pub mod parameter_bus;
pub mod proxy;
pub mod stamp;

pub use events::{AmbientParameter, CreationEvent, StampInfluenceKind, WorldFluxEvent};
pub use filter::{band_0_to_3, clamp_q, ema_q, variance_q};
pub use ids::{AssetId, SignalSourceId, Tick, ToolId, Vec3i, ZoneId};
pub use metronome::WorldMetronome;
pub use parameter_bus::WorldParameterBus;
pub use proxy::{FilteredSignalFrame, RawSignalFrame, SignalHealth, SignalProxy};
pub use stamp::{CreationStamp, CreationStampHash, StampGameplayEffect};
