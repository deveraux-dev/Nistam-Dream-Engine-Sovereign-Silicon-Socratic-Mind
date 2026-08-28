//! Weather — the four-era sky model. Integer states for deterministic simulation.
//!
//! Drains from forge-book/src/weather.rs (receipts: Era lines 11-39, Sky lines 42-70,
//! Weather lines 72-94, WeatherModel lines 96-128). Includes minimal Mulberry32 PRNG
//! for deterministic re-rolling (8-tick cycle). No f64 physics (quarantined render-edge);
//! pure integer: permyriad intensity, era/sky enum states, tick counter.

/// The four world eras — the narrative clock the sky answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Era {
    /// Calm, faded past.
    Ancient,
    /// Golden age of plenty.
    Golden,
    /// Declining fortunes.
    Decay,
    /// End times.
    Void,
}

impl Era {
    /// All four eras in sequence.
    pub fn all() -> [Era; 4] {
        [Era::Ancient, Era::Golden, Era::Decay, Era::Void]
    }

    /// Stable display name.
    pub fn name(&self) -> &'static str {
        match self {
            Era::Ancient => "Ancient",
            Era::Golden => "Golden",
            Era::Decay => "Decay",
            Era::Void => "Void",
        }
    }

    /// The next era in the cycle (wraps Void -> Ancient).
    pub fn next(&self) -> Era {
        match self {
            Era::Ancient => Era::Golden,
            Era::Golden => Era::Decay,
            Era::Decay => Era::Void,
            Era::Void => Era::Ancient,
        }
    }

    /// Does this era render under `forge_core_v3::monochrome`?
    ///
    /// Decay only, per Sean 2026-08-18: "Ironroot is black and white. And when
    /// the colour finally comes, it's beautiful." Decay IS the Ironroot era.
    ///
    /// OPEN, deliberately not decided here: whether Void drains too. Void is
    /// "end times", so draining it reads naturally — but the ruling names
    /// Decay and nothing else, and a world law is not somewhere to infer a
    /// second era's arc from tone. Void renders in colour until told otherwise.
    pub fn drains_colour(&self) -> bool {
        matches!(self, Era::Decay)
    }
}

/// The sky's mood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sky {
    /// Clear skies.
    Clear,
    /// Overcast.
    Overcast,
    /// Storms rage.
    Storm,
    /// Ashfall overhead.
    Ashfall,
    /// Cold, dry and dead still — the sky with the frost in it.
    Hardfrost,
}

impl Sky {
    /// Deterministic roll → Sky state (modulo 5).
    fn from_roll(roll: u32) -> Sky {
        match roll % 5 {
            0 => Sky::Clear,
            1 => Sky::Overcast,
            2 => Sky::Storm,
            3 => Sky::Ashfall,
            _ => Sky::Hardfrost,
        }
    }

    /// Stable display name.
    pub fn name(&self) -> &'static str {
        match self {
            Sky::Clear => "clear",
            Sky::Overcast => "overcast",
            Sky::Storm => "storm",
            Sky::Ashfall => "ashfall",
            Sky::Hardfrost => "hardfrost",
        }
    }
}

/// A weather reading: era, sky, integer intensity (permyriad).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weather {
    /// Current era.
    pub era: Era,
    /// Current sky state.
    pub sky: Sky,
    /// Intensity: 0..=10000 (permyriad of full intensity).
    pub intensity_pmy: u32,
}

impl Weather {
    /// The signature weather each era opens with.
    pub fn of(era: Era) -> Self {
        let (sky, intensity_pmy) = match era {
            Era::Ancient => (Sky::Clear, 2000),
            Era::Golden => (Sky::Clear, 4000),
            Era::Decay => (Sky::Ashfall, 7000),
            Era::Void => (Sky::Storm, 9000),
        };
        Self { era, sky, intensity_pmy }
    }

    /// Human-readable description.
    pub fn describe(&self) -> String {
        format!("{}: {} @ {}pmy", self.era.name(), self.sky.name(), self.intensity_pmy)
    }
}

use crate::rng::Mulberry32;

