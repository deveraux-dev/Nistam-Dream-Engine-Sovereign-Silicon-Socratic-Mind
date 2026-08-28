//! Per-subsystem watchmen. Trait + registry now live in `forge-watchmen`.
//! This module owns only GPU-domain watchmen (thermal, power, vram, deadline, integrity).

/// Deadline-miss watchman.
pub mod deadline;
/// Shader-integrity watchman.
pub mod integrity;
/// Power-draw watchman.
pub mod power;
/// Thermal watchman.
pub mod thermal;
/// Per-lane VRAM ledger watchman.
pub mod vram;
/// System-wide VRAM watchman (mock and, with `nvml`, real NVML-backed).
pub mod vram_system;

pub use deadline::DeadlineMiss;
pub use integrity::IntegrityWatchman;
pub use power::MockPower;
pub use thermal::MockThermal;
pub use vram::VramLedger;
pub use vram_system::MockVramSystem;
#[cfg(feature = "nvml")]
pub use vram_system::NvmlVramSystem;
pub use forge_watchmen_v3::{Broadcaster, HealthSignal, Severity, SignalKind, Watchman, WatchmanRegistry};

#[cfg(feature = "nvml")]
pub use power::NvmlPower;
#[cfg(feature = "nvml")]
pub use thermal::NvmlThermal;
