//! Room text woven, not generated — ported verbatim 2026-08-26 from v2
//! sf-wasm/src/umwelt_loom.rs (adaptations: Form/Senses = crate::magic::umwelt,
//! Cell = 5 raw lanes, t_tolerance ladder inlined from v2 mud_sieve.rs:60).

use crate::magic::umwelt::{Form, Senses};

/// The full 5D address a weave reads: x, y, z, t, s — the same five lanes the
/// operator's `MortonKey5D` position carries.
pub type Cell5 = [i64; 5];

/// Deterministic slot pick off the full 5D address plus the body and the slot.
/// Same cell, same tick, same form, same words — a replay reads back
/// character for character.
#[inline]
pub fn slot_hash(at: Cell5, form: Form, slot: u8) -> usize {
    let mut h = (at[0] as u64)
        ^ ((at[1] as u64) << 13)
        ^ ((at[2] as u64) << 27)
        ^ ((at[3] as u64) << 39)
        ^ ((at[4] as u64) << 51);
    h ^= (form.as_u8() as u64) << 3;
    h ^= slot as u64;
    h = h.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    h ^= h >> 31;
    h as usize
}

/// How many `t` cells a reading stays true for, from its own tick rate. Fast
/// banks describe NOW and go stale immediately; slow banks describe a condition
/// and hold. (Donor: v2 mud_sieve::t_tolerance — the one staleness rule.)
pub fn t_tolerance(tick_frequency_hz: u32) -> i64 {
    match tick_frequency_hz {
        0 | 1 => 12,
        2..=10 => 8,
        11..=30 => 5,
        31..=60 => 3,
        _ => 1,
    }
}

