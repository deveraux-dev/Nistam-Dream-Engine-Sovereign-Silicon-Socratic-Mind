//! A recurrence over [`EphemeralEnvelope`] seasons, unconditioned at the API surface.
//!
//! The crate's own [`crate::Disposition`] already names expiry "the fixed point — what
//! happens when nobody chooses." This module adds the other half: [`PerennialCycle`]
//! offers the next [`Season`] without a parameter through which a prior [`crate::ChainLink`]
//! or [`crate::Disposition`] could reach it — `next(&mut self)` and `Season::open(&self,
//! payload)` have no argument slot for either.
//!
//! **What that does and doesn't prove — stated precisely, not oversold.** It proves the
//! *offer* of a new season cannot be gated by the *method itself* reading what the last
//! one resolved to. It does **not** prove a caller can't choose to stop calling
//! [`Iterator::next`] after inspecting a disposition externally, and it does not prevent a
//! caller from bypassing [`Season::open`] entirely and calling
//! [`EphemeralEnvelope::new`] directly with arbitrary ticks — the unconditioned guarantee
//! holds for code that uses this API as given, not against deliberate circumvention of it.
//! Adversarial review (2026-08-22, 5 independent passes) is why this paragraph exists
//! instead of the stronger, unqualified claim an earlier draft made.
//!
//! **The property this module actually delivers more strongly than "unconditioned":**
//! `start_tick` is a pure function of `index` and the cycle's fixed `season_len_ticks`
//! alone (see [`PerennialCycle::next`]) — every `Season` at a given index is
//! deterministically identical no matter what happened in any prior season, on any
//! machine, on any run. That determinism, not just the absence of a gate, is what makes
//! two independent replays agree.
//!
//! **What this module does not own:** whether `T`'s `AsRef<[u8]>` output is stable across
//! calls (a `T` with interior mutability could break seal determinism) is a property of
//! [`EphemeralEnvelope`] itself, not introduced here. Whether a season's evidence survives
//! at all also isn't this module's to guarantee — an opened envelope that's dropped
//! without calling [`EphemeralEnvelope::resolve`] leaves no [`crate::ChainLink`] behind
//! (its `Drop` only wipes bytes); the cycle still advances correctly either way, but
//! nothing about that season is remembered. And [`crate::ChainLink::follows`] proves hash
//! continuity and integrity, not temporal or causal ordering — it would accept a link
//! chained onto a predecessor with a *later* tick.
//!
//! "Perennial" — not that every season resolves well, but that the offer of a new one
//! isn't conditioned on the last one having.

use zeroize::Zeroize;

use crate::EphemeralEnvelope;

/// One bounded tick window a commitment may be opened and resolved within.
///
/// `PerennialCycle::new(0)` is legal and produces seasons whose window is already closed
/// the instant they open (`duration_ticks: 0` makes `expiry_tick == start_tick`) — every
/// such season resolves to `Expired` unless revoked first. Not rejected, because an
/// always-expired season is still a well-formed disposition, just an unusual one to ask
/// for on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Season {
    /// Which season this is, counting from zero.
    pub index: u64,
    /// The tick this season's window opens at.
    pub start_tick: u64,
    /// How many ticks this season's window lasts.
    pub duration_ticks: u64,
}

impl Season {
    /// Open a commitment for this season. Takes no chain, no prior
    /// [`crate::Disposition`] — there is nothing here to read, nothing to gate on. See
    /// the module doc for exactly what this does and doesn't guarantee against a caller
    /// who bypasses this method and constructs [`EphemeralEnvelope`] directly.
    pub fn open<T: Zeroize + AsRef<[u8]>>(&self, payload: T) -> EphemeralEnvelope<T> {
        EphemeralEnvelope::new(payload, self.start_tick, self.duration_ticks)
    }
}

/// An infinite recurrence of fixed-length seasons — this iterator never returns `None`,
/// by design, the same contract [`core::iter::repeat`] makes. Callers must bound it
/// themselves (`.take(n)`, an explicit `break`); a bare `.collect()` will hang.
///
/// [`Iterator::next`]'s signature takes `&mut self` only — no [`crate::EvidenceChain`],
/// no [`crate::Disposition`] parameter. See the module doc for the precise scope of what
/// that proves. Two independent `PerennialCycle`s are not coordinated with each other and
/// will produce identical `index`/`start_tick` pairs given the same `season_len_ticks` —
/// this type guarantees no repetition WITHIN one cycle, nothing across cycles.
#[derive(Debug, Clone)]
pub struct PerennialCycle {
    next_index: u64,
    season_len_ticks: u64,
}

impl PerennialCycle {
    /// Start a recurrence of seasons, each `season_len_ticks` long.
    pub fn new(season_len_ticks: u64) -> Self {
        Self { next_index: 0, season_len_ticks }
    }
}

impl Iterator for PerennialCycle {
    type Item = Season;

    fn next(&mut self) -> Option<Season> {
        let index = self.next_index;
        // Saturating both ways: a cycle run past u64::MAX indices stops advancing
        // `index` rather than wrapping back to a repeated one — the same
        // never-repeat guarantee `start_tick`'s multiply already makes.
        let start_tick = index.saturating_mul(self.season_len_ticks);
        self.next_index = self.next_index.saturating_add(1);
        Some(Season { index, start_tick, duration_ticks: self.season_len_ticks })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvidenceChain, Disposition};

