//! Skills — the seven arts and the twenty-one concrete skills that train
//! them. Skills rise BY USE, never by spending points (ARCH000 2026-08-11:
//! "mostly everquest and UO skill based"). A hard TOTAL cap of 3000 (of a
//! possible 7000 — seven arts at 1000 each) forces every operator into a
//! build: mastering some arts means abandoning others.
//!
//! Two layers, one register: the SEVEN ARTS (`ARTS`) are the governing
//! registers, one per [`crate::hermetics::SEVENFOLD`] row (L05: that table
//! is the one home for the sevenfold split — this module does not repeat
//! it, only rides its `Stat` order). The TWENTY-ONE CONCRETE SKILLS
//! (`SKILLS`) are the named verbs that actually train: three per art, each
//! pointing at its governing art's index. Training is CONTEXTUAL — the
//! caller (the conductor's act-resolution, not this module) picks which
//! concrete skill index qualifies for a given act, e.g. camping at night
//! trains dark camouflage, camping in a forest biome trains woods
//! camouflage. Every concrete skill under one art shares that art's single
//! stored register (0..=1000): `art_value` is that register's value
//! directly — with one register per art there is nothing to take a max
//! over, so the "as good as its best-practiced face" ruling collapses to
//! identity, and the slide lives entirely in WHICH named skill a use
//! counts toward, not in separate storage per skill.

use crate::hermetics::Stat;
use crate::operator::seed_hash;
use forge_core_v3::sprite_blob::u64_to_nistam;

/// The seven arts — one governing register per [`crate::hermetics::SEVENFOLD`]
/// row, same order (Vigor..Guilt), so an art's index is also its
/// `Stat::index()` and its `crate::hermetics::CORE_PALETTE` slot.
pub const ARTS: [(&str, Stat); 7] = [
    ("the Hunt", Stat::Vigor),
    ("the Veil", Stat::ShadowWeight),
    ("the Craft", Stat::LogicDepth),
    ("the Current", Stat::Momentum),
    ("the Rust", Stat::Tarnish),
    ("the Parley", Stat::Resonance),
    ("the Vigil", Stat::Guilt),
];

/// This art's CORE_PALETTE colour (L05: the one palette lives in
/// `hermetics`; this just indexes it by the shared row order).
pub fn art_color(art: usize) -> u32 {
    crate::hermetics::CORE_PALETTE[art]
}

/// The twenty-one concrete skills: `(name, governing art index)`, three per
/// art in `ARTS` order. Word-only names, no digits. Includes, per the exact
/// ruling: dark camouflage, woods camouflage, camping, skinning, fishing,
/// wisdom, resistance.
pub const SKILLS: [(&str, usize); 21] = [
    // the Hunt (Vigor) — active, physical, outdoors.
    ("skinning", 0),
    ("camping", 0),
    ("fishing", 0),
    // the Veil (ShadowWeight) — poise, staying unseen.
    ("dark camouflage", 1),
    ("woods camouflage", 1),
    ("scavenging", 1),
    // the Craft (LogicDepth) — mind, made things.
    ("brewing", 2),
    ("wisdom", 2),
    ("warding", 2),
    // the Current (Momentum) — speed, finding the way.
    ("wayfinding", 3),
    ("angling", 3),
    ("tracking", 3),
    // the Rust (Tarnish) — corruption, decay, salvage.
    ("resistance", 4),
    ("salvaging", 4),
    ("scrapping", 4),
    // the Parley (Resonance) — attunement, charm, the spoken word.
    ("parley", 5),
    ("bartering", 5),
    ("witnessing", 5),
    // the Vigil (Guilt) — the ledger's weight.
    ("confession", 6),
    ("atonement", 6),
    ("vigilance", 6),
];

/// The spoken rank ladder — UO's own seven rungs, never a bare number.
const RANK_WORDS: [&str; 7] =
    ["untried", "dabbling", "apprentice", "journeyman", "adept", "master", "grandmaster"];

/// The trade's ladder as LAW (Sean 2026-08-24, journeyman painter, 23 years
/// in the trade): a tier is not a label, it is rights and efficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TradeTier {
    /// Never picked up the brush.
    Untried,
    /// Watching the trade, touching the tools.
    Dabbling,
    /// Works under someone; the trade's door is open.
    Apprentice,
    /// Works alone, charges for the work.
    Journeyman,
    /// The hand knows before the eye does.
    Adept,
    /// Takes apprentices; teaches the trade.
    Master,
    /// Names new work into the trade itself.
    Grandmaster,
}

impl TradeTier {
    /// The tier an art's register stands at — the same /143 split the
    /// spoken ladder has always used.
    pub const fn of(value: u16) -> TradeTier {
        match value as usize / 143 {
            0 => TradeTier::Untried,
            1 => TradeTier::Dabbling,
            2 => TradeTier::Apprentice,
            3 => TradeTier::Journeyman,
            4 => TradeTier::Adept,
            5 => TradeTier::Master,
            _ => TradeTier::Grandmaster,
        }
    }

