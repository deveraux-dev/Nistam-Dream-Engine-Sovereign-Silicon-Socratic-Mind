//! What a body can SENSE — five grants, four suppressors, nine lanes.
//! Ported from v2 `sf-wasm/src/perception.rs`; the registers are the
//! SEVENFOLD's (`crate::hermetics`), never a parallel stat block.

use crate::hermetics::HermeticStats;

/// Full scale. Every lane in this module is permyriad, so the authored lane and
/// the permyriad ceiling are the same number — v2 read this off its sieve
/// registry (`mud_sieve::PERCEPTION_AUTHORED`), which has no v3 home.
pub const AUTHORED_Q: i64 = 10_000;

/// Nothing granted, nothing taken: half the authored lane. A numb body is not a
/// deaf one — total deafness is something DONE to you, never a starting state.
pub const FLOOR_Q: i64 = 5_000;

/// Everything granted, nothing taken: one and a half lanes over authored.
pub const CEILING_Q: i64 = 20_000;

/// What one grant is worth at full. Five grants, evenly weighted — no single
/// register buys the whole parish.
pub const GRANT_WEIGHT_Q: i64 = (CEILING_Q - FLOOR_Q) / 5;

/// Where a body was raised, and what that did to its senses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Upbringing {
    /// Raised indoors, among people. Reads a room, not a horizon.
    Hearthborn,
    /// Raised in open country. Sees weather coming; misses what is whispered.
    Fieldborn,
    /// Raised moving. Watches for the actor before the ground.
    Roadborn,
    /// Raised below grade. At home where there is nothing to see by.
    Cellarborn,
}

impl Upbringing {
    /// The standing grant, permyriad against [`GRANT_WEIGHT_Q`].
    pub const fn grant_q(self) -> i64 {
        match self {
            Self::Hearthborn => 4_000,
            Self::Fieldborn => 7_000,
            Self::Roadborn => 6_000,
            Self::Cellarborn => 5_000,
        }
    }

    /// What this upbringing does to a suppressor, permyriad. Below full scale is
    /// a resistance: the cellar-raised are not blinded by the dark that stops
    /// everyone else.
    pub const fn resists(self, s: Suppressor) -> i64 {
        match (self, s) {
            (Self::Cellarborn, Suppressor::Shadowed) => 4_000,
            (Self::Roadborn, Suppressor::Blocked) => 6_000,
            (Self::Fieldborn, Suppressor::Muted) => 7_000,
            (Self::Hearthborn, Suppressor::Dulled) => 7_000,
            _ => AUTHORED_Q,
        }
    }

    /// The spoken name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Hearthborn => "hearthborn",
            Self::Fieldborn => "fieldborn",
            Self::Roadborn => "roadborn",
            Self::Cellarborn => "cellarborn",
        }
    }
}

/// The four ways a sense is taken from you.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suppressor {
    /// A bad actor is working at it — someone in the room wants you deaf.
    Blocked,
    /// Saturn's register: SHA, the weight that absorbs before it reaches you.
    Shadowed,
    /// What you carry silences the world. The HEAR face only.
    Muted,
    /// Venus's register: TAR, decay in the instrument itself.
    Dulled,
}

/// One body's senses. Grants are permyriad of their own weight; suppressors are
/// permyriad of TOTAL suppression, so 0 is clear air and full scale is nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Senses {
    /// LOG — reasoning fills in what the ear missed.
    pub logic_q: i64,
    /// RES — knowing what a thing means.
    pub wisdom_q: i64,
    /// Where you were raised, standing.
    pub upbringing_q: i64,
    /// What the ROOM grants — an open room carries, a dead end does not.
    pub sightline_q: i64,
    /// The growth vector for being tuned in.
    pub attunement_q: i64,
    /// A bad actor working against you.
    pub blocked_q: i64,
    /// SHA.
    pub shadowed_q: i64,
    /// What you carry. HEAR face only.
    pub muted_q: i64,
    /// TAR.
    pub dulled_q: i64,
    /// Applies the upbringing's resistances.
    pub upbringing: Option<Upbringing>,
}

/// A `u8` sevenfold register as permyriad.
#[inline]
fn reg_q(v: u8) -> i64 {
    v as i64 * AUTHORED_Q / u8::MAX as i64
}

impl Senses {
    /// Read the sevenfold registers that speak to perception — LOG and RES
    /// grant, SHA and TAR take — and take the three the sevenfold does NOT own
    /// from the caller: where you were raised, what the room affords, and who is
    /// working on you.
    pub fn of(
        stats: &HermeticStats,
        upbringing: Upbringing,
        sightline_q: i64,
        wisdom_q: i64,
        blocked_q: i64,
    ) -> Self {
        Self {
            logic_q: reg_q(stats.logic_depth),
            wisdom_q: wisdom_q.clamp(0, AUTHORED_Q),
            upbringing_q: upbringing.grant_q(),
            sightline_q: sightline_q.clamp(0, AUTHORED_Q),
            attunement_q: reg_q(stats.resonance),
            blocked_q: blocked_q.clamp(0, AUTHORED_Q),
            shadowed_q: reg_q(stats.shadow_weight),
            muted_q: 0,
            dulled_q: reg_q(stats.tarnish),
            upbringing: Some(upbringing),
        }
    }

