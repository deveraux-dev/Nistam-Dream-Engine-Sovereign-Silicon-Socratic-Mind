//! Prior-Authority layer — the #1 death loop.
//!
//! A death becomes a SCAR holding bounded, replayable authority over future state
//! (the `` ` `` / `Lane::PriorAuthority` operator). Identity is deterministic from
//! `(seed, tick, subject, position, cause)` — no wall clock, no platform entropy —
//! so a replay reproduces the exact same scar.

use forge_cart_sink_v3::{DeterminismSink, EvidenceSink, Permyriad};

/// What killed an entity (mirrors the quarry dirge `DeathScar` `DeathCause`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCause {
    /// Death by combat damage.
    Combat,
    /// Death by falling.
    Fall,
    /// Death by environmental hazard.
    Hazard,
    /// Death by erasure/elimination.
    Erasure,
    /// Death by sacrifice.
    Sacrifice,
    /// Death by refusal/surrender.
    Refusal,
}

impl DeathCause {
    /// Stable u8 tag (never reorder).
    pub const fn tag(self) -> u8 {
        self as u8
    }
}

/// Bounded future pressure a fresh scar exerts (Permyriad, 10_000 = 1.0).
pub const SCAR_BASE_PRESSURE_PMY: Permyriad = 5_000;

/// Ticks over which a scar's pressure decays to zero — the mercy-tick TTL: a
/// scar's authority eventually fades / is Forgotten (10s @ 120Hz).
pub const SCAR_TTL_TICKS: u64 = 1_200;

/// A death scar — a Prior-Authority record. Deterministic + replayable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeathScar {
    /// `BrutalHash` identity (via the sink at creation).
    pub scar_hash: u64,
    /// Prior-Authority `source_tick` — when the death happened.
    pub source_tick: u64,
    /// Who/what died (entity identity).
    pub subject_hash: u64,
    /// The sealed `EvidenceSink` receipt (provenance / `proof_hash`).
    pub proof_hash: u64,
    /// World position (millimetres) where the entity died.
    pub position_mm: [i64; 2],
    /// What caused the death.
    pub cause: DeathCause,
    /// Bounded future pressure at creation (Permyriad).
    pub pressure_q: Permyriad,
}

impl DeathScar {
    /// The bounded pressure this scar exerts at `current_tick` — full at the
    /// source tick, decaying linearly to zero across [`SCAR_TTL_TICKS`]. Zero
    /// before the source tick: a scar cannot reach into its own past.
    pub fn pressure_at(&self, current_tick: u64) -> Permyriad {
        if current_tick < self.source_tick {
            return 0;
        }
        let age = current_tick - self.source_tick;
        if age >= SCAR_TTL_TICKS {
            return 0;
        }
        let remaining = SCAR_TTL_TICKS - age;
        (self.pressure_q as i64 * remaining as i64 / SCAR_TTL_TICKS as i64) as Permyriad
    }
}

/// Apply integer damage. Returns `true` iff this hit caused a death TRANSITION
/// (hp went from > 0 to <= 0) — it never re-fires once already dead.
pub fn apply_damage(hp: &mut i32, amount: i32) -> bool {
    let was_alive = *hp > 0;
    *hp = hp.saturating_sub(amount);
    was_alive && *hp <= 0
}

/// Forge a deterministic death scar + seal its provenance through the sinks.
/// Identity is a `BrutalHash` over `(seed, tick, subject, position, cause)` — no
/// wall clock, no RNG draw — so a replay reproduces the exact same scar.
pub fn forge_scar(
    seed: u64,
    source_tick: u64,
    subject_hash: u64,
    position_mm: [i64; 2],
    cause: DeathCause,
    rng: &dyn DeterminismSink,
    evidence: &dyn EvidenceSink,
) -> DeathScar {
    let mut buf = [0u8; 8 + 8 + 8 + 8 + 8 + 1];
    buf[0..8].copy_from_slice(&seed.to_le_bytes());
    buf[8..16].copy_from_slice(&source_tick.to_le_bytes());
    buf[16..24].copy_from_slice(&subject_hash.to_le_bytes());
    buf[24..32].copy_from_slice(&position_mm[0].to_le_bytes());
    buf[32..40].copy_from_slice(&position_mm[1].to_le_bytes());
    buf[40] = cause.tag();
    let scar_hash = rng.hash_state(&buf);
    let receipt = evidence.seal(subject_hash, cause.tag() as u32, source_tick);
    DeathScar {
        scar_hash,
        source_tick,
        subject_hash,
        proof_hash: receipt.0,
        position_mm,
        cause,
        pressure_q: SCAR_BASE_PRESSURE_PMY,
    }
}

