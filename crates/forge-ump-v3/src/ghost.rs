//! ghost.rs — the Ghost Playhead: futuresight over the [`TimelineTape`]. A cursor
//! DECOUPLED from the real `world_clock.tick_id` that scrubs AHEAD to the last pending
//! commit, projects the future into a read-only holographic map, and radars collisions
//! BEFORE the real clock reaches them. Reads only — never writes the tape, the registry,
//! or the riverbed. The always-on map that hits the error in projection first.

use crate::timeline::{SealedTuple, SynthPoint, TimelineTape};

/// Highest legal codeword (the 64-slot essence codebook, 0..=63).
const MAX_ESSENCE: u8 = 63;
/// Highest legal moon (1..=13 Cree; 0 = unbound). Anything above is a scheduling fault.
const MAX_MOON: u8 = 13;

/// A read-only cursor that runs AHEAD of `now` over the tape's future segment. Detached
/// from `world_clock`: it holds its own `now_tick` and reads every commit past it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GhostPlayhead {
    now_tick: u64,
}

impl GhostPlayhead {
    /// Anchor the ghost at the real clock's current tick.
    pub fn at(now_tick: u64) -> Self {
        Self { now_tick }
    }

    /// The real-clock tick the ghost is anchored to.
    #[inline]
    pub fn now(&self) -> u64 {
        self.now_tick
    }

    /// Advance the anchor as the real `world_clock` ticks forward.
    pub fn advance_now(&mut self, real_tick: u64) {
        self.now_tick = real_tick;
    }

    /// Project the future: every committed moment on `tape` with `tick > now`, in order.
    /// The projection borrows the tape; it copies nothing and mutates nothing.
    pub fn project<'t>(&self, tape: &'t TimelineTape) -> Projection<'t> {
        // window(now+1, MAX) = entries with tick strictly greater than now.
        let future = tape.window(self.now_tick.saturating_add(1), u64::MAX);
        Projection { future, now_tick: self.now_tick }
    }
}

/// The Holographic State — a read-only view of the tape from `now` to its end. It "lines
/// the systems up" without touching real state; nothing here writes.
#[derive(Debug, Clone, Copy)]
pub struct Projection<'t> {
    future: &'t [SealedTuple],
    now_tick: u64,
}

impl<'t> Projection<'t> {
    /// Build a projection directly over a frame slice measured from `now_tick` — for
    /// probes and hypothetical plans that aren't a live tape window.
    pub fn over_frames(frames: &'t [SealedTuple], now_tick: u64) -> Self {
        Self { future: frames, now_tick }
    }

    /// The real-clock tick this projection is measured from.
    #[inline]
    pub fn now(&self) -> u64 {
        self.now_tick
    }

    /// The furthest pending tick the ghost can see (the last commit). `None` if no future.
    pub fn horizon(&self) -> Option<u64> {
        self.future.last().map(|e| e.tick_id)
    }

    /// How many ticks ahead the ghost sees (`horizon - now`), 0 when the future is empty.
    pub fn lead_ticks(&self) -> u64 {
        self.horizon().map(|h| h.saturating_sub(self.now_tick)).unwrap_or(0)
    }

    /// The pending moments, in tick order.
    #[inline]
    pub fn frames(&self) -> &'t [SealedTuple] {
        self.future
    }

    /// Number of pending moments.
    #[inline]
    pub fn len(&self) -> usize {
        self.future.len()
    }

    /// True when nothing is pending past `now`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.future.is_empty()
    }

    /// The projected state at `tick` — the last pending moment at-or-before it (`None`
    /// if `tick <= now` or before the first pending commit).
    pub fn state_at(&self, tick: u64) -> Option<SealedTuple> {
        if tick <= self.now_tick {
            return None;
        }
        let p = self.future.partition_point(|e| e.tick_id <= tick);
        if p == 0 {
            None
        } else {
            Some(self.future[p - 1])
        }
    }

    /// The projected END state (the moment at the horizon).
    pub fn horizon_state(&self) -> Option<SealedTuple> {
        self.future.last().copied()
    }

    /// The decode seed at the horizon — what the world will sound/look like at the end.
    pub fn synth_at_horizon(&self) -> Option<SynthPoint> {
        self.future.last().map(|e| e.synth_point())
    }

    /// The future essence stream — the sheet music the ghost reads ahead.
    pub fn seeds(&self) -> impl Iterator<Item = SynthPoint> + 't {
        self.future.iter().map(|e| e.synth_point())
    }
}

