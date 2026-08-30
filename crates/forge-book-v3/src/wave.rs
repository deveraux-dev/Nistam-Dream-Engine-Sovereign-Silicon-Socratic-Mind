//! Wave — integer waveform generators (amplitude over a permyriad phase). Sine
//! rides a 16-point LUT with linear interp; saw/square/triangle are exact.

use serde::{Deserialize, Serialize};

const SINE16: [i32; 17] = [
    0, 3827, 7071, 9239, 10_000, 9239, 7071, 3827, 0, -3827, -7071, -9239, -10_000, -9239, -7071,
    -3827, 0,
];

/// Integer sine of a permyriad phase (`0..10000` = one period) -> `-10000..10000`.
fn sine(phase_pmy: u32) -> i32 {
    let p = phase_pmy.min(9_999);
    let scaled = p * 16;
    let idx = (scaled / 10_000) as usize;
    let frac = (scaled % 10_000) as i32;
    let a = SINE16[idx];
    let b = SINE16[idx + 1];
    a + (b - a) * frac / 10_000
}

/// A waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wave {
    /// Smooth sine wave, 16-point LUT with linear interpolation.
    Sine,
    /// Sawtooth wave ramping linearly from -10000 to +10000.
    Saw,
    /// Square wave stepping from +10000 to -10000 at half period.
    Square,
    /// Triangle wave bouncing symmetrically from -10000 to +10000.
    Triangle,
}

impl Wave {
    /// Amplitude at `phase_pmy` (`0..10000` = one period), range `-10000..=10000`.
    pub fn at(&self, phase_pmy: u32) -> i32 {
        let p = phase_pmy.min(10_000) as i32;
        match self {
            Wave::Sine => sine(phase_pmy),
            Wave::Saw => p * 20_000 / 10_000 - 10_000,
            Wave::Square => {
                if p < 5_000 {
                    10_000
                } else {
                    -10_000
                }
            }
            Wave::Triangle => {
                if p < 5_000 {
                    p * 20_000 / 5_000 - 10_000
                } else {
                    10_000 - (p - 5_000) * 20_000 / 5_000
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_hits_known_points() {
        assert_eq!(Wave::Sine.at(0), 0);
        assert_eq!(Wave::Sine.at(2_500), 10_000); // quarter = peak
        assert_eq!(Wave::Sine.at(7_500), -10_000); // three-quarter = trough
    }

    #[test]
    fn saw_ramps_linearly() {
        assert_eq!(Wave::Saw.at(0), -10_000);
        assert_eq!(Wave::Saw.at(5_000), 0);
        assert_eq!(Wave::Saw.at(10_000), 10_000);
    }

    #[test]
    fn square_flips_at_half() {
        assert_eq!(Wave::Square.at(0), 10_000);
        assert_eq!(Wave::Square.at(6_000), -10_000);
    }

    #[test]
    fn all_waves_stay_in_range() {
        for p in (0..=10_000).step_by(101) {
            for w in [Wave::Sine, Wave::Saw, Wave::Square, Wave::Triangle] {
                let v = w.at(p);
                assert!((-10_000..=10_000).contains(&v));
            }
        }
    }
}
