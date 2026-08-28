//! The reactions corpus — v2's `forge-reactions` world-building content, as RON.
//!
//! Ported from `F:\NewRepo\crates\forge-reactions` (23 files, ~85% authored data). The values
//! are drained verbatim; only the SHAPE changes — tables become RON, logic stays Rust.
//!
//! ## The loop this serves
//!
//! Cozy crafting with a weighted underside. A substrate's `public_name` is what the market
//! calls it; its `true_label` is what it is. "Sweet Draft" is bottled fae breath;
//! "Weatherhide" is a stolen skin. The distance between those two strings is the game.
//!
//! ## Why the ethics are 3-based, mirroring the CDK
//!
//! `PARARITY.md` §3 Prop. 1 gives `n = 2m + k`, and Cor. 1 forces `k >= 1` for odd `n`. The
//! five crafting paths resolve as **m=2, k=1** — two mirror pairs and one fixed point:
//! `Exploit <-> Release`, `Bargain <-> Replace`, and `Preserve` fixed.
//!
//! `Preserve` is the fixed point **in the data**, not by assertion: every pressure lane it
//! carries is exactly zero. It pushes nothing and only raises world stability — which is what
//! [`forge_core_v3::cdk::Triad`]'s entropy lane is documented as, "what neither love nor
//! strife holds" (`cdk.rs:49`). Strife separates you from the fae, love binds you back, and
//! balance is its own state rather than a midpoint between them.
//!
//! Content ships **embedded** (`include_str!`), deliberately unlike `physics_tune::load`,
//! which falls back to `Default` on any error because it is tuning. World content is not
//! tuning: a silent fallback would drop the corpus and still report green.

use serde::{Deserialize, Serialize};
use crate::overlay::Ledger;

/// The 7 living substrates — fae-bound crafting materials.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubstrateType {
    /// "Red Sap".
    FaeBlood,
    /// "Sweet Draft".
    FaeBreath,
    /// "Harmonic Thread".
    FaeSong,
    /// "Root-Stay" — the spirit holding back Ironroot.
    FaeRootSpirit,
    /// "Weatherhide".
    FaeSkinOrCoat,
    /// "Hollow Ivory".
    FaeBone,
    /// "Soft Map".
    FaeDream,
}

/// How a crafter obtained and treated their source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CraftingEthicsPath {
    /// Take. The strife pole.
    Exploit,
    /// Take on a bond in exchange.
    Bargain,
    /// Give back. The love pole, exact mirror of [`Self::Exploit`].
    Release,
    /// Substitute a source and shed the bond.
    Replace,
    /// Touch nothing. The fixed point.
    Preserve,
}

impl CraftingEthicsPath {
    /// All five, in authored order.
    pub const ALL: [Self; 5] =
        [Self::Exploit, Self::Bargain, Self::Release, Self::Replace, Self::Preserve];
}

/// One substrate's authored row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateDef {
    /// Which substrate.
    pub id: SubstrateType,
    /// What the market calls it.
    pub public_name: String,
    /// What it actually is.
    pub true_label: String,
    /// Permyriad weight of taking it. 10_000 = 1.0.
    pub ethical_pressure_q: u16,
    /// The named way this material goes wrong.
    pub corruption_risk: String,
}

/// The substrate catalog as it sits in RON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateCatalog {
    /// Every substrate, authored order.
    pub substrates: Vec<SubstrateDef>,
}

/// Permyriad deltas an ethics path applies to the world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftingModifiers {
    /// Which path these belong to.
    pub path: CraftingEthicsPath,
    /// Power granted to the crafted item.
    pub item_power_q: i16,
    /// Bond taken on (positive) or shed (negative).
    pub obligation_pressure_q: i16,
    /// The fae turning against you. The STRIFE lane — the exact mirror axis.
    pub fae_hostility_q: i16,
    /// Pull toward ownership.
    pub crown_temptation_q: i16,
    /// Harm to the living world.
    pub ecology_pressure_q: i16,
    /// How hard the craft is to perform.
    pub crafting_difficulty_q: i16,
    /// How detectable the provenance is.
    pub provenance_shimmer_q: i16,
    /// The BALANCE lane — what `Preserve` alone raises.
    pub world_stability_q: i16,
}

