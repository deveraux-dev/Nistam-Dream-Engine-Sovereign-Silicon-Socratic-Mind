#![allow(missing_docs)]
//! Deterministic world metronome.

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorldMetronome {
    pub period_ticks: u32,
    pub phase_tick: u32,
    pub amplitude_q: i16,
}

impl WorldMetronome {
    pub const fn new(period_ticks: u32, amplitude_q: i16) -> Self {
        Self { period_ticks, phase_tick: 0, amplitude_q }
    }

    pub const fn ten_second_120hz() -> Self {
        Self { period_ticks: 1200, phase_tick: 0, amplitude_q: 10000 }
    }

    pub fn advance(&mut self, ticks: u32) {
        if self.period_ticks == 0 {
            self.phase_tick = 0;
            return;
        }
        self.phase_tick = (self.phase_tick + ticks) % self.period_ticks;
    }

    pub fn phase_q(&self) -> i16 {
        if self.period_ticks == 0 {
            return 0;
        }
        ((self.phase_tick as u64 * 10000) / self.period_ticks as u64) as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_second_metronome_wraps_at_1200_ticks() {
        let mut m = WorldMetronome::ten_second_120hz();
        m.advance(1199);
        assert_eq!(m.phase_tick, 1199);
        m.advance(1);
        assert_eq!(m.phase_tick, 0);
    }
}
