//! Laban effort → keyframe compiler — the motion-laban skill's named open seam
//! ("the effort→curve adapter is the remaining work, not the rig itself"),
//! closed against the LIVE substrate (gamecompiler run 2026-08-18, knowledge
//! drop `.forge/knowledge-drops/2026-08-18-forgevision-lab-2d3d.md`).
//!
//! Laban Movement Analysis describes PERCEIVED motion only. Everything this
//! module emits is `Lane::Speculative` — cosmetic, decimatable under tick
//! strain, and it never writes the sim (the skill's firewall; the lane is
//! const-locked in `forge_core_v3::spine`).
//!
//! The compiler's found isomorphism: Laban's four effort axes are BIPOLAR
//! WITH A NEUTRAL — light/neutral/strong, sustained/neutral/sudden,
//! indirect/neutral/direct, free/neutral/bound. That is balanced ternary.
//! Four effort trits + one phase trit = five trits = one `TritCell5D`:
//! **a motion phrase's personality is one byte**, and the pack/unpack is
//! L07-bijection-tested below against `TritCell5D::trits()` itself.
//!
//! Authored constants ([`MIN_READABLE_RATIO`], [`INPUT_REPEAT_TICKS`],
//! amplitude/grid tables) are L12 `Authored` — design choices made here,
//! marked, test-held; no quarry source pins them.

use crate::bone_timeline::BoneTimeline;
use forge_core_v3::atom::TritCell5D;
use forge_core_v3::fixed_point::MilliUnit;
use forge_core_v3::spine::Lane;

/// Weight — the mass the gesture reads as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weight {
    /// Reads light (trit -1).
    Light,
    /// Unmarked (trit 0).
    Neutral,
    /// Reads strong (trit +1).
    Strong,
}

/// Time — the urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffortTime {
    /// Sustained (trit -1).
    Sustained,
    /// Unmarked (trit 0).
    Neutral,
    /// Sudden (trit +1).
    Sudden,
}

/// Space — the intent's focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Space {
    /// Indirect, curving (trit -1).
    Indirect,
    /// Unmarked (trit 0).
    Neutral,
    /// Direct (trit +1).
    Direct,
}

/// Flow — how much the motion can be arrested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flow {
    /// Free — runs out on the spring (trit -1).
    Free,
    /// Unmarked (trit 0).
    Neutral,
    /// Bound — arrested, settles explicitly (trit +1).
    Bound,
}

/// Which phrase segment a packed cell describes (the fifth trit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Wind-up (trit -1).
    Anticipation,
    /// Impact (trit 0).
    Strike,
    /// Settle (trit +1).
    Recovery,
}

/// One Laban effort: a point in the four-axis space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Effort {
    /// Mass axis.
    pub weight: Weight,
    /// Urgency axis.
    pub time: EffortTime,
    /// Focus axis.
    pub space: Space,
    /// Arrest axis.
    pub flow: Flow,
}

impl Weight {
    /// Balanced-ternary reading.
    pub const fn trit(self) -> i8 {
        match self {
            Weight::Light => -1,
            Weight::Neutral => 0,
            Weight::Strong => 1,
        }
    }
}
impl EffortTime {
    /// Balanced-ternary reading.
    pub const fn trit(self) -> i8 {
        match self {
            EffortTime::Sustained => -1,
            EffortTime::Neutral => 0,
            EffortTime::Sudden => 1,
        }
    }
}
impl Space {
    /// Balanced-ternary reading.
    pub const fn trit(self) -> i8 {
        match self {
            Space::Indirect => -1,
            Space::Neutral => 0,
            Space::Direct => 1,
        }
    }
}
impl Flow {
    /// Balanced-ternary reading.
    pub const fn trit(self) -> i8 {
        match self {
            Flow::Free => -1,
            Flow::Neutral => 0,
            Flow::Bound => 1,
        }
    }
}
impl Phase {
    /// Balanced-ternary reading.
    pub const fn trit(self) -> i8 {
        match self {
            Phase::Anticipation => -1,
            Phase::Strike => 0,
            Phase::Recovery => 1,
        }
    }
}

