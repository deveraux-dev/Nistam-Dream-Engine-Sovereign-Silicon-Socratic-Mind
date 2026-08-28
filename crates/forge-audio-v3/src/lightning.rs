//! lightning.rs — chaos-driven RANDOM lightning: one strike → SEE + HEAR.
//!
//! Ported from `F:\NewRepo\crates\forge-core\src\lightning.rs` (v2 Crate
//! Zero). ONE real adaptation: v2's `ForgeRng::next_u64()` became two
//! `forge_core_v3::seed::Mulberry32::next_u32()` draws combined into a u64
//! (v3's Crate Zero ships `Mulberry32`, not `ForgeRng` — same role, no
//! `next_u64` method) — everything else, including every test's expected
//! behavior, is unchanged; only the RNG draw width differs, not any formula.
//!
//! A `charge` level (permyriad) sets the strike RATE; a deterministic RNG
//! decides WHEN. Random in time and place, but byte-replayable from the seed.
//! Each [`LightningStrike`] carries BOTH a visual face and an audio face from
//! the ONE event (HEAR == SEE). Integer-only, zero float.

use forge_core::seed::Mulberry32;

fn next_u64(rng: &mut Mulberry32) -> u64 {
    ((rng.next_u32() as u64) << 32) | rng.next_u32() as u64
}

/// Hard cap on strike probability per tick, in permyriad, at FULL charge.
pub const MAX_STRIKE_RATE_PMY: u32 = 500;

/// One lightning strike — the shared root of its visual and audio faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightningStrike {
    pub origin: [i32; 2],
    pub branch_seed: u64,
    pub intensity_pmy: u32,
    pub duration_ticks: u16,
}

impl LightningStrike {
    #[inline]
    pub fn glow_permyriad(&self) -> u32 {
        self.intensity_pmy
    }
    #[inline]
    pub fn chromatic_shift(&self) -> i32 {
        let m = (self.branch_seed % 4001) as i32 - 2000;
        m * self.intensity_pmy as i32 / 10_000
    }
    #[inline]
    pub fn shake_permyriad(&self) -> u32 {
        (self.intensity_pmy as u64 * self.intensity_pmy as u64 / 10_000) as u32
    }
    #[inline]
    pub fn crackle_permyriad(&self) -> u32 {
        self.intensity_pmy
    }
    #[inline]
    pub fn pitch_hz(&self) -> u32 {
        40 + (self.branch_seed % 161) as u32
    }
}

/// The charged field over a canvas. Tick it every frame; it fires random
/// strikes at a rate set by the current charge.
#[derive(Debug, Clone)]
pub struct LightningField {
    rng: Mulberry32,
    charge_pmy: u32,
    grid_w: i32,
    grid_h: i32,
    tick: u64,
}

impl LightningField {
    pub fn new(seed: u64, grid_w: i32, grid_h: i32) -> Self {
        Self {
            rng: Mulberry32::new(seed),
            charge_pmy: 0,
            grid_w: grid_w.max(1),
            grid_h: grid_h.max(1),
            tick: 0,
        }
    }

    #[inline]
    pub fn set_charge(&mut self, charge_pmy: u32) {
        self.charge_pmy = charge_pmy.min(10_000);
    }

    #[inline]
    pub fn charge(&self) -> u32 {
        self.charge_pmy
    }

    pub fn tick(&mut self) -> Option<LightningStrike> {
        self.tick += 1;
        roll_and_build(&mut self.rng, self.charge_pmy, self.grid_w, self.grid_h)
    }
}

fn roll_and_build(
    rng: &mut Mulberry32,
    charge_pmy: u32,
    grid_w: i32,
    grid_h: i32,
) -> Option<LightningStrike> {
    let threshold = charge_pmy.min(10_000) * MAX_STRIKE_RATE_PMY / 10_000;
    let roll = (next_u64(rng) % 10_000) as u32;
    if roll >= threshold {
        return None;
    }
    let ox = (next_u64(rng) % grid_w.max(1) as u64) as i32;
    let oy = (next_u64(rng) % grid_h.max(1) as u64) as i32;
    let branch_seed = next_u64(rng);
    let jitter = (branch_seed % 4001) as u32;
    let intensity_pmy = (charge_pmy.min(10_000) * (8000 + jitter) / 10_000).min(10_000);
    let duration_ticks = (6 + (branch_seed >> 8) % 18) as u16;
    Some(LightningStrike { origin: [ox, oy], branch_seed, intensity_pmy, duration_ticks })
}