/// Hash key for a cell address — unchanged from the old VoxelState::field builder.
pub fn cell_key(at: Cell5) -> u64 {
    at.iter().fold(0u64, |h, &v| (h ^ v as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
}

/// Quiet cell field at a 5D address — ready for index-writing channels.
pub fn quiet_cell_field(at: Cell5) -> PentaractField {
    let key = cell_key(at);
    let point = forge_core_v3::pentaract::Pentaract::new(key, 0, 0, 0, 0, 0, 0);
    PentaractField::quiet_at(point)
}

/// The channels a body is built to hear, and the one it stands in.
pub use forge_core_v3::pentaract_field::{
    mood_point, PentaractField, SenseChannel, SenseGain, SenseMask, SENSE_COUNT,
};

/// Which channels a worn body hears, and how loudly. Replaces the per-form
/// match arm: a new body is a new mask, not a new branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormFilter {
    /// The channels this body is built for.
    pub mask: SenseMask,
    /// Permyriad gain per channel. Integer by law — float would make the room
    /// depend on FMA order and break the same-bytes-everywhere contract.
    pub gain: SenseGain,
}

impl FormFilter {
    /// A body that hears exactly these channels, each at 1.0x.
    pub const fn unity(mask: SenseMask) -> Self {
        Self { mask, gain: SenseGain::UNITY }
    }

    /// What this body hears on a channel: nothing at all when it is deaf to it.
    pub fn read(&self, f: &PentaractField, c: SenseChannel) -> Option<i32> {
        f.read_masked(self.mask, c).map(|raw| self.gain.apply(c, raw))
    }

    /// The S⁴ point a body's own nature sits at: the shape of the channels it
    /// was born with, read on the same scale a cell's shape is.
    pub fn mood_of(form: Form) -> forge_core_v3::pentaract::Pentaract {
        mood_point(form.as_u8() as u64, Self::of(form).mask.band_share())
    }

    /// The filter a worn body brings to THIS cell. The mask is the body's
    /// birth channels; the gain is one number off how close the body's own
    /// shape sits to the cell's — a room shaped like you is a room you hear.
    pub fn attuned(form: Form, field: &PentaractField) -> Self {
        let mask = Self::of(form).mask;
        Self { mask, gain: SenseGain::attuned(Self::mood_of(form).cos_similarity(&field.at)) }
    }

    /// The filter the worn body was born with.
    pub const fn of(form: Form) -> Self {
        Self::unity(match form {
            Form::Djinn => SenseMask::of(SenseChannel::AtmospherePa),
            Form::Lich => SenseMask::of(SenseChannel::NecroticDecay),
            Form::Skeleton | Form::Treant => SenseMask::of(SenseChannel::MasonryStress),
            Form::Werewolf => SenseMask::of(SenseChannel::ScentAge),
            // The one body allowed to be wrong reaches for all three at once.
            Form::Wraith => SenseMask::of(SenseChannel::NecroticDecay)
                .with(SenseChannel::AtmospherePa)
                .with(SenseChannel::ScentAge),
            Form::Mortal => SenseMask::of(SenseChannel::LuxZero),
        })
    }
}

/// Everything above the naming threshold reads hot; below it, the body knows
/// something is there and does not get to say what.
const HOT_Q: i32 = 5_000;
const FAINT_Q: i32 = 1_500;

/// Your own noise, in the two places it starts to cost you.
const NOISE_DROWNING_Q: i64 = 4_000;
const NOISE_EDGE_Q: i64 = 1_000;

/// `trail_hz` is the tick rate of the bank that owns the passage channel —
/// the weaver never decides how fast a fact goes stale, it asks the lattice.
pub fn weave(form: Form, senses: &Senses, field: &PentaractField, scent_age_t: i64, at: Cell5, trail_hz: u32) -> String {
    let h = |slot: u8| slot_hash(at, form, slot);
    let filter = FormFilter::attuned(form, field);
    // The mask decides WHAT the body may read; the match decides what it says
    // about it. A deaf channel reads 0 here, not "skipped" — the empty branch
    // of each body's own prose is the honest report of nothing.
    let hear = |c: SenseChannel| filter.read(field, c).unwrap_or(0);
    let channel = match form {
        Form::Djinn => djinn(hear(SenseChannel::AtmospherePa), h(1)),
        Form::Lich => lich(hear(SenseChannel::NecroticDecay), h(1)),
        Form::Skeleton => skeleton(hear(SenseChannel::MasonryStress), h(1)),
        Form::Werewolf => werewolf(scent_age_t, trail_hz, h(1)),
        Form::Treant => treant(hear(SenseChannel::MasonryStress), h(1)),
        Form::Wraith => wraith(field, senses, scent_age_t, trail_hz, h(1)),
        Form::Mortal => mortal(senses, h(1)),
    };
    format!("{} {} {}", form.body_line(), channel, noise(senses, h(2)))
}

/// What the noise you carry is doing to the reading.
fn noise(senses: &Senses, seed: usize) -> &'static str {
    if senses.muted_q >= NOISE_DROWNING_Q {
        const DROWNED: [&str; 3] = [
            "Whatever else is in here is under your own racket.",
            "You are the loudest thing in the room and it is costing you the room.",
            "Everything subtle is behind the noise you brought with you.",
        ];
        DROWNED[seed % DROWNED.len()]
    } else if senses.muted_q >= NOISE_EDGE_Q {
        const EDGED: [&str; 2] = [
            "There is a hum off you that keeps getting in the way.",
            "You are not quite quiet, and it shows at the edges.",
        ];
        EDGED[seed % EDGED.len()]
    } else {
        const CLEAR: [&str; 2] = [
            "Nothing of yours is in the way of it.",
            "You are giving the room nothing back, which helps.",
        ];
        CLEAR[seed % CLEAR.len()]
    }
}

// ── One channel per body. No branch reaches outside its own sense. ───────────