/// Pack an effort + phase into one [`TritCell5D`] — trit order
/// [weight, time, space, flow, phase], balanced offset encoding
/// (all-zero = the atom's ORIGIN). Bijection with `TritCell5D::trits()`
/// is test-held (L07).
pub fn pack_phrase_cell(effort: Effort, phase: Phase) -> TritCell5D {
    let trits = [
        effort.weight.trit(),
        effort.time.trit(),
        effort.space.trit(),
        effort.flow.trit(),
        phase.trit(),
    ];
    let mut v: u8 = 0;
    let mut place: u8 = 1;
    let mut i = 0;
    while i < 5 {
        v += ((trits[i] + 1) as u8) * place;
        place *= 3;
        i += 1;
    }
    TritCell5D(v)
}

/// One motion phrase: the effort plus its tick envelope. All ticks are on the
/// authoring grid's clock (the 120 Hz UI lane — never the sim's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionPhrase {
    /// The gesture's personality.
    pub effort: Effort,
    /// Wind-up length, ticks.
    pub anticipation_ticks: u32,
    /// Impact length, ticks.
    pub impact_ticks: u32,
    /// Held frame on impact, ticks.
    pub hit_stop_ticks: u32,
    /// Settle length, ticks.
    pub recovery_ticks: u32,
}

/// Everything this module emits rides the cosmetic lane — decimatable,
/// never authority (the skill's firewall).
pub const LABAN_LANE: Lane = Lane::Speculative;

/// Readability floor: anticipation must be at least this multiple of impact
/// (`Authored` — the guard_overhead reference phrase is 18t/4t = 4.5x).
pub const MIN_READABLE_RATIO: u32 = 2;

/// Input-repeat interval on the 120 Hz lane, ticks. `[ASSUMED]` 6 ticks
/// (50 ms) — hit-stop at or under this can never eat a repeated input.
pub const INPUT_REPEAT_TICKS: u32 = 6;

/// Gesture amplitude by weight, MilliUnits (`Authored` — mirrors the weapon
/// wireframe weight_milli bands: light 600 / standard 1000 / heavy 1600).
pub const fn amplitude_mu(weight: Weight) -> i64 {
    match weight {
        Weight::Light => 600,
        Weight::Neutral => 1_000,
        Weight::Strong => 1_600,
    }
}

/// Authoring snap grid by time effort, ticks (`Authored` — sustained motion
/// authors on wide beats, sudden on tight ones).
pub const fn snap_grid(time: EffortTime) -> u32 {
    match time {
        EffortTime::Sustained => 8,
        EffortTime::Neutral => 4,
        EffortTime::Sudden => 2,
    }
}

// ── Rulify: the skill's animation gates as predicates ───────────────────────

/// Gate 1 — anticipation readable: wind-up at least [`MIN_READABLE_RATIO`] x
/// the impact, and never zero.
pub fn anticipation_readable(p: &MotionPhrase) -> bool {
    p.anticipation_ticks > 0 && p.anticipation_ticks >= p.impact_ticks * MIN_READABLE_RATIO
}

/// Gate 2 — recovery fair: heavier gestures pay longer. Required settle is
/// `impact x (weight_trit + 2)`: light 1x, neutral 2x, strong 3x (`Authored`).
pub fn recovery_fair(p: &MotionPhrase) -> bool {
    let factor = (p.effort.weight.trit() + 2) as u32;
    p.recovery_ticks >= p.impact_ticks * factor
}

/// Gate 3 — hit-stop preserves input feel: the held frame never outlasts the
/// input-repeat interval.
pub fn hit_stop_preserves_input(p: &MotionPhrase) -> bool {
    p.hit_stop_ticks <= INPUT_REPEAT_TICKS
}

/// All gates at once — a phrase must pass before it compiles.
pub fn phrase_legal(p: &MotionPhrase) -> bool {
    anticipation_readable(p) && recovery_fair(p) && hit_stop_preserves_input(p)
}

// ── Condense: effort → keyframes (the adapter itself) ───────────────────────

/// Why a phrase refused to compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhraseError {
    /// Wind-up too short to read.
    Unreadable,
    /// Settle too short for the weight.
    UnfairRecovery,
    /// Held frame would eat inputs.
    BreaksInputFeel,
}

