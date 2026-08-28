//! The gift half of the Dream Forge (`ORACLE-C-DREAM-DIAMONDS-EUX.md` §8:238-247):
//! a night leaves a gift, not a transcript. The transcript is shredded by
//! `session_vault`; what survives enters only through the airlock.

use forge_envelope::typed_manifold::{admit, GiftKind, GiftProposal, ManifoldVerdict, SealedGift};
use forge_envelope::EvidenceChain;

use super::journal::SENTINEL_GIFT;
use super::score::{day_quality_pmy, ROUGH_PATCH_FLOOR_PMY};
use crate::operator::seed_hash;

/// Quality floor a night must clear to leave anything — the same `<0.3` line
/// the rough-patch watch uses, read from the other side.
pub const GIFT_FLOOR_PMY: u32 = ROUGH_PATCH_FLOOR_PMY;

/// Domain separator for the night-roll, keeping it distinct from the
/// `b"oversleep"` roll `game::wake` draws from the same `seed_hash`.
const GIFT_ROLL_TAG: &[u8] = b"dream_gift";

/// The fragment a night mints: sentinel, kind, quality, tick.
const FRAGMENT_BYTES: usize = 1 + 1 + 4 + 8;

fn kind_from_roll(roll: u64) -> GiftKind {
    match roll % 5 {
        0 => GiftKind::Song,
        1 => GiftKind::Route,
        2 => GiftKind::Word,
        3 => GiftKind::Face,
        _ => GiftKind::GeomMacro,
    }
}

/// Mint the night's proposal, or `None` if the night was too rough to leave one.
///
/// Deterministic in `(node_seed, sleep_tick, quality_pmy)` — the same night
/// mints the same fragment, and therefore the same seal, on every replay.
pub fn gift_from_night(node_seed: u64, sleep_tick: u64, quality_pmy: u32) -> Option<GiftProposal> {
    if quality_pmy < GIFT_FLOOR_PMY {
        return None;
    }
    let roll = seed_hash(&[
        &node_seed.to_be_bytes(),
        &sleep_tick.to_be_bytes(),
        &quality_pmy.to_be_bytes(),
        GIFT_ROLL_TAG,
    ]);
    let kind = kind_from_roll(roll);

    let mut fragment = Vec::with_capacity(FRAGMENT_BYTES);
    fragment.push(SENTINEL_GIFT);
    fragment.push(kind as u8);
    fragment.extend_from_slice(&quality_pmy.to_be_bytes());
    fragment.extend_from_slice(&sleep_tick.to_be_bytes());

    Some(GiftProposal { kind, fragment })
}

/// The night's three scored terms, as `wake` measured them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NightScore {
    /// The world's settledness, from the operator's heat.
    pub balance_pmy: u32,
    /// The body's reserve, from vitality.
    pub energy_pmy: u32,
    /// The rest's cadence — ticks slept against [`super::score::SLEEP_TTL_TICKS`].
    pub beat_pmy: u32,
}

/// A night's minted gift and whether the repair pass had to run.
#[derive(Clone, Debug)]
pub struct MintedGift {
    /// The proposal to carry through the airlock.
    pub proposal: GiftProposal,
    /// The quality the fragment was actually forged at.
    pub quality_pmy: u32,
    /// True if the first forging fell below the floor and the repair pass saved it.
    pub repaired: bool,
}

/// The cadence a corrected rest is scored at: the safe-rest ideal less the
/// rough-patch margin. The repair pass mends the beat term only — balance is
/// the world's heat and energy is the body's reserve, neither the dream's to
/// correct — and it mends toward the ideal without reaching it, so a night
/// with nothing left in it stays lost. A repair pass, not a second night.
const REPAIRED_BEAT_PMY: u32 = 10_000 - ROUGH_PATCH_FLOOR_PMY;

/// Mint the night's gift, allowing exactly one repair pass (`§8:248-249`:
/// "a poor dream re-forges once before waking — score < threshold → one
/// correction retry"). A night that is still below the floor after its one
/// correction leaves nothing; there is no second retry.
pub fn mint_with_one_repair(
    node_seed: u64,
    sleep_tick: u64,
    score: NightScore,
) -> Option<MintedGift> {
    let first = day_quality_pmy(score.balance_pmy, score.energy_pmy, score.beat_pmy);
    if let Some(proposal) = gift_from_night(node_seed, sleep_tick, first) {
        return Some(MintedGift { proposal, quality_pmy: first, repaired: false });
    }
    let repaired = day_quality_pmy(score.balance_pmy, score.energy_pmy, REPAIRED_BEAT_PMY);
    let proposal = gift_from_night(node_seed, sleep_tick, repaired)?;
    Some(MintedGift { proposal, quality_pmy: repaired, repaired: true })
}

/// Carry a proposal through the airlock onto `chain`. `None` is a refusal —
/// the verdict's reason stays inside `forge-envelope`; this seam only reports
/// whether the world kept it.
pub fn admit_gift(
    proposal: &GiftProposal,
    current_tick: u64,
    chain: &mut EvidenceChain,
) -> Option<SealedGift> {
    match admit(proposal, current_tick, chain) {
        ManifoldVerdict::Admitted(sealed) => Some(sealed),
        ManifoldVerdict::Rejected(_) => None,
    }
}