    /// Your own noise, as a HEAR-face suppressor. The body that came in roaring
    /// is the one that cannot hear the room.
    pub fn muted_by(mut self, own_noise_q: i64) -> Self {
        self.muted_q = own_noise_q.clamp(0, AUTHORED_Q);
        self
    }

    /// Everything granted, before anything is taken.
    pub fn granted_q(&self) -> i64 {
        let share = |g: i64| g.clamp(0, AUTHORED_Q) * GRANT_WEIGHT_Q / AUTHORED_Q;
        FLOOR_Q
            + share(self.logic_q)
            + share(self.wisdom_q)
            + share(self.upbringing_q)
            + share(self.sightline_q)
            + share(self.attunement_q)
    }

    /// One suppressor's surviving fraction after the upbringing's resistance.
    fn survives(&self, s: Suppressor, raw_q: i64) -> i64 {
        let resist = self.upbringing.map_or(AUTHORED_Q, |u| u.resists(s));
        let effective = raw_q.clamp(0, AUTHORED_Q) * resist / AUTHORED_Q;
        AUTHORED_Q - effective
    }

    /// How far the world reaches you. Suppressors MULTIPLY rather than sum — two
    /// half-blindings leave a quarter, and no single one can push below nothing.
    pub fn resolve(&self) -> i64 {
        let mut q = self.granted_q();
        for (s, raw) in [
            (Suppressor::Blocked, self.blocked_q),
            (Suppressor::Shadowed, self.shadowed_q),
            (Suppressor::Dulled, self.dulled_q),
        ] {
            q = q * self.survives(s, raw) / AUTHORED_Q;
        }
        q.clamp(0, CEILING_Q)
    }

    /// The same, for the HEAR face — what you carry silences you on top of it.
    pub fn resolve_heard(&self) -> i64 {
        let q = self.resolve() * self.survives(Suppressor::Muted, self.muted_q) / AUTHORED_Q;
        q.clamp(0, CEILING_Q)
    }
}

/// What you are wearing as a body. Shifting form does not adjust a stat block —
/// it REPLACES the sensory apparatus. The lattice underneath does not change;
/// what can reach across it does.
///
/// Each form rewrites the same nine lanes. Nothing here is a bonus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Form {
    /// Eyes, ears, a throat that gives you away. The only form that pays for all
    /// four suppressors at once, so everything else is measured against it.
    #[default]
    Mortal,
    /// Smokeless fire. A wall is a density you route around; a shut door is a
    /// pressure difference. You do not have eyes; you have the room.
    Djinn,
    /// You went cold on purpose. No breath, so no noise, so nothing to mask and
    /// nothing to give you away. Light is meaningless to you now.
    Lich,
    /// The nose runs the body. You track what a room has been doing, you are
    /// loud, and you do not care.
    Werewolf,
    /// Rooted. Blind to open air, but every footfall on ground you have grown
    /// through arrives in the roots before the walker does.
    Treant,
    /// Off the floor entirely. Sensitivity enormous and untrustworthy — the one
    /// body that hears things which are not there and cannot tell which.
    Wraith,
    /// Bone and habit. Weight on stone and nothing else, which is also why
    /// nothing anyone says to you works.
    Skeleton,
}

/// How many forms a body can wear — [`Form::ALL`]'s length, save-codec bound.
pub const FORM_COUNT: u8 = 7;

impl Form {
    /// Every form.
    pub const ALL: [Form; FORM_COUNT as usize] = [
        Form::Mortal,
        Form::Djinn,
        Form::Lich,
        Form::Werewolf,
        Form::Treant,
        Form::Wraith,
        Form::Skeleton,
    ];

    /// The save-codec byte for this form — index into [`Form::ALL`].
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// A form back off its save-codec byte. `None` past [`FORM_COUNT`] — a
    /// corrupt save refuses whole (L10), never defaults to a body.
    pub const fn from_u8(v: u8) -> Option<Form> {
        if v < FORM_COUNT {
            Some(Self::ALL[v as usize])
        } else {
            None
        }
    }

