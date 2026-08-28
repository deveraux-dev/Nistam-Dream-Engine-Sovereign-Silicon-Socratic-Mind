//! GPU dispatch warden — fence, lane, and manifest guards over device
//! submission. Ported as-is from `F:\NewRepo\crates\forge-gpu-warden`.

/// Non-blocking completion handles for dispatched work.
pub mod fence;
/// Five priority lanes (P0 audio through P4 marketplace) and their scheduler.
pub mod lanes;
/// Signed budget manifests and the priority-lane enum.
pub mod manifest;
/// Game-mode / non-game-mode drain-and-flip switch.
pub mod mode;
/// Opaque, never-dereferenced pointer into caller-owned dispatch state.
pub mod opaque;
/// Admission gate: verifies manifests, enforces VRAM ceilings, consults watchmen.
pub mod sieve_gate;
/// Double-buffered 2 × 64 KB VRAM staging slots for zero-stall hot-swapping.
pub mod vram_staging;
/// Real device-side VRAM bridge: wgpu::Buffer staging halves, dirty-chunk DMA,
/// MoE indirect-dispatch args. Complements `vram_staging` (host-only sync).
pub mod vram_bridge;
/// Minimal buffer-only HAL trait — a deliberate narrowing of the donor
/// `forge-hal` crate's 20-method HalBackend, scoped to `hal_bridge`'s 2 calls.
pub mod hal;
/// Trait seam routing CanvasRenderer buffer uploads through `hal::HalBackend`.
pub mod hal_bridge;
/// Concrete watchman implementations (thermal, power, VRAM, deadline, integrity).
pub mod watchmen;
/// 32×32 SPIR-V workgroup tile contracts matching NVIDIA Ampere warp dispatch.
pub mod workgroup;
/// Driver-reported VRAM residency (total/used/free) — the demo's ACTUAL bar.
pub mod vram_probe;
/// Health-signal broadcast + lane cancellation on trip.
pub mod wrightguard;

pub use fence::{
    timeline_fence_pair, DispatchFence, FenceOutcome, FenceState, TimelineError, TimelineFence,
    TimelineSemaphore, TimelineSink,
};
pub use lanes::{LaneScheduler, WorkloadClass};
pub use manifest::{BudgetManifest, ManifestError, ManifestSignature, Priority, ShaderId, WorkloadId};
pub use mode::ModeSwitch;
pub use opaque::OpaqueSieveState;
pub use sieve_gate::{SieveDecision, SieveGate, SieveRefusal};
pub use vram_staging::{
    DoubleBufferedStagingBuffers, DoubleBufferedVramStaging, StagingError, StagingSlot,
    NUM_STAGING_SLOTS, STAGING_SLOT_SIZE,
};
pub use vram_bridge::{
    dispatch_dims_for, VramBridge, CHUNK_BYTES, COMPUTE_BUFFER_SIZE, INDIRECT_ARGS_SIZE,
    MAX_CHUNKS, STAGING_SIZE,
};
pub use watchmen::{DeadlineMiss, IntegrityWatchman, Watchman, WatchmanRegistry};
pub use workgroup::{
    WorkgroupDispatchPlan, WorkgroupError, WorkgroupTileContract, CACHE_LINE_BYTES,
    SHARED_MEM_BANKS, SHARED_MEM_BANK_BYTES, WARPS_PER_WORKGROUP, WARP_SIZE,
    WORKGROUP_DIM_X, WORKGROUP_DIM_Y, WORKGROUP_DIM_Z, WORKGROUP_THREADS,
};
pub use vram_probe::{VramReading, VramSource};
pub use wrightguard::{PanicSignal, WrightGuard};

use ed25519_dalek::VerifyingKey;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// A unit of GPU work submitted to the [`Warden`] for admission.
pub struct DispatchTicket {
    /// Signed resource budget for this dispatch.
    pub manifest: BudgetManifest,
    /// Opaque handle to the caller-owned state the dispatch operates on.
    pub state: OpaqueSieveState,
    /// Priority lane this ticket competes on.
    pub lane: Priority,
}

/// The GPU dispatch warden: gates admission, tracks lanes, broadcasts health.
pub struct Warden {
    /// Priority-lane admission and VRAM accounting.
    pub scheduler: Arc<LaneScheduler>,
    /// Admission gate (manifest verification + budget + watchman veto).
    pub gate: Arc<SieveGate>,
    /// Installed subsystem watchmen (thermal, power, VRAM, deadline, integrity).
    pub watchmen: Arc<WatchmanRegistry>,
    /// Health-signal broadcaster; trips lane cancellation on fault.
    pub wrightguard: Arc<WrightGuard>,
    /// GPU temperature in °C. Updated by thermal watchman. Read by RenderSieve.
    pub thermal_c: Arc<AtomicU32>,
    /// Total VRAM ceiling in MB. The host sets this from the device's REAL total
    /// (see [`Warden::with_detected_vram`] / [`Warden::detect_vram_ceiling_mb`]);
    /// `new()` falls back to a conservative 8192 (8 GB) for tests / non-NVIDIA /
    /// probe failure. NOT from wgpu adapter limits — wgpu exposes no portable total
    /// VRAM (`Limits::max_buffer_size` is max single-alloc, not capacity).
    pub vram_ceiling_mb: u32,
}

