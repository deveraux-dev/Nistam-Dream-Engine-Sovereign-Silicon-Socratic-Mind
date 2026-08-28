//! Magic — a school is an UMWELT, not a spell list.
//!
//! `body.is.the.sensor` (v2 `forge-vix/rules/umwelt-sense.rules.md`, von
//! Uexküll 1934): what a body can perceive IS what it can reach. So a school
//! is one faculty of the world, and casting is paid for in perception — there
//! is no pool, and `forge-cart-v3/src/lint.rs` errors on "mana".

pub mod loadout;
pub mod umwelt;

use crate::casting::GLYPH_WORDS;
use crate::hermetics::{ConnectionRoll, Principle};
use crate::magic_words::{school_of, words_of, School, WORDS_PER_SCHOOL};
use crate::itemforge::pressure_vector;
use crate::operator::{seed_hash, Operator};
use crate::overlay::Ledger;
use umwelt::{Form, Senses, Upbringing, AUTHORED_Q};

/// The one faculty a school reads the world through. Each school gets exactly
/// one — that is the whole constraint, and it is why no school is general.
///
/// Named apart from [`crate::casting::Channel`], which is the cast-bar state
/// machine (a word being spoken glyph by glyph). Different thing entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Faculty {
    /// Mirror / Mentalism — what reasoning fills in behind the eye.
    Thought,
    /// Map / Correspondence — the lattice address itself, as above so below.
    Address,
    /// Bell / Vibration — frequency, and what it pierces.
    Pitch,
    /// Edge / Polarity — difference across a boundary.
    Gradient,
    /// Tide / Rhythm — the t axis, and where in its swing you are.
    Phase,
    /// Ledger / Cause and Effect — accumulated debt, scars, what is owed.
    Debt,
    /// River / Gender — flow, and the fusing of active with passive.
    Flow,
}

impl Faculty {
    /// The lane of [`Senses`] this faculty is read through. A school whose lane
    /// is dead reads nothing, which is the point.
    pub const fn lane(self, s: &Senses) -> i64 {
        match self {
            Self::Thought => s.logic_q,
            Self::Address => s.sightline_q,
            Self::Pitch => s.attunement_q,
            Self::Gradient => s.shadowed_q,
            Self::Phase => s.wisdom_q,
            Self::Debt => s.upbringing_q,
            Self::Flow => s.dulled_q,
        }
    }

    /// The spoken name of what this faculty carries.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Thought => "thought",
            Self::Address => "address",
            Self::Pitch => "pitch",
            Self::Gradient => "gradient",
            Self::Phase => "phase",
            Self::Debt => "debt",
            Self::Flow => "flow",
        }
    }
}

/// The faculty a school reads. Bound to the school's hermetic principle, not
/// invented beside it — `School::principle()` is the authority.
pub const fn faculty_of(school: School) -> Faculty {
    match school {
        School::Mirror => Faculty::Thought,
        School::Map => Faculty::Address,
        School::Bell => Faculty::Pitch,
        School::Edge => Faculty::Gradient,
        School::Tide => Faculty::Phase,
        School::Ledger => Faculty::Debt,
        School::River => Faculty::Flow,
    }
}

/// The school's WAR word — its row in [`GLYPH_WORDS`], which `casting.rs`
/// already annotates with the same SEVENFOLD register and principle. This is
/// the join between the two word systems: five SUNG words per school here,
/// one SPOKEN glyph word there, same seven schools.
pub const fn war_word_index(school: School) -> usize {
    match school {
        School::Edge => 0,    // CLASH — Vigor, Mars, Polarity
        School::Map => 1,     // SHADOW — ShadowWeight, Saturn, Correspondence
        School::Mirror => 2,  // THOUGHT — LogicDepth, Mercury, Mentalism
        School::Tide => 3,    // CYCLE — Momentum, Luna, Rhythm
        School::River => 4,   // BALANCE — Tarnish, Venus, Gender
        School::Bell => 5,    // RESONANCE — Resonance, Sol, Vibration
        School::Ledger => 6,  // CONSEQUENCE — Guilt, Jupiter, CauseEffect
    }
}

/// The school's war word itself.
pub fn war_word(school: School) -> &'static str {
    GLYPH_WORDS[war_word_index(school)].0
}

