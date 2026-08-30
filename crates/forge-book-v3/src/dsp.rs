//! DSP — an integer ADSR envelope (permyriad amplitude on a tick clock). The
//! shape of a sound page; deterministic, no float.

use serde::{Deserialize, Serialize};

/// Attack/Decay/Sustain/Release — durations in ticks, sustain in permyriad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adsr {
    /// Duration of the attack phase in ticks.
    pub attack: u32,
    /// Duration of the decay phase in ticks.
    pub decay: u32,
    /// Sustain amplitude in permyriad (0..10000).
    pub sustain_pmy: u32,
    /// Duration of the release phase in ticks.
    pub release: u32,
}

impl Adsr {
    /// Creates a new ADSR envelope, clamping sustain to the permyriad range.
    pub fn new(attack: u32, decay: u32, sustain_pmy: u32, release: u32) -> Self {
        Self { attack, decay, sustain_pmy: sustain_pmy.min(10_000), release }
    }

    /// Amplitude (permyriad) at `tick`. `note_off` starts the release phase.
    pub fn sample(&self, tick: u32, note_off: Option<u32>) -> u32 {
        // Release phase.
        if let Some(off) = note_off {
            if tick >= off {
                if self.release == 0 {
                    return 0;
                }
                let held = self.level_held(off);
                let into = tick - off;
                if into >= self.release {
                    return 0;
                }
                return held - (held * into / self.release);
            }
        }
        self.level_held(tick)
    }

    /// The pre-release envelope: attack ramp -> decay -> sustain.
    fn level_held(&self, tick: u32) -> u32 {
        if tick < self.attack {
            return if self.attack == 0 { 10_000 } else { 10_000 * tick / self.attack };
        }
        let t = tick - self.attack;
        if t < self.decay {
            let drop = (10_000 - self.sustain_pmy) * t / self.decay.max(1);
            return 10_000 - drop;
        }
        self.sustain_pmy
    }
}

impl Default for Adsr {
    /// Creates a default ADSR with attack=10, decay=20, sustain=6000, release=30 ticks.
    fn default() -> Self {
        Self::new(10, 20, 6000, 30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_ramps_to_full() {
        let e = Adsr::new(10, 10, 5000, 10);
        assert_eq!(e.sample(0, None), 0);
        assert_eq!(e.sample(5, None), 5000); // halfway up the attack
        assert_eq!(e.sample(10, None), 10_000); // peak at end of attack
    }

    #[test]
    fn decays_to_sustain_and_holds() {
        let e = Adsr::new(10, 10, 5000, 10);
        assert_eq!(e.sample(20, None), 5000); // end of decay = sustain
        assert_eq!(e.sample(100, None), 5000); // holds at sustain
    }

    #[test]
    fn release_falls_to_zero() {
        let e = Adsr::new(10, 10, 5000, 10);
        assert_eq!(e.sample(20, Some(20)), 5000); // release starts at sustain
        assert_eq!(e.sample(30, Some(20)), 0); // fully released
        assert_eq!(e.sample(99, Some(20)), 0);
    }
}