    /// The spoken name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mortal => "mortal",
            Self::Djinn => "djinn",
            Self::Lich => "lich",
            Self::Werewolf => "werewolf",
            Self::Treant => "treant",
            Self::Wraith => "wraith",
            Self::Skeleton => "skeleton",
        }
    }

    /// What it is like in there. Second person, present tense.
    pub const fn body_line(self) -> &'static str {
        match self {
            Self::Mortal => "Two eyes, two ears, and a chest that keeps announcing you.",
            Self::Djinn => "You are the air in here. The door being shut is a detail about the door.",
            Self::Lich => "Nothing of you moves. The room goes on without noticing, which is the point.",
            Self::Werewolf => "The room is a record of what walked through it, and it is still warm.",
            Self::Treant => "The open air is nothing to you. Everything standing on you, you have already counted.",
            Self::Wraith => "You hear more than is here. You have stopped being able to tell which part.",
            Self::Skeleton => "Weight on stone. That is the whole of it, and it has been enough so far.",
        }
    }

    /// Rewrite a mortal reading into this body's. The base carries the trained
    /// registers; the form decides which of them a body like this can even use.
    pub fn wear(self, base: Senses) -> Senses {
        match self {
            Self::Mortal => base,

            // Air needs no sightline and cannot be boxed in by bodies. What it
            // cannot do is stop burning: the fire it is made of is its own noise.
            Self::Djinn => Senses {
                sightline_q: AUTHORED_Q,
                blocked_q: 0,
                shadowed_q: 0,
                muted_q: AUTHORED_Q / 2,
                ..base
            },

            // No breath, no burn, no give-away. Light stops meaning anything, so
            // shadow cannot take from you — but the instincts stayed in the body
            // you gave up, so wisdom carries this one.
            Self::Lich => Senses {
                sightline_q: 0,
                shadowed_q: 0,
                muted_q: 0,
                dulled_q: 0,
                attunement_q: base.wisdom_q,
                ..base
            },

            // The nose is the whole build. Loud costs you nothing you were
            // using, but a room that wants you dead still closes around you.
            Self::Werewolf => Senses {
                attunement_q: AUTHORED_Q,
                sightline_q: base.sightline_q / 2,
                muted_q: AUTHORED_Q,
                dulled_q: 0,
                ..base
            },

            // Blind to air, deaf to talk, and nothing on your own ground goes
            // unrecorded. Being worked on by an actor does not happen to
            // something this large.
            Self::Treant => Senses {
                sightline_q: 0,
                logic_q: base.logic_q / 2,
                wisdom_q: AUTHORED_Q,
                blocked_q: 0,
                muted_q: 0,
                ..base
            },

            // Everything reaches you and you cannot vouch for any of it — the
            // sensitivity is real, the reliability is gone.
            Self::Wraith => Senses {
                sightline_q: AUTHORED_Q,
                attunement_q: AUTHORED_Q,
                logic_q: base.logic_q / 4,
                blocked_q: 0,
                shadowed_q: 0,
                muted_q: 0,
                dulled_q: AUTHORED_Q / 4,
                ..base
            },

            // Vibration through bone. Everything social, spoken or implied is
            // not addressed to anything you still have.
            Self::Skeleton => Senses {
                logic_q: 0,
                wisdom_q: 0,
                upbringing_q: 0,
                sightline_q: 0,
                attunement_q: base.attunement_q / 2,
                blocked_q: 0,
                shadowed_q: 0,
                muted_q: 0,
                dulled_q: 0,
                ..base
            },
        }
    }
}

/// Whether a watcher notices a body it is looking for.
///
/// Masking is symmetric and this is the side that makes it a trade: a quarry
/// roaring at full is deafened, AND is inside a wall of its own sound the
/// watcher has to hear through. Returns permyriad confidence, 0 = missed.
pub fn noticed_q(
    watcher: &Senses,
    quarry_noise_q: i64,
    distance_cells: i64,
    reach_cells: i64,
) -> i64 {
    if reach_cells <= 0 {
        return 0;
    }
    let reach = watcher.resolve() * reach_cells / AUTHORED_Q;
    if reach <= 0 || distance_cells > reach {
        return 0;
    }
    let closeness = (reach - distance_cells.max(0)) * AUTHORED_Q / reach;
    let noise = quarry_noise_q.clamp(0, AUTHORED_Q);
    // Past the halfway mark the wall of sound starts hiding the body inside it.
    let cover = if noise > AUTHORED_Q / 2 { noise - AUTHORED_Q / 2 } else { 0 };
    (closeness + noise / 4 - cover).clamp(0, AUTHORED_Q)
}

// ── Detection curves (Buckland 2001, distance sampling) ──────────────────────

/// `exp(-t)` in permyriad, sampled at half-integer `t` from 0 to 8.
/// Past `t = 8` the value is under 4 permyriad — below anything this module
/// can act on — so the lookup floors to 0 rather than carrying a longer table.
const EXP_NEG_Q: [i64; 17] = [
    10_000, 6_065, 3_679, 2_231, 1_353, 821, 498, 302, 183, 111, 67, 41, 25, 15, 9, 6, 3,
];

/// `exp(-t)` where `t` is carried in permyriad (`t_q = t * 10_000`).
/// Linear interpolation between table steps — integer throughout, no float.
fn exp_neg_q(t_q: i64) -> i64 {
    if t_q <= 0 {
        return AUTHORED_Q;
    }
    // Each table step is t = 0.5, i.e. 5_000 permyriad.
    let step = t_q / 5_000;
    if step >= (EXP_NEG_Q.len() - 1) as i64 {
        return 0;
    }
    let lo = EXP_NEG_Q[step as usize];
    let hi = EXP_NEG_Q[step as usize + 1];
    let frac = t_q - step * 5_000;
    lo - (lo - hi) * frac / 5_000
}