/// The class of a radar contact — a predicted collision in the pending plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionKind {
    /// A pending commit ticks backward from its predecessor (a scheduling inversion).
    NonMonotonic,
    /// A codeword outside the 64-slot essence codebook.
    InvalidEssence,
    /// A moon outside `0..=13`.
    InvalidMoon,
    /// Two pending commits share a tick but disagree on essence — they ring the same
    /// instant differently.
    SameTickConflict,
    /// A caller-registered locked-law predicate rejected this moment.
    LawViolation,
}

/// One predicted collision — the ghost hit it in projection, `lead_ticks` before the
/// real clock will.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contact {
    /// The pending tick where the collision sits.
    pub tick: u64,
    /// Ticks until the real clock arrives (`tick - now`).
    pub lead_ticks: u64,
    /// The predicted fault's classification.
    pub kind: CollisionKind,
    /// Index of the offending moment within the projected future slice.
    pub index: usize,
}

/// The Collision Radar — scans the projected future for faults the real clock hasn't hit.
pub struct CollisionRadar;

impl CollisionRadar {
    /// Scan a projection with the built-in checks (monotonicity, codebook bounds,
    /// same-tick conflicts). Contacts come earliest-lead first.
    pub fn scan(proj: &Projection) -> Vec<Contact> {
        Self::scan_frames(proj.now(), proj.frames())
    }

    /// Scan raw future `frames` measured from `now`. The engine behind [`Self::scan`].
    pub fn scan_frames(now: u64, frames: &[SealedTuple]) -> Vec<Contact> {
        let mut out = Vec::new();
        let mut prev: Option<&SealedTuple> = None;
        for (i, e) in frames.iter().enumerate() {
            let push = |out: &mut Vec<Contact>, kind| {
                out.push(Contact {
                    tick: e.tick_id,
                    lead_ticks: e.tick_id.saturating_sub(now),
                    kind,
                    index: i,
                });
            };
            if e.essence_id > MAX_ESSENCE {
                push(&mut out, CollisionKind::InvalidEssence);
            }
            if e.moon > MAX_MOON {
                push(&mut out, CollisionKind::InvalidMoon);
            }
            if let Some(p) = prev {
                if e.tick_id < p.tick_id {
                    push(&mut out, CollisionKind::NonMonotonic);
                } else if e.tick_id == p.tick_id && e.essence_id != p.essence_id {
                    push(&mut out, CollisionKind::SameTickConflict);
                }
            }
            prev = Some(e);
        }
        out
    }

    /// Scan with an extra locked-law predicate — any moment where `law` returns `false`
    /// raises a `LawViolation` contact (in addition to the built-in checks).
    pub fn scan_with<F>(proj: &Projection, law: F) -> Vec<Contact>
    where
        F: Fn(&SealedTuple) -> bool,
    {
        let now = proj.now();
        let mut out = Self::scan_frames(now, proj.frames());
        for (i, e) in proj.frames().iter().enumerate() {
            if !law(e) {
                out.push(Contact {
                    tick: e.tick_id,
                    lead_ticks: e.tick_id.saturating_sub(now),
                    kind: CollisionKind::LawViolation,
                    index: i,
                });
            }
        }
        out.sort_by_key(|c| (c.lead_ticks, c.index));
        out
    }

    /// The soonest predicted collision, if any — the one the real clock will hit first.
    pub fn first_contact(proj: &Projection) -> Option<Contact> {
        Self::scan(proj).into_iter().min_by_key(|c| (c.lead_ticks, c.index))
    }
}

/// The always-on holographic map: one call projects the future, lines it into moon and
/// essence lanes, and bakes in the radar contacts. Read-only; a temporary membrane that
/// never alters the tape or registry.
#[derive(Debug, Clone)]
pub struct HologramMap {
    now_tick: u64,
    horizon: Option<u64>,
    lead: u64,
    /// `by_moon[m]` = pending ticks under moon `m` (0..=13; >13 folds to 0).
    by_moon: [Vec<u64>; 14],
    /// The pending `(tick, essence_id)` stream — the future sheet music.
    essence_track: Vec<(u64, u8)>,
    contacts: Vec<Contact>,
}