    /// The spoken rank. No digits.
    pub const fn word(self) -> &'static str {
        RANK_WORDS[self as usize]
    }

    /// How many of a school's five sung words this tier holds open. The
    /// door gifts one; the trade opens the rest.
    pub const fn words_open(self) -> usize {
        match self {
            TradeTier::Untried | TradeTier::Dabbling => 1,
            TradeTier::Apprentice => 2,
            TradeTier::Journeyman => 3,
            TradeTier::Adept => 4,
            TradeTier::Master | TradeTier::Grandmaster => 5,
        }
    }

    /// The trade's hand, permyriad of a cast's full cost: the same wall,
    /// fewer strokes — a master paints faster than the rest of the parish.
    pub const fn efficiency_q(self) -> i64 {
        match self {
            TradeTier::Untried => 10_000,
            TradeTier::Dabbling => 9_500,
            TradeTier::Apprentice => 9_000,
            TradeTier::Journeyman => 8_000,
            TradeTier::Adept => 6_500,
            TradeTier::Master => 5_000,
            TradeTier::Grandmaster => 4_000,
        }
    }

    /// Journeyman and up works for pay — the vendor's counter opens.
    pub const fn may_charge(self) -> bool {
        matches!(self, TradeTier::Journeyman | TradeTier::Adept | TradeTier::Master | TradeTier::Grandmaster)
    }

    /// Master and up takes apprentices — the right to TEACH (gift words).
    pub const fn may_teach(self) -> bool {
        matches!(self, TradeTier::Master | TradeTier::Grandmaster)
    }

    /// Only a grandmaster names NEW work into the trade — player-authored
    /// words, sung by the chain already, canonized by this right alone.
    pub const fn may_name(self) -> bool {
        matches!(self, TradeTier::Grandmaster)
    }
}

/// A skill value's ceiling (UO's 0-100.0 x 10) and an art's per-register cap.
pub const SKILL_MAX: u16 = 1000;
/// The hard TOTAL cap across all seven arts — three full masteries OR seven
/// 437-deep dabblings; the cap is what forces the build (of a possible
/// 7000 = 7 arts x 1000).
pub const TOTAL_CAP: u32 = 3000;

/// The ruled honeymoon ceiling: below this, every qualifying use gains +1.
const HONEYMOON: u16 = 150;

/// The tuned gain-curve ladder: `(xp_upper_bound_exclusive, D)`. Breakpoints
/// 150/250/300/400/550/700/850/950/1000 are ARCH000-ruled and untouched;
/// the D values are tuned x6 from the ruled 2/3/5/8/13/21/34/55 seed so
/// three masteries land in the 800-1200 hour band at 4 qualifying
/// uses/minute (see `expected_uses_to_master`) — the ruled seed alone
/// landed three masteries at ~168 hours, far under band.
const LADDER: [(u16, u32); 8] =
    [(250, 12), (300, 18), (400, 30), (550, 48), (700, 78), (850, 126), (950, 204), (1000, 330)];

/// D for a value already at or above the honeymoon ceiling.
fn ladder_d(value: u16) -> u32 {
    for &(upper, d) in &LADDER {
        if value < upper {
            return d;
        }
    }
    LADDER[LADDER.len() - 1].1
}

/// The seven arts' standing plus how many qualifying uses each has seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Skills {
    /// Each art's current register, 0..=1000.
    pub value: [u16; 7],
    /// Each art's lifetime qualifying-use count (the roll's own entropy).
    pub uses: [u64; 7],
}

impl Skills {
    /// This art's effective standing (see the module doc: one register per
    /// art, so this is that register directly).
    pub fn art_value(&self, art: usize) -> u16 {
        self.value[art]
    }

    /// Total across all seven arts — never exceeds [`TOTAL_CAP`].
    pub fn total(&self) -> u32 {
        self.value.iter().map(|&v| v as u32).sum()
    }

    /// The trade tier an art currently stands at.
    pub fn tier(&self, art: usize) -> TradeTier {
        TradeTier::of(self.value[art])
    }