/// How a lane's carry falls off with distance.
///
/// Replaces the single reach scalar `noticed_q` still takes: a scalar can only
/// describe a wall, and a wall is the one thing a real detection function is
/// not. Two published shapes (Buckland 2001, §2.4), chosen by `falloff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectCurve {
    /// Scale `σ` in cells — how far the lane carries before the fall bites.
    pub shoulder_cells: i64,
    /// `0` selects the half-normal (no shoulder, falls from the first step).
    /// `1..=4` selects the hazard-rate with that exponent: near-certain
    /// detection close in, then a sharper drop. Bigger = squarer shoulder.
    pub falloff: i64,
}

impl DetectCurve {
    /// Ambience: half-normal, `g(r) = exp(-r² / 2σ²)`. No shoulder — it starts
    /// thinning immediately, which is what "trails off soft" actually means.
    pub const fn soft(shoulder_cells: i64) -> Self {
        Self { shoulder_cells, falloff: 0 }
    }

    /// A carrying lane: hazard-rate, `g(r) = 1 - exp(-(r/σ)^-b)`. Flat near the
    /// body, then a real edge — audible all the way out and then not.
    pub const fn hard(shoulder_cells: i64) -> Self {
        Self { shoulder_cells, falloff: 3 }
    }
}

/// Detection confidence at `distance_cells`, permyriad. `AUTHORED_Q` at the
/// body, monotonically falling, 0 once the curve is spent.
///
/// A curve with a non-positive shoulder detects nothing at any distance — a
/// lane that carries zero cells is off, not infinite.
pub fn detect_q(curve: DetectCurve, distance_cells: i64) -> i64 {
    let sigma = curve.shoulder_cells;
    if sigma <= 0 {
        return 0;
    }
    let r = distance_cells.max(0);
    if r == 0 {
        return AUTHORED_Q;
    }

    if curve.falloff <= 0 {
        // Half-normal: t = r² / 2σ².
        let t_q = (r as i128 * r as i128 * AUTHORED_Q as i128
            / (2 * sigma as i128 * sigma as i128)) as i64;
        return exp_neg_q(t_q).clamp(0, AUTHORED_Q);
    }

    // Hazard-rate: t = (r/σ)^-b = (σ/r)^b, g = 1 - exp(-t).
    let b = curve.falloff.min(4) as u32;
    let ratio_q = sigma as i128 * AUTHORED_Q as i128 / r as i128;
    let mut t_q = ratio_q;
    for _ in 1..b {
        t_q = t_q * ratio_q / AUTHORED_Q as i128;
        if t_q > i64::MAX as i128 {
            return AUTHORED_Q;
        }
    }
    let t_q = t_q.min(i64::MAX as i128) as i64;
    (AUTHORED_Q - exp_neg_q(t_q)).clamp(0, AUTHORED_Q)
}

/// The distance where a curve crosses half — the row's own definition of
/// reach, and the number that replaces the scalar.
///
/// Walked outward rather than solved: the curves are monotone, the search is
/// bounded by `probe_limit_cells`, and an integer bisection on a table-backed
/// function would only trade honesty for cycles this module does not need.
pub fn effective_reach_cells(curve: DetectCurve, probe_limit_cells: i64) -> i64 {
    if curve.shoulder_cells <= 0 {
        return 0;
    }
    let limit = probe_limit_cells.max(0);
    let mut last = 0;
    for r in 0..=limit {
        if detect_q(curve, r) < AUTHORED_Q / 2 {
            return last;
        }
        last = r;
    }
    last
}

/// [`noticed_q`] with the wall taken out: the same masking trade, but the
/// carry read off a [`DetectCurve`] so the tell THINS past reach instead of
/// stopping on a step the player can pace out.
///
/// Additive on purpose. Folding this into `noticed_q` itself was tried and
/// REVERTED 2026-08-25: it broke two authored laws that the hard cutoff is
/// holding up, not merely expressing —
/// `the_watcher_reach_decides_the_miss` requires a heavily blocked watcher to
/// score exactly 0 ("the blocked one never knew"), which a curve turns into a
/// faint reading; and `a_roaring_body_is_harder_to_find_than_a_grunting_one`
/// depends on the linear ramp's scale for its noise/cover ordering. Whether a
/// blocked watcher should hear a little is a design ruling, not a refactor.
pub fn noticed_through_q(
    watcher: &Senses,
    quarry_noise_q: i64,
    distance_cells: i64,
    curve: DetectCurve,
) -> i64 {
    let resolve = watcher.resolve();
    if resolve <= 0 || curve.shoulder_cells <= 0 {
        return 0;
    }
    let scaled = DetectCurve {
        shoulder_cells: (resolve * curve.shoulder_cells / AUTHORED_Q).max(1),
        falloff: curve.falloff,
    };
    let closeness = detect_q(scaled, distance_cells.max(0));
    if closeness <= 0 {
        return 0;
    }
    let noise = quarry_noise_q.clamp(0, AUTHORED_Q);
    let cover = if noise > AUTHORED_Q / 2 { noise - AUTHORED_Q / 2 } else { 0 };
    (closeness + noise / 4 - cover).clamp(0, AUTHORED_Q)
}

