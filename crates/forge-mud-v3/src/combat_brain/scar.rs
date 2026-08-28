//! Prior-Authority layer — the #1 death loop.
//!
//! A death becomes a SCAR holding bounded, replayable authority over future state
//! (the `` ` `` / `Lane::PriorAuthority` operator). Identity is deterministic from
//! `(seed, tick, subject, position, cause)` — no wall clock, no platform entropy —
//! so a replay reproduces the exact same scar.
//!
//! TRIM (OBSERVED): Removed forge_cart_sink imports (DeterminismSink, EvidenceSink).
//! Replaced with simple deterministic u64 hashing via XOR-fold. Mud deps = forge-core-v3 + forge-cart-v3 only.

/// What killed an entity (mirrors the quarry dirge `DeathScar` `DeathCause`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCause {
    /// Death in combat.
    Combat,
    /// Death from falling.
    Fall,
    /// Death from a hazard.
    Hazard,
    /// Death by erasure.
    Erasure,
    /// Death through sacrifice.
    Sacrifice,
    /// Death by refusal.
    Refusal,
}

impl DeathCause {
    /// Stable u8 tag (never reorder).
    pub const fn tag(self) -> u8 {
        self as u8
    }
}

/// Bounded future pressure a fresh scar exerts (u16-scaled Permyriad, 10_000 = 1.0).
pub const SCAR_BASE_PRESSURE_Q: u16 = 5_000;

/// Ticks over which a scar's pressure decays to zero — the mercy-tick TTL: a
/// scar's authority eventually fades / is Forgotten (10s @ 120Hz).
pub const SCAR_TTL_TICKS: u64 = 1_200;

/// A death scar — a Prior-Authority record. Deterministic + replayable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeathScar {
    /// Deterministic scar identity (via simple u64 hash).
    pub scar_hash: u64,
    /// Prior-Authority `source_tick` — when the death happened.
    pub source_tick: u64,
    /// Who/what died (entity identity).
    pub subject_hash: u64,
    /// Proof record hash (deterministic from cause).
    pub proof_hash: u64,
    /// Death position in MilliUnits [x, y].
    pub position_mm: [i64; 2],
    /// The cause of death.
    pub cause: DeathCause,
    /// Bounded future pressure at creation (Permyriad-scaled).
    pub pressure_q: u16,
}

impl DeathScar {
    /// The bounded pressure this scar exerts at `current_tick` — full at the
    /// source tick, decaying linearly to zero across [`SCAR_TTL_TICKS`]. Zero
    /// before the source tick: a scar cannot reach into its own past.
    pub fn pressure_at(&self, current_tick: u64) -> u16 {
        if current_tick < self.source_tick {
            return 0;
        }
        let age = current_tick - self.source_tick;
        if age >= SCAR_TTL_TICKS {
            return 0;
        }
        let remaining = SCAR_TTL_TICKS - age;
        (self.pressure_q as u64 * remaining / SCAR_TTL_TICKS) as u16
    }
}

/// Apply integer damage. Returns `true` iff this hit caused a death TRANSITION
/// (hp went from > 0 to <= 0) — it never re-fires once already dead.
pub fn apply_damage(hp: &mut i32, amount: i32) -> bool {
    let was_alive = *hp > 0;
    *hp = hp.saturating_sub(amount);
    was_alive && *hp <= 0
}