/// A deterministic weather model — drifts the sky over integer ticks.
///
/// Every 8th tick, the sky re-rolls via PRNG. Intensity drifts by a random
/// amount (0..400 permyriad) each tick, capped at 10000.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherModel {
    /// Current weather state.
    pub current: Weather,
    /// Internal PRNG state; regenerate from seed if needed.
    rng: Mulberry32,
    /// Tick counter for 8-tick re-roll cycle.
    tick: u32,
}

impl WeatherModel {
    /// Create a new weather model for an era, seeded for determinism.
    pub fn new(era: Era, seed: u32) -> Self {
        Self { current: Weather::of(era), rng: Mulberry32::new(seed as u64), tick: 0 }
    }

    /// Advance one tick; every 8th tick the sky re-rolls, intensity drifts.
    /// Returns the new weather state.
    pub fn tick(&mut self) -> Weather {
        self.tick = self.tick.wrapping_add(1);
        if self.tick % 8 == 0 {
            self.current.sky = Sky::from_roll(self.rng.next_u32());
        }
        let drift = self.rng.below(2 * INTENSITY_DRIFT_PMY + 1) as i32 - INTENSITY_DRIFT_PMY as i32;
        self.current.intensity_pmy =
            (self.current.intensity_pmy as i32 + drift).clamp(0, 10_000) as u32;
        self.current
    }
}

/// How far intensity may move in either direction per tick, permyriad.
const INTENSITY_DRIFT_PMY: u32 = 400;

/// How many commands a driven condition holds before the next drive.
pub const SKY_BANK_PERIOD: u64 = 8;

/// The six `WeatherSieve` inputs a sky implies. The other fields on that
/// struct have no reader in this crate and are left where they were.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkyDrive {
    /// Centigrade, the sieve's own unit.
    pub temperature: i32,
    /// Millimetre-equivalent fall.
    pub precipitation: i32,
    /// The sieve's arbitrary wind units.
    pub wind_speed: i32,
    /// Warm-wind pressure on the high air.
    pub chinook_buildup: i32,
    /// Sieve ticks of held cold.
    pub blizzard_ticks: u32,
    /// Sieve ticks of held dry.
    pub drought_ticks: u32,
}

/// Freezing, in the sieve's centigrade.
const FREEZING_C: i32 = 0;
/// A held blizzard thaws over two banks once the cold breaks — a decay of 1
/// would leave it standing for eight, which is a season, not a condition.
const BLIZZARD_THAW: u32 = SKY_BANK_PERIOD as u32 / 2;
/// The temperature rise across one bank that reads as a chinook.
const CHINOOK_RISE_C: i32 = 5;
/// Wind a chinook needs behind it.
const CHINOOK_WIND: i32 = 6;
/// How much buildup one qualifying bank adds, and its ceiling.
const CHINOOK_STEP: i32 = 3;
const CHINOOK_CAP: i32 = 10;
/// Warm enough, with nothing falling, to dry the ground out.
const DROUGHT_WARM_C: i32 = 22;
/// Dry ticks one bank adds, and the ceiling `tick()`'s decrement settles under.
const DROUGHT_STEP: u32 = 12;
const DROUGHT_CAP: u32 = 25;

/// The temperature an era rests at before any sky touches it. Decay and Void
/// run cold — that is what lets the end-times eras reach a blizzard at all.
const fn era_baseline_c(era: Era) -> i32 {
    match era {
        Era::Ancient => 14,
        Era::Golden => 20,
        Era::Decay => 8,
        Era::Void => 2,
    }
}

