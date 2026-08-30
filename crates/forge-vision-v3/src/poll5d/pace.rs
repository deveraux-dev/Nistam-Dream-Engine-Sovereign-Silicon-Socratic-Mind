//! From F:\NewRepo\crates\forge-vision\src\poll5d\pace.rs (lines 1-89)
//! AIMD poll pacer: idle grows interval additively toward max, motion halves toward min.

/// Adaptive polling pacer using AIMD (Additive Increase, Multiplicative Decrease).
#[derive(Debug, Clone, Copy)]
pub struct Pacer {
    min: u64,
    max: u64,
    cur: u64,
    step: u64,
}

impl Pacer {
    /// Create a new pacer with min/max interval and step size.
    pub fn new(min_ms: u64, max_ms: u64, step_ms: u64) -> Self {
        let min = min_ms.max(1);
        let max = max_ms.max(min);
        Self { min, max, cur: min, step: step_ms.max(1).min(max) }
    }

    /// Called when motion is detected; halves interval toward min.
    pub fn on_change(&mut self) {
        self.cur = (self.cur / 2).max(self.min);
    }

    /// Called when frame is idle; increases interval toward max.
    pub fn on_idle(&mut self) {
        self.cur = (self.cur + self.step).min(self.max);
    }

    /// Route signal based on whether changed tiles were detected.
    pub fn observe(&mut self, changed: u32) {
        if changed > 0 {
            self.on_change();
        } else {
            self.on_idle();
        }
    }

    /// Current interval in milliseconds.
    pub fn interval_ms(&self) -> u64 {
        self.cur
    }

    /// Get (min, max) bounds.
    pub fn bounds(&self) -> (u64, u64) {
        (self.min, self.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boots_at_min() {
        assert_eq!(Pacer::new(100, 2000, 100).interval_ms(), 100);
    }

    #[test]
    fn idle_grows_additively_and_clamps() {
        let mut p = Pacer::new(100, 500, 100);
        p.on_idle();
        p.on_idle();
        assert_eq!(p.interval_ms(), 300);
        for _ in 0..10 {
            p.on_idle();
        }
        assert_eq!(p.interval_ms(), 500, "clamped at max");
    }

    #[test]
    fn change_halves_toward_min_and_clamps() {
        let mut p = Pacer::new(100, 2000, 100);
        for _ in 0..30 {
            p.on_idle();
        }
        assert_eq!(p.interval_ms(), 2000);
        p.on_change();
        p.on_change();
        assert_eq!(p.interval_ms(), 500);
        for _ in 0..10 {
            p.on_change();
        }
        assert_eq!(p.interval_ms(), 100, "clamped at min");
    }

    #[test]
    fn observe_routes_signal() {
        let mut p = Pacer::new(100, 2000, 100);
        p.observe(0);
        assert_eq!(p.interval_ms(), 200);
        p.observe(5);
        assert_eq!(p.interval_ms(), 100);
    }
}