fn djinn(pressure_q: i32, seed: usize) -> &'static str {
    if pressure_q >= HOT_Q {
        const OPEN: [&str; 3] = [
            "Something opened. The room is drawing through a gap that was shut a moment ago.",
            "There is a pull to the north, thin and steady, where the stone stops being stone.",
            "A seam is breathing. You are already partly on the other side of it.",
        ];
        OPEN[seed % OPEN.len()]
    } else if pressure_q >= FAINT_Q {
        const SOFT: [&str; 2] = [
            "The pressure leans, slightly, and will not say toward what.",
            "Something displaces you a little. Not enough to place it.",
        ];
        SOFT[seed % SOFT.len()]
    } else {
        "The air is holding the shape of the room and nothing is disturbing it."
    }
}

fn lich(decay_q: i32, seed: usize) -> &'static str {
    if decay_q >= HOT_Q {
        const BURNING: [&str; 3] = [
            "Something here is burning through what it has, fast, and does not know you are counting.",
            "A body is spending itself nearby. You can hear the rate.",
            "There is a warm ledger in this room running down toward nothing.",
        ];
        BURNING[seed % BURNING.len()]
    } else if decay_q >= FAINT_Q {
        const EMBER: [&str; 2] = [
            "Something is alive in here and barely. It has not registered you.",
            "A low burn, somewhere. Too thin to place, too steady to be nothing.",
        ];
        EMBER[seed % EMBER.len()]
    } else {
        "Nothing in here is spending anything. It is as quiet as you are."
    }
}

fn skeleton(load_q: i32, seed: usize) -> &'static str {
    if load_q >= HOT_Q {
        const HEAVY: [&str; 3] = [
            "Weight, ahead, standing. It shifts from one side to the other and back.",
            "Something heavy is bearing on the flags and has been for a while.",
            "The floor is carrying more than the floor.",
        ];
        HEAVY[seed % HEAVY.len()]
    } else if load_q >= FAINT_Q {
        const LIGHT: [&str; 2] = [
            "A small load somewhere. It does not move.",
            "Something rests on the stone. It is not enough to be a person.",
        ];
        LIGHT[seed % LIGHT.len()]
    } else {
        "The stone is holding nothing but itself."
    }
}

fn werewolf(scent_age_t: i64, trail_hz: u32, seed: usize) -> &'static str {
    let cold = t_tolerance(trail_hz);
    if scent_age_t * 4 < cold {
        const FRESH: [&str; 3] = [
            "It went through here just now and it was not being careful.",
            "The trail is still warm and it runs west, low, along the wall.",
            "Something crossed this floor and left the whole of itself in the air.",
        ];
        FRESH[seed % FRESH.len()]
    } else if scent_age_t < cold {
        const AGING: [&str; 2] = [
            "The trail is hours old. You can still follow it if you go now.",
            "Something came through. It has had time to get somewhere since.",
        ];
        AGING[seed % AGING.len()]
    } else {
        "The trail here is cold. Whatever it was, it is not worth the legs."
    }
}

fn treant(load_q: i32, seed: usize) -> &'static str {
    if load_q >= FAINT_Q {
        const LOADED: [&str; 3] = [
            "Three loads on ground you have grown through. Two of them have not moved in a long time.",
            "Something is standing on you and has not worked out that this is what it is doing.",
            "The pressure comes down through worked stone and you have already counted it.",
        ];
        LOADED[seed % LOADED.len()]
    } else {
        "Nothing is standing on anything of yours."
    }
}

fn wraith(field: &PentaractField, senses: &Senses, scent_age_t: i64, trail_hz: u32, seed: usize) -> &'static str {
    // Off the floor, sensitivity enormous, reliability gone: the wraith is the
    // one body whose channel is allowed to be wrong, and it never says which.
    let anything = field[SenseChannel::NecroticDecay] >= FAINT_Q
        || field[SenseChannel::AtmospherePa] >= FAINT_Q
        || scent_age_t < t_tolerance(trail_hz);
    if anything || senses.resolve() > 12_000 {
        const HEARD: [&str; 3] = [
            "Someone said something in here. It may not have been recently.",
            "There is a step behind you on a floor you are not standing on.",
            "Something is happening in this room and some of it already happened.",
        ];
        HEARD[seed % HEARD.len()]
    } else {
        "There is nothing here, and you do not trust that either."
    }
}

