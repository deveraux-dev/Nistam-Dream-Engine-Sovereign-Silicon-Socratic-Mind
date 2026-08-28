//! Power watchman. 200 W hard cap. Transient detector: kill if instant draw > 1.5x rolling average.
//! MockPower: test/CI path.
//! NvmlPower: production path (feature = "nvml") — background thread polls GPU power every 2s.

use crate::manifest::Priority;
use forge_watchmen_v3::{HealthSignal, Watchman};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Hard power-draw cap in watts; crossing it kills the lane.
pub const POWER_HARD_CAP_W: u32 = 200;
/// Transient-spike ratio, in tenths (15 = 1.5x rolling average).
pub const TRANSIENT_RATIO_X10: u32 = 15; // 1.5x = 15 tenths

// ── Mock (test / CI) ───────────────────────────────────────────────────────

/// Test/CI power watchman driven by a caller-controlled atomic.
pub struct MockPower {
    /// Current simulated power draw in watts.
    pub draw_w: Arc<AtomicU32>,
}

impl MockPower {
    /// Build a mock watchman starting at `initial_w` watts.
    pub fn new(initial_w: u32) -> Self {
        Self { draw_w: Arc::new(AtomicU32::new(initial_w)) }
    }

    /// Set the simulated power draw.
    pub fn set(&self, w: u32) {
        self.draw_w.store(w, Ordering::SeqCst);
    }

    /// Shared handle to the draw value, for callers that need direct access.
    pub fn handle(&self) -> Arc<AtomicU32> {
        self.draw_w.clone()
    }
}

impl Watchman for MockPower {
    fn name(&self) -> &'static str { "power" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let w = self.draw_w.load(Ordering::Relaxed);
        if w >= POWER_HARD_CAP_W { Some(HealthSignal::PowerKill { draw_w: w }) } else { None }
    }

    fn veto(&self, lane: u8) -> Option<(&'static str, HealthSignal)> {
        if lane == Priority::P0Audio as u8 { return None; }
        let w = self.draw_w.load(Ordering::Relaxed);
        if w >= POWER_HARD_CAP_W {
            return Some(("power", HealthSignal::PowerKill { draw_w: w }));
        }
        None
    }
}

// ── NVML production ────────────────────────────────��─────────────────────────

/// Production power watchman: a background NVML poller holding instantaneous
/// draw and a rolling integer EMA (watts × 10). Reports 0 W — never a kill — if
/// NVML fails to init.
#[cfg(feature = "nvml")]
pub struct NvmlPower {
    draw_w: Arc<AtomicU32>,
    avg_w_x10: Arc<AtomicU32>, // rolling EMA * 10 to stay integer
    _poll_thread: std::thread::JoinHandle<()>,
}

#[cfg(feature = "nvml")]
impl NvmlPower {
    /// Spawn background power poller. Falls back to 0 W (no kill) on NVML failure.
    pub fn new() -> Self {
        let draw_w = Arc::new(AtomicU32::new(0));
        let avg_w_x10 = Arc::new(AtomicU32::new(1000)); // initial assumed avg 100 W
        let shared_draw = draw_w.clone();
        let shared_avg = avg_w_x10.clone();
        let _poll_thread = std::thread::Builder::new()
            .name("nvml-power".into())
            .spawn(move || {
                let Ok(nvml) = nvml_wrapper::Nvml::init() else {
                    log::warn!("nvml-power: NVML init failed — power monitoring inactive");
                    return;
                };
                let Ok(device) = nvml.device_by_index(0) else {
                    log::warn!("nvml-power: no GPU at index 0");
                    return;
                };
                loop {
                    if let Ok(mw) = device.power_usage() {
                        let w = mw / 1000;
                        shared_draw.store(w, Ordering::Relaxed);
                        // EMA alpha=0.1: new_avg*10 = (9 * old_avg*10 + w*10) / 10
                        let old = shared_avg.load(Ordering::Relaxed);
                        let new_avg = (old.saturating_mul(9).saturating_add(w * 10)) / 10;
                        shared_avg.store(new_avg, Ordering::Relaxed);
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            })
            .expect("nvml-power thread");
        Self { draw_w, avg_w_x10, _poll_thread }
    }

    fn over_cap(&self) -> bool {
        self.draw_w.load(Ordering::Relaxed) >= POWER_HARD_CAP_W
    }

    fn transient_spike(&self) -> bool {
        let w = self.draw_w.load(Ordering::Relaxed);
        let avg_x10 = self.avg_w_x10.load(Ordering::Relaxed);
        // spike if w > 1.5 * avg: w*10 > avg_x10 * 15/10 => w*100 > avg_x10 * 15
        w.saturating_mul(100) > avg_x10.saturating_mul(TRANSIENT_RATIO_X10)
    }
}

#[cfg(feature = "nvml")]
impl Watchman for NvmlPower {
    fn name(&self) -> &'static str { "power-nvml" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let w = self.draw_w.load(Ordering::Relaxed);
        if self.over_cap() || self.transient_spike() {
            Some(HealthSignal::PowerKill { draw_w: w })
        } else {
            None
        }
    }

    fn veto(&self, lane: u8) -> Option<(&'static str, HealthSignal)> {
        if lane == Priority::P0Audio as u8 { return None; }
        let w = self.draw_w.load(Ordering::Relaxed);
        if self.over_cap() || (self.transient_spike() && lane >= Priority::P3Heavy as u8) {
            return Some(("power-nvml", HealthSignal::PowerKill { draw_w: w }));
        }
        None
    }
}