/// Lowest and highest note the canon can hand back IN A GIVEN KEY. Read off the
/// key's own span rather than a frozen literal, so an operator born under any
/// star still pays the floor for their lowest word and the room for their highest.
fn note_bounds(key: forge_harmonics::CamelotKey) -> (u8, u8) {
    let span = key.pentatonic_span_7(0);
    (span[0], span[6])
}

/// One sung word, resolved: what it is, what it cost, and what you can still
/// hear afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cast {
    /// The canonical word sung.
    pub word: &'static str,
    /// The school that teaches it.
    pub school: School,
    /// The faculty that school reads.
    pub faculty: Faculty,
    /// The pentatonic note the word sings at (`forge_harmonics::word_note`).
    pub note: u8,
    /// What the singing cost, permyriad of total muting.
    pub cost_q: i64,
    /// The caster's senses AFTER paying — the cast is loud, and loud is deaf.
    pub after: Senses,
    /// How far the caster reaches through the school's own faculty, permyriad.
    pub reach_q: i64,
}

/// What opening your mouth costs at all, permyriad, before pitch is counted.
/// Without this the lowest note in the canon would be a free cast, and a free
/// cast is a pool by another name.
pub const SUNG_FLOOR_Q: i64 = 1_000;

/// What singing this note costs, permyriad.
///
/// THE PRICE OF MAGIC: the higher you sing the louder you are, so the less you
/// hear — and, by the symmetry [`umwelt::noticed_q`] already enforces, the
/// harder you are to find. There is no pool to drain; you pay in the room.
pub fn cost_of_note(note: u8, key: forge_harmonics::CamelotKey) -> i64 {
    let (lo, hi) = note_bounds(key);
    let span = (hi - lo) as i64;
    if span <= 0 {
        return SUNG_FLOOR_Q;
    }
    let over = (note.max(lo) - lo) as i64;
    let pitched = over * (AUTHORED_Q - SUNG_FLOOR_Q) / span;
    (SUNG_FLOOR_Q + pitched).clamp(SUNG_FLOOR_Q, AUTHORED_Q)
}

/// The school that IS a hermetic principle — `School::principle()`'s inverse,
/// total over both sevens.
pub const fn school_of_principle(p: Principle) -> School {
    match p {
        Principle::Mentalism => School::Mirror,
        Principle::Correspondence => School::Map,
        Principle::Vibration => School::Bell,
        Principle::Polarity => School::Edge,
        Principle::Rhythm => School::Tide,
        Principle::CauseEffect => School::Ledger,
        Principle::Gender => School::River,
    }
}

/// The school the operator was born into — the birth discipline's principle,
/// never a second derivation.
pub fn birth_school(op: &Operator) -> School {
    school_of_principle(op.birth_discipline().principle)
}

/// The words this operator has been taught. Acquisition law: today's set is
/// the birth school's five, Gifted at the door; wider channels come with trade.
pub fn known_words(op: &Operator) -> [&'static str; WORDS_PER_SCHOOL] {
    words_of(birth_school(op))
}

/// The art a school's trade rides — its SEVENFOLD register's own index.
pub fn art_of(school: School) -> usize {
    school.stat().index()
}

/// The operator's trade tier IN their birth school's art.
pub fn school_tier(op: &Operator) -> crate::skills::TradeTier {
    op.skills.tier(art_of(birth_school(op)))
}

/// Whether a word has been taught to this operator: it must be the birth
/// school's, AND the trade must have opened its rung — the door gifts one
/// word; apprentice a second; a master holds all five.
pub fn knows(op: &Operator, word: &str) -> bool {
    let words = known_words(op);
    let open = school_tier(op).words_open();
    words.iter().take(open).any(|&w| w == word)
}

/// Where this body was raised — dealt from birth identity, never stored.
pub fn birth_upbringing(op: &Operator) -> Upbringing {
    const ALL: [Upbringing; 4] =
        [Upbringing::Hearthborn, Upbringing::Fieldborn, Upbringing::Roadborn, Upbringing::Cellarborn];
    let h = seed_hash(&[op.name.as_bytes(), &[op.moon, op.day], b"upbringing"]);
    ALL[(h % 4) as usize]
}

/// The room settles this much carried noise per command tick.
pub const SETTLE_Q: i64 = 250;