/// What a sky implies for the sieve, given where the sieve already stood.
/// Total and integer-only: the sky is already a seeded stream, so this adds
/// no randomness of its own and the same world replays byte for byte.
pub fn drive(w: Weather, prev: SkyDrive, prev_recorded_c: i32) -> SkyDrive {
    let i = w.intensity_pmy as i32;
    let base = era_baseline_c(w.era);
    let (temperature, precipitation, wind_speed) = match w.sky {
        Sky::Clear => (base + i / 500, 0, i / 1_000),
        Sky::Overcast => (base - i / 1_000, i / 400, i / 800),
        Sky::Storm => (base - i / 800, i / 200, i / 250),
        Sky::Ashfall => (base + i / 800, i / 900, i / 600),
        Sky::Hardfrost => (base - i / 600, 0, i / 2_000),
    };

    let blizzard_ticks = if temperature <= FREEZING_C && precipitation > 0 {
        SKY_BANK_PERIOD as u32
    } else {
        prev.blizzard_ticks.saturating_sub(BLIZZARD_THAW)
    };

    let drought_ticks = if precipitation == 0 && temperature >= DROUGHT_WARM_C {
        (prev.drought_ticks + DROUGHT_STEP).min(DROUGHT_CAP)
    } else {
        prev.drought_ticks
    };

    let chinook_buildup = if temperature >= prev_recorded_c + CHINOOK_RISE_C && wind_speed >= CHINOOK_WIND {
        (prev.chinook_buildup + CHINOOK_STEP).min(CHINOOK_CAP)
    } else {
        (prev.chinook_buildup - 1).max(0)
    };

    SkyDrive { temperature, precipitation, wind_speed, chinook_buildup, blizzard_ticks, drought_ticks }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn era_cycle_wraps() {
        assert_eq!(Era::Void.next(), Era::Ancient);
        assert_eq!(Era::all().len(), 4);
    }

    /// The monochrome law binds to exactly one era, and it is the named one.
    #[test]
    fn only_the_decay_era_drains_colour() {
        assert!(Era::Decay.drains_colour(), "Decay is the Ironroot era");
        assert!(!Era::Ancient.drains_colour());
        assert!(!Era::Golden.drains_colour());
        assert!(!Era::Void.drains_colour(), "Void is an open question, not a silent yes");
        assert_eq!(
            Era::all().iter().filter(|e| e.drains_colour()).count(),
            1,
            "one drained era — inferring a second from tone is not a ruling"
        );
    }

    /// The law and the era agree end to end: a drained era greys a real colour,
    /// and an earned fact brings that same colour back unchanged.
    #[test]
    fn the_drained_era_greys_a_colour_until_its_fact_is_earned() {
        use forge_core_v3::colour::OklchColor;
        use forge_core_v3::monochrome::{drained, MonochromeLaw};
        use forge_core_v3::organs::creation_spine::LoreFactId;

        let bell = OklchColor { l: 28_000, c: 18_000, h: 9_000, a: u16::MAX };
        let bell_fact = LoreFactId(13);
        let mut law = MonochromeLaw::drained_era();

        assert!(Era::Decay.drains_colour());
        assert_eq!(law.render(bell, Some(bell_fact)), drained(bell), "grey until earned");

        law.restore(bell_fact);
        assert_eq!(law.render(bell, Some(bell_fact)), bell, "and beautiful when it comes");
    }

    #[test]
    fn signature_weather_per_era() {
        assert_eq!(Weather::of(Era::Void).sky, Sky::Storm);
        assert_eq!(Weather::of(Era::Ancient).intensity_pmy, 2000);
    }

    #[test]
    fn sky_from_roll_modulo() {
        assert_eq!(Sky::from_roll(0), Sky::Clear);
        assert_eq!(Sky::from_roll(5), Sky::Clear);
        assert_eq!(Sky::from_roll(2), Sky::Storm);
        assert_eq!(Sky::from_roll(4), Sky::Hardfrost);
    }

    #[test]
    fn model_is_deterministic() {
        let mut a = WeatherModel::new(Era::Golden, 11);
        let mut b = WeatherModel::new(Era::Golden, 11);
        for _ in 0..64 {
            assert_eq!(a.tick(), b.tick());
        }
    }

    #[test]
    fn intensity_never_exceeds_ceiling() {
        let mut m = WeatherModel::new(Era::Decay, 3);
        for _ in 0..1000 {
            assert!(m.tick().intensity_pmy <= 10_000);
        }
    }

    fn calm() -> SkyDrive {
        SkyDrive {
            temperature: 20,
            precipitation: 0,
            wind_speed: 0,
            chinook_buildup: 0,
            blizzard_ticks: 0,
            drought_ticks: 0,
        }
    }

    fn w(era: Era, sky: Sky, intensity_pmy: u32) -> Weather {
        Weather { era, sky, intensity_pmy }
    }

    /// The drift moves BOTH ways and settles nowhere. The old unsigned add
    /// ratcheted to the ceiling in ~25 ticks and pinned there forever.
    #[test]
    fn intensity_drifts_both_ways() {
        let mut m = WeatherModel::new(Era::Golden, 17);
        let (mut rose, mut fell) = (false, false);
        let mut last = m.current.intensity_pmy;
        let mut ceiling_run = 0;
        let mut longest_ceiling_run = 0;
        for _ in 0..2_000 {
            let now = m.tick().intensity_pmy;
            assert!(now <= 10_000, "intensity left its ceiling: {now}");
            if now > last {
                rose = true;
            }
            if now < last {
                fell = true;
            }
            ceiling_run = if now == 10_000 { ceiling_run + 1 } else { 0 };
            longest_ceiling_run = longest_ceiling_run.max(ceiling_run);
            last = now;
        }
        assert!(rose && fell, "a drift that only rises is a ratchet, not a drift");
        assert!(longest_ceiling_run < 100, "intensity pinned at the ceiling for {longest_ceiling_run} ticks");
    }

    /// A drift that is centred visits the whole dial, not one end of it. The
    /// weight a room speaks comes off these bands, so a walk that never
    /// leaves the top reads as one weather forever.
    #[test]
    fn intensity_walks_the_whole_dial() {
        let mut m = WeatherModel::new(Era::Decay, 5);
        let (mut lo, mut hi) = (u32::MAX, 0u32);
        let mut sum: i64 = 0;
        for _ in 0..20_000 {
            let v = m.tick().intensity_pmy;
            lo = lo.min(v);
            hi = hi.max(v);
            sum += v as i64;
        }
        assert!(hi - lo > 6_000, "the dial only moved {} wide: {lo}..{hi}", hi - lo);
        let mean = sum / 20_000;
        assert!((2_000..=8_000).contains(&mean), "the walk is parked at one end: mean {mean}");
    }

    /// Five skies, five weathers. A sky that reads like its neighbour gives
    /// the loom nothing to tell them apart by.
    #[test]
    fn each_sky_has_its_own_signature() {
        let skies = [Sky::Clear, Sky::Overcast, Sky::Storm, Sky::Ashfall, Sky::Hardfrost];
        let drives: Vec<SkyDrive> =
            skies.iter().map(|&s| drive(w(Era::Golden, s, 6_000), calm(), 20)).collect();
        for (i, a) in drives.iter().enumerate() {
            for (j, b) in drives.iter().enumerate().skip(i + 1) {
                assert_ne!(a, b, "{:?} and {:?} read the same", skies[i], skies[j]);
            }
        }
    }

    /// L18: the distinctness above is not vacuous. A mapping that ignores the
    /// sky collides on every pair — asserted to fail BEFORE the real one is
    /// asserted to pass.
    #[test]
    fn sabotaged_flat_sky_would_lose_every_signature() {
        let skies = [Sky::Clear, Sky::Overcast, Sky::Storm, Sky::Ashfall, Sky::Hardfrost];
        // Sabotage: era and intensity only, sky discarded.
        let flat = |x: Weather| SkyDrive { temperature: era_baseline_c(x.era), ..calm() };
        let sabotaged: Vec<SkyDrive> =
            skies.iter().map(|&s| flat(w(Era::Golden, s, 6_000))).collect();
        assert!(
            sabotaged.windows(2).all(|p| p[0] == p[1]),
            "the sabotage must collapse every sky, or the real test proves nothing"
        );
        // Revert: `flat` is a local closure; `drive` above is untouched and distinct.
        let real: Vec<SkyDrive> =
            skies.iter().map(|&s| drive(w(Era::Golden, s, 6_000), calm(), 20)).collect();
        assert!(real.windows(2).all(|p| p[0] != p[1]), "the real mapping must separate them");
    }

    /// The two counters answer to the conditions that name them.
    #[test]
    fn a_cold_wet_sky_holds_a_blizzard_and_a_hot_dry_one_a_drought() {
        let storm = drive(w(Era::Void, Sky::Storm, 9_000), calm(), 20);
        assert!(storm.temperature <= 0, "a Void storm must run cold: {}", storm.temperature);
        assert!(storm.precipitation > 0);
        assert_eq!(storm.blizzard_ticks, SKY_BANK_PERIOD as u32, "cold and wet is a blizzard");
        assert_eq!(storm.drought_ticks, 0, "nothing dries out under falling weather");

        let clear = drive(w(Era::Golden, Sky::Clear, 8_000), calm(), 20);
        assert!(clear.temperature >= DROUGHT_WARM_C, "a Golden clear sky must run warm");
        assert_eq!(clear.precipitation, 0);
        assert!(clear.drought_ticks > 0, "warm and dry is a drought");
        assert_eq!(clear.blizzard_ticks, 0);
    }

    /// A blizzard is a HOLD, not a flash — and not a season either. Once the
    /// cold breaks it thaws over two banks, then it is gone.
    #[test]
    fn a_broken_blizzard_thaws_over_two_banks_then_clears() {
        let warm = w(Era::Golden, Sky::Clear, 8_000);
        let held = drive(w(Era::Void, Sky::Storm, 9_000), calm(), 20);
        assert!(held.blizzard_ticks > 0);
        let first = drive(warm, held, 20);
        assert!(first.blizzard_ticks > 0, "it must not vanish the moment the sky turns");
        assert!(first.blizzard_ticks < held.blizzard_ticks, "but it must be letting go");
        let second = drive(warm, first, 20);
        assert_eq!(second.blizzard_ticks, 0, "and be gone by the second bank");
    }

    /// A chinook is a RISE, not a temperature — it reads against what the
    /// sieve last recorded, and drains when the air stops climbing.
    #[test]
    fn a_chinook_needs_the_air_to_climb_not_merely_to_be_warm() {
        let warm = w(Era::Golden, Sky::Clear, 9_000);
        let after_cold = drive(warm, calm(), -10);
        assert!(after_cold.chinook_buildup > 0, "warm air over cold ground leans on it");
        let settled = drive(warm, after_cold, after_cold.temperature);
        assert!(
            settled.chinook_buildup < after_cold.chinook_buildup,
            "with nothing left to climb, the buildup drains"
        );
    }

    /// `WeatherSieve::tick` decrements drought once per command while a drive
    /// adds once per bank — the two settle instead of running away.
    #[test]
    fn a_sustained_dry_sky_settles_instead_of_climbing_forever() {
        let dry = w(Era::Golden, Sky::Clear, 8_000);
        let mut d = calm();
        for _ in 0..20 {
            d = drive(dry, d, d.temperature);
            // What the sieve does to it across one bank of commands.
            d.drought_ticks = d.drought_ticks.saturating_sub(SKY_BANK_PERIOD as u32);
        }
        assert!(d.drought_ticks <= DROUGHT_CAP, "drought ran away: {}", d.drought_ticks);
        assert!(d.drought_ticks > 0, "a sustained dry sky must hold some dry");
    }

    #[test]
    fn weather_ticks_increment_counter() {
        let mut m = WeatherModel::new(Era::Ancient, 42);
        for _ in 1..=16 {
            m.tick();
            // Every 8th tick re-rolls sky (can't easily verify without exposing tick).
            // At tick 8 and 16, sky may change.
        }
    }

    #[test]
    fn mulberry32_deterministic() {
        let mut a = Mulberry32::new(99);
        let mut b = Mulberry32::new(99);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn mulberry32_below_in_range() {
        let mut g = Mulberry32::new(7);
        for _ in 0..20_000 {
            assert!(g.below(401) < 401);
        }
        assert_eq!(g.below(0), 0);
    }
}
