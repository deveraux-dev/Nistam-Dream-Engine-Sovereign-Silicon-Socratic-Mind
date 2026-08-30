//! The fold — the grimoire's open/close mechanic as an integer state machine.
//! Cover `rotateY(0 -> -170deg)` becomes a permyriad ratio eased on 120Hz ticks.
//! Deterministic: no wall clock, no float — a hitbox could read this safely.

use serde::{Deserialize, Serialize};

/// Where a fold (the whole book, or one page leaf) sits in its cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoldState {
    /// The fold is fully closed (ratio = 0).
    Closed,
    /// The fold is animating from closed to open.
    Opening,
    /// The fold is fully open (ratio = 10000).
    Open,
    /// The fold is animating from open to closed.
    Folding,
}

/// A fold: an integer ratio (permyriad, `0`=closed .. `10000`=open) that eases
/// toward its target one tick at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fold {
    state: FoldState,
    ratio_pmy: u32,
    duration_ticks: u32,
}

impl Fold {
    /// Permyriad ratio value for a fully open fold.
    pub const OPEN: u32 = 10_000;
    /// Permyriad ratio value for a fully closed fold.
    pub const CLOSED: u32 = 0;

    /// A closed fold that sweeps open over `duration_ticks` (min 1).
    pub fn new(duration_ticks: u32) -> Self {
        Self { state: FoldState::Closed, ratio_pmy: 0, duration_ticks: duration_ticks.max(1) }
    }

    /// Returns the current fold state.
    pub fn state(&self) -> FoldState { self.state }
    /// Returns the current permyriad ratio (0 = closed, 10000 = open).
    pub fn ratio_pmy(&self) -> u32 { self.ratio_pmy }
    /// Returns `true` if the fold is fully open.
    pub fn is_open(&self) -> bool { self.state == FoldState::Open }
    /// Returns `true` if the fold is fully closed.
    pub fn is_closed(&self) -> bool { self.state == FoldState::Closed }

    /// Begin opening (no-op if already open).
    pub fn open(&mut self) {
        if self.state != FoldState::Open { self.state = FoldState::Opening; }
    }

    /// Begin folding shut (no-op if already closed).
    pub fn close(&mut self) {
        if self.state != FoldState::Closed { self.state = FoldState::Folding; }
    }

    /// Snap fully open, no animation.
    pub fn snap_open(&mut self) { self.state = FoldState::Open; self.ratio_pmy = Self::OPEN; }
    /// Snap fully closed, no animation.
    pub fn snap_closed(&mut self) { self.state = FoldState::Closed; self.ratio_pmy = Self::CLOSED; }

    /// Advance one 120Hz tick. Returns `true` while still animating.
    pub fn tick(&mut self) -> bool {
        let step = (Self::OPEN / self.duration_ticks).max(1);
        match self.state {
            FoldState::Opening => {
                self.ratio_pmy = (self.ratio_pmy + step).min(Self::OPEN);
                if self.ratio_pmy >= Self::OPEN {
                    self.state = FoldState::Open;
                    false
                } else {
                    true
                }
            }
            FoldState::Folding => {
                self.ratio_pmy = self.ratio_pmy.saturating_sub(step);
                if self.ratio_pmy == Self::CLOSED {
                    self.state = FoldState::Closed;
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    /// Run the fold to rest, returning the tick count taken. Deterministic settle
    /// used by headless export and tests. Bounded so a bad duration can't hang.
    pub fn settle(&mut self) -> u32 {
        let mut n = 0;
        while self.tick() {
            n += 1;
            if n > 1_000_000 { break; }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_to_full() {
        let mut f = Fold::new(48);
        assert!(f.is_closed());
        f.open();
        f.settle();
        assert!(f.is_open());
        assert_eq!(f.ratio_pmy(), Fold::OPEN);
    }

    #[test]
    fn closes_to_zero() {
        let mut f = Fold::new(48);
        f.snap_open();
        f.close();
        f.settle();
        assert!(f.is_closed());
        assert_eq!(f.ratio_pmy(), Fold::CLOSED);
    }

    #[test]
    fn ratio_is_monotonic_while_opening() {
        let mut f = Fold::new(100);
        f.open();
        let mut last = 0;
        while f.tick() {
            assert!(f.ratio_pmy() >= last);
            last = f.ratio_pmy();
        }
        assert_eq!(f.ratio_pmy(), Fold::OPEN);
    }

    #[test]
    fn deterministic_same_input_same_ticks() {
        let mut a = Fold::new(37);
        let mut b = Fold::new(37);
        a.open();
        b.open();
        assert_eq!(a.settle(), b.settle());
    }

    #[test]
    fn closed_fold_ticks_are_noops() {
        let mut f = Fold::new(48);
        assert!(!f.tick());
        assert_eq!(f.settle(), 0);
    }
}