impl HologramMap {
    /// Project `tape` from `now_tick` into the full holographic map + radar sweep.
    pub fn project(tape: &TimelineTape, now_tick: u64) -> Self {
        let ghost = GhostPlayhead::at(now_tick);
        let proj = ghost.project(tape);
        let mut by_moon: [Vec<u64>; 14] = Default::default();
        let mut essence_track = Vec::with_capacity(proj.len());
        for e in proj.frames() {
            let m = if e.moon <= MAX_MOON { e.moon } else { 0 };
            by_moon[m as usize].push(e.tick_id);
            essence_track.push((e.tick_id, e.essence_id));
        }
        Self {
            now_tick,
            horizon: proj.horizon(),
            lead: proj.lead_ticks(),
            by_moon,
            essence_track,
            contacts: CollisionRadar::scan(&proj),
        }
    }

    #[inline]
    /// The real-clock tick this map was projected from.
    pub fn now(&self) -> u64 {
        self.now_tick
    }

    #[inline]
    /// The furthest pending tick this map can see.
    pub fn horizon(&self) -> Option<u64> {
        self.horizon
    }

    #[inline]
    /// How many ticks ahead this map sees.
    pub fn lead(&self) -> u64 {
        self.lead
    }

    /// True when the projected future has no radar contacts — the plan is collision-free.
    #[inline]
    pub fn is_clear(&self) -> bool {
        self.contacts.is_empty()
    }

    /// Pending ticks under `moon` (0 = unbound; >13 folds to 0).
    pub fn moon_lane(&self, moon: u8) -> &[u64] {
        let m = if moon <= MAX_MOON { moon } else { 0 };
        &self.by_moon[m as usize]
    }

    /// The pending `(tick, essence)` stream.
    #[inline]
    pub fn essence_track(&self) -> &[(u64, u8)] {
        &self.essence_track
    }

    /// Every predicted collision, earliest-lead first.
    #[inline]
    pub fn contacts(&self) -> &[Contact] {
        &self.contacts
    }

    /// The soonest predicted collision.
    pub fn first_contact(&self) -> Option<Contact> {
        self.contacts.iter().copied().min_by_key(|c| (c.lead_ticks, c.index))
    }

