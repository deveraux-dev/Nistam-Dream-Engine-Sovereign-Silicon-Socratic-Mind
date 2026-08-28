//! The Thought Cabinet — the Sevenfold registers as internal voices that argue
//! with you. No new state: every voice is a READER of `hermetics::HermeticStats`,
//! and how loudly it speaks is that register's own value.

use crate::hermetics::{HermeticStats, RegisterPool, Stat};

/// Register value at which a voice first becomes audible at all.
pub const MURMUR_AT: u8 = 40;
/// Register value at which a voice speaks plainly.
pub const SPEAKS_AT: u8 = 96;
/// Register value at which a voice pushes.
pub const INSISTS_AT: u8 = 160;
/// Register value past which a voice stops advising and starts overriding.
pub const OVERRIDES_AT: u8 = 216;

/// How hard a register is pressing on this decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    /// Below hearing. The register is there; it has nothing to say.
    Silent,
    /// A half-thought you can dismiss.
    Murmur,
    /// A clear argument.
    Speaking,
    /// It will not let the point go.
    Insistent,
    /// It is no longer advising you — it is answering FOR you. High is not
    /// free: a register loud enough to override is a liability, which is the
    /// whole point of the thought cabinet as a build system.
    Overriding,
}

impl Tone {
    /// True when the voice reaches the transcript at all.
    pub fn audible(self) -> bool {
        self != Tone::Silent
    }

    /// True when the voice has stopped being counsel. A caller that ignores
    /// this reads an override as if it were advice.
    pub fn is_override(self) -> bool {
        self == Tone::Overriding
    }
}

/// The tone a register speaks in at its current value.
pub fn tone_of(stats: &HermeticStats, stat: Stat) -> Tone {
    match stat.read(stats) {
        v if v >= OVERRIDES_AT => Tone::Overriding,
        v if v >= INSISTS_AT => Tone::Insistent,
        v if v >= SPEAKS_AT => Tone::Speaking,
        v if v >= MURMUR_AT => Tone::Murmur,
        _ => Tone::Silent,
    }
}

/// The register's confidence in permyriad — its value against the 8-bit ceiling.
pub fn confidence_pmy(stats: &HermeticStats, stat: Stat) -> u16 {
    (stat.read(stats) as u32 * 10_000 / u8::MAX as u32) as u16
}

/// What this register is called when it speaks. A voice is not its statistic —
/// LogicDepth reasons at you, Tarnish does not.
pub fn voice_name(stat: Stat) -> &'static str {
    match stat {
        Stat::Vigor => "The Body",
        Stat::Momentum => "The Forward Foot",
        Stat::LogicDepth => "Reason",
        Stat::ShadowWeight => "The Weight",
        Stat::Tarnish => "The Rust",
        Stat::Resonance => "The Room",
        Stat::Guilt => "The Ledger",
        Stat::Clarity => "The Quiet",
    }
}

/// What a voice's counsel is WORTH, as distinct from how loudly it is given.
///
/// The three-pool split is not decoration here. An Active register is rolled,
/// so its voice is making a claim about odds it will actually be held to. A
/// Passive register is never rolled — it reports a condition. A Wild register
/// (Guilt, Clarity) accrues from play and is never rolled at all: it does not
/// argue about outcomes, it judges. Treating all eight identically would throw
/// away the one structural fact the Sevenfold spine already carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counsel {
    /// A claim about what will happen if you try. Testable, and falsifiable.
    Wager,
    /// A report on what is already true of you or the room.
    Reading,
    /// A verdict. It is not offering odds and cannot be argued with.
    Judgement,
}

/// The kind of counsel a register gives, from its pool.
pub fn counsel_of(stat: Stat) -> Counsel {
    match stat.pool() {
        RegisterPool::Active => Counsel::Wager,
        RegisterPool::Passive => Counsel::Reading,
        RegisterPool::Wild => Counsel::Judgement,
    }
}

/// One voice's standing on a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Speaking {
    /// Which register spoke.
    pub stat: Stat,
    /// How hard it pressed.
    pub tone: Tone,
    /// What its counsel is worth.
    pub counsel: Counsel,
    /// Its confidence, permyriad.
    pub confidence_pmy: u16,
}

/// Every register loud enough to reach the transcript, loudest first.
///
/// Ties break on canonical register order so the same block always produces the
/// same transcript — a cabinet that reorders itself between reads is not a
/// build system, it is noise.
pub fn cabinet(stats: &HermeticStats) -> Vec<Speaking> {
    let mut out: Vec<Speaking> = Stat::ALL
        .iter()
        .map(|&stat| Speaking {
            stat,
            tone: tone_of(stats, stat),
            counsel: counsel_of(stat),
            confidence_pmy: confidence_pmy(stats, stat),
        })
        .filter(|s| s.tone.audible())
        .collect();
    out.sort_by(|a, b| {
        b.confidence_pmy
            .cmp(&a.confidence_pmy)
            .then_with(|| a.stat.index().cmp(&b.stat.index()))
    });
    out
}