impl Default for CraftingEthicsPath {
    fn default() -> Self {
        Self::Preserve
    }
}

/// One row of the ethical involution: which path the mirror sends this one to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvolutionRow {
    /// The path.
    pub path: CraftingEthicsPath,
    /// Where the mirror sends it. Equal to `path` exactly at the fixed point.
    pub mirrors: CraftingEthicsPath,
}

/// The ethics catalog: the involution, its fixed point, and the authored deltas.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthicsCatalog {
    /// `f` as a table, one row per path.
    pub involution: Vec<InvolutionRow>,
    /// `Fix(f)`. `|Fix(f)| = k = 1` for this set.
    pub fixed_point: CraftingEthicsPath,
    /// The permyriad deltas.
    pub modifiers: Vec<CraftingModifiers>,
}

impl EthicsCatalog {
    /// Where the ethical mirror sends `path`.
    pub fn mirror_of(&self, path: CraftingEthicsPath) -> Option<CraftingEthicsPath> {
        self.involution.iter().find(|r| r.path == path).map(|r| r.mirrors)
    }

    /// The authored deltas for `path`.
    pub fn modifiers_for(&self, path: CraftingEthicsPath) -> Option<&CraftingModifiers> {
        self.modifiers.iter().find(|m| m.path == path)
    }
}

/// How a player treated a fae reward.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaeItemOutcome {
    /// Took it, openly.
    Claimed,
    /// Took it, and owes for it.
    Bargained,
    /// Received it freely.
    Gifted,
    /// Took it without leave — the only outcome the world can detect.
    Stolen,
    /// Would not take it.
    Refused,
}

impl FaeItemOutcome {
    /// All five, in authored order.
    pub const ALL: [Self; 5] =
        [Self::Claimed, Self::Bargained, Self::Gifted, Self::Stolen, Self::Refused];
}

/// The five signed pressure lanes one outcome writes into the world.
///
/// Each lane is a balanced trit (`-/0/+`) carrying a permyriad magnitude. Five lanes at three
/// states is `3^5 = 243` — the shape of [`forge_core_v3::atom::TritCell5D`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaeItemPressure {
    /// Which outcome.
    pub outcome: FaeItemOutcome,
    /// Pull toward owning.
    pub ownership_pressure_q: i16,
    /// Bond taken on or shed.
    pub obligation_pressure_q: i16,
    /// Pull toward the crown.
    pub crown_temptation_q: i16,
    /// The fae turning against you.
    pub fae_hostility_q: i16,
    /// Harm to the living world.
    pub ecology_pressure_q: i16,
    /// Whether the act leaves a trace the world can read.
    pub shimmer_detectable: bool,
}

impl FaeItemPressure {
    /// The five lanes as balanced trits — sign only, magnitude discarded.
    ///
    /// This is the direction the act pushes the world, in the form a lattice reads. It is
    /// deliberately lossy: [`FaeItemOutcome::Stolen`] and [`FaeItemOutcome::Claimed`] return
    /// the SAME vector, because stealing is claiming in every direction and differs only in
    /// how hard and in whether it is detectable.
    pub const fn trits(&self) -> [i8; 5] {
        const fn s(v: i16) -> i8 {
            if v > 0 {
                1
            } else if v < 0 {
                -1
            } else {
                0
            }
        }
        [
            s(self.ownership_pressure_q),
            s(self.obligation_pressure_q),
            s(self.crown_temptation_q),
            s(self.fae_hostility_q),
            s(self.ecology_pressure_q),
        ]
    }
}

/// The fae-reward catalog as it sits in RON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaeItemCatalog {
    /// Every outcome, authored order.
    pub outcomes: Vec<FaeItemPressure>,
}

impl FaeItemCatalog {
    /// The authored pressure for `outcome`.
    pub fn pressure_for(&self, outcome: FaeItemOutcome) -> Option<&FaeItemPressure> {
        self.outcomes.iter().find(|p| p.outcome == outcome)
    }
}