/// Compile one phrase into a single-bone [`BoneTimeline`] (v0: the striking
/// tip; multi-bone spreads ride the same shape later). Key layout on the
/// gesture's own axis (y = MilliUnits of displacement):
///
/// - t0: origin.
/// - anticipation peak: wind-up AWAY from the strike (-amp/2) — the readable
///   telegraph.
/// - impact: full +amp.
/// - impact + hit_stop: the SAME pose again — Catmull-Rom passes exactly
///   through both knots, so the held frame is two identical grid keys, a
///   plateau the sampler cannot overshoot.
/// - recovery end: back to origin iff flow is Bound; Free flow ends at the
///   impact plateau and lets the catch-up spring run it out (skill: "the
///   integer springs catch it up").
/// - Indirect space adds one off-axis key (x = amp/2) mid-anticipation —
///   the curving approach.
pub fn compile_phrase(p: &MotionPhrase) -> Result<BoneTimeline, PhraseError> {
    if !anticipation_readable(p) {
        return Err(PhraseError::Unreadable);
    }
    if !recovery_fair(p) {
        return Err(PhraseError::UnfairRecovery);
    }
    if !hit_stop_preserves_input(p) {
        return Err(PhraseError::BreaksInputFeel);
    }

    let amp = amplitude_mu(p.effort.weight);
    let mut tl = BoneTimeline::new(snap_grid(p.effort.time));
    let origin = [MilliUnit(0), MilliUnit(0), MilliUnit(0)];

    tl.set_key(0, origin);
    if p.effort.space == Space::Indirect {
        tl.set_key(
            p.anticipation_ticks / 2,
            [MilliUnit(amp / 2), MilliUnit(-amp / 4), MilliUnit(0)],
        );
    }
    tl.set_key(p.anticipation_ticks, [MilliUnit(0), MilliUnit(-amp / 2), MilliUnit(0)]);
    let impact_t = p.anticipation_ticks + p.impact_ticks;
    let strike = [MilliUnit(0), MilliUnit(amp), MilliUnit(0)];
    // set_key returns the SNAPPED tick; the hold is placed relative to where
    // the strike actually landed. Plateau law: a held frame occupies at least
    // one grid step — a hit-stop shorter than the authoring grid rounds UP to
    // the next grid line, never onto the strike's own knot (which would erase
    // the plateau entirely, as the first red run of this gate proved).
    let st_impact = tl.set_key(impact_t, strike);
    let hold_step = p.hit_stop_ticks.max(tl.grid).max(1);
    let st_hold = tl.set_key(st_impact + hold_step, strike);
    if p.effort.flow == Flow::Bound {
        tl.set_key(st_hold + p.recovery_ticks, origin);
    }
    Ok(tl)
}

