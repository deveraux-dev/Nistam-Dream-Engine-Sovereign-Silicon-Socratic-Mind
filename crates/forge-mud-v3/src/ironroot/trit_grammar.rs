//! Trit-partitioned word banks — 3×3 grid (not 3×3×3), extending
//! [`super::dialogue`]'s `WordClass`/`WordBanks`/`fill_template` (real,
//! landed, unchanged) rather than a new dialogue engine (C06 revascularize).
//!
//! ## The grid
//!
//! Two free trit axes, not three:
//! - `Ta` (Adjective/tone) — sign of [`forge_core_v3::cdk::Triad::disposition`].
//! - `Tn` (Noun/register) — [`forge_core_v3::soul::cynatic_depth`] (lineage
//!   hop count to [`forge_core_v3::soul::SoulId::ROOT`]), quantized.
//!
//! `Tv` (Verb) is derived, not free: `Tv = balanced_add(Ta, Tn)`, the same
//! digit law `F:\NewRepo\crates\forge-calligraphy\src\cremantic.rs:176-184`
//! proves for its `code_to_balanced`/`balanced_to_code` pair (unported to
//! v3 — this is a fresh, small, independently-authored fn following the
//! same law, not an import, per F06 lateral-reach: cite the mechanism,
//! don't claim a port that doesn't exist). The "2 inverses + fixed point"
//! property that law's own `Mirror` lane proves (`cremantic.rs:44-55`) holds
//! exactly on the `Ta=0` row: `Tv(0,Tn)=Tn`, so `Tn=-1`/`Tn=+1` are mutual
//! inverses under sign-swap, `Tn=0` the fixed point. Off that row the table
//! is still total (a Latin square, ℤ/3's own group table) but the mutual-
//! inverse claim is not asserted more broadly than that (C09 aperture).
//!
//! ## Animacy gate (Sean 2026-08-16, real Cree grammar)
//!
//! Cree noun animacy is lexical, not biological: stones, celestial bodies,
//! and objects of narrative/spiritual weight are routinely grammatically
//! animate; ordinary inert objects are not (F06 lateral-reach — external
//! Algonquian-linguistics fact, not found anywhere in this repo, checked via
//! `grep -i "animate"` across `forge-calligraphy` and `forge-mud-v3`, no
//! hits). `SoulId` presence already encodes exactly this: a
//! [`forge_core_v3::soul::SoulIdentity`] is "who, under whose authority,
//! first seen when, descended from whom" — a legendary sword, a named ring,
//! a comet with a history, or an heirloom pet all naturally earn one; a bare
//! `EntityKind::Ground`/`Wall` ("a rock," `super::scene_loader`) never does.
//! An entity with no `SoulId` takes the neutral `(Ta,Tn,Tv)=(0,0,0)` reading
//! only — never graded, matching Cree's own hard (not scalar) animate/
//! inanimate split.
//!
//! Not yet wired: nothing today attaches a `SoulId` to a `SceneEntity` or a
//! dialogue speaker. This module accepts `Option<SoulId>` + the raw inputs
//! directly, so it is usable the moment that wiring lands, without changing
//! this file (L15, named gap not a blocker).

use forge_core_v3::cdk::Triad;
use forge_core_v3::soul::{cynatic_depth, SoulId};
use forge_lighting_v3::trit_dir::TritDir;

use super::dialogue::{mix, WordClass};

/// Hop-count buckets for [`quantize_lineage_depth`]. [Authored, not
/// measured against real content yet — Sean's tuning call if these feel
/// wrong once real lineage chains exist.]
const SHALLOW_MAX_HOPS: u32 = 1;
/// Upper bound of the "mid" bucket (inclusive).
const MID_MAX_HOPS: u32 = 4;

/// Q15 unit, the fixed point [`forge_core_v3::pentaract::Pentaract::cos_similarity`]
/// answers in.
pub const Q15: i64 = 32767;

/// `⌊Q15 / √3⌋` — folds the disposition pole `(1,-1,-1)`'s own magnitude into
/// the dot so [`quantize_disposition`]'s read is a true unit-direction cosine.
pub const Q15_OVER_SQRT3: i64 = 18_918;

