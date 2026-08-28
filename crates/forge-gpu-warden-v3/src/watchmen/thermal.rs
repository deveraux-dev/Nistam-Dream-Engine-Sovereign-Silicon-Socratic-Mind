//! Thermal watchman.
//! MockThermal: test/CI path — caller drives the AtomicU32.
//! NvmlThermal: production path (feature = "nvml") — background thread polls GPU every 2s.

use crate::manifest::Priority;
use forge_watchmen_v3::{HealthSignal, Watchman};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Temperature at or above which non-P0 heavy/marketplace lanes are vetoed.
pub const THROTTLE_C: u32 = 78;
/// Temperature at or above which every non-P0 lane is killed.
pub const KILL_C: u32 = 82;

// ── Mock (test / CI) ─────────────────────────────────────────────────────────

/// Test/CI thermal watchman driven by a caller-controlled atomic.
pub struct MockThermal {
    /// Current simulated temperature in Celsius.
    pub temp_c: Arc<AtomicU32>,
}

impl MockThermal {
    /// Build a mock watchman starting at `initial_c` degrees Celsius.
    pub fn new(initial_c: u32) -> Self {
        Self { temp_c: Arc::new(AtomicU32::new(initial_c)) }
    }

    /// Set the simulated temperature.
    pub fn set(&self, c: u32) {
        self.temp_c.store(c, Ordering::SeqCst);
    }

    /// Shared handle to the temperature value, for callers that need direct access.
    pub fn handle(&self) -> Arc<AtomicU32> {
        self.temp_c.clone()
    }
}

impl Watchman for MockThermal {
    fn name(&self) -> &'static str { "thermal" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let c = self.temp_c.load(Ordering::Relaxed);
        if c >= KILL_C { Some(HealthSignal::ThermalKill { temp_c: c }) } else { None }
    }

    fn veto(&self, lane: u8) -> Option<(&'static str, HealthSignal)> {
        let c = self.temp_c.load(Ordering::Relaxed);
        if lane == Priority::P0Audio as u8 { return None; }
        if c >= KILL_C {
            return Some(("thermal", HealthSignal::ThermalKill { temp_c: c }));
        }
        if c >= THROTTLE_C && (lane == Priority::P3Heavy as u8 || lane == Priority::P4Marketplace as u8) {
            return Some(("thermal", HealthSignal::ThermalKill { temp_c: c }));
        }
        None
    }
}

// ── NVML production ──────────────────────────────────────────────────────────

/// Production thermal watchman: a background NVML poller holding the device
/// temperature in °C. Falls back to 65 °C if NVML fails to init.
#[cfg(feature = "nvml")]
pub struct NvmlThermal {
    temp_c: Arc<AtomicU32>,
    _poll_thread: std::thread::JoinHandle<()>,
}

#[cfg(feature = "nvml")]
impl NvmlThermal {
    /// Spawn background poller. Falls back silently to 65 C on NVML init failure.
    pub fn new() -> Self {
        use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
        let temp_c = Arc::new(AtomicU32::new(65));
        let shared = temp_c.clone();
        let _poll_thread = std::thread::Builder::new()
            .name("nvml-thermal".into())
            .spawn(move || {
                let Ok(nvml) = nvml_wrapper::Nvml::init() else {
                    log::warn!("nvml-thermal: NVML init failed — thermal monitoring inactive");
                    return;
                };
                let Ok(device) = nvml.device_by_index(0) else {
                    log::warn!("nvml-thermal: no GPU at index 0");
                    return;
                };
                loop {
                    if let Ok(t) = device.temperature(TemperatureSensor::Gpu) {
                        shared.store(t, Ordering::Relaxed);
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            })
            .expect("nvml-thermal thread");
        Self { temp_c, _poll_thread }
    }
}

#[cfg(feature = "nvml")]
impl Watchman for NvmlThermal {
    fn name(&self) -> &'static str { "thermal-nvml" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let c = self.temp_c.load(Ordering::Relaxed);
        if c >= KILL_C { Some(HealthSignal::ThermalKill { temp_c: c }) } else { None }
    }

    fn veto(&self, lane: u8) -> Option<(&'static str, HealthSignal)> {
        let c = self.temp_c.load(Ordering::Relaxed);
        if lane == Priority::P0Audio as u8 { return None; }
        if c >= KILL_C {
            return Some(("thermal-nvml", HealthSignal::ThermalKill { temp_c: c }));
        }
        if c >= THROTTLE_C && (lane == Priority::P3Heavy as u8 || lane == Priority::P4Marketplace as u8) {
            return Some(("thermal-nvml", HealthSignal::ThermalKill { temp_c: c }));
        }
        None
    }
}