    /// Distinct pending moons, ascending.
    pub fn moons_present(&self) -> Vec<u8> {
        (0u8..14).filter(|&m| !self.by_moon[m as usize].is_empty()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{Stamped, Ump};
    use crate::provenance_tag::Tier;

    fn ev(t: u32) -> Vec<Stamped<Ump>> {
        vec![Stamped { universal_tick_us: t as i64, payload: Ump::new([t, 0, 0, 0]) }]
    }

    /// A well-formed tape: ticks 0,100,..,(n-1)*100, moon (i%13)+1, essence i%64.
    fn tape_of(n: u64) -> TimelineTape {
        let mut t = TimelineTape::new(10);
        for i in 0..n {
            t.record(i * 100, ((i % 13) + 1) as u8, (i % 64) as u8, Tier::Local, &ev(i as u32 + 1))
                .unwrap();
        }
        t
    }

    /// A hand-built sealed tuple (fields are pub) for radar-slice tests.
    fn st(tick: u64, moon: u8, essence: u8) -> SealedTuple {
        SealedTuple {
            tick_id: tick,
            content_seal: tick ^ 0xABCD,
            chain_seal: tick.wrapping_mul(31),
            moon,
            essence_id: essence,
            source_kind: 0,
            flags: 0,
            reserved: 0,
        }
    }

    // ── ghost playhead / decoupling ──

    #[test]
    fn ghost_is_detached_from_world_clock() {
        let mut g = GhostPlayhead::at(300);
        assert_eq!(g.now(), 300);
        g.advance_now(500); // the real clock ticked; the ghost re-anchors, nothing else
        assert_eq!(g.now(), 500);
    }

    #[test]
    fn project_sees_only_the_future() {
        let tape = tape_of(10); // ticks 0..900
        let proj = GhostPlayhead::at(450).project(&tape);
        // strictly greater than now(450): 500,600,700,800,900
        let ticks: Vec<u64> = proj.frames().iter().map(|e| e.tick_id).collect();
        assert_eq!(ticks, vec![500, 600, 700, 800, 900]);
    }

    #[test]
    fn project_now_past_end_is_empty() {
        let tape = tape_of(5); // last 400
        let proj = GhostPlayhead::at(9999).project(&tape);
        assert!(proj.is_empty());
        assert_eq!(proj.lead_ticks(), 0);
        assert!(proj.horizon().is_none());
    }

    #[test]
    fn project_before_all_sees_everything() {
        let tape = tape_of(4); // 0,100,200,300
        let proj = GhostPlayhead::at(0).project(&tape); // now=0 → strictly > 0
        assert_eq!(proj.len(), 3); // 100,200,300 (0 is not > 0)
    }

    // ── projection / horizon ──

    #[test]
    fn horizon_and_lead_measure_futuresight_depth() {
        let tape = tape_of(10);
        let proj = GhostPlayhead::at(250).project(&tape);
        assert_eq!(proj.horizon(), Some(900));
        assert_eq!(proj.lead_ticks(), 650); // 900 - 250
    }

    #[test]
    fn state_at_reads_the_projected_future() {
        let tape = tape_of(10);
        let proj = GhostPlayhead::at(150).project(&tape);
        assert_eq!(proj.state_at(650).unwrap().tick_id, 600);
        assert!(proj.state_at(150).is_none(), "at-or-before now is not future");
        assert!(proj.state_at(50).is_none());
    }

    #[test]
    fn horizon_state_and_synth_seed() {
        let tape = tape_of(6); // last tick 500, essence 5, moon 6
        let proj = GhostPlayhead::at(0).project(&tape);
        let h = proj.horizon_state().unwrap();
        assert_eq!(h.tick_id, 500);
        assert_eq!(proj.synth_at_horizon().unwrap().coord.tick_id, 500);
    }

    #[test]
    fn seeds_stream_the_future_sheet_music() {
        let tape = tape_of(10);
        let proj = GhostPlayhead::at(700).project(&tape);
        let seeds: Vec<u64> = proj.seeds().map(|s| s.coord.tick_id).collect();
        assert_eq!(seeds, vec![800, 900]);
    }

    // ── collision radar ──

    #[test]
    fn clean_future_has_no_contacts() {
        let tape = tape_of(30);
        let proj = GhostPlayhead::at(0).project(&tape);
        assert!(CollisionRadar::scan(&proj).is_empty());
        assert!(CollisionRadar::first_contact(&proj).is_none());
    }

    #[test]
    fn radar_flags_invalid_essence_in_the_plan() {
        // essence 200 is outside the 64-slot codebook — record() accepts it, the radar catches it.
        let mut t = TimelineTape::new(10);
        t.record(100, 1, 5, Tier::Local, &ev(1)).unwrap();
        t.record(200, 1, 200, Tier::Local, &ev(2)).unwrap();
        let proj = GhostPlayhead::at(0).project(&t);
        let c = CollisionRadar::first_contact(&proj).unwrap();
        assert_eq!(c.kind, CollisionKind::InvalidEssence);
        assert_eq!(c.tick, 200);
        assert_eq!(c.lead_ticks, 200);
    }

    #[test]
    fn radar_flags_invalid_moon() {
        let mut t = TimelineTape::new(10);
        t.record(100, 50, 5, Tier::Local, &ev(1)).unwrap(); // moon 50 > 13
        let proj = GhostPlayhead::at(0).project(&t);
        assert_eq!(CollisionRadar::scan(&proj)[0].kind, CollisionKind::InvalidMoon);
    }

    #[test]
    fn radar_flags_same_tick_conflict() {
        let mut t = TimelineTape::new(10);
        t.record(100, 1, 5, Tier::Local, &ev(1)).unwrap();
        t.record(100, 1, 9, Tier::Local, &ev(2)).unwrap(); // same tick, different essence
        let proj = GhostPlayhead::at(0).project(&t);
        let kinds: Vec<CollisionKind> = CollisionRadar::scan(&proj).iter().map(|c| c.kind).collect();
        assert!(kinds.contains(&CollisionKind::SameTickConflict));
    }

    #[test]
    fn radar_catches_non_monotonic_hand_built_slice() {
        // record() forbids backward ticks, so this guards corrupt/hand-built tapes.
        let frames = [st(500, 1, 3), st(300, 1, 3)]; // ticks go backward
        let c = CollisionRadar::scan_frames(0, &frames);
        assert!(c.iter().any(|x| x.kind == CollisionKind::NonMonotonic));
    }

    #[test]
    fn first_contact_is_the_soonest() {
        let frames = [st(900, 1, 200), st(300, 1, 200)]; // both invalid essence
        // measured from now=0: the soonest lead is tick 300.
        let c = CollisionRadar::scan_frames(0, &frames);
        let soonest = c.iter().min_by_key(|x| x.lead_ticks).unwrap();
        assert_eq!(soonest.tick, 300);
    }

    #[test]
    fn lead_ticks_count_down_to_the_real_clock() {
        let mut t = TimelineTape::new(10);
        t.record(1000, 1, 200, Tier::Local, &ev(1)).unwrap(); // fault far ahead
        let proj = GhostPlayhead::at(600).project(&t);
        let c = CollisionRadar::first_contact(&proj).unwrap();
        assert_eq!(c.lead_ticks, 400, "400 ticks of warning before impact");
    }

    #[test]
    fn scan_with_locked_law_raises_violations() {
        let tape = tape_of(10); // essences 0..9, all valid codewords
        let proj = GhostPlayhead::at(0).project(&tape);
        // A locked law: "no essence above 3 is permitted in the plan."
        let contacts = CollisionRadar::scan_with(&proj, |e| e.essence_id <= 3);
        assert!(contacts.iter().all(|c| c.tick >= 100));
        let viols: Vec<_> = contacts.iter().filter(|c| c.kind == CollisionKind::LawViolation).collect();
        assert!(!viols.is_empty(), "law violations must be raised");
        // earliest violation is essence 4 at tick 400.
        assert_eq!(viols.iter().min_by_key(|c| c.lead_ticks).unwrap().tick, 400);
    }

    #[test]
    fn futuresight_catches_the_fault_before_now_arrives() {
        // The core promise: a pending bad commit is seen in projection while the real
        // clock is still well behind it.
        let mut t = TimelineTape::new(10);
        for i in 0..5u64 {
            t.record(i * 100, 1, 1, Tier::Local, &ev(i as u32 + 1)).unwrap(); // clean past
        }
        t.record(500, 99, 1, Tier::Local, &ev(99)).unwrap(); // a bad moon lands in the future
        let proj = GhostPlayhead::at(200).project(&t); // real clock only at 200
        let c = CollisionRadar::first_contact(&proj).unwrap();
        assert_eq!(c.kind, CollisionKind::InvalidMoon);
        assert!(c.lead_ticks > 0, "seen before the real clock reaches it");
        assert_eq!(c.tick, 500);
    }

    // ── hologram map ──

    #[test]
    fn hologram_projects_lanes_and_radar() {
        let tape = tape_of(14); // moons cycle 1..13,1
        let map = HologramMap::project(&tape, 0);
        assert_eq!(map.now(), 0);
        assert_eq!(map.horizon(), Some(1300));
        assert!(map.is_clear(), "well-formed tape → no contacts");
        assert!(!map.moon_lane(1).is_empty());
        assert_eq!(map.essence_track().len(), 13); // ticks 100..1300 (0 excluded, now=0)
    }

    #[test]
    fn hologram_bakes_contacts_and_first() {
        let mut t = TimelineTape::new(10);
        t.record(100, 1, 5, Tier::Local, &ev(1)).unwrap();
        t.record(200, 1, 250, Tier::Local, &ev(2)).unwrap(); // invalid essence
        let map = HologramMap::project(&t, 0);
        assert!(!map.is_clear());
        assert_eq!(map.first_contact().unwrap().kind, CollisionKind::InvalidEssence);
        assert_eq!(map.first_contact().unwrap().tick, 200);
    }

    #[test]
    fn hologram_moon_lanes_and_present() {
        let mut t = TimelineTape::new(10);
        t.record(100, 3, 0, Tier::Local, &ev(1)).unwrap();
        t.record(200, 3, 0, Tier::Local, &ev(2)).unwrap();
        t.record(300, 7, 0, Tier::Local, &ev(3)).unwrap();
        let map = HologramMap::project(&t, 0);
        assert_eq!(map.moon_lane(3), &[100, 200]);
        assert_eq!(map.moon_lane(7), &[300]);
        assert_eq!(map.moons_present(), vec![3, 7]);
    }

    #[test]
    fn hologram_from_mid_clock_only_shows_ahead() {
        let tape = tape_of(10);
        let map = HologramMap::project(&tape, 550);
        assert_eq!(map.essence_track().len(), 4); // 600,700,800,900
        assert_eq!(map.lead(), 350); // 900 - 550
    }

    #[test]
    fn read_only_projection_does_not_mutate_the_tape() {
        let tape = tape_of(8);
        let root_before = tape.chain_root();
        let len_before = tape.len();
        let _map = HologramMap::project(&tape, 100);
        let _proj = GhostPlayhead::at(100).project(&tape);
        let _ = CollisionRadar::first_contact(&_proj);
        // the ghost only reads — the real tape is untouched.
        assert_eq!(tape.chain_root(), root_before);
        assert_eq!(tape.len(), len_before);
    }
}