/// Half-width of the neutral band on the direction cosine. AUTHORED against the
/// census gate `affect::tests::neutral_register_is_reachable`: wide enough that
/// `Ta = 0` holds a live share of sparse bonds, narrow enough that bound and
/// torn keep the majority.
pub const NEUTRAL_BAND_Q15: i64 = 3_277;

/// The `Ta` axis as a pentaract read: the direction cosine of the triad vector
/// against the disposition pole `(1,-1,-1)/√3`, banded. Magnitude divides out,
/// so the reading is bond STATE, not drive scale, and the middle register is a
/// solid-angle band rather than the measure-zero point `disposition() == 0`.
/// The zero vector has no direction and reads the fixed point.
#[inline]
pub fn quantize_disposition(triad: &Triad) -> i8 {
    let r = disposition_cosine(triad);
    if r > NEUTRAL_BAND_Q15 {
        1
    } else if r < -NEUTRAL_BAND_Q15 {
        -1
    } else {
        0
    }
}

/// The raw direction cosine [`quantize_disposition`] bands, in Q15. `0` for the
/// zero vector, which has no direction. Exposed so the band can be MEASURED
/// against real drives rather than argued about — one home for the formula.
#[inline]
pub fn disposition_cosine(triad: &Triad) -> i64 {
    let (l, s, e) = (triad.love as i64, triad.strife as i64, triad.entropy as i64);
    let mag_sq = (l * l) as u128 + (s * s) as u128 + (e * e) as u128;
    if mag_sq == 0 {
        return 0;
    }
    (l - s - e) * Q15_OVER_SQRT3 / mag_sq.isqrt() as i64
}

/// [`cynatic_depth`]'s hop count, bucketed to a trit — the `Tn` axis.
/// `None` (a cycle, a break, or a chain deeper than the caller's bound)
/// reads as maximal depth (`+1`) rather than defaulting to neutral: an
/// unterminated lineage is not the same fact as a shallow one, and folding
/// it into `0` would erase that distinction silently.
#[inline]
pub fn quantize_lineage_depth(hops: Option<u32>) -> i8 {
    match hops {
        Some(h) if h <= SHALLOW_MAX_HOPS => -1,
        Some(h) if h <= MID_MAX_HOPS => 0,
        _ => 1,
    }
}

/// `Tv = Ta ⊕ Tn`, balanced-ternary addition — the real ℤ/3 group table:
/// `-1↦2, 0↦0, 1↦1` (residues), add mod 3, map back `0↦0, 1↦1, 2↦-1`. `0`
/// is the group identity (`balanced_add(0, x) == x` for every `x`), which is
/// what makes the `Ta=0` row the mutual-inverse-plus-fixed-point law this
/// module's doc claims — an earlier draft used a shifted (non-identity)
/// mapping and was caught by its own test, not shipped.
#[inline]
pub fn balanced_add(a: i8, b: i8) -> i8 {
    #[inline]
    fn residue(t: i8) -> u8 {
        t.rem_euclid(3) as u8 // -1 -> 2, 0 -> 0, 1 -> 1
    }
    #[inline]
    fn from_residue(r: u8) -> i8 {
        if r == 2 {
            -1
        } else {
            r as i8
        }
    }
    from_residue((residue(a) + residue(b)) % 3)
}

/// The full `(Ta, Tn, Tv)` reading for one entity. `soul` gates animacy: an
/// entity with no [`SoulId`] reads the neutral triple unconditionally —
/// never graded, matching Cree's hard animate/inanimate split (module doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TritReading {
    /// Adjective axis: sign of `Triad::disposition()`.
    pub ta: i8,
    /// Noun axis: bucketed lineage-hop depth.
    pub tn: i8,
    /// Verb axis: `balanced_add(ta, tn)`, derived, not free.
    pub tv: i8,
}

impl TritReading {
    /// The fixed-point reading every inanimate entity takes.
    pub const NEUTRAL: Self = Self { ta: 0, tn: 0, tv: 0 };

    /// The reading as a direction on the 26-point cube lattice. A reading is
    /// three balanced trits, which is exactly what [`TritDir`] packs — same
    /// Horner radix-3 order, so this reuses that bijection rather than
    /// repacking it (L05). [`Self::NEUTRAL`] lands on `DIR_ORIGIN`, which
    /// `TritDir` refuses as a direction: an inanimate entity has no bearing.
    pub fn bearing(self) -> TritDir {
        TritDir::from_trits([self.ta, self.tn, self.tv])
    }

