//! System-wide VRAM watchman. Monitors TOTAL GPU memory usage (all processes),
//! not just forge-gpu-warden's tracked allocations.
//!
//! This catches external processes (Python/PyTorch/CUDA) eating VRAM.
//! Trips WrightGuard if free VRAM drops below floor.
//!
//! Part of the 4-layer GPU Memory Gate (Layer 4: System Watchman)
//! See: ~/.kiro/steering/gpu-memory-gate.md
//!
//! Requires feature = "nvml" to activate. Without it, compiles as a no-op stub.

use forge_watchmen_v3::{HealthSignal, Watchman};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Floor: minimum free VRAM in MB before tripping. Default 1024 MB (1 GB).
pub const VRAM_FLOOR_MB: u32 = 1024;

/// Signal file path (Unix). Written when VRAM is critical.
/// External tools (Kiro steering, watchdog scripts) can poll this.
#[cfg(not(windows))]
pub const SIGNAL_FILE: &str = "/tmp/forge-vram-critical";
/// Signal file path (Windows). Written when VRAM is critical.
/// External tools (Kiro steering, watchdog scripts) can poll this.
#[cfg(windows)]
pub const SIGNAL_FILE: &str = ".forge/forge-vram-critical";

// ── Mock (test / CI) ─────────────────────────────────────────────────────────

/// Test/CI system-wide VRAM watchman driven by a caller-controlled atomic.
pub struct MockVramSystem {
    /// Currently simulated system-wide free VRAM, in MB.
    pub free_mb: Arc<AtomicU32>,
    /// Free-VRAM floor below which this watchman trips, in MB.
    pub floor_mb: u32,
}

impl MockVramSystem {
    /// Build a mock watchman starting at `initial_free_mb` MB free, with
    /// the given floor.
    pub fn new(initial_free_mb: u32, floor_mb: u32) -> Self {
        Self {
            free_mb: Arc::new(AtomicU32::new(initial_free_mb)),
            floor_mb,
        }
    }

    /// Set the simulated free VRAM.
    pub fn set_free(&self, mb: u32) {
        self.free_mb.store(mb, Ordering::SeqCst);
    }

    /// Shared handle to the free-VRAM value, for callers that need direct access.
    pub fn handle(&self) -> Arc<AtomicU32> {
        self.free_mb.clone()
    }
}

impl Watchman for MockVramSystem {
    fn name(&self) -> &'static str { "vram-system" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let free = self.free_mb.load(Ordering::Relaxed);
        if free < self.floor_mb {
            Some(HealthSignal::VramOverflow {
                ticket_id: 0,
                req_mb: self.floor_mb - free,
                budget_mb: self.floor_mb,
            })
        } else {
            None
        }
    }

    fn veto(&self, _lane: u8) -> Option<(&'static str, HealthSignal)> {
        let free = self.free_mb.load(Ordering::Relaxed);
        if free < self.floor_mb {
            Some(("vram-system", HealthSignal::VramOverflow {
                ticket_id: 0,
                req_mb: self.floor_mb - free,
                budget_mb: self.floor_mb,
            }))
        } else {
            None
        }
    }
}

// ── NVML production ──────────────────────────────────────────────────────────

/// Production system-VRAM watchman: polls card-wide free MB every 2 s against
/// `floor_mb`, writing a signal file when it dips critical and removing it on
/// recovery. Assumes 8192 MB free until the first poll lands.
#[cfg(feature = "nvml")]
pub struct NvmlVramSystem {
    free_mb: Arc<AtomicU32>,
    floor_mb: u32,
    _poll_thread: std::thread::JoinHandle<()>,
}

#[cfg(feature = "nvml")]
impl NvmlVramSystem {
    /// Spawn background poller. Checks system-wide free VRAM every 2 seconds.
    /// Writes signal file when critical. Removes signal file when recovered.
    pub fn new(floor_mb: u32) -> Self {
        let free_mb = Arc::new(AtomicU32::new(8192)); // assume 8GB until first poll
        let shared = free_mb.clone();
        let floor = floor_mb;
        let _poll_thread = std::thread::Builder::new()
            .name("nvml-vram-system".into())
            .spawn(move || {
                let Ok(nvml) = nvml_wrapper::Nvml::init() else {
                    log::warn!("nvml-vram-system: NVML init failed — system VRAM monitoring inactive");
                    return;
                };
                let Ok(device) = nvml.device_by_index(0) else {
                    log::warn!("nvml-vram-system: no GPU at index 0");
                    return;
                };
                loop {
                    if let Ok(mem) = device.memory_info() {
                        let free = (mem.free / (1024 * 1024)) as u32;
                        shared.store(free, Ordering::Relaxed);

                        if free < floor {
                            log::warn!(
                                "[vram-system] CRITICAL: {}MB free < {}MB floor — writing signal",
                                free, floor
                            );
                            // Write signal file for external consumers
                            let _ = std::fs::write(
                                SIGNAL_FILE,
                                format!("free_mb={}\nfloor_mb={}\ntimestamp={}\n",
                                    free, floor, std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default().as_secs()
                                ),
                            );
                        } else {
                            // Remove signal file if it exists and we've recovered
                            let _ = std::fs::remove_file(SIGNAL_FILE);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            })
            .expect("nvml-vram-system thread");
        Self { free_mb, floor_mb, _poll_thread }
    }

    /// Current free VRAM in MB (system-wide).
    pub fn free_mb(&self) -> u32 {
        self.free_mb.load(Ordering::Relaxed)
    }
}

#[cfg(feature = "nvml")]
impl Watchman for NvmlVramSystem {
    fn name(&self) -> &'static str { "vram-system" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let free = self.free_mb.load(Ordering::Relaxed);
        if free < self.floor_mb {
            Some(HealthSignal::VramOverflow {
                ticket_id: 0,
                req_mb: self.floor_mb - free,
                budget_mb: self.floor_mb,
            })
        } else {
            None
        }
    }

    fn veto(&self, _lane: u8) -> Option<(&'static str, HealthSignal)> {
        let free = self.free_mb.load(Ordering::Relaxed);
        if free < self.floor_mb {
            Some(("vram-system", HealthSignal::VramOverflow {
                ticket_id: 0,
                req_mb: self.floor_mb - free,
                budget_mb: self.floor_mb,
            }))
        } else {
            None
        }
    }
}
