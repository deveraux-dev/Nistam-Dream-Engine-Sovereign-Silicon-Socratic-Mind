//! ThermalGovernor — DSP-driven thermal prediction for the 120Hz metronome.
//!
//! Lock-free, no-alloc hot path. Background thread polls OS thermal sensors
//! every 100ms and publishes via AtomicI32. The TickEngine reads in ~1ns.
//!
//! Invention #147 (Universal Watchman Fabric), #127 (BrickwallLimiter transfer).

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Governor decision returned at each tick boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernorAction {
    /// Temperature nominal — no action needed.
    Optimal,
    /// Within refractory period after a gain reduction — hold current throttle.
    MaintainThrottle,
    /// Predicted thermal breach — evacuate to next P-Core.
    TriggerGainReduction,
}

/// Subsystem-local thermal governor. Lives on Thread 0, reads atomics only.
///
/// All math is `i32` saturating — no floats, no heap, no locks.
/// Temperature unit: MilliCelsius (85_000 = 85.0°C).
pub struct ThermalGovernor {
    /// Shared telemetry written by the background poller (Thread 2).
    current_temp_mc: Arc<AtomicI32>,
    /// Absolute thermal ceiling in MilliCelsius.
    ceiling_mc: i32,
    /// How many polling intervals to extrapolate (5-10 recommended).
    look_ahead_intervals: i32,
    /// Refractory countdown — ticks remaining before restoring full load.
    release_timer_ticks: i32,
    /// Previous temperature reading for delta calculation.
    previous_temp_mc: i32,
    /// First evaluation flag — seed previous_temp from live reading.
    first_eval: bool,
}

impl ThermalGovernor {
    /// Create a new governor.
    ///
    /// - `shared_telemetry`: `Arc<AtomicI32>` also held by the background poller.
    /// - `ceiling_mc`: thermal limit in MilliCelsius (e.g. 85_000).
    /// - `look_ahead_intervals`: polling intervals to predict ahead (5-10).
    pub fn new(
        shared_telemetry: Arc<AtomicI32>,
        ceiling_mc: i32,
        look_ahead_intervals: i32,
    ) -> Self {
        Self {
            current_temp_mc: shared_telemetry,
            ceiling_mc,
            look_ahead_intervals,
            release_timer_ticks: 0,
            previous_temp_mc: 40_000, // safe boot default (40°C)
            first_eval: true,
        }
    }

    /// Evaluate thermal state. Call once per tick, at the tick boundary.
    ///
    /// Cost: one atomic load + a few saturating i32 ops. ~1-2ns.
    pub fn evaluate(&mut self) -> GovernorAction {
        let current_mc = self.current_temp_mc.load(Ordering::Acquire);

        // Sensor sanity — dead sensor or WMI hang returns 0 or garbage.
        if !(1_000..=120_000).contains(&current_mc) {
            self.release_timer_ticks = 120; // 1s refractory
            return GovernorAction::TriggerGainReduction;
        }

        // On first evaluation, seed previous from live reading (delta = 0).
        if self.first_eval {
            self.previous_temp_mc = current_mc;
            self.first_eval = false;
        }

        // Delta per polling interval (not per tick).
        let delta_mc = current_mc.saturating_sub(self.previous_temp_mc);
        self.previous_temp_mc = current_mc;

        let predicted_mc = current_mc.saturating_add(
            delta_mc.saturating_mul(self.look_ahead_intervals),
        );

        if predicted_mc >= self.ceiling_mc {
            self.release_timer_ticks = 120; // 1s refractory
            return GovernorAction::TriggerGainReduction;
        }

        if self.release_timer_ticks > 0 {
            self.release_timer_ticks -= 1;
            return GovernorAction::MaintainThrottle;
        }

        GovernorAction::Optimal
    }

    /// Current release timer value (ticks remaining in refractory period).
    pub fn release_remaining(&self) -> i32 {
        self.release_timer_ticks
    }

    /// Read the latest temperature without side effects.
    pub fn current_temp_mc(&self) -> i32 {
        self.current_temp_mc.load(Ordering::Acquire)
    }
}