/// The operator's senses RIGHT NOW — derived whole, never stored: dealt
/// registers, birth upbringing, the room's sightline, the worn form, then the
/// carried noise ATOP the form's own.
/// Fold the five fae acquisition pressures into one HEAR-face suppressor.
///
/// Evenly weighted, one fifth each — `umwelt.rs:19-21`'s own law, that no
/// single register buys the whole parish, applied to the acquisition side.
/// Only NET pressure mutes: gifting and refusing run their lanes negative
/// (`fae_ethics.rs:41,43`), and that buys clarity back toward zero rather than
/// sharpening hearing past a clean bond. What you take, you stop hearing over.
pub fn pressure_muting_q(lanes: &[i64; 5]) -> i64 {
    (lanes.iter().sum::<i64>() / lanes.len() as i64).clamp(0, AUTHORED_Q)
}

/// The operator's senses RIGHT NOW — derived whole, never stored: dealt
/// registers, birth upbringing, the room's sightline, the worn form, then the
/// carried noise ATOP the form's own, and atop THAT the standing fae pressure
/// the ledger holds. The acquisition law reaches perception here: how a thing
/// was come by is paid for in the same currency singing is.
pub fn senses_now(op: &Operator, sightline_q: i64, ledger: &Ledger) -> Senses {
    let stats = ConnectionRoll::deal(op.node_seed).stats;
    let wisdom_q = op.skills.value[2] as i64 * 10;
    let base = Senses::of(&stats, birth_upbringing(op), sightline_q, wisdom_q, 0);
    let worn = Form::from_u8(op.form).unwrap_or_default().wear(base);
    let pressure = pressure_muting_q(&pressure_vector(ledger, op.node_seed));
    let carried = (worn.muted_q + op.muted_q as i64 + pressure).min(AUTHORED_Q);
    worn.muted_by(carried)
}

/// The cost, spoken — no digits (the `hazard_words` pattern, abyss.rs).
pub fn cost_words(cost_q: i64) -> &'static str {
    match cost_q {
        q if q >= 9_000 => "the word takes the whole room with it — you are singing into a silence of your own making",
        q if q >= 6_500 => "the singing swallows every other sound; the world thins to your own voice",
        q if q >= 4_000 => "the word rings loud enough that the room's small sounds drop away",
        q if q >= 2_000 => "your voice covers the room's murmur while the word lasts",
        _ => "the word leaves your lips soft, and the room barely stirs",
    }
}

/// Below this much wisdom, a caster still believes (Sean 2026-08-24: the
/// first cast is Santa Claus — you believe until a certain time; some don't).
pub const BELIEF_BREAKS_Q: i64 = 2_500;

/// Whether this body still believes its own casts. Wisdom is the thief:
/// an untrained Craft never stops believing.
pub fn believes(senses: &Senses) -> bool {
    senses.wisdom_q < BELIEF_BREAKS_Q
}

/// The reach as the caster EXPERIENCES it: a believer is told the room
/// answered, whatever actually came back; a knower hears the truth.
pub fn felt_reach_words(reach_q: i64, senses: &Senses) -> &'static str {
    if believes(senses) {
        "and you feel it take — the room answered, you are sure of it"
    } else {
        reach_words(reach_q)
    }
}

/// The reach, spoken — no digits.
pub fn reach_words(reach_q: i64) -> &'static str {
    match reach_q {
        q if q <= 0 => "but nothing of it reaches — the lane it needs is dark in you",
        q if q < 1_500 => "and the barest thread of it goes out",
        q if q < 3_500 => "and a measure of it reaches into the room",
        q if q < 6_000 => "and it moves through the room like a current given a bed",
        _ => "and the room answers it whole, to the walls and past them",
    }
}

/// The carried noise, spoken — the `look` line while the room settles.
pub fn muted_words(muted_q: i64) -> &'static str {
    match muted_q {
        q if q >= 9_000 => "your own voice still fills your ears; the room is a picture with the sound gone",
        q if q >= 6_000 => "the singing still rings in you — the room's sounds arrive late and thin",
        q if q >= 3_000 => "an after-hum of the word sits on your hearing",
        q if q >= 1 => "the last of your own noise is settling out of the air",
        _ => "",
    }
}