/// The substrate catalog, embedded at compile time.
///
/// `include_str!` rather than a disk read: content that can go missing is content that can
/// silently vanish while every test stays green. Same idiom `forge-canvas-v3::text` uses for
/// its fonts.
const SUBSTRATES_RON: &str = include_str!("../content/reactions/substrates.ron");
/// The ethics catalog, embedded at compile time.
const ETHICS_RON: &str = include_str!("../content/reactions/crafting_ethics.ron");

/// Parse the embedded substrate catalog.
///
/// # Panics
/// On malformed embedded RON — which is a build-time authoring error, not a runtime
/// condition. Failing loud here is the point: a corpus that half-loads is worse than one that
/// refuses to.
/// Bias a crafting ethics path toward force (Exploit/Bargain) or water (Release/Replace) based on
/// archetype pole tally — called before applying crafting modifiers to nudge which path rolls.
pub fn archetype_biased_craft_path(base: CraftingEthicsPath, ledger: &Ledger, seed: u64) -> CraftingEthicsPath {
    let pole = crate::ironroot::archetype_ledger::dominant_pole(ledger, seed);
    match base {
        CraftingEthicsPath::Bargain | CraftingEthicsPath::Preserve => {
            if pole > 2000 { CraftingEthicsPath::Exploit } else if pole < -2000 { CraftingEthicsPath::Release } else { base }
        },
        _ => base,
    }
}

/// Parse the embedded substrates catalog. Panics on malformed embedded RON, as above.
pub fn substrates() -> SubstrateCatalog {
    ron::from_str(SUBSTRATES_RON).expect("embedded substrates.ron is malformed")
}

/// Parse the embedded ethics catalog. Panics on malformed embedded RON, as above.
pub fn ethics() -> EthicsCatalog {
    ron::from_str(ETHICS_RON).expect("embedded crafting_ethics.ron is malformed")
}

/// The fae-reward catalog, embedded at compile time.
const FAE_ITEM_RON: &str = include_str!("../content/reactions/fae_item_pressure.ron");