    /// True where the verb runs against both free axes. `Tv = balanced_add(Ta,
    /// Tn)` inverts exactly when `Ta` and `Tn` agree and are non-zero — two
    /// 120-degree rotations the same way land 240 degrees on, which reads as
    /// -120. Those are precisely the lattice CORNERS (`order() == 3`), so this
    /// is the lattice's own classifier, not a second rule laid over it.
    pub fn inverts(self) -> bool {
        self.bearing().order() == 3
    }

    /// Compute a live reading for an animate entity (`soul.is_some()`), or
    /// [`Self::NEUTRAL`] for an inanimate one.
    pub fn for_entity(
        soul: Option<SoulId>,
        triad: &Triad,
        max_hops: u32,
        parent_of: impl FnMut(SoulId) -> Option<SoulId>,
    ) -> Self {
        let Some(soul) = soul else { return Self::NEUTRAL };
        let ta = quantize_disposition(triad);
        let tn = quantize_lineage_depth(cynatic_depth(soul, max_hops, parent_of));
        let tv = balanced_add(ta, tn);
        Self { ta, tn, tv }
    }
}

/// Trit index: `-1/0/+1 -> 0/1/2`, the array index into a trit-partitioned
/// bank triple.
#[inline]
fn trit_index(t: i8) -> usize {
    (t + 1) as usize
}

/// One bank PER TRIT per class: index `0`=`-1`, `1`=`0`, `2`=`+1`. Extends
/// [`super::dialogue::WordBanks`] (unchanged, still the `0`-only shape for
/// callers that don't need grading) rather than replacing it.
#[derive(Debug, Clone, Default)]
pub struct TritWordBanks {
    /// Words for `{adj}` slots, indexed `0`=`-1`, `1`=`0`, `2`=`+1`.
    pub adjectives: [Vec<String>; 3],
    /// Words for `{noun}` slots, indexed `0`=`-1`, `1`=`0`, `2`=`+1`.
    pub nouns: [Vec<String>; 3],
    /// Words for `{verb}` slots, indexed `0`=`-1`, `1`=`0`, `2`=`+1`.
    pub verbs: [Vec<String>; 3],
}

impl TritWordBanks {
    /// The bank for a given class at a given trit value.
    pub fn bank(&self, class: WordClass, trit: i8) -> &[String] {
        let i = trit_index(trit);
        match class {
            WordClass::Adjective => &self.adjectives[i],
            WordClass::Noun => &self.nouns[i],
            WordClass::Verb => &self.verbs[i],
        }
    }
}