    /// The spoken rank word for an art's current standing. No digits.
    pub fn word(&self, art: usize) -> &'static str {
        self.tier(art).word()
    }

    /// Decay an art's register toward zero by `amount` (saturating). A
    /// future `unlearn` verb's own hook — nothing calls this today, so
    /// nothing decays silently (L10-adjacent: only an explicit call moves
    /// this number).
    pub fn decay_toward(&mut self, art: usize, amount: u16) {
        self.value[art] = self.value[art].saturating_sub(amount);
    }

    /// Seed an art at birth — the natal star's gift, not earned use. Obeys the
    /// same two caps `train` does ([`SKILL_MAX`] per art, [`TOTAL_CAP`] across
    /// the seven), so a boon can never carry an operator past a ceiling the
    /// grind itself respects. Returns the art's new value.
    ///
    /// `uses` is deliberately NOT incremented: this is standing the operator
    /// was born with, and counting it as practice would corrupt the ladder's
    /// own entropy (`train` deals off `uses`).
    pub fn seed_art(&mut self, art: usize, points: u16) -> u16 {
        let head_room_art = SKILL_MAX.saturating_sub(self.value[art]);
        let head_room_total = TOTAL_CAP.saturating_sub(self.total()).min(u16::MAX as u32) as u16;
        let grant = points.min(head_room_art).min(head_room_total);
        self.value[art] += grant;
        self.value[art]
    }

    /// One qualifying use against `skill` (an index into [`SKILLS`]).
    /// `seed`/`xp` deal the roll once the honeymoon (< 150) is spent; same
    /// (seed, xp, art, uses) always deals the same verdict. `None` = no
    /// gain this use (still counted); `Some(new_value)` = the art rose by
    /// one. `None` also when this art or the total is already at cap.
    pub fn train(&mut self, skill: usize, seed: u64, xp: u64) -> Option<u16> {
        let art = SKILLS[skill].1;
        if self.value[art] >= SKILL_MAX || self.total() >= TOTAL_CAP {
            return None;
        }
        self.uses[art] += 1;
        let v = self.value[art];
        let gain = if v < HONEYMOON {
            true
        } else {
            let d = ladder_d(v);
            let roll = seed_hash(&[
                &u64_to_nistam(seed),
                &u64_to_nistam(xp),
                &[art as u8],
                &u64_to_nistam(self.uses[art]),
            ]);
            roll % d as u64 == 0
        };
        if gain {
            self.value[art] += 1;
            Some(self.value[art])
        } else {
            None
        }
    }
}

/// The 1000-hour tuning proof: expected qualifying uses to march ONE art
/// from 0 to 1000 — 150 free (the honeymoon) plus, per ladder band, its
/// width times its D (a 1-in-D roll's expectation is D trials per success).
pub fn expected_uses_to_master() -> u64 {
    let mut uses: u64 = HONEYMOON as u64;
    let mut floor = HONEYMOON;
    for &(upper, d) in &LADDER {
        let width = (upper - floor) as u64;
        uses += width * d as u64;
        floor = upper;
    }
    uses
}

#[cfg(test)]
mod tests {
    use super::*;

    /// From 0, forced uses raise an art's value monotonically, and the gain
    /// rate slows with height: early calls land more gains per window than
    /// late ones near the cap's approach.
    #[test]
    fn gain_curve_is_monotonic_and_slows_with_height() {
        let mut s = Skills::default();
        let skill = 0; // skinning -> art 0
        let mut values = Vec::with_capacity(10_000);
        for xp in 0..10_000u64 {
            s.train(skill, 42, xp);
            values.push(s.value[0]);
        }
        for w in values.windows(2) {
            assert!(w[1] >= w[0], "value must never fall");
        }
        let early_gain = values[999] - values[0];
        let late_gain = values[9_999] - values[9_000];
        assert!(
            early_gain > late_gain,
            "early window gained {early_gain}, late window gained {late_gain} \
             — the curve should slow near the cap"
        );
    }

    /// Same (seed, xp-sequence) deals the same skills, forever.
    #[test]
    fn training_is_deterministic() {
        let mut a = Skills::default();
        let mut b = Skills::default();
        for xp in 0..5_000u64 {
            a.train(xp as usize % SKILLS.len(), 7, xp);
            b.train(xp as usize % SKILLS.len(), 7, xp);
        }
        assert_eq!(a, b);
    }

    /// The total never exceeds TOTAL_CAP; once there, train() always
    /// returns None.
    #[test]
    fn total_cap_holds() {
        let mut s = Skills::default();
        for xp in 0..200_000u64 {
            s.train(xp as usize % SKILLS.len(), 99, xp);
            assert!(s.total() <= TOTAL_CAP, "total {} exceeded cap {}", s.total(), TOTAL_CAP);
        }
        assert!(s.total() >= TOTAL_CAP - 6, "200k forced uses should have reached the cap");
        for xp in 200_000..200_100u64 {
            assert_eq!(s.train(xp as usize % SKILLS.len(), 99, xp), None, "at cap, no gain");
        }
    }

    /// Every rank word, over the whole 0..=1000 range, carries no digit.
    #[test]
    fn rank_words_carry_no_digits() {
        let mut s = Skills::default();
        for v in (0..=1000u16).step_by(50) {
            s.value[0] = v;
            let word = s.word(0);
            assert!(!word.chars().any(|c| c.is_ascii_digit()), "{word} has a digit");
        }
    }

    /// The tuning proof: three masteries at ~4 qualifying uses/minute land
    /// in the 800-1200 hour band.
    #[test]
    fn expected_uses_lands_in_the_1000_hour_band() {
        let one = expected_uses_to_master();
        let three = one * 3;
        let hours = three / (4 * 60);
        assert!(
            (800..=1200).contains(&hours),
            "measured: one mastery = {one} uses, three = {three} uses, \
             {hours} hours at 4 uses/min — expected inside 800-1200"
        );
    }
}