/// Parse the embedded fae-reward catalog. Panics on malformed embedded RON, as above.
pub fn fae_item_pressure() -> FaeItemCatalog {
    ron::from_str(FAE_ITEM_RON).expect("embedded fae_item_pressure.ron is malformed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_substrate_catalog_holds_all_seven() {
        let c = substrates();
        assert_eq!(c.substrates.len(), 7, "a substrate was dropped in the port");
    }

    /// The loop's whole premise: the market name and the true name must differ on every row.
    /// A substrate whose two names matched would be a material with nothing to hide, and the
    /// economy's tension would quietly vanish from that row.
    #[test]
    fn every_substrate_hides_what_it_is() {
        for s in substrates().substrates {
            assert_ne!(
                s.public_name, s.true_label,
                "{} does not conceal anything — the loop's tension is gone from this row",
                s.public_name
            );
            assert!(!s.corruption_risk.is_empty(), "{} has no named failure", s.public_name);
        }
    }

    /// The authored weight ladder, spot-checked at both ends: a borrowed identity outweighs a
    /// death, which outweighs blood. This is authored judgement, not a derived scale.
    #[test]
    fn the_ethical_ladder_is_the_authored_one() {
        let c = substrates();
        let w = |id: SubstrateType| {
            c.substrates.iter().find(|s| s.id == id).expect("substrate present").ethical_pressure_q
        };
        assert_eq!(w(SubstrateType::FaeSkinOrCoat), 9500, "stolen identity is the heaviest");
        assert_eq!(w(SubstrateType::FaeRootSpirit), 9000);
        assert_eq!(w(SubstrateType::FaeBone), 7500);
        assert_eq!(w(SubstrateType::FaeBreath), 6000, "breath is the lightest");
        assert!(w(SubstrateType::FaeSkinOrCoat) > w(SubstrateType::FaeBone));
    }

    #[test]
    fn the_ethics_catalog_holds_all_five_paths() {
        let e = ethics();
        assert_eq!(e.modifiers.len(), 5);
        assert_eq!(e.involution.len(), 5);
        for p in CraftingEthicsPath::ALL {
            assert!(e.modifiers_for(p).is_some(), "{p:?} has no authored deltas");
            assert!(e.mirror_of(p).is_some(), "{p:?} is not in the involution table");
        }
    }

    /// `f(f(x)) == x` for every path — the defining property of an involution (PARARITY §2).
    /// If this fails the table is a permutation, not a mirror, and `Fix(f)` means nothing.
    #[test]
    fn the_ethical_mirror_is_an_involution() {
        let e = ethics();
        for p in CraftingEthicsPath::ALL {
            let once = e.mirror_of(p).expect("in table");
            let twice = e.mirror_of(once).expect("in table");
            assert_eq!(twice, p, "f(f({p:?})) != {p:?} — this is not a mirror");
        }
    }

    /// `n = 2m + k` with n=5 ⇒ exactly one fixed point (PARARITY §3 Prop. 1, Cor. 1: odd n
    /// forces k >= 1). Preserve is it, and the table must agree with its own declaration.
    #[test]
    fn exactly_one_path_is_fixed_and_it_is_preserve() {
        let e = ethics();
        let fixed: Vec<_> = CraftingEthicsPath::ALL
            .into_iter()
            .filter(|p| e.mirror_of(*p) == Some(*p))
            .collect();
        assert_eq!(fixed.len(), 1, "k must be 1 for n=5; found {fixed:?}");
        assert_eq!(fixed[0], CraftingEthicsPath::Preserve);
        assert_eq!(e.fixed_point, CraftingEthicsPath::Preserve, "declaration disagrees with f");
    }

    /// THE FIND, as a test: Preserve pushes NOTHING. Every pressure lane is zero and only
    /// stability moves. That is what makes it the fixed point in the data rather than by
    /// assertion — the same role `cdk::Triad`'s entropy lane plays, "what neither love nor
    /// strife holds".
    #[test]
    fn the_fixed_point_pushes_no_pressure() {
        let e = ethics();
        let m = e.modifiers_for(CraftingEthicsPath::Preserve).expect("Preserve present");
        assert_eq!(m.obligation_pressure_q, 0);
        assert_eq!(m.fae_hostility_q, 0);
        assert_eq!(m.crown_temptation_q, 0);
        assert_eq!(m.ecology_pressure_q, 0);
        assert_eq!(m.crafting_difficulty_q, 0);
        assert_eq!(m.provenance_shimmer_q, 0);
        assert!(m.world_stability_q > 0, "the fixed point must still hold the world steady");
    }

    /// Exploit and Release mirror EXACTLY on the two unambiguous lanes. This is the arithmetic
    /// the structure rests on — if these stop being negations, the pairing is decorative.
    #[test]
    fn the_taking_and_giving_poles_negate_each_other() {
        let e = ethics();
        let x = *e.modifiers_for(CraftingEthicsPath::Exploit).expect("Exploit");
        let r = *e.modifiers_for(CraftingEthicsPath::Release).expect("Release");
        assert_eq!(x.fae_hostility_q, -r.fae_hostility_q, "strife lane must mirror");
        assert_eq!(x.crown_temptation_q, -r.crown_temptation_q, "crown lane must mirror");
        assert!(x.fae_hostility_q > 0 && r.fae_hostility_q < 0, "poles are the right way round");
    }

    /// The Bargain/Replace pair mirrors on its own axis: one takes a bond on, the other sheds
    /// one. They are not sign-negations across every lane, and the test says which lane carries
    /// the pairing rather than pretending the whole row flips.
    #[test]
    fn the_binding_pair_mirrors_on_obligation() {
        let e = ethics();
        let b = *e.modifiers_for(CraftingEthicsPath::Bargain).expect("Bargain");
        let r = *e.modifiers_for(CraftingEthicsPath::Replace).expect("Replace");
        assert!(b.obligation_pressure_q > 0, "a bargain takes a bond on");
        assert!(r.obligation_pressure_q < 0, "a replacement sheds one");
    }

    #[test]
    fn the_fae_reward_catalog_holds_all_five_outcomes() {
        let c = fae_item_pressure();
        assert_eq!(c.outcomes.len(), 5);
        for o in FaeItemOutcome::ALL {
            assert!(c.pressure_for(o).is_some(), "{o:?} has no authored pressure");
        }
    }

    /// Every lane is a balanced trit — `-1`, `0`, or `+1`, never a magnitude that leaked
    /// through. Five lanes at three states is the `3^5 = 243` shape of `TritCell5D`.
    #[test]
    fn every_pressure_lane_is_balanced_ternary() {
        for p in fae_item_pressure().outcomes {
            for lane in p.trits() {
                assert!((-1..=1).contains(&lane), "{:?} lane outside ternary", p.outcome);
            }
        }
    }

    /// THE FIND: stealing and claiming push the world in the SAME direction on all five lanes.
    /// They are not different acts ethically — theft is claiming, harder, and it is the only
    /// outcome that leaves a trace. If this ever stops holding, the moral model changed.
    #[test]
    fn stealing_is_claiming_only_louder_and_detectable() {
        let c = fae_item_pressure();
        let claimed = *c.pressure_for(FaeItemOutcome::Claimed).expect("Claimed");
        let stolen = *c.pressure_for(FaeItemOutcome::Stolen).expect("Stolen");
        assert_eq!(claimed.trits(), stolen.trits(), "same direction on every lane");
        assert!(stolen.ownership_pressure_q > claimed.ownership_pressure_q, "louder");
        assert!(stolen.fae_hostility_q > claimed.fae_hostility_q, "louder");
        assert!(stolen.shimmer_detectable, "theft leaves a trace");
        assert!(!claimed.shimmer_detectable, "an open claim does not");
        assert_eq!(
            c.outcomes.iter().filter(|p| p.shimmer_detectable).count(),
            1,
            "exactly one outcome is detectable, and it is the theft"
        );
    }

    /// Claimed -> Refused mirrors on FOUR lanes, not five — and refusal is ACTIVE SEVERANCE.
    ///
    /// An earlier draft claimed an exact sign-inverse. It is not: Claimed carries obligation 0
    /// and Refused carries -500, and `-0 != -500`. Refusal is not a passive vacuum. Rejecting
    /// an offer sheds an existing bond rather than leaving it untouched.
    ///
    /// The obligation lane is asserted at its RAW MAGNITUDE, not merely its sign. Asserting
    /// only the trit would let a future tidy-up rewrite -500 as -1 and still pass, quietly
    /// flattening authored narrative design into matrix symmetry. The exact number IS the
    /// mechanic; this test is the thing standing in front of it.
    #[test]
    fn refusing_mirrors_claiming_on_four_lanes_and_severs_a_bond_on_the_fifth() {
        let c = fae_item_pressure();
        let a = *c.pressure_for(FaeItemOutcome::Claimed).expect("Claimed");
        let r = *c.pressure_for(FaeItemOutcome::Refused).expect("Refused");

        // The clean inversion, lane by lane, on signs.
        assert_eq!(r.ownership_pressure_q.signum(), -a.ownership_pressure_q.signum());
        assert_eq!(r.crown_temptation_q.signum(), -a.crown_temptation_q.signum());
        assert_eq!(r.fae_hostility_q.signum(), -a.fae_hostility_q.signum());
        assert_eq!(r.ecology_pressure_q.signum(), -a.ecology_pressure_q.signum());

        // The fifth lane, pinned at magnitude — the asymmetry is deliberate, not a rounding.
        assert_eq!(a.obligation_pressure_q, 0, "claiming takes on no bond");
        assert_eq!(
            r.obligation_pressure_q, -500,
            "refusing must ACTIVELY SEVER a bond — not merely fail to take one on"
        );
    }

    /// RON round-trip: the embedded text must survive serialise/deserialise unchanged. A table
    /// that parses but silently drops a field would otherwise pass every count test above.
    #[test]
    fn both_catalogs_round_trip_through_ron() {
        let s = substrates();
        let s2: SubstrateCatalog =
            ron::from_str(&ron::ser::to_string(&s).expect("serialise")).expect("re-parse");
        assert_eq!(s, s2);

        let e = ethics();
        let e2: EthicsCatalog =
            ron::from_str(&ron::ser::to_string(&e).expect("serialise")).expect("re-parse");
        assert_eq!(e, e2);
    }
}