/// Fill every `{adj}`/`{noun}`/`{verb}` in `text` from a [`TritWordBanks`],
/// grading each class by its own trit (`adj`->`ta`, `noun`->`tn`,
/// `verb`->`tv`). Same left-to-right scan and same loud-on-empty-bank
/// behavior as [`super::dialogue::fill_template`] (an empty bank at the
/// selected trit leaves the slot token visible, never blank) — reuses that
/// function's exact mixer ([`mix`]) rather than a second hash law.
pub fn fill_template_trit(text: &str, id: &str, banks: &TritWordBanks, seed: u64, reading: TritReading) -> String {
    let mut out = text.to_string();
    let mut ordinal = 0usize;
    loop {
        let next = WordClass::ALL.iter().filter_map(|c| out.find(c.slot()).map(|at| (at, *c))).min_by_key(|(at, _)| *at);
        let Some((at, class)) = next else { break };
        let trit = match class {
            WordClass::Adjective => reading.ta,
            WordClass::Noun => reading.tn,
            WordClass::Verb => reading.tv,
        };
        let bank = banks.bank(class, trit);
        if bank.is_empty() {
            let end = at + class.slot().len();
            let (head, tail) = out.split_at(end);
            let mut joined = head.to_string();
            joined.push_str(&fill_template_trit(tail, id, banks, seed.wrapping_add(1), reading));
            return joined;
        }
        let pick = (mix(seed, id, ordinal) % bank.len() as u64) as usize;
        out.replace_range(at..at + class.slot().len(), &bank[pick]);
        ordinal += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_add_is_total_and_matches_the_group_table() {
        // Every (Ta,Tn) pair produces a value in -1..=1 -- no panics, no gaps.
        for a in [-1i8, 0, 1] {
            for b in [-1i8, 0, 1] {
                let v = balanced_add(a, b);
                assert!((-1..=1).contains(&v));
            }
        }
        // The Ta=0 row IS the identity: Tv(0,Tn) = Tn.
        assert_eq!(balanced_add(0, -1), -1);
        assert_eq!(balanced_add(0, 0), 0);
        assert_eq!(balanced_add(0, 1), 1);
        // 0 is a two-sided identity.
        assert_eq!(balanced_add(-1, 0), -1);
        assert_eq!(balanced_add(1, 0), 1);
    }

    #[test]
    fn ta_zero_row_is_the_mutual_inverse_pair_plus_fixed_point() {
        // Same law cremantic.rs's Mirror lane proves: negating Tn negates Tv
        // exactly on the Ta=0 row.
        for tn in [-1i8, 0, 1] {
            assert_eq!(balanced_add(0, -tn), -balanced_add(0, tn));
        }
    }

    /// Every reading the law can produce, in (ta, tn) order.
    fn every_reading() -> Vec<TritReading> {
        let mut out = Vec::new();
        for ta in [-1i8, 0, 1] {
            for tn in [-1i8, 0, 1] {
                out.push(TritReading { ta, tn, tv: balanced_add(ta, tn) });
            }
        }
        out
    }

    #[test]
    fn a_reading_is_a_direction_and_survives_the_round_trip() {
        for r in every_reading() {
            assert_eq!(
                r.bearing().trits(),
                [r.ta, r.tn, r.tv],
                "the lattice must hand back the reading it was given"
            );
        }
    }

    #[test]
    fn the_neutral_reading_is_the_refused_origin() {
        // An entity with no SoulId has no bearing. TritDir refuses the all-zero
        // word as a direction; NEUTRAL is exactly that word.
        assert!(TritReading::NEUTRAL.bearing().is_origin());
        assert!(!TritReading::NEUTRAL.inverts());
    }

    #[test]
    fn the_law_reaches_nine_of_twenty_seven_states_and_never_a_face() {
        let (mut origins, mut faces, mut edges, mut corners) = (0, 0, 0, 0);
        for r in every_reading() {
            match r.bearing().order() {
                0 => origins += 1,
                1 => faces += 1,
                2 => edges += 1,
                _ => corners += 1,
            }
        }
        assert_eq!(origins, 1, "only (0,0) is the origin");
        assert_eq!(edges, 6);
        assert_eq!(corners, 2);
        assert_eq!(
            faces, 0,
            "a face needs two zero trits, but Tv = Ta + Tn is zero only when both inputs are"
        );
    }

    #[test]
    fn inversion_is_exactly_the_lattice_corner() {
        for r in every_reading() {
            let doubled_down = r.ta != 0 && r.ta == r.tn;
            assert_eq!(
                r.inverts(),
                doubled_down,
                "reading ({}, {}, {}) disagreed with the corner test",
                r.ta, r.tn, r.tv
            );
            if r.inverts() {
                assert_eq!(r.tv, -r.ta, "a corner runs its verb against both free axes");
            }
        }
        assert_eq!(every_reading().iter().filter(|r| r.inverts()).count(), 2);
    }

    #[test]
    fn the_quantizer_is_exactly_a_band_over_the_cosine() {
        for love in [-3_000i32, -700, 0, 700, 3_000] {
            for strife in [-3_000i32, 0, 900, 3_000] {
                for entropy in [0i32, 400, 5_000] {
                    let t = Triad { love, strife, entropy };
                    let r = disposition_cosine(&t);
                    let want = if r > NEUTRAL_BAND_Q15 {
                        1
                    } else if r < -NEUTRAL_BAND_Q15 {
                        -1
                    } else {
                        0
                    };
                    assert_eq!(quantize_disposition(&t), want, "{t:?} cosine {r}");
                    assert!(r.abs() <= Q15, "a direction cosine cannot exceed unity: {r}");
                }
            }
        }
        let pole = Triad { love: 1_000, strife: -1_000, entropy: -1_000 };
        assert!(disposition_cosine(&pole) > Q15 - 200, "the pole is the cosine's own maximum");
        let anti = Triad { love: -1_000, strife: 1_000, entropy: 1_000 };
        assert!(disposition_cosine(&anti) < -(Q15 - 200));
        assert_eq!(disposition_cosine(&Triad { love: 0, strife: 0, entropy: 0 }), 0);
    }

    #[test]
    fn quantize_disposition_reads_banded_direction_cosine() {
        assert_eq!(quantize_disposition(&Triad { love: 1000, strife: 0, entropy: 0 }), 1);
        assert_eq!(quantize_disposition(&Triad { love: 0, strife: 500, entropy: 600 }), -1);
        assert_eq!(quantize_disposition(&Triad { love: 0, strife: 0, entropy: 0 }), 0);
        // A near-balanced bond lands in the neutral band, not on a knife edge.
        assert_eq!(quantize_disposition(&Triad { love: 1000, strife: 980, entropy: 0 }), 0);
    }

    #[test]
    fn quantize_disposition_is_scale_free() {
        for (l, s, e) in [(1000, 0, 0), (0, 500, 600), (1000, 980, 0), (300, 200, 50)] {
            let one = quantize_disposition(&Triad { love: l, strife: s, entropy: e });
            let hundred =
                quantize_disposition(&Triad { love: l * 100, strife: s * 100, entropy: e * 100 });
            assert_eq!(one, hundred, "({l},{s},{e}) x100 must read the same register");
        }
    }

    #[test]
    fn quantize_lineage_depth_buckets_hops_and_treats_none_as_deepest() {
        assert_eq!(quantize_lineage_depth(Some(0)), -1);
        assert_eq!(quantize_lineage_depth(Some(1)), -1);
        assert_eq!(quantize_lineage_depth(Some(2)), 0);
        assert_eq!(quantize_lineage_depth(Some(4)), 0);
        assert_eq!(quantize_lineage_depth(Some(5)), 1);
        assert_eq!(quantize_lineage_depth(None), 1);
    }

    #[test]
    fn inanimate_entity_reads_neutral_unconditionally() {
        let triad = Triad { love: 1000, strife: 0, entropy: 0 };
        let reading = TritReading::for_entity(None, &triad, 10, |_| None);
        assert_eq!(reading, TritReading::NEUTRAL);
    }

    #[test]
    fn animate_entity_reads_a_live_graded_triple() {
        let triad = Triad { love: 1000, strife: 0, entropy: 0 };
        // 3 -> 2 -> 1 -> ROOT: 3 hops, mid bucket.
        let parent_of = |s: SoulId| match s.0 {
            3 => Some(SoulId(2)),
            2 => Some(SoulId(1)),
            1 => Some(SoulId::ROOT),
            _ => None,
        };
        let reading = TritReading::for_entity(Some(SoulId(3)), &triad, 10, parent_of);
        assert_eq!(reading.ta, 1);
        assert_eq!(reading.tn, 0);
        assert_eq!(reading.tv, balanced_add(1, 0));
    }

    #[test]
    fn fill_template_trit_leaves_slot_visible_on_empty_bank_at_that_trit() {
        let mut banks = TritWordBanks::default();
        banks.nouns[1] = vec!["forge".into()]; // only the 0-trit noun bank is filled
        let reading = TritReading::NEUTRAL;
        let out = fill_template_trit("the {noun} stands", "n1", &banks, 7, reading);
        assert_eq!(out, "the forge stands");

        // Same banks, but a +1 noun reading has no words -- slot stays visible.
        let live = TritReading { ta: 0, tn: 1, tv: 0 };
        let out2 = fill_template_trit("the {noun} stands", "n1", &banks, 7, live);
        assert_eq!(out2, "the {noun} stands");
    }

    #[test]
    fn fill_template_trit_grades_each_class_by_its_own_trit() {
        let mut banks = TritWordBanks::default();
        banks.adjectives[2] = vec!["gleaming".into()]; // +1
        banks.nouns[0] = vec!["wound".into()]; // -1
        banks.verbs[trit_index(balanced_add(1, -1))] = vec!["remain".into()]; // Tv=0
        let reading = TritReading { ta: 1, tn: -1, tv: balanced_add(1, -1) };
        let out = fill_template_trit("the {adj} {noun} will {verb}", "n2", &banks, 3, reading);
        assert_eq!(out, "the gleaming wound will remain");
    }
}