/// The skill's own reference phrase — "a heavy guard winding up a slow
/// overhead" (motion-laban SKILL.md:36-43), as data.
pub const fn guard_overhead() -> MotionPhrase {
    MotionPhrase {
        effort: Effort {
            weight: Weight::Strong,
            time: EffortTime::Sustained,
            space: Space::Direct,
            flow: Flow::Bound,
        },
        anticipation_ticks: 18,
        impact_ticks: 4,
        hit_stop_ticks: 3,
        recovery_ticks: 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L07: pack → `TritCell5D::trits()` → the same trits, for every corner of
    /// the effort space and every phase — the atom itself is the decoder.
    #[test]
    fn laban_pack_bijects_with_the_atoms_own_decode() {
        let weights = [Weight::Light, Weight::Neutral, Weight::Strong];
        let times = [EffortTime::Sustained, EffortTime::Neutral, EffortTime::Sudden];
        let spaces = [Space::Indirect, Space::Neutral, Space::Direct];
        let flows = [Flow::Free, Flow::Neutral, Flow::Bound];
        let phases = [Phase::Anticipation, Phase::Strike, Phase::Recovery];
        for w in weights {
            for t in times {
                for s in spaces {
                    for f in flows {
                        for ph in phases {
                            let effort = Effort { weight: w, time: t, space: s, flow: f };
                            let cell = pack_phrase_cell(effort, ph);
                            let trits = cell.trits().expect("packed cell is always a legal cell");
                            assert_eq!(
                                trits,
                                [w.trit(), t.trit(), s.trit(), f.trit(), ph.trit()],
                                "pack/decode disagree for {effort:?} {ph:?}"
                            );
                        }
                    }
                }
            }
        }
        // The unmarked phrase sits on the atom's ORIGIN by construction.
        let neutral = Effort {
            weight: Weight::Neutral,
            time: EffortTime::Neutral,
            space: Space::Neutral,
            flow: Flow::Neutral,
        };
        assert_eq!(pack_phrase_cell(neutral, Phase::Strike).0, 121);
    }

    /// The skill's reference phrase passes every gate and rides the
    /// Speculative lane.
    #[test]
    fn laban_guard_overhead_is_legal_and_cosmetic() {
        let p = guard_overhead();
        assert!(anticipation_readable(&p), "18t wind-up over 4t impact reads");
        assert!(recovery_fair(&p), "strong pays 3x impact; 12 >= 12");
        assert!(hit_stop_preserves_input(&p), "3t hold under the 6t repeat");
        assert!(phrase_legal(&p));
        assert_eq!(LABAN_LANE, Lane::Speculative, "perceived motion never writes the sim");
    }

    /// Each gate refuses its own violation, with the typed error.
    #[test]
    fn laban_gates_refuse_their_violations() {
        let mut p = guard_overhead();
        p.anticipation_ticks = 3;
        assert_eq!(compile_phrase(&p).unwrap_err(), PhraseError::Unreadable);

        let mut p = guard_overhead();
        p.recovery_ticks = 5;
        assert_eq!(compile_phrase(&p).unwrap_err(), PhraseError::UnfairRecovery);

        let mut p = guard_overhead();
        p.hit_stop_ticks = 20;
        assert_eq!(compile_phrase(&p).unwrap_err(), PhraseError::BreaksInputFeel);
    }

    /// Compiled shape: keys sorted on the grid, wind-up telegraphs away from
    /// the strike, the hit-stop is a plateau (two identical knots the sampler
    /// lands on exactly), bound flow settles home.
    #[test]
    fn laban_compiled_guard_overhead_has_the_phrase_shape() {
        let p = guard_overhead();
        let tl = compile_phrase(&p).expect("reference phrase compiles");
        let keys = tl.keys();
        assert_eq!(keys.len(), 5, "origin, wind-up, impact, hold, settle");
        for pair in keys.windows(2) {
            assert!(pair[0].tick < pair[1].tick, "keys sorted");
        }
        let amp = amplitude_mu(Weight::Strong);
        assert_eq!(keys[1].pos[1], MilliUnit(-amp / 2), "wind-up goes AWAY from the strike");
        assert_eq!(keys[2].pos[1], MilliUnit(amp), "impact hits full amplitude");
        assert_eq!(keys[2].pos, keys[3].pos, "hit-stop is a held plateau");
        assert_eq!(keys[4].pos[1], MilliUnit(0), "bound flow settles home");
        // The sampler lands exactly on the plateau (snap law).
        assert_eq!(tl.sample(keys[2].tick), Some(keys[2].pos));
    }

    /// Effort steers the compile: weight scales amplitude monotonically,
    /// sudden time authors on a tighter grid than sustained, indirect space
    /// adds the curving key, free flow never settles home.
    #[test]
    fn laban_effort_axes_steer_the_curve() {
        let mut light = guard_overhead();
        light.effort.weight = Weight::Light;
        light.recovery_ticks = 4; // light pays 1x impact
        let l = compile_phrase(&light).unwrap();
        let s = compile_phrase(&guard_overhead()).unwrap();
        assert!(s.keys()[2].pos[1].0 > l.keys()[2].pos[1].0, "strong strikes bigger than light");

        assert!(snap_grid(EffortTime::Sudden) < snap_grid(EffortTime::Sustained));

        let mut indirect = guard_overhead();
        indirect.effort.space = Space::Indirect;
        assert_eq!(
            compile_phrase(&indirect).unwrap().len(),
            6,
            "indirect adds the curving approach key"
        );

        let mut free = guard_overhead();
        free.effort.flow = Flow::Free;
        let f = compile_phrase(&free).unwrap();
        let last = f.keys()[f.len() - 1];
        assert_ne!(last.pos[1], MilliUnit(0), "free flow ends on the spring, not at home");
    }
}