/// Max simultaneously-tracked scars (bounded, zero-alloc ring).
pub const MAX_SCARS: usize = 32;

/// The death-scar ledger — a bounded ring of Prior-Authority records.
pub struct ScarLedger {
    /// Array of scar slots (None = empty).
    scars: [Option<DeathScar>; MAX_SCARS],
    /// Write index for the next scar (circular).
    write: usize,
    /// Number of scars currently stored (0..=MAX_SCARS).
    count: usize,
}

impl Default for ScarLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ScarLedger {
    /// Create a new empty scar ledger.
    pub fn new() -> Self {
        Self { scars: [None; MAX_SCARS], write: 0, count: 0 }
    }

    /// Record a scar, evicting the oldest when full.
    pub fn record(&mut self, scar: DeathScar) {
        self.scars[self.write] = Some(scar);
        self.write = (self.write + 1) % MAX_SCARS;
        if self.count < MAX_SCARS {
            self.count += 1;
        }
    }

    /// Number of recorded scars (0..=MAX_SCARS).
    pub fn count(&self) -> usize {
        self.count
    }

    /// Total bounded Prior-Authority pressure all live scars exert at `tick`.
    /// This is `resolve_authority_tickets` in miniature — the executable past
    /// summed into a present consequence.
    pub fn total_pressure_at(&self, tick: u64) -> i64 {
        self.scars
            .iter()
            .flatten()
            .map(|s| s.pressure_at(tick) as i64)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_cart_sink_v3::{NullDeterminism, NullEvidence};

    #[test]
    fn lethal_damage_triggers_single_death_transition() {
        let mut hp = 10;
        assert!(!apply_damage(&mut hp, 3)); // 10 -> 7, alive
        assert!(apply_damage(&mut hp, 20)); // 7 -> -13, DEATH transition
        assert!(!apply_damage(&mut hp, 5)); // already dead, no re-fire
    }

    #[test]
    fn scar_is_deterministic() {
        let rng = NullDeterminism::new(0);
        let ev = NullEvidence;
        let a = forge_scar(0xC0FFEE, 120, 0xA1, [1000, -2000], DeathCause::Combat, &rng, &ev);
        let b = forge_scar(0xC0FFEE, 120, 0xA1, [1000, -2000], DeathCause::Combat, &rng, &ev);
        assert_eq!(a, b, "same (seed,tick,subject,pos,cause) => identical scar (replayable)");
    }

    #[test]
    fn scar_differs_on_divergent_death() {
        let rng = NullDeterminism::new(0);
        let ev = NullEvidence;
        let a = forge_scar(1, 120, 7, [0, 0], DeathCause::Combat, &rng, &ev);
        let b = forge_scar(1, 121, 7, [0, 0], DeathCause::Combat, &rng, &ev); // tick + 1
        assert_ne!(
            a.scar_hash, b.scar_hash,
            "a later death must forge a distinct scar (discriminator)"
        );
    }

    #[test]
    fn prior_authority_pressure_is_bounded_and_decays() {
        let rng = NullDeterminism::new(0);
        let ev = NullEvidence;
        let scar = forge_scar(1, 1000, 7, [0, 0], DeathCause::Erasure, &rng, &ev);
        assert_eq!(scar.pressure_at(999), 0, "no authority before the death");
        assert_eq!(scar.pressure_at(1000), SCAR_BASE_PRESSURE_PMY, "full pressure at the source tick");
        let mid = scar.pressure_at(1000 + SCAR_TTL_TICKS / 2);
        assert!(mid > 0 && mid < SCAR_BASE_PRESSURE_PMY, "pressure decays mid-TTL");
        assert_eq!(
            scar.pressure_at(1000 + SCAR_TTL_TICKS),
            0,
            "authority fully fades at TTL (mercy-tick Forgotten)"
        );
    }

    #[test]
    fn scar_ledger_records_bounds_and_sums_pressure() {
        let rng = NullDeterminism::new(0);
        let ev = NullEvidence;
        let mut ledger = ScarLedger::new();
        for t in 0..40u64 {
            ledger.record(forge_scar(1, 1000 + t, t, [0, 0], DeathCause::Hazard, &rng, &ev));
        }
        assert_eq!(ledger.count(), MAX_SCARS, "ledger is bounded at MAX_SCARS");
        assert!(ledger.total_pressure_at(1040) > 0, "live scars exert summed pressure");
    }
}