/// Sing a word. `None` when no school teaches it — the Rosetta chain still
/// sings arbitrary bytes (`cdk::word_world_line`), but an untaught word is not
/// a cast.
///
/// The caster's own noise ADDS to whatever they were already carrying, so
/// casting twice without letting the room settle costs more the second time.
pub fn sing(word: &str, senses: Senses, key: forge_harmonics::CamelotKey) -> Option<Cast> {
    let school = school_of(word)?;
    let canon = crate::magic_words::MAGIC_WORDS
        .iter()
        .find(|&&(w, _)| w == word)
        .map(|&(w, _)| w)?;
    let note = forge_harmonics::word_note_in_key(word.as_bytes(), key);
    let cost_q = cost_of_note(note, key);
    let after = senses.muted_by((senses.muted_q + cost_q).min(AUTHORED_Q));
    let faculty = faculty_of(school);
    // Reach is the school's own lane, scaled by what the caster can still hear
    // once the singing is paid for. A deaf caster reaches nothing.
    let reach_q = faculty.lane(&after) * after.resolve_heard() / AUTHORED_Q;
    Some(Cast { word: canon, school, faculty, note, cost_q, after, reach_q })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magic_words::{words_of, MAGIC_WORDS};
    use umwelt::{Form, Upbringing};

    fn trained() -> Senses {
        Senses {
            logic_q: 7_000,
            wisdom_q: 6_000,
            upbringing_q: 5_000,
            sightline_q: 7_500,
            attunement_q: 6_000,
            upbringing: Some(Upbringing::Hearthborn),
            ..Default::default()
        }
    }

    #[test]
    fn every_school_reads_exactly_one_faculty_and_no_two_share() {
        let mut seen = std::collections::BTreeSet::new();
        for s in School::ALL {
            assert!(seen.insert(faculty_of(s).as_str()), "{} shares a faculty", s.as_str());
        }
        assert_eq!(seen.len(), School::ALL.len(), "seven schools, seven faculties");
    }

    /// The join: every school owns exactly one war word, and no two share.
    #[test]
    fn every_school_owns_one_war_word() {
        let mut seen = std::collections::BTreeSet::new();
        for s in School::ALL {
            assert!(seen.insert(war_word_index(s)), "{} shares a war word", s.as_str());
            assert!(war_word_index(s) < GLYPH_WORDS.len());
        }
        assert_eq!(seen.len(), GLYPH_WORDS.len(), "seven schools, seven glyph words");
        assert_eq!(war_word(School::Bell), "RESONANCE");
        assert_eq!(war_word(School::Mirror), "THOUGHT");
    }

    /// The key an operator born under Sirius' relative major sings in — the
    /// canon's old frozen register, now just one key among twenty-four.
    const K: forge_harmonics::CamelotKey = forge_harmonics::CamelotKey::DEFAULT_8B;

    /// Every Camelot key on the wheel.
    fn all_keys() -> Vec<forge_harmonics::CamelotKey> {
        (1..=12u8)
            .flat_map(|n| {
                [
                    forge_harmonics::CamelotKey::new(n, true),
                    forge_harmonics::CamelotKey::new(n, false),
                ]
            })
            .collect()
    }

    #[test]
    fn every_canon_word_sings_and_lands_in_its_own_school() {
        for &(word, school) in MAGIC_WORDS.iter() {
            let cast = sing(word, trained(), K).unwrap_or_else(|| panic!("{word} did not sing"));
            assert_eq!(cast.school, school, "{word} landed in the wrong school");
            assert_eq!(cast.faculty, faculty_of(school));
            assert!(
                K.pentatonic_span_7(0).contains(&cast.note),
                "{word} sang off the scale at {}",
                cast.note
            );
        }
    }

    /// The canon sings in every key an operator can be born into, never off-scale.
    #[test]
    fn every_canon_word_sings_in_every_key() {
        for key in all_keys() {
            let span = key.pentatonic_span_7(0);
            for &(word, _) in MAGIC_WORDS.iter() {
                let cast = sing(word, trained(), key).expect("canon");
                assert!(
                    span.contains(&cast.note),
                    "{word} sang {} outside {}{}",
                    cast.note,
                    key.number,
                    if key.is_minor { "A" } else { "B" }
                );
                assert!(cast.cost_q >= SUNG_FLOOR_Q, "{word} sang free in {}", key.number);
            }
        }
    }

    /// No word in the canon is free — the floor is what stops a low-hashing
    /// word from being a pool by another name.
    #[test]
    fn no_canon_word_is_free_to_sing() {
        for &(word, _) in MAGIC_WORDS.iter() {
            let cast = sing(word, trained(), K).expect("canon");
            assert!(cast.cost_q >= SUNG_FLOOR_Q, "{word} sang free at note {}", cast.note);
        }
    }

    #[test]
    fn an_untaught_word_is_not_a_cast() {
        assert!(sing("mana", trained(), K).is_none(), "no school teaches it");
        assert!(sing("", trained(), K).is_none());
        assert!(sing("thorn", trained(), K).is_some(), "but the canon does sing");
    }

    /// The price: singing costs hearing. No pool, no bar — the room is the bill.
    #[test]
    fn singing_costs_you_the_room() {
        let before = trained();
        let cast = sing("gold", before, K).expect("canon");
        assert!(cast.cost_q > 0, "a sung word must cost something");
        assert!(
            cast.after.resolve_heard() < before.resolve_heard(),
            "casting must cost hearing"
        );
        assert_eq!(
            cast.after.resolve(),
            before.resolve(),
            "but it must not blind — muting is the HEAR face alone"
        );
    }

    #[test]
    fn casting_twice_without_settling_costs_more() {
        let first = sing("bell", trained(), K).expect("canon");
        let second = sing("bell", first.after, K).expect("canon");
        assert!(second.after.muted_q > first.after.muted_q, "noise accumulates");
        assert!(second.reach_q <= first.reach_q, "and reach falls with it");
    }

    #[test]
    fn the_cost_rises_with_the_note() {
        for key in all_keys() {
            let (lo, hi) = note_bounds(key);
            assert_eq!(cost_of_note(lo, key), SUNG_FLOOR_Q, "no word is free to sing");
            assert_eq!(
                cost_of_note(hi, key),
                AUTHORED_Q,
                "the highest costs the whole room"
            );
            let mut last = -1;
            for n in key.pentatonic_span_7(0) {
                let c = cost_of_note(n, key);
                assert!(c > last, "cost must rise with pitch: {n} gave {c}");
                last = c;
            }
        }
    }

    /// The 8B numbers are bit-exact to the frozen-literal era — keying the
    /// canon re-priced nobody who was already singing in C.
    #[test]
    fn the_c_major_prices_did_not_move() {
        let expected = [1000, 2285, 3571, 5500, 6785, 8714, 10000];
        for (deg, n) in K.pentatonic_span_7(0).into_iter().enumerate() {
            assert_eq!(cost_of_note(n, K), expected[deg], "degree {deg} re-priced");
        }
    }

    /// A body that gave up a lane cannot reach through the school that reads
    /// it. Structural, not checked — the skeleton has no thought left to spend.
    #[test]
    fn a_body_that_lost_the_lane_cannot_reach_through_it() {
        let bone = Form::Skeleton.wear(trained());
        assert_eq!(bone.logic_q, 0, "the skeleton gave up thought");
        let mirror_word = words_of(School::Mirror)[0];
        let cast = sing(mirror_word, bone, K).expect("the word still sings");
        assert_eq!(cast.reach_q, 0, "but nothing of it reaches");

        let alive = sing(mirror_word, trained(), K).expect("canon");
        assert!(alive.reach_q > 0, "a mortal still thinks");
    }

    #[test]
    fn the_same_word_in_the_same_body_casts_the_same_forever() {
        let a = sing("river", trained(), K).expect("canon");
        let b = sing("river", trained(), K).expect("canon");
        assert_eq!(a, b, "same word, same body, same cast");
    }

    /// Birth gifts one school and its FIRST word; the trade opens the rest,
    /// rung by rung, until a master holds all five. Another school's words
    /// stay real-but-untaught at every rung.
    #[test]
    fn the_birth_school_gifts_five_words_and_they_sing() {
        let mut op = Operator::birth("Selos", 3, 12).unwrap();
        let art = art_of(birth_school(&op));
        op.skills.value[art] = 0;
        let words = known_words(&op);
        assert!(knows(&op, words[0]), "the door gifts the first word");
        assert!(!knows(&op, words[1]), "the second waits on the trade");
        let natal = op.natal_key();
        let cast = sing(words[0], senses_now(&op, 5_500, &Ledger::default()), natal)
            .expect("the gifted word sings");
        assert!(cast.cost_q >= SUNG_FLOOR_Q, "no gift sings free");
        assert!(
            natal.pentatonic_span_7(0).contains(&cast.note),
            "the gifted word sings in the operator's own natal key"
        );

        op.skills.value[art] = 1_000;
        for w in words {
            assert!(knows(&op, w), "{w} opens at mastery");
            assert!(
                sing(w, senses_now(&op, 5_500, &Ledger::default()), natal).is_some(),
                "{w} sings"
            );
        }
        let other = School::ALL
            .into_iter()
            .find(|s| *s != birth_school(&op))
            .expect("seven schools");
        assert!(!knows(&op, words_of(other)[0]), "an ungifted school's word is untaught");
    }

    /// The trade's ladder is law: rights and efficiency per rung, and the
    /// master's hand never sings for free.
    #[test]
    fn the_trade_tier_carries_rights_and_a_faster_hand() {
        use crate::skills::TradeTier;
        assert!(!TradeTier::Apprentice.may_charge(), "an apprentice works under someone");
        assert!(TradeTier::Journeyman.may_charge(), "a journeyman charges for the work");
        assert!(!TradeTier::Journeyman.may_teach());
        assert!(TradeTier::Master.may_teach(), "a master takes apprentices");
        assert!(!TradeTier::Master.may_name());
        assert!(TradeTier::Grandmaster.may_name(), "only a grandmaster names new work");
        let mut last = 0;
        for v in [0u16, 150, 300, 450, 600, 750, 900] {
            let t = TradeTier::of(v);
            assert!(t.efficiency_q() <= TradeTier::of(last).efficiency_q(), "the hand only quickens");
            assert!(t.words_open() >= TradeTier::of(last).words_open(), "the trade only opens");
            last = v;
        }
        let full = 10_000i64;
        let master_pays = full * TradeTier::Master.efficiency_q() / 10_000;
        assert!(master_pays.max(SUNG_FLOOR_Q) >= SUNG_FLOOR_Q, "even mastery pays the floor");
        assert!(master_pays < full, "the same wall, fewer strokes");
    }

    /// Senses derive whole from the operator; carried noise mutes the HEAR
    /// face only, and it rides the v5 save codec byte-exact.
    #[test]
    fn senses_derive_and_carried_noise_rides_the_save() {
        let mut op = Operator::birth("Selos", 3, 12).unwrap();
        let quiet = senses_now(&op, 5_500, &Ledger::default());
        op.muted_q = 4_000;
        op.form = Form::Lich.as_u8();
        let loud = senses_now(&op, 5_500, &Ledger::default());
        assert!(loud.resolve_heard() < senses_now_heard_baseline(&op), "noise must mute");
        assert_eq!(loud.resolve(), senses_now(&op, 5_500, &Ledger::default()).resolve(), "but never blind");
        let _ = quiet;
        let back = Operator::decode(&op.encode()).expect("v5 roundtrip");
        assert_eq!(back, op, "form and noise survive the codec");
    }

    // A clean ledger must reproduce the senses exactly as they were before the
    // fold existed. The fae only ever ADD to what you carry; they never hand a
    // never-acquiring player sharper hearing than the form and the noise give.
    #[test]
    fn an_empty_ledger_changes_nothing_about_the_senses() {
        let op = Operator::birth("Selos", 3, 12).unwrap();
        let with = senses_now(&op, 5_500, &Ledger::default());
        let stats = ConnectionRoll::deal(op.node_seed).stats;
        let wisdom_q = op.skills.value[2] as i64 * 10;
        let base = Senses::of(&stats, birth_upbringing(&op), 5_500, wisdom_q, 0);
        let worn = Form::from_u8(op.form).unwrap_or_default().wear(base);
        let expected = worn.muted_by((worn.muted_q + op.muted_q as i64).min(AUTHORED_Q));
        assert_eq!(with.muted_q, expected.muted_q, "a clean ledger must mute nothing extra");
        assert_eq!(with.resolve(), expected.resolve());
    }

    // The acquisition law reaching perception: what you take, you stop hearing
    // over. Stealing must cost the same currency singing does.
    #[test]
    fn stealing_mutes_the_room_and_refusing_buys_it_back() {
        use crate::itemforge::apply_fae_pressure;
        use forge_reactions_v3::fae_ethics::FaeItemOutcome;

        let op = Operator::birth("Selos", 3, 12).unwrap();
        let clean = senses_now(&op, 5_500, &Ledger::default());

        let mut thief = Ledger::default();
        for _ in 0..4 {
            apply_fae_pressure(&mut thief, FaeItemOutcome::Stolen);
        }
        let stolen = senses_now(&op, 5_500, &thief);
        assert!(
            stolen.muted_q > clean.muted_q,
            "four thefts must be carried: {} vs {}",
            stolen.muted_q,
            clean.muted_q
        );
        assert!(stolen.resolve_heard() < clean.resolve_heard(), "and it must cost hearing");
        assert!(stolen.resolve() > 0, "but pressure never blinds outright");

        let mut penitent = thief.clone();
        for _ in 0..4 {
            apply_fae_pressure(&mut penitent, FaeItemOutcome::Refused);
        }
        let given_back = senses_now(&op, 5_500, &penitent);
        assert!(
            given_back.muted_q < stolen.muted_q,
            "refusing must walk it back: {} vs {}",
            given_back.muted_q,
            stolen.muted_q
        );

        // The line the player actually reads on `look`, at each standing.
        for (what, s) in [("clean", &clean), ("four thefts", &stolen), ("made good", &given_back)] {
            eprintln!("phasec/fold {what:<12} muted_q {:>6}  \"{}\"", s.muted_q, muted_words(s.muted_q));
        }
    }

    // Evenly weighted, one fifth each — no single lane buys the whole parish
    // (umwelt.rs:19-21's own law, applied to the acquisition side).
    #[test]
    fn no_single_pressure_lane_carries_the_whole_fold() {
        let full = [AUTHORED_Q; 5];
        assert_eq!(pressure_muting_q(&full), AUTHORED_Q, "all five maxed is a full mute");

        let one = [AUTHORED_Q, 0, 0, 0, 0];
        assert_eq!(pressure_muting_q(&one), AUTHORED_Q / 5, "one lane is worth one fifth");

        // Net-negative pressure buys clarity back to zero, never past it.
        assert_eq!(pressure_muting_q(&[-AUTHORED_Q; 5]), 0, "a clean bond mutes nothing");
        assert_eq!(pressure_muting_q(&[0; 5]), 0);
    }

    fn senses_now_heard_baseline(op: &Operator) -> i64 {
        let mut clean = op.clone();
        clean.muted_q = 0;
        senses_now(&clean, 5_500, &Ledger::default()).resolve_heard()
    }

    /// A v4 save (no magic bytes) decodes Mortal and silent; a corrupt form
    /// byte refuses whole.
    #[test]
    fn a_v4_save_decodes_mortal_and_silent_and_corrupt_forms_refuse() {
        let op = Operator::birth("Selos", 3, 12).unwrap();
        let mut v4 = op.encode();
        let n = v4.len();
        v4.truncate(n - 3);
        v4[4..8].copy_from_slice(&forge_core_v3::sprite_blob::u32_to_nistam(4));
        let back = Operator::decode(&v4).expect("a v4 save still opens");
        assert_eq!(back.form, 0, "no body was saved, so Mortal");
        assert_eq!(back.muted_q, 0, "no noise was saved, so silent");

        let mut corrupt = op.encode();
        let last = corrupt.len() - 3;
        corrupt[last] = umwelt::FORM_COUNT;
        assert!(Operator::decode(&corrupt).is_none(), "an unworn form refuses whole");
    }

    /// Santa Claus: a fresh body believes its casts whatever came back; a
    /// wisdom-trained one hears the truth; an untrained one never stops.
    #[test]
    fn the_first_cast_is_believed_and_wisdom_is_the_thief() {
        let op = Operator::birth("Selos", 3, 12).unwrap();
        let fresh = senses_now(&op, 5_500, &Ledger::default());
        assert!(believes(&fresh), "a fresh body believes");
        assert_eq!(
            felt_reach_words(0, &fresh),
            "and you feel it take — the room answered, you are sure of it",
            "zero reach and the believer is still told it took"
        );

        let mut knower = op.clone();
        knower.skills.value[2] = 300;
        let wise = senses_now(&knower, 5_500, &Ledger::default());
        assert!(!believes(&wise), "trained wisdom breaks the spell");
        assert_eq!(felt_reach_words(0, &wise), reach_words(0), "the knower hears the truth");

        let mut old_believer = op.clone();
        old_believer.xp = 1_000_000;
        assert!(
            believes(&senses_now(&old_believer, 5_500, &Ledger::default())),
            "age alone never breaks it — some don't"
        );
    }

    /// The spoken ladders carry no digits — the surface-word law.
    #[test]
    fn worded_ladders_hold_no_digits() {
        for q in [0i64, 1, 1_500, 3_000, 5_000, 8_000, 10_000] {
            for s in [cost_words(q), reach_words(q), muted_words(q)] {
                assert!(!s.chars().any(|c| c.is_ascii_digit()), "digits leaked: {s}");
            }
        }
    }
}