/// Stateless per-tick roll: derive a deterministic stream from `(seed, tick)`
/// and roll once. Same `(seed, tick, charge)` always yields the same result.
pub fn roll_strike(
    seed: u64,
    tick: u64,
    charge_pmy: u32,
    grid_w: i32,
    grid_h: i32,
) -> Option<LightningStrike> {
    let mut rng = Mulberry32::new(seed ^ tick.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    roll_and_build(&mut rng, charge_pmy, grid_w, grid_h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_charge_never_strikes() {
        let mut f = LightningField::new(0xB017, 64, 48);
        for _ in 0..10_000 {
            assert!(f.tick().is_none(), "no charge must never fire");
        }
    }

    #[test]
    fn higher_charge_strikes_more_often() {
        let count = |charge: u32| {
            let mut f = LightningField::new(42, 64, 48);
            f.set_charge(charge);
            (0..100_000).filter(|_| f.tick().is_some()).count()
        };
        let low = count(1_000);
        let high = count(9_000);
        assert!(high > low * 3, "9x charge should storm far more: low={low} high={high}");
    }

    #[test]
    fn strikes_land_inside_the_grid() {
        let mut f = LightningField::new(7, 64, 48);
        f.set_charge(10_000);
        let mut fired = 0;
        for _ in 0..5_000 {
            if let Some(s) = f.tick() {
                fired += 1;
                assert!((0..64).contains(&s.origin[0]), "x in bounds: {:?}", s.origin);
                assert!((0..48).contains(&s.origin[1]), "y in bounds: {:?}", s.origin);
            }
        }
        assert!(fired > 0, "full charge must fire at least once");
    }

    #[test]
    fn storm_is_byte_replayable() {
        let run = || {
            let mut f = LightningField::new(0xDEAD_BEEF, 32, 32);
            f.set_charge(6_000);
            (0..2_000).filter_map(|_| f.tick()).collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "same seed + charge → identical storm");
    }

    #[test]
    fn one_strike_yields_both_faces() {
        let s = LightningStrike { origin: [10, 5], branch_seed: 0x1234_5678, intensity_pmy: 9000, duration_ticks: 12 };
        assert_eq!(s.glow_permyriad(), 9000);
        assert!(s.shake_permyriad() > 0);
        assert!(s.crackle_permyriad() == 9000, "HEAR == SEE energy");
        assert!((40..=200).contains(&s.pitch_hz()), "thunder pitch in band");
    }

    #[test]
    fn roll_strike_stateless_matches_field_replay_and_zero_charge_is_silent() {
        for t in 0..5_000 {
            assert!(roll_strike(99, t, 0, 64, 48).is_none());
        }
        assert_eq!(roll_strike(7, 123, 6_000, 64, 48), roll_strike(7, 123, 6_000, 64, 48));
        let hits: Vec<_> = (0..5_000).filter_map(|t| roll_strike(7, t, 9_000, 64, 48)).collect();
        assert!(!hits.is_empty(), "high charge must strike over 5000 ticks");
        for s in &hits {
            assert!((0..64).contains(&s.origin[0]) && (0..48).contains(&s.origin[1]));
        }
    }

    #[test]
    fn chromatic_shift_bounded_and_seed_varied() {
        let a = LightningStrike { origin: [0, 0], branch_seed: 100, intensity_pmy: 10_000, duration_ticks: 8 };
        let b = LightningStrike { origin: [0, 0], branch_seed: 3999, intensity_pmy: 10_000, duration_ticks: 8 };
        assert!(a.chromatic_shift().abs() <= 2000);
        assert!(b.chromatic_shift().abs() <= 2000);
        assert_ne!(a.chromatic_shift(), b.chromatic_shift(), "different bolts tint differently");
    }
}