// ── The social room (Zuberbühler: alarm calls fire on intrusion) ─────────────

/// Disturbance a room must carry before the talk stops. Below this the room
/// absorbs an arrival without breaking stride.
pub const ROOM_NOTICE_Q: i64 = 3_000;

/// Ticks over which an undisturbed room settles back to silence. A room that
/// forgets instantly can be re-startled by pacing in and out of a doorway.
pub const ROOM_SETTLE_TICKS: i64 = 240;

/// A room's social channel — how disturbed the talk in it currently is.
///
/// The tell fires on the TRANSIENT, not the level. An alarm call answers a new
/// intrusion; it does not run continuously while the intruder stands there.
/// Standing in a crowded square must not keep announcing that the room went
/// quiet, which is what a level-triggered version would do every tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SocialRoom {
    disturbance_q: i64,
}

impl SocialRoom {
    /// A room at ease.
    pub const fn at_ease() -> Self {
        Self { disturbance_q: 0 }
    }

    /// Someone arrives. `presence_q` is how much room they take up and
    /// `own_noise_q` how loudly they took it; the disturbance is their product,
    /// so a quiet nobody barely registers and a loud somebody stops the talk.
    ///
    /// Returns `true` only when THIS arrival is what crossed the line — the
    /// room must have been below [`ROOM_NOTICE_Q`] before it. Walking back in
    /// on a room that has not settled raises the level and says nothing, which
    /// is the whole difference between a transient and a threshold.
    pub fn arrive(&mut self, presence_q: i64, own_noise_q: i64) -> bool {
        let presence = presence_q.clamp(0, AUTHORED_Q);
        let noise = own_noise_q.clamp(0, AUTHORED_Q);
        let rise = presence * noise / AUTHORED_Q;
        let was_quiet = self.disturbance_q < ROOM_NOTICE_Q;
        self.disturbance_q = (self.disturbance_q + rise).min(AUTHORED_Q);
        was_quiet && self.disturbance_q >= ROOM_NOTICE_Q
    }

    /// Let `ticks` pass. The room decays linearly back to ease over
    /// [`ROOM_SETTLE_TICKS`] from full.
    pub fn settle(&mut self, ticks: i64) {
        let shed = AUTHORED_Q * ticks.max(0) / ROOM_SETTLE_TICKS;
        self.disturbance_q = (self.disturbance_q - shed).max(0);
    }

    /// How disturbed the room is right now, permyriad.
    pub fn level_q(&self) -> i64 {
        self.disturbance_q
    }

    /// True while the talk has not resumed.
    pub fn is_disturbed(&self) -> bool {
        self.disturbance_q >= ROOM_NOTICE_Q
    }
}