impl Warden {
    /// Build a warden with a fresh, unsigned-manifest-only gate and a
    /// conservative 8192 MB (8 GB) VRAM ceiling.
    pub fn new() -> Self {
        let wrightguard = Arc::new(WrightGuard::new());
        let scheduler = Arc::new(LaneScheduler::new());
        wrightguard.attach_scheduler(Arc::downgrade(&scheduler));
        let watchmen = Arc::new(WatchmanRegistry::new(wrightguard.clone() as Arc<dyn forge_watchmen_v3::Broadcaster>, 8));
        let gate = Arc::new(SieveGate::new(scheduler.clone(), watchmen.clone()));
        Self { scheduler, gate, watchmen, wrightguard, thermal_c: Arc::new(AtomicU32::new(45)), vram_ceiling_mb: 8192 }
    }

    /// Build a warden whose gate also accepts real ed25519-signed manifests
    /// verified against `vk`.
    pub fn with_key(vk: Arc<VerifyingKey>) -> Self {
        let wrightguard = Arc::new(WrightGuard::new());
        let scheduler = Arc::new(LaneScheduler::new());
        wrightguard.attach_scheduler(Arc::downgrade(&scheduler));
        let watchmen = Arc::new(WatchmanRegistry::new(wrightguard.clone() as Arc<dyn forge_watchmen_v3::Broadcaster>, 8));
        let gate = Arc::new(SieveGate::with_key(scheduler.clone(), watchmen.clone(), vk));
        Self { scheduler, gate, watchmen, wrightguard, thermal_c: Arc::new(AtomicU32::new(45)), vram_ceiling_mb: 8192 }
    }

    /// `new()` + a REAL total-VRAM probe ([`Warden::detect_vram_ceiling_mb`]). The
    /// live host (studio shell, NDE backend) uses THIS so [`Warden::vram_pressure_pct`]
    /// reads against the card's true capacity, not a hardcoded 8 GB. Tests keep
    /// `new()` (hermetic, no subprocess, 8192).
    pub fn with_detected_vram() -> Self {
        let mut w = Self::new();
        w.set_vram_ceiling_mb(Self::detect_vram_ceiling_mb());
        w
    }

    /// Set the VRAM ceiling (MB) the host measured from the real device. A `0` is
    /// ignored (keeps the prior value) so a failed probe never zeroes the gate
    /// (which would make `vram_pressure_pct` return 0% under any load).
    #[inline]
    pub fn set_vram_ceiling_mb(&mut self, mb: u32) {
        if mb != 0 {
            self.vram_ceiling_mb = mb;
        }
    }

    /// Probe the device's REAL total VRAM in MB. Sovereign + link-dep-free: shells
    /// `nvidia-smi --query-gpu=memory.total` (the same tool `forge-gpu::devtools`
    /// already uses for `memory.used`). Returns 8192 on any failure / non-NVIDIA —
    /// an honest conservative fallback, never a silent 0. (NVML is the richer path
    /// behind the optional `nvml` feature; nvidia-smi keeps this dep-free.) Cold
    /// path only — call once at host init, never per frame.
    pub fn detect_vram_ceiling_mb() -> u32 {
        const FALLBACK_MB: u32 = 8192;
        let mut cmd = std::process::Command::new("nvidia-smi");
        cmd.args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        match cmd.output()
        {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .and_then(|l| l.trim().parse::<u32>().ok())
                .filter(|&mb| mb > 0)
                .unwrap_or(FALLBACK_MB),
            _ => FALLBACK_MB,
        }
    }

    /// Query current GPU temperature in °C.
    #[inline]
    pub fn gpu_temp_c(&self) -> u32 {
        self.thermal_c.load(Ordering::Relaxed)
    }

    /// Query VRAM pressure as percentage (0-100).
    #[inline]
    pub fn vram_pressure_pct(&self) -> u8 {
        if self.vram_ceiling_mb == 0 { return 0; }
        let used = self.scheduler.total_vram_used_mb();
        ((used as u64 * 100) / self.vram_ceiling_mb as u64).min(100) as u8
    }

    /// Evaluate `ticket` against the sieve gate and admit, queue, or refuse it.
    pub fn dispatch(&self, ticket: DispatchTicket) -> Result<DispatchFence, PanicSignal> {
        match self.gate.evaluate(&ticket) {
            SieveDecision::Allow => self.scheduler.admit(ticket),
            SieveDecision::Queue { .. } => self.scheduler.queue(ticket),
            SieveDecision::Refuse { reason } => Err(wrightguard::health_from_refusal(reason)),
        }
    }

    /// Subscribe to health signals broadcast by [`WrightGuard`].
    pub fn panic_subscribe(&self) -> crossbeam_channel::Receiver<PanicSignal> {
        self.wrightguard.subscribe()
    }

    /// Drain all lanes, waiting up to `drain_timeout_ms`.
    pub fn shutdown(&self, drain_timeout_ms: u64) {
        self.scheduler.drain(drain_timeout_ms);
    }
}

impl Default for Warden {
    fn default() -> Self { Self::new() }
}