    #[test]
    fn recurrence_is_unconditioned_on_prior_disposition() {
        let mut chain = EvidenceChain::new();
        let mut cycle = PerennialCycle::new(60);

        // Season 0: falls through unwitnessed. Expired — the fixed point, not a failure.
        let s0 = cycle.next().unwrap();
        let e0 = s0.open(b"season zero".to_vec());
        let link0 = e0.resolve(s0.start_tick + s0.duration_ticks, &mut chain);
        assert_eq!(link0.record(), Disposition::Expired);

        // Season 1: revoked deliberately.
        let s1 = cycle.next().unwrap();
        let mut e1 = s1.open(b"season one".to_vec());
        e1.revoke();
        let link1 = e1.resolve(s1.start_tick, &mut chain);
        assert_eq!(link1.record(), Disposition::Revoked);
        assert!(link1.follows(&link0));

        // Season 2: still opens, still can attest — nothing above blocked it. This is
        // the actual proof, and it's a type-level one: Season::open and next() never had
        // a parameter through which link0/link1's disposition could reach them. follows()
        // above proves the CHAIN is hash-continuous and untampered — a real but narrower
        // property than "unconditioned," not that nothing gated the offer.
        let s2 = cycle.next().unwrap();
        let e2 = s2.open(b"season two".to_vec());
        let link2 = e2.resolve(s2.start_tick, &mut chain);
        assert!(matches!(link2.record(), Disposition::Attested(_)));
        assert!(link2.follows(&link1));

        assert!(link0.verify() && link1.verify() && link2.verify());
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn season_open_takes_no_chain_or_prior_disposition() {
        // Compile-time proof by signature inspection, not runtime behavior:
        // Season::open's only inputs are &self and payload. If this test file
        // compiles, that's the whole proof — there is no argument slot here
        // through which a chain or a Disposition could have been passed. It does
        // NOT prove a caller can't bypass this method entirely (see module doc).
        let s = Season { index: 0, start_tick: 0, duration_ticks: 60 };
        let _e = s.open(b"proof".to_vec());
    }

    #[test]
    fn cycle_never_repeats_a_season_index() {
        let mut cycle = PerennialCycle::new(60);
        let seasons: Vec<Season> = (0..5).map(|_| cycle.next().unwrap()).collect();
        for (i, s) in seasons.iter().enumerate() {
            assert_eq!(s.index, i as u64);
            assert_eq!(s.start_tick, i as u64 * 60);
        }
    }

    /// Adversarial finding (2026-08-22 review): the original test always ran the same
    /// fixed order (Expired, Revoked, Attested). This proves the recurrence doesn't care
    /// about disposition order either, not just that it survives one particular order.
    #[test]
    fn recurrence_is_unconditioned_regardless_of_disposition_order() {
        let mut chain = EvidenceChain::new();
        let mut cycle = PerennialCycle::new(60);

        let s0 = cycle.next().unwrap();
        let e0 = s0.open(b"season zero".to_vec());
        let link0 = e0.resolve(s0.start_tick, &mut chain); // resolved while live -> Attested
        assert!(matches!(link0.record(), Disposition::Attested(_)));

        let s1 = cycle.next().unwrap();
        let e1 = s1.open(b"season one".to_vec());
        let link1 = e1.resolve(s1.start_tick + s1.duration_ticks, &mut chain); // past deadline -> Expired
        assert_eq!(link1.record(), Disposition::Expired);
        assert!(link1.follows(&link0));

        let s2 = cycle.next().unwrap();
        let mut e2 = s2.open(b"season two".to_vec());
        e2.revoke();
        let link2 = e2.resolve(s2.start_tick, &mut chain);
        assert_eq!(link2.record(), Disposition::Revoked);
        assert!(link2.follows(&link1));

        assert!(link0.verify() && link1.verify() && link2.verify());
        assert_eq!(chain.len(), 3);
    }

    /// Adversarial finding: an opened-but-never-resolved season leaves no chain entry
    /// (`EphemeralEnvelope`'s `Drop` only wipes bytes) — but the cycle keeps advancing
    /// correctly regardless, and the chain that DOES exist skips straight past it.
    #[test]
    fn dropping_a_season_unresolved_leaves_no_chain_entry_but_cycle_continues() {
        let mut chain = EvidenceChain::new();
        let mut cycle = PerennialCycle::new(60);

        let s0 = cycle.next().unwrap();
        drop(s0.open(b"season zero".to_vec())); // never resolved
        assert_eq!(chain.len(), 0);

        let s1 = cycle.next().unwrap();
        let e1 = s1.open(b"season one".to_vec());
        let link1 = e1.resolve(s1.start_tick, &mut chain);
        assert!(matches!(link1.record(), Disposition::Attested(_)));

        let s2 = cycle.next().unwrap();
        assert_eq!(s2.index, 2, "cycle indexing is unaffected by an unresolved season");
        assert_eq!(s2.start_tick, 2 * 60);

        let e2 = s2.open(b"season two".to_vec());
        let link2 = e2.resolve(s2.start_tick, &mut chain);
        assert!(link2.follows(&link1), "the chain links straight past the season that left no trace");
        assert_eq!(chain.len(), 2);
    }

    /// Adversarial finding: two independent `PerennialCycle`s are not coordinated and
    /// will produce colliding `index`/`start_tick` pairs — expected, since nothing about
    /// this type claims cross-cycle uniqueness, only within-cycle non-repetition. Named
    /// explicitly so the assumption is checked, not just implied.
    #[test]
    fn independent_cycles_are_not_coordinated_and_can_share_indices() {
        let mut cycle_a = PerennialCycle::new(60);
        let mut cycle_b = PerennialCycle::new(60);
        let s_a0 = cycle_a.next().unwrap();
        let s_b0 = cycle_b.next().unwrap();
        assert_eq!(s_a0, s_b0, "same season_len_ticks, same index -> identical Season");
    }
}