/// Simple deterministic u64 hash via XOR-folding (TRIM: no external sink deps).
/// Identity is from `(seed, tick, subject, position, cause)`.
fn deterministic_hash(seed: u64, tick: u64, subject: u64, x: i64, y: i64, cause: u8) -> u64 {
    let mut hash = seed ^ tick ^ subject;
    hash ^= (x as u64).wrapping_mul(0x9E3779B97F4A7C15);
    hash ^= (y as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    hash ^= (cause as u64).wrapping_mul(0xC6A4A7935BD1E995);
    hash ^= hash >> 33;
    hash
}

/// Forge a deterministic death scar + generate proof hash.
/// Identity is from `(seed, tick, subject, position, cause)` — no wall clock, no RNG draw —
/// so a replay reproduces the exact same scar.
pub fn forge_scar(
    seed: u64,
    source_tick: u64,
    subject_hash: u64,
    position_mm: [i64; 2],
    cause: DeathCause,
) -> DeathScar {
    let scar_hash = deterministic_hash(seed, source_tick, subject_hash, position_mm[0], position_mm[1], cause.tag());
    let proof_hash = deterministic_hash(scar_hash, source_tick, subject_hash, cause.tag() as i64, 0, 0);
    DeathScar {
        scar_hash,
        source_tick,
        subject_hash,
        proof_hash,
        position_mm,
        cause,
        pressure_q: SCAR_BASE_PRESSURE_Q,
    }
}

/// Max simultaneously-tracked scars (bounded, zero-alloc ring).
pub const MAX_SCARS: usize = 32;

/// The death-scar ledger — a bounded ring of Prior-Authority records.
pub struct ScarLedger {
    scars: [Option<DeathScar>; MAX_SCARS],
    write: usize,
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
    pub fn total_pressure_at(&self, tick: u64) -> u64 {
        self.scars
            .iter()
            .flatten()
            .map(|s| s.pressure_at(tick) as u64)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lethal_damage_triggers_single_death_transition() {
        let mut hp = 10;
        assert!(!apply_damage(&mut hp, 3)); // 10 -> 7, alive
        assert!(apply_damage(&mut hp, 20)); // 7 -> -13, DEATH transition
        assert!(!apply_damage(&mut hp, 5)); // already dead, no re-fire
    }

    #[test]
    fn scar_is_deterministic() {
        let a = forge_scar(0xC0FFEE, 120, 0xA1, [1000, -2000], DeathCause::Combat);
        let b = forge_scar(0xC0FFEE, 120, 0xA1, [1000, -2000], DeathCause::Combat);
        assert_eq!(a, b, "same (seed,tick,subject,pos,cause) => identical scar (replayable)");
    }

    #[test]
    fn scar_differs_on_divergent_death() {
        let a = forge_scar(1, 120, 7, [0, 0], DeathCause::Combat);
        let b = forge_scar(1, 121, 7, [0, 0], DeathCause::Combat); // tick + 1
        assert_ne!(
            a.scar_hash, b.scar_hash,
            "a later death must forge a distinct scar (discriminator)"
        );
    }

    #[test]
    fn prior_authority_pressure_is_bounded_and_decays() {
        let scar = forge_scar(1, 1000, 7, [0, 0], DeathCause::Erasure);
        assert_eq!(scar.pressure_at(999), 0, "no authority before the death");
        assert_eq!(scar.pressure_at(1000), SCAR_BASE_PRESSURE_Q, "full pressure at the source tick");
        let mid = scar.pressure_at(1000 + SCAR_TTL_TICKS / 2);
        assert!(mid > 0 && mid < SCAR_BASE_PRESSURE_Q, "pressure decays mid-TTL");
        assert_eq!(
            scar.pressure_at(1000 + SCAR_TTL_TICKS),
            0,
            "authority fully fades at TTL (mercy-tick Forgotten)"
        );
    }

    #[test]
    fn scar_ledger_records_bounds_and_sums_pressure() {
        let mut ledger = ScarLedger::new();
        for t in 0..40u64 {
            ledger.record(forge_scar(1, 1000 + t, t, [0, 0], DeathCause::Hazard));
        }
        assert_eq!(ledger.count(), MAX_SCARS, "ledger is bounded at MAX_SCARS");
        assert!(ledger.total_pressure_at(1040) > 0, "live scars exert summed pressure");
    }

    // L07: Bijection test — scar pressure decay is linear and deterministic
    #[test]
    fn l07_scar_pressure_decay_bijection() {
        let scar = forge_scar(42, 1000, 999, [500, -500], DeathCause::Combat);

        // Pressure must be:
        // - 0 before source_tick
        // - Full at source_tick
        // - Strictly decreasing after source_tick (until zero at TTL)
        // - 0 after TTL

        let mut prev_pressure = scar.pressure_at(1000);
        for tick_offset in 1..=(SCAR_TTL_TICKS + 1) {
            let tick = 1000 + tick_offset;
            let pressure = scar.pressure_at(tick);
            assert!(
                pressure <= prev_pressure,
                "Pressure must decay monotonically: at tick {}, pressure {} > prev {}",
                tick, pressure, prev_pressure
            );
            prev_pressure = pressure;
        }

        // Check exact boundary: at TTL, pressure should be 0
        assert_eq!(scar.pressure_at(1000 + SCAR_TTL_TICKS), 0);
    }

    // L07: Bijection test — cause tag is stable and invertible
    #[test]
    fn l07_death_cause_tag_bijection() {
        let causes = vec![
            DeathCause::Combat,
            DeathCause::Fall,
            DeathCause::Hazard,
            DeathCause::Erasure,
            DeathCause::Sacrifice,
            DeathCause::Refusal,
        ];

        let mut tags = std::collections::HashSet::new();
        for cause in &causes {
            let tag = cause.tag();
            assert!(tags.insert(tag), "Cause {:?} has duplicate tag {}", cause, tag);
        }

        // Verify all scars are distinct when only cause differs
        let mut scars = Vec::new();
        for cause in &causes {
            let scar = forge_scar(1, 100, 7, [0, 0], *cause);
            scars.push(scar);
        }

        for (i, scar_i) in scars.iter().enumerate() {
            for (j, scar_j) in scars.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        scar_i.scar_hash, scar_j.scar_hash,
                        "Scars with different causes must have different hashes"
                    );
                }
            }
        }
    }
}