/// Spawn the background thermal poller on Thread 2.
///
/// Polls OS thermal sensors every 100ms, writes MilliCelsius to the shared atomic.
/// The actual WMI/sensor call is behind `poll_fn` so callers can inject real or mock reads.
///
/// Returns the `JoinHandle` for shutdown coordination.
pub fn spawn_thermal_poller<F>(
    shared_telemetry: Arc<AtomicI32>,
    poll_fn: F,
) -> thread::JoinHandle<()>
where
    F: Fn() -> i32 + Send + 'static,
{
    thread::Builder::new()
        .name("watchman-thermal-poller".into())
        .spawn(move || loop {
            let temp_mc = poll_fn();
            shared_telemetry.store(temp_mc, Ordering::Release);
            thread::sleep(Duration::from_millis(100));
        })
        .expect("failed to spawn thermal poller thread")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_governor(temp_mc: i32, ceiling: i32, look_ahead: i32) -> ThermalGovernor {
        let atom = Arc::new(AtomicI32::new(temp_mc));
        ThermalGovernor::new(atom, ceiling, look_ahead)
    }

    #[test]
    fn optimal_when_cool() {
        let mut g = make_governor(50_000, 85_000, 5);
        assert_eq!(g.evaluate(), GovernorAction::Optimal);
    }

    #[test]
    fn triggers_on_ceiling_breach() {
        let atom = Arc::new(AtomicI32::new(70_000));
        let mut g = ThermalGovernor::new(atom.clone(), 85_000, 5);
        // First call seeds previous_temp_mc = 70_000, delta = 0 → Optimal
        assert_eq!(g.evaluate(), GovernorAction::Optimal);
        // Temp jumps to 84_000 → delta = 14_000, predicted = 84_000 + 70_000 → triggers
        atom.store(84_000, Ordering::Release);
        assert_eq!(g.evaluate(), GovernorAction::TriggerGainReduction);
    }

    #[test]
    fn refractory_period_holds() {
        let atom = Arc::new(AtomicI32::new(70_000));
        let mut g = ThermalGovernor::new(atom.clone(), 85_000, 5);
        assert_eq!(g.evaluate(), GovernorAction::Optimal); // seed
        atom.store(84_000, Ordering::Release);
        assert_eq!(g.evaluate(), GovernorAction::TriggerGainReduction);

        // Drop temp back to safe — should still throttle during refractory
        atom.store(50_000, Ordering::Release);
        assert_eq!(g.evaluate(), GovernorAction::MaintainThrottle);
        assert_eq!(g.release_remaining(), 119);
    }

    #[test]
    fn sensor_failure_triggers_evacuation() {
        let mut g = make_governor(0, 85_000, 5); // dead sensor
        assert_eq!(g.evaluate(), GovernorAction::TriggerGainReduction);
    }

    #[test]
    fn sensor_garbage_triggers_evacuation() {
        let mut g = make_governor(200_000, 85_000, 5); // impossible temp
        assert_eq!(g.evaluate(), GovernorAction::TriggerGainReduction);
    }

    #[test]
    fn refractory_expires_to_optimal() {
        let mut g = make_governor(50_000, 85_000, 5);
        g.release_timer_ticks = 2;
        assert_eq!(g.evaluate(), GovernorAction::MaintainThrottle);
        assert_eq!(g.evaluate(), GovernorAction::MaintainThrottle);
        assert_eq!(g.evaluate(), GovernorAction::Optimal);
    }

    #[test]
    fn saturating_math_no_overflow() {
        let atom = Arc::new(AtomicI32::new(20_000));
        let mut g = ThermalGovernor::new(atom.clone(), 85_000, 10);
        assert_eq!(g.evaluate(), GovernorAction::Optimal); // seed at 20_000
        // Jump to 80_000 → delta = 60_000, predicted = 80_000 + 600_000 → triggers
        atom.store(80_000, Ordering::Release);
        assert_eq!(g.evaluate(), GovernorAction::TriggerGainReduction);
    }
}