/// The register currently answering FOR the body, if any has grown loud enough
/// to. At most one — the loudest override wins, ties on canonical order.
pub fn overriding(stats: &HermeticStats) -> Option<Speaking> {
    cabinet(stats).into_iter().find(|s| s.tone.is_override())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(vigor: u8, logic: u8, tarnish: u8, guilt: u8) -> HermeticStats {
        HermeticStats { vigor, logic_depth: logic, tarnish, guilt, ..Default::default() }
    }

    #[test]
    fn a_low_register_has_nothing_to_say() {
        let stats = block(10, 0, 0, 0);
        assert_eq!(tone_of(&stats, Stat::Vigor), Tone::Silent);
        assert!(cabinet(&stats).is_empty(), "a quiet body speaks with no voices");
    }

    #[test]
    fn the_tone_climbs_with_the_register() {
        let at = |v: u8| tone_of(&block(v, 0, 0, 0), Stat::Vigor);
        assert_eq!(at(MURMUR_AT - 1), Tone::Silent);
        assert_eq!(at(MURMUR_AT), Tone::Murmur);
        assert_eq!(at(SPEAKS_AT), Tone::Speaking);
        assert_eq!(at(INSISTS_AT), Tone::Insistent);
        assert_eq!(at(OVERRIDES_AT), Tone::Overriding);
        assert_eq!(at(u8::MAX), Tone::Overriding);
    }

    /// The build-system half: a maxed register is not a pure win. It stops
    /// advising and starts answering, which a caller must be able to see.
    #[test]
    fn a_maxed_register_overrides_instead_of_advising() {
        let stats = block(250, 0, 0, 0);
        let voice = overriding(&stats).expect("a maxed register must override");
        assert_eq!(voice.stat, Stat::Vigor);
        assert!(voice.tone.is_override());
        assert_eq!(voice_name(voice.stat), "The Body");
    }

    #[test]
    fn a_merely_loud_register_still_only_advises() {
        let stats = block(INSISTS_AT, 0, 0, 0);
        assert!(overriding(&stats).is_none(), "insistent is not overriding");
        assert_eq!(cabinet(&stats).len(), 1);
    }

    /// The pool split does real work: the same tone means different things.
    #[test]
    fn the_three_pools_give_three_kinds_of_counsel() {
        assert_eq!(counsel_of(Stat::Vigor), Counsel::Wager, "an active register is rolled");
        assert_eq!(counsel_of(Stat::LogicDepth), Counsel::Wager);
        assert_eq!(counsel_of(Stat::Tarnish), Counsel::Reading, "a passive one reports");
        assert_eq!(counsel_of(Stat::Guilt), Counsel::Judgement, "a wild one judges");
        assert_eq!(counsel_of(Stat::Clarity), Counsel::Judgement);
    }

    #[test]
    fn every_register_has_a_pool_a_name_and_a_counsel() {
        for stat in Stat::ALL {
            assert!(!voice_name(stat).is_empty(), "{stat:?} has no voice");
            let _ = counsel_of(stat);
            let _ = stat.pool();
        }
    }

    #[test]
    fn the_cabinet_speaks_loudest_first_and_replays_the_same_way() {
        let stats = block(200, 120, 0, 60);
        let first = cabinet(&stats);
        assert_eq!(first[0].stat, Stat::Vigor, "the loudest speaks first");
        assert_eq!(first.len(), 3, "three registers clear the murmur floor");
        assert_eq!(first, cabinet(&stats), "the same block must replay identically");
        for pair in first.windows(2) {
            assert!(pair[0].confidence_pmy >= pair[1].confidence_pmy, "order must not rise");
        }
    }

    /// Ties are broken by canonical register order, never by chance.
    #[test]
    fn equal_registers_break_on_canonical_order() {
        let stats = HermeticStats { vigor: 120, logic_depth: 120, ..Default::default() };
        let voices = cabinet(&stats);
        assert_eq!(voices[0].stat, Stat::Vigor, "Vigor is index 0");
        assert_eq!(voices[1].stat, Stat::LogicDepth, "LogicDepth is index 2");
    }

    #[test]
    fn confidence_reads_the_register_against_the_ceiling() {
        assert_eq!(confidence_pmy(&block(0, 0, 0, 0), Stat::Vigor), 0);
        assert_eq!(confidence_pmy(&block(u8::MAX, 0, 0, 0), Stat::Vigor), 10_000);
        let half = confidence_pmy(&block(128, 0, 0, 0), Stat::Vigor);
        assert!((4_900..=5_100).contains(&half), "half a register is about half scale: {half}");
    }

    /// The cabinet is a READER — it must never write to the block it reads.
    #[test]
    fn reading_the_cabinet_does_not_touch_the_block() {
        let stats = block(200, 120, 30, 60);
        let before = stats;
        let _ = cabinet(&stats);
        let _ = overriding(&stats);
        assert_eq!(stats, before, "no new state: the cabinet only reads");
    }
}