fn mortal(senses: &Senses, seed: usize) -> &'static str {
    if senses.resolve() >= 10_000 {
        const WIDE: [&str; 2] = [
            "The room is open to you and it carries a long way.",
            "You have the whole of this place, out to the walls and past them.",
        ];
        WIDE[seed % WIDE.len()]
    } else {
        const NARROW: [&str; 2] = [
            "It falls off fast past arm's reach and you are working with what is close.",
            "You have this much of the room and no more of it.",
        ];
        NARROW[seed % NARROW.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magic::umwelt::AUTHORED_Q;

    fn senses(muted: i64) -> Senses {
        Senses { attunement_q: 6_000, logic_q: 6_000, sightline_q: 5_000, ..Default::default() }
            .muted_by(muted)
    }

    /// v3 carries no sieve-registry manifest yet: 1 Hz — the slow-bank ladder
    /// rung the donor's `well_head_monitors` bank sat on.
    fn slow_bank_hz() -> u32 {
        1
    }

    fn hot_field() -> (PentaractField, i64) {
        let at = [0, 0, 0, 0, 0];
        let mut f = quiet_cell_field(at);
        f[SenseChannel::AtmospherePa] = 9_000;
        f[SenseChannel::NecroticDecay] = 9_000;
        f[SenseChannel::MasonryStress] = 9_000;
        f[SenseChannel::HeatGradient] = 0;
        f[SenseChannel::ParticulateFlux] = 0;
        f[SenseChannel::HateVector] = 0;
        f[SenseChannel::VitalityLux] = 0;
        (f.oriented(cell_key(at)), 1)
    }

    #[test]
    fn the_same_cell_reads_the_same_way_on_every_machine() {
        let at = [12, 34, 2, 120, 3];
        let (field, scent_age_t) = hot_field();
        let a = weave(Form::Lich, &senses(0), &field, scent_age_t, at, slow_bank_hz());
        let b = weave(Form::Lich, &senses(0), &field, scent_age_t, at, slow_bank_hz());
        assert_eq!(a, b);
    }

    #[test]
    fn the_lattice_is_what_makes_it_various() {
        let s = senses(0);
        let (field, scent_age_t) = hot_field();
        let base = [12, 34, 2, 0, 0];
        let mut seen = std::collections::HashSet::new();
        for t in 0..64 {
            seen.insert(weave(Form::Djinn, &s, &field, scent_age_t, [12, 34, 2, t, 0], slow_bank_hz()));
        }
        assert!(seen.len() > 1, "the tick axis must re-weave the room");

        let mut spatial = std::collections::HashSet::new();
        for x in 0..64 {
            spatial.insert(weave(Form::Djinn, &s, &field, scent_age_t, [x, 34, 2, 0, 0], slow_bank_hz()));
        }
        assert!(spatial.len() > 1, "and so must the ground");
        assert_ne!(
            weave(Form::Djinn, &s, &field, scent_age_t, base, slow_bank_hz()),
            weave(Form::Lich, &s, &field, scent_age_t, base, slow_bank_hz()),
            "two bodies in one cell do not file the same report"
        );
    }

    #[test]
    fn a_blind_body_has_no_words_for_light() {
        const FORBIDDEN: [&str; 6] = ["light", "dark", "shadow", "colour", "glow", " see "];
        let s = senses(0);

        let quiet_field = {
            let at = [0, 0, 0, 0, 0];
            quiet_cell_field(at).oriented(cell_key(at))
        };
        let faint_field = {
            let at = [0, 0, 0, 0, 0];
            let mut f = quiet_cell_field(at);
            f[SenseChannel::AtmospherePa] = 2_000;
            f[SenseChannel::NecroticDecay] = 2_000;
            f[SenseChannel::MasonryStress] = 2_000;
            f.oriented(cell_key(at))
        };
        let (hot_f, _) = hot_field();

        for form in [Form::Lich, Form::Skeleton, Form::Treant] {
            for (field, scent) in [(hot_f, 1i64), (quiet_field, 10_000i64), (faint_field, 6i64)] {
                for t in 0..32 {
                    let line =
                        weave(form, &s, &field, scent, [1, 2, 3, t, 0], slow_bank_hz()).to_lowercase();
                    for word in FORBIDDEN {
                        assert!(
                            !line.contains(word),
                            "{} reached for '{}': {line}",
                            form.name(),
                            word
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn an_empty_cell_is_reported_empty() {
        let at = [0, 0, 0, 0, 0];
        let quiet_f = quiet_cell_field(at).oriented(cell_key(at));
        let quiet_s = 10_000i64;
        assert!(weave(Form::Lich, &senses(0), &quiet_f, quiet_s, at, slow_bank_hz()).contains("spending"));
        assert!(weave(Form::Skeleton, &senses(0), &quiet_f, quiet_s, at, slow_bank_hz()).contains("holding nothing"));
        assert!(weave(Form::Werewolf, &senses(0), &quiet_f, quiet_s, at, slow_bank_hz()).contains("cold"));
        // Except the wraith, which does not get to be sure of an empty room.
        assert!(weave(Form::Wraith, &senses(0), &quiet_f, quiet_s, at, slow_bank_hz()).contains("do not trust"));
    }

    /// Every body's filter names the channel its own prose describes, and no
    /// body is born deaf. This is the match arm, now checkable as data.
    #[test]
    fn each_body_hears_its_own_channel_and_no_other() {
        let expect = [
            (Form::Djinn, SenseChannel::AtmospherePa),
            (Form::Lich, SenseChannel::NecroticDecay),
            (Form::Skeleton, SenseChannel::MasonryStress),
            (Form::Treant, SenseChannel::MasonryStress),
            (Form::Werewolf, SenseChannel::ScentAge),
            (Form::Mortal, SenseChannel::LuxZero),
        ];
        for (form, channel) in expect {
            let f = FormFilter::of(form);
            assert!(f.mask.listens_to(channel), "{} is deaf to its own sense", form.name());
            assert_eq!(f.mask.count(), 1, "{} should hear exactly one channel", form.name());
        }
        // The wraith is the exception the loom already documents: enormous
        // sensitivity, no reliability, three channels at once.
        let wraith = FormFilter::of(Form::Wraith);
        assert_eq!(wraith.mask.count(), 3);
        for c in [SenseChannel::NecroticDecay, SenseChannel::AtmospherePa, SenseChannel::ScentAge] {
            assert!(wraith.mask.listens_to(c));
        }
    }

    /// A body cannot read a channel it was not built for, whatever the cell holds.
    #[test]
    fn a_body_cannot_read_outside_its_own_sense() {
        let at = [1, 2, 3, 4, 5];
        let mut field = quiet_cell_field(at);
        field[SenseChannel::AtmospherePa] = 9_000;
        field[SenseChannel::NecroticDecay] = 9_000;
        field[SenseChannel::MasonryStress] = 9_000;
        let field = field.oriented(cell_key(at));
        let skeleton = FormFilter::of(Form::Skeleton);
        assert_eq!(skeleton.read(&field, SenseChannel::MasonryStress), Some(9_000));
        assert_eq!(
            skeleton.read(&field, SenseChannel::NecroticDecay),
            None,
            "the skeleton must not learn what the lich knows"
        );
        assert_eq!(skeleton.read(&field, SenseChannel::AtmospherePa), None);
    }

    /// Golden: every body, every field shape, eight ticks, folded to one hash.
    /// A refactor that changes any byte of any woven room fails right here.
    #[test]
    fn the_woven_rooms_are_byte_for_byte_what_they_were() {
        let s = senses(0);
        let quiet_field = {
            let at = [0, 0, 0, 0, 0];
            quiet_cell_field(at).oriented(cell_key(at))
        };
        let faint_field = {
            let at = [0, 0, 0, 0, 0];
            let mut f = quiet_cell_field(at);
            f[SenseChannel::AtmospherePa] = 2_000;
            f[SenseChannel::NecroticDecay] = 2_000;
            f[SenseChannel::MasonryStress] = 2_000;
            f.oriented(cell_key(at))
        };
        let (hot_field, _) = hot_field();
        let mut h: u64 = 0xCBF2_9CE4_8422_2325;
        for form in [
            Form::Djinn,
            Form::Lich,
            Form::Skeleton,
            Form::Werewolf,
            Form::Treant,
            Form::Wraith,
            Form::Mortal,
        ] {
            for (field, scent_age) in [(hot_field, 1i64), (faint_field, 6i64), (quiet_field, 10_000i64)] {
                for t in 0..8 {
                    let line = weave(form, &s, &field, scent_age, [3, 5, 1, t, 2], slow_bank_hz());
                    for b in line.bytes() {
                        h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3);
                    }
                }
            }
        }
        assert_eq!(
            h, 5_779_870_982_066_480_609,
            "a woven room changed — the loom's prose is a contract, not an implementation detail"
        );
    }

    /// A body's sensitivity is one number off the angle between its own shape
    /// and the cell's — nothing per-channel is authored anywhere.
    #[test]
    fn a_body_is_deafened_by_a_room_shaped_like_something_else() {
        let at = [7, 7, 0, 3, 0];
        let mut mine_f = quiet_cell_field(at);
        mine_f[SenseChannel::NecroticDecay] = 1_800;
        let mine = mine_f.oriented(cell_key(at));

        let mut foreign_f = quiet_cell_field(at);
        foreign_f[SenseChannel::NecroticDecay] = 1_800;
        foreign_f[SenseChannel::HeatGradient] = 18_000;
        let foreign = foreign_f.oriented(cell_key(at));

        let aligned = FormFilter::attuned(Form::Lich, &mine);
        let off = FormFilter::attuned(Form::Lich, &foreign);
        // Unity to within the BAM table's quantization, never past it.
        assert!(aligned.gain[SenseChannel::NecroticDecay] >= 9_990, "its own room is unity");
        assert!(
            off.gain[SenseChannel::NecroticDecay] < aligned.gain[SenseChannel::NecroticDecay],
            "a foreign-shaped room must cost the body gain: {} vs {}",
            off.gain[SenseChannel::NecroticDecay],
            aligned.gain[SenseChannel::NecroticDecay]
        );
        assert_eq!(aligned.mask, off.mask, "the mask is birth, only the gain moves");

        // And that cost reaches the prose: the same burn goes unnameable.
        let named = weave(Form::Lich, &senses(0), &mine, 10_000, at, slow_bank_hz());
        let lost = weave(Form::Lich, &senses(0), &foreign, 10_000, at, slow_bank_hz());
        assert!(!named.contains("spending anything"), "the aligned body must name it: {named}");
        assert!(lost.contains("spending anything"), "the deafened body must not: {lost}");
    }

    #[test]
    fn the_noise_you_carry_is_always_stated() {
        let at = [5, 5, 0, 0, 0];
        let (field, scent_age_t) = hot_field();
        let loud = weave(Form::Mortal, &senses(AUTHORED_Q), &field, scent_age_t, at, slow_bank_hz());
        assert!(
            loud.contains("racket") || loud.contains("loudest") || loud.contains("noise you brought"),
            "{loud}"
        );
        let quiet = weave(Form::Mortal, &senses(0), &field, scent_age_t, at, slow_bank_hz());
        assert!(quiet.contains("Nothing of yours") || quiet.contains("giving the room nothing"), "{quiet}");
    }
}