/// The word the MUD prints for a gift shape.
pub fn gift_word(kind: GiftKind) -> &'static str {
    match kind {
        GiftKind::Song => "a song",
        GiftKind::Route => "a route",
        GiftKind::Word => "a word",
        GiftKind::Face => "a face",
        GiftKind::GeomMacro => "a shape",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rough_night_leaves_nothing() {
        assert!(gift_from_night(7, 100, GIFT_FLOOR_PMY - 1).is_none());
        assert!(gift_from_night(7, 100, 0).is_none());
    }

    #[test]
    fn the_floor_itself_still_leaves_a_gift() {
        assert!(gift_from_night(7, 100, GIFT_FLOOR_PMY).is_some());
    }

    #[test]
    fn the_same_night_mints_the_same_fragment() {
        let a = gift_from_night(7, 100, 8_000).expect("clear night");
        let b = gift_from_night(7, 100, 8_000).expect("clear night");
        assert_eq!(a.fragment, b.fragment);
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.fragment.len(), FRAGMENT_BYTES);
        assert_eq!(a.fragment[0], SENTINEL_GIFT, "a gift carries the gift mark, not the crossing");
    }

    /// `§8:248-249` — a poor night re-forges exactly once.
    #[test]
    fn a_poor_night_reforges_once_and_is_saved() {
        let poor = NightScore { balance_pmy: 5_000, energy_pmy: 2_000, beat_pmy: 0 };
        assert!(
            day_quality_pmy(poor.balance_pmy, poor.energy_pmy, poor.beat_pmy) < GIFT_FLOOR_PMY,
            "precondition: the first forging must fall below the floor"
        );
        let minted = mint_with_one_repair(7, 100, poor).expect("the repair pass must save it");
        assert!(minted.repaired, "this gift only exists because of the correction retry");
        assert!(minted.quality_pmy >= GIFT_FLOOR_PMY);
    }

    /// A night too poor for its one correction leaves nothing — no second
    /// retry, and the repair is not a miracle: mending the cadence alone can
    /// never carry a night that had nothing else in it.
    #[test]
    fn a_night_below_the_floor_even_repaired_leaves_nothing() {
        let hopeless = NightScore { balance_pmy: 0, energy_pmy: 0, beat_pmy: 0 };
        assert!(mint_with_one_repair(7, 100, hopeless).is_none());
        assert!(
            day_quality_pmy(0, 0, REPAIRED_BEAT_PMY) < GIFT_FLOOR_PMY,
            "the repaired beat alone must not clear the floor, or the floor means nothing"
        );
    }

    /// A good night never enters the repair pass.
    #[test]
    fn a_clear_night_is_never_repaired() {
        let clear = NightScore { balance_pmy: 10_000, energy_pmy: 10_000, beat_pmy: 5_000 };
        let minted = mint_with_one_repair(7, 100, clear).expect("a clear night mints");
        assert!(!minted.repaired, "a night above the floor must not be re-forged");
        assert_eq!(
            minted.quality_pmy,
            day_quality_pmy(clear.balance_pmy, clear.energy_pmy, clear.beat_pmy)
        );
    }

    /// The repair mends the cadence only — the world's heat and the body's
    /// reserve are carried into the second forging untouched.
    #[test]
    fn the_repair_pass_mends_the_beat_and_nothing_else() {
        let poor = NightScore { balance_pmy: 5_000, energy_pmy: 2_000, beat_pmy: 0 };
        let minted = mint_with_one_repair(7, 100, poor).expect("repaired");
        assert_eq!(
            minted.quality_pmy,
            day_quality_pmy(poor.balance_pmy, poor.energy_pmy, REPAIRED_BEAT_PMY),
            "only the beat term may differ between the two forgings"
        );
    }

    #[test]
    fn a_different_night_mints_a_different_fragment() {
        let a = gift_from_night(7, 100, 8_000).expect("clear night");
        let b = gift_from_night(7, 101, 8_000).expect("clear night");
        assert_ne!(a.fragment, b.fragment);
    }

    #[test]
    fn admitting_a_gift_seals_it_onto_the_chain() {
        let mut chain = EvidenceChain::new();
        let p = gift_from_night(7, 100, 8_000).expect("clear night");
        let sealed = admit_gift(&p, 150, &mut chain).expect("a shaped gift passes the airlock");
        assert_eq!(sealed.kind, p.kind);
        assert_eq!(chain.len(), 1, "the airlock appends exactly one link");
    }

    #[test]
    fn an_empty_fragment_never_reaches_the_chain() {
        let mut chain = EvidenceChain::new();
        let hollow = GiftProposal { kind: GiftKind::Word, fragment: Vec::new() };
        assert!(admit_gift(&hollow, 150, &mut chain).is_none());
        assert!(chain.is_empty(), "a refusal must leave the chain untouched");
    }

    #[test]
    fn the_seal_is_reproducible_across_replays() {
        let mut chain_a = EvidenceChain::new();
        let mut chain_b = EvidenceChain::new();
        let a = admit_gift(&gift_from_night(7, 100, 8_000).unwrap(), 150, &mut chain_a).unwrap();
        let b = admit_gift(&gift_from_night(7, 100, 8_000).unwrap(), 150, &mut chain_b).unwrap();
        assert_eq!(a.seal, b.seal);
    }

    #[test]
    fn every_kind_is_reachable_from_some_night() {
        let mut seen = [false; 5];
        for tick in 0..512u64 {
            if let Some(p) = gift_from_night(7, tick, 8_000) {
                seen[p.kind as usize] = true;
            }
        }
        assert!(seen.iter().all(|&s| s), "all five gift shapes must be drawable: {seen:?}");
    }
}