/// What a room affords the eye: every open exit is a line out of it. A sealed
/// room grants nothing; four ways out grants the lot.
pub fn sightline_of(open_exits: usize) -> i64 {
    (open_exits.min(4) as i64) * AUTHORED_Q / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Detection curves ────────────────────────────────────────────────

    #[test]
    fn every_curve_is_certain_at_the_body_and_spent_far_away() {
        for c in [DetectCurve::soft(8), DetectCurve::hard(8)] {
            assert_eq!(detect_q(c, 0), AUTHORED_Q, "{c:?} must be certain at zero");
            assert_eq!(detect_q(c, 10_000), 0, "{c:?} must be spent far out");
        }
    }

    #[test]
    fn a_lane_that_carries_nothing_detects_nothing() {
        assert_eq!(detect_q(DetectCurve::soft(0), 0), 0);
        assert_eq!(detect_q(DetectCurve::hard(-4), 1), 0);
        assert_eq!(effective_reach_cells(DetectCurve::soft(0), 64), 0);
    }

    #[test]
    fn both_curves_fall_monotonically() {
        for c in [DetectCurve::soft(12), DetectCurve::hard(12)] {
            let mut prev = AUTHORED_Q + 1;
            for r in 0..80 {
                let q = detect_q(c, r);
                assert!(q <= prev, "{c:?} rose at r={r}: {q} > {prev}");
                prev = q;
            }
        }
    }

    /// The shape difference the row exists for: a half-normal starts thinning
    /// from the first step, a hazard-rate holds a shoulder near the body.
    #[test]
    fn the_hard_curve_keeps_a_shoulder_the_soft_one_never_has() {
        let sigma = 16;
        let near = sigma / 4;
        let soft = detect_q(DetectCurve::soft(sigma), near);
        let hard = detect_q(DetectCurve::hard(sigma), near);
        assert!(soft < AUTHORED_Q, "the half-normal must already be thinning: {soft}");
        assert!(
            hard > soft,
            "the hazard-rate must hold its shoulder where the half-normal has given ground: hard={hard} soft={soft}"
        );
    }

    /// Reach is now READ off the curve rather than declared beside it.
    #[test]
    fn reach_is_where_the_curve_crosses_half() {
        for c in [DetectCurve::soft(16), DetectCurve::hard(16)] {
            let reach = effective_reach_cells(c, 256);
            assert!(reach > 0, "{c:?} must reach somewhere");
            assert!(
                detect_q(c, reach) >= AUTHORED_Q / 2,
                "{c:?} must still be above half AT its reach"
            );
            assert!(
                detect_q(c, reach + 1) < AUTHORED_Q / 2,
                "{c:?} must be below half one cell past it"
            );
        }
    }

    /// One lattice, two honest shapes: same shoulder, different reach.
    #[test]
    fn the_two_shapes_do_not_reach_the_same_distance() {
        let soft = effective_reach_cells(DetectCurve::soft(16), 256);
        let hard = effective_reach_cells(DetectCurve::hard(16), 256);
        assert_ne!(soft, hard, "two shapes that reach identically are one shape");
    }

    /// A wider shoulder always carries further, whichever shape it wears.
    #[test]
    fn a_wider_shoulder_carries_further() {
        for shape in [DetectCurve::soft, DetectCurve::hard] {
            let near = effective_reach_cells(shape(8), 256);
            let far = effective_reach_cells(shape(24), 256);
            assert!(far > near, "a wider shoulder must carry further: {near} -> {far}");
        }
    }

    /// The whole point of the curve seam: past reach the scalar path returns a
    /// hard 0 while the curve path still carries something.
    #[test]
    fn the_curve_thins_past_reach_where_the_scalar_walls() {
        let keen = Senses { attunement_q: AUTHORED_Q, logic_q: AUTHORED_Q, ..Default::default() };
        let past = 20;
        let walled = noticed_q(&keen, 0, past, 12);
        let thinned = noticed_through_q(&keen, 0, past, DetectCurve::hard(12));
        assert_eq!(walled, 0, "the scalar path stops dead past reach");
        assert!(thinned > 0, "the curve path must still carry: {thinned}");
    }

    #[test]
    fn the_curve_path_still_lets_a_roar_cover_the_body() {
        let keen = Senses { attunement_q: AUTHORED_Q, logic_q: AUTHORED_Q, ..Default::default() };
        let at = |noise| noticed_through_q(&keen, noise, 6, DetectCurve::hard(12));
        assert!(at(AUTHORED_Q) < at(4_000), "a full roar must still hide the body inside it");
    }

    /// A fully suppressed watcher notices nothing through any curve. NOTE: a
    /// `Senses::default()` is NOT blind — the module's own doc calls it "half
    /// the authored lane", a numb body rather than a dead one — so this uses
    /// full blocking, which is what actually zeroes `resolve()`.
    #[test]
    fn a_fully_blocked_watcher_notices_nothing_through_any_curve() {
        let blocked = Senses { blocked_q: AUTHORED_Q, ..Default::default() };
        assert_eq!(blocked.resolve(), 0, "precondition: full blocking zeroes resolve");
        assert_eq!(noticed_through_q(&blocked, 0, 1, DetectCurve::hard(12)), 0);
    }

    /// A zero-carry lane is off through the curve seam too.
    #[test]
    fn a_zero_shoulder_lane_notices_nothing() {
        let keen = Senses { attunement_q: AUTHORED_Q, logic_q: AUTHORED_Q, ..Default::default() };
        assert_eq!(noticed_through_q(&keen, 0, 1, DetectCurve::hard(0)), 0);
    }

    // ── The social room ─────────────────────────────────────────────────

    #[test]
    fn a_loud_arrival_stops_the_talk_and_a_quiet_one_does_not() {
        let mut room = SocialRoom::at_ease();
        assert!(!room.arrive(2_000, 2_000), "a quiet nobody is absorbed");
        assert!(!room.is_disturbed());

        let mut room = SocialRoom::at_ease();
        assert!(room.arrive(AUTHORED_Q, AUTHORED_Q), "a loud somebody stops it");
        assert!(room.is_disturbed());
    }

    /// The row's whole point: the tell fires on the TRANSIENT. Standing in the
    /// room, or walking back into one that has not settled, says nothing.
    #[test]
    fn the_tell_fires_once_on_arrival_not_while_you_stand_there() {
        let mut room = SocialRoom::at_ease();
        assert!(room.arrive(AUTHORED_Q, AUTHORED_Q), "the first arrival is noticed");
        assert!(!room.arrive(AUTHORED_Q, AUTHORED_Q), "the second is not — you are already here");
        assert!(!room.arrive(AUTHORED_Q, AUTHORED_Q));
        assert!(room.is_disturbed(), "though the room is still very much disturbed");
    }

    /// A room that forgets instantly could be re-startled by pacing a doorway.
    #[test]
    fn a_room_must_settle_before_it_can_be_startled_again() {
        let mut room = SocialRoom::at_ease();
        assert!(room.arrive(AUTHORED_Q, AUTHORED_Q));

        room.settle(1);
        assert!(!room.arrive(AUTHORED_Q, AUTHORED_Q), "a moment is not enough");

        room.settle(ROOM_SETTLE_TICKS);
        assert_eq!(room.level_q(), 0, "the room comes fully back to ease");
        assert!(room.arrive(AUTHORED_Q, AUTHORED_Q), "and can be startled anew");
    }

    #[test]
    fn disturbance_never_leaves_the_lane() {
        let mut room = SocialRoom::at_ease();
        for _ in 0..20 {
            room.arrive(AUTHORED_Q, AUTHORED_Q);
        }
        assert_eq!(room.level_q(), AUTHORED_Q, "it cannot climb past full");
        room.settle(ROOM_SETTLE_TICKS * 10);
        assert_eq!(room.level_q(), 0, "nor fall below ease");
    }

    /// Presence and noise multiply: taking up the room quietly, or being loud
    /// while barely there, both fall short of stopping the talk.
    #[test]
    fn presence_and_noise_are_a_product_not_a_sum() {
        let mut big_and_quiet = SocialRoom::at_ease();
        assert!(!big_and_quiet.arrive(AUTHORED_Q, 1_000));
        let mut small_and_loud = SocialRoom::at_ease();
        assert!(!small_and_loud.arrive(1_000, AUTHORED_Q));
        let mut both = SocialRoom::at_ease();
        assert!(both.arrive(6_000, 6_000), "but both together carries");
    }

    #[test]
    fn the_integer_exp_matches_its_table_at_the_sample_points() {
        assert_eq!(exp_neg_q(0), AUTHORED_Q);
        assert_eq!(exp_neg_q(5_000), 6_065, "exp(-0.5)");
        assert_eq!(exp_neg_q(10_000), 3_679, "exp(-1)");
        assert_eq!(exp_neg_q(20_000), 1_353, "exp(-2)");
        assert_eq!(exp_neg_q(80_000), 0, "past the table the value floors");
        let mid = exp_neg_q(7_500);
        assert!(mid < 6_065 && mid > 3_679, "interpolation must sit between steps: {mid}");
    }

    fn stats(logic: u8, res: u8, sha: u8, tar: u8) -> HermeticStats {
        let mut s = HermeticStats::default();
        s.logic_depth = logic;
        s.resonance = res;
        s.shadow_weight = sha;
        s.tarnish = tar;
        s
    }

    #[test]
    fn a_blank_body_sits_at_the_floor() {
        let bare = Senses::default();
        assert_eq!(bare.granted_q(), FLOOR_Q);
        assert_eq!(bare.resolve(), FLOOR_Q);
        let raised = Senses::of(&stats(0, 0, 0, 0), Upbringing::Hearthborn, 0, 0, 0);
        assert!(raised.resolve() > FLOOR_Q, "being raised somewhere is worth something");
    }

    #[test]
    fn each_grant_moves_it_and_none_of_them_owns_it() {
        let full = Senses {
            logic_q: AUTHORED_Q,
            wisdom_q: AUTHORED_Q,
            upbringing_q: AUTHORED_Q,
            sightline_q: AUTHORED_Q,
            attunement_q: AUTHORED_Q,
            ..Default::default()
        };
        assert_eq!(full.resolve(), CEILING_Q);
        for one in [
            Senses { logic_q: AUTHORED_Q, ..Default::default() },
            Senses { wisdom_q: AUTHORED_Q, ..Default::default() },
            Senses { upbringing_q: AUTHORED_Q, ..Default::default() },
            Senses { sightline_q: AUTHORED_Q, ..Default::default() },
            Senses { attunement_q: AUTHORED_Q, ..Default::default() },
        ] {
            assert_eq!(one.resolve(), FLOOR_Q + GRANT_WEIGHT_Q, "grants are evenly weighted");
            assert!(one.resolve() < CEILING_Q, "no single grant may buy the parish");
        }
    }

    #[test]
    fn suppressors_stack_by_multiplying() {
        let base = Senses { attunement_q: AUTHORED_Q, ..Default::default() };
        let clear = base.resolve();
        assert_eq!(Senses { shadowed_q: 5_000, ..base }.resolve(), clear / 2);
        assert_eq!(
            Senses { shadowed_q: 5_000, dulled_q: 5_000, ..base }.resolve(),
            clear / 4,
            "two half-blindings leave a quarter, never nothing"
        );
    }

    #[test]
    fn a_bad_actor_can_take_it_all() {
        let keen = Senses {
            logic_q: AUTHORED_Q,
            wisdom_q: AUTHORED_Q,
            attunement_q: AUTHORED_Q,
            ..Default::default()
        };
        assert_eq!(Senses { blocked_q: AUTHORED_Q, ..keen }.resolve(), 0);
        let half = Senses { blocked_q: 5_000, ..keen }.resolve();
        assert!(half < keen.resolve() && half > 0, "blocking is a scale, not a switch");
    }

    #[test]
    fn where_you_were_raised_answers_one_of_them() {
        let dark = stats(0, 0, 200, 0);
        let cellar = Senses::of(&dark, Upbringing::Cellarborn, 0, 0, 0);
        let hearth = Senses::of(&dark, Upbringing::Hearthborn, 0, 0, 0);
        assert!(cellar.resolve() > hearth.resolve(), "the cellar-raised out-sense in the dark");
    }

    #[test]
    fn what_you_carry_silences_without_blinding() {
        let s = Senses { attunement_q: AUTHORED_Q, ..Default::default() }.muted_by(AUTHORED_Q);
        assert!(s.resolve() > 0, "burden must not blind");
        assert_eq!(s.resolve_heard(), 0, "burden must silence");
    }

    #[test]
    fn the_room_itself_grants_sightlines() {
        assert_eq!(sightline_of(0), 0);
        assert_eq!(sightline_of(2), AUTHORED_Q / 2);
        assert_eq!(sightline_of(4), AUTHORED_Q);
        assert_eq!(sightline_of(9), AUTHORED_Q, "there are only four walls");
    }

    /// Noise blinds the one making it AND covers them. Past halfway the wall of
    /// sound hides the body inside it — the loud build is a trade, not a tax.
    #[test]
    fn a_roaring_body_is_harder_to_find_than_a_grunting_one() {
        let watcher =
            Senses { attunement_q: AUTHORED_Q, logic_q: AUTHORED_Q, ..Default::default() };
        let at = |noise| noticed_q(&watcher, noise, 4, 12);
        assert!(at(0) > 0, "a silent quarry inside reach is still seen");
        let grunting = at(4_000);
        let roaring = at(AUTHORED_Q);
        assert!(grunting > at(0), "some noise gives you away");
        assert!(roaring < grunting, "all of it hides you again: {roaring} vs {grunting}");
    }

    #[test]
    fn the_watcher_reach_decides_the_miss() {
        let keen = Senses { attunement_q: AUTHORED_Q, logic_q: AUTHORED_Q, ..Default::default() };
        let dull = Senses { blocked_q: 9_000, ..keen };
        assert!(noticed_q(&keen, 3_000, 9, 12) > 0);
        assert_eq!(noticed_q(&dull, 3_000, 9, 12), 0, "the blocked one never knew");
        assert_eq!(noticed_q(&keen, 3_000, 400, 12), 0, "nobody hears across a parish");
    }

    #[test]
    fn changing_bodies_changes_what_can_reach_you() {
        let loud = Senses {
            attunement_q: AUTHORED_Q,
            logic_q: AUTHORED_Q,
            wisdom_q: AUTHORED_Q,
            ..Default::default()
        }
        .muted_by(AUTHORED_Q);
        assert_eq!(Form::Mortal.wear(loud).resolve_heard(), 0, "your own roar silences you");
        let lich = Form::Lich.wear(loud);
        assert_eq!(lich.muted_q, 0, "a body with no breath makes no noise");
        assert_eq!(lich.resolve_heard(), lich.resolve(), "for the dead the two faces are one");
    }

    #[test]
    fn some_things_cannot_be_boxed_in() {
        let hemmed =
            Senses { attunement_q: AUTHORED_Q, blocked_q: AUTHORED_Q, ..Default::default() };
        assert_eq!(Form::Mortal.wear(hemmed).resolve(), 0, "a mortal can be shut out");
        assert!(Form::Djinn.wear(hemmed).resolve() > 0, "you cannot corner the air");
        assert!(Form::Treant.wear(hemmed).resolve() > 0, "nor lean on something rooted");
        assert!(Form::Wraith.wear(hemmed).resolve() > 0, "nor block what is not standing there");
    }

    /// THE anti-power-creep gate: nothing in this table is a free upgrade over
    /// the mortal it replaced. If one ever is, this says so.
    #[test]
    fn no_body_is_strictly_better_than_the_one_you_left() {
        let trained = Senses {
            logic_q: 7_000,
            wisdom_q: 6_000,
            upbringing_q: 5_000,
            sightline_q: 7_500,
            attunement_q: 6_000,
            shadowed_q: 2_000,
            dulled_q: 1_500,
            upbringing: Some(Upbringing::Hearthborn),
            ..Default::default()
        }
        .muted_by(3_000);

        for form in Form::ALL {
            if form == Form::Mortal {
                continue;
            }
            let worn = form.wear(trained);
            let gave_up = worn.logic_q < trained.logic_q
                || worn.wisdom_q < trained.wisdom_q
                || worn.sightline_q < trained.sightline_q
                || worn.upbringing_q < trained.upbringing_q
                || worn.muted_q > trained.muted_q;
            assert!(gave_up, "{} costs nothing — that is a buff, not a form", form.name());
            assert!(!form.body_line().is_empty());
        }
    }
}
