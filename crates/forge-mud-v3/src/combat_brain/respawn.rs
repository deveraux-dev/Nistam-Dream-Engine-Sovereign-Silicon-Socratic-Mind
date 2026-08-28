//! Respawn domain — the timer between death and revival.
//!
//! The compounding penalty pairs with the scar's haunt drag: each subsequent
//! death costs MORE time, making early mistakes matter longer — the roguelike
//! ratchet lives here, not in luck.

/// Base respawn delay — one second at 120Hz. Long enough to feel the death,
/// short enough that the player is back in the arena before frustration lands.
pub const RESPAWN_BASE_TICKS: u32 = 120;

/// Each additional death adds half a second — the compounding ratchet.
/// Run-over total for 5 deaths: 120 + 60×5 = 420 ticks ≈ 3.5 s.
pub const RESPAWN_SCALE_PER_DEATH: u32 = 60;

// ── State machine ─────────────────────────────────────────────────────────────

/// The two observable states of a respawn cycle.
/// `Dead` carries the integer countdown so `ticks_remaining()` is O(1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnState {
    /// Entity is alive and may act.
    Alive,
    /// Entity is dead. Stores ticks remaining until respawn.
    Dead {
        /// Ticks remaining until respawn.
        ticks_remaining: u32
    },
}

// ── Timer ─────────────────────────────────────────────────────────────────────

/// Per-entity respawn timer. `Copy`, zero heap, integer-only.
///
/// Lives outside the main simulation state because death penalty is run-persistent:
/// a rollback restores sim positions and HP, but the death count and penalties persist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespawnTimer {
    /// Current respawn state (alive or dead with countdown).
    pub state: RespawnState,
    /// Total deaths this run — drives both the penalty ratchet and the
    /// scar ledger pressure. Never resets; it IS the ratchet.
    pub deaths: u16,
    /// World-space respawn origin X coordinate (mm). Set at spawn; restore the entity here.
    pub spawn_x_mm: i64,
    /// World-space respawn origin Y coordinate (mm). Set at spawn; restore the entity here.
    pub spawn_y_mm: i64,
}

impl RespawnTimer {
    /// Create a new respawn timer with the given spawn coordinates.
    pub fn new(spawn_x_mm: i64, spawn_y_mm: i64) -> Self {
        Self {
            state: RespawnState::Alive,
            deaths: 0,
            spawn_x_mm,
            spawn_y_mm,
        }
    }

    /// Called on a death TRANSITION (hp went >0→0). Increments the death
    /// counter and starts the scaled countdown.
    ///
    /// Penalty = `RESPAWN_BASE_TICKS + RESPAWN_SCALE_PER_DEATH × deaths_this_run`
    /// (post-increment — the FIRST death uses `deaths=1`).
    pub fn die(&mut self) {
        self.deaths = self.deaths.saturating_add(1);
        let ticks = RESPAWN_BASE_TICKS + RESPAWN_SCALE_PER_DEATH * (self.deaths as u32);
        self.state = RespawnState::Dead { ticks_remaining: ticks };
    }

    /// Advance the countdown one 120Hz tick. Returns `true` on the single frame
    /// the entity transitions back to Alive — the caller should revive it.
    ///
    /// Called unconditionally regardless of alive/dead so there is NO branch on
    /// the hot-path caller side; the check is internal.
    #[inline]
    pub fn tick(&mut self) -> bool {
        if let RespawnState::Dead { ticks_remaining } = &mut self.state {
            *ticks_remaining -= 1;
            if *ticks_remaining == 0 {
                self.state = RespawnState::Alive;
                return true; // respawn frame
            }
        }
        false
    }

    /// True when the entity is alive and may move / take damage.
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.state == RespawnState::Alive
    }

    /// Ticks remaining until respawn (0 when alive).
    #[inline]
    pub fn ticks_remaining(&self) -> u32 {
        match self.state {
            RespawnState::Dead { ticks_remaining } => ticks_remaining,
            RespawnState::Alive => 0,
        }
    }

    /// Total deaths this run.
    #[inline]
    pub fn deaths(&self) -> u16 {
        self.deaths
    }
}

impl Default for RespawnTimer {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

// ── Respawn routing (deterministic hash for the death anchor) ──────────────────

/// A deterministic hash that identifies WHERE and WHEN the entity died.
/// Uses FNV-1a over `[player_hash, zone_hash, x_mm, y_mm, tick]`.
///
/// Used as the `subject_hash` when forging a death record (links the scar to
/// the exact world location).
pub fn death_anchor_hash(
    player_seed: u64,
    zone_seed: u64,
    x_mm: i64,
    y_mm: i64,
    tick: u64,
) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for v in [player_seed, zone_seed, x_mm as u64, y_mm as u64, tick] {
        h ^= v;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_alive() {
        let timer = RespawnTimer::new(0, 0);
        assert!(timer.is_alive());
        assert_eq!(timer.ticks_remaining(), 0);
        assert_eq!(timer.deaths(), 0);
    }

    #[test]
    fn die_transitions_to_dead_and_sets_timer() {
        let mut timer = RespawnTimer::new(0, 0);
        timer.die();
        // First death: base + scale×1
        let expected = RESPAWN_BASE_TICKS + RESPAWN_SCALE_PER_DEATH;
        assert!(!timer.is_alive());
        assert_eq!(timer.ticks_remaining(), expected);
        assert_eq!(timer.deaths(), 1);
    }

    #[test]
    fn die_twice_scales_penalty() {
        let mut timer = RespawnTimer::new(0, 0);
        timer.die(); // death 1
        // Force into Alive so die() can be called again.
        timer.state = RespawnState::Alive;
        timer.die(); // death 2
        let expected = RESPAWN_BASE_TICKS + RESPAWN_SCALE_PER_DEATH * 2;
        assert_eq!(timer.ticks_remaining(), expected);
        assert_eq!(timer.deaths(), 2);
    }

    #[test]
    fn tick_returns_true_on_the_respawn_frame() {
        let mut timer = RespawnTimer::new(0, 0);
        timer.die(); // first death: base + scale = 120+60 = 180 ticks
        let deadline = timer.ticks_remaining();
        let mut respawn_frames = 0u32;
        for _ in 0..deadline {
            if timer.tick() {
                respawn_frames += 1;
            }
        }
        assert_eq!(respawn_frames, 1, "exactly one respawn frame must fire");
        assert!(timer.is_alive(), "must be Alive after the countdown");
    }

    #[test]
    fn tick_noop_when_already_alive() {
        let mut timer = RespawnTimer::new(0, 0);
        // Repeated ticks must not panic or change state.
        for _ in 0..300 {
            assert!(!timer.tick(), "alive timer must never fire a respawn event");
        }
        assert!(timer.is_alive());
    }

    #[test]
    fn deaths_counter_saturates_rather_than_wrapping() {
        let mut timer = RespawnTimer::new(0, 0);
        timer.deaths = u16::MAX;
        timer.die(); // saturating_add should not wrap
        assert_eq!(timer.deaths(), u16::MAX, "death count must saturate at u16::MAX");
    }

    #[test]
    fn death_anchor_hash_is_deterministic() {
        let a = death_anchor_hash(42, 7, 1000, -500, 120);
        let b = death_anchor_hash(42, 7, 1000, -500, 120);
        assert_eq!(a, b, "same inputs must produce the same hash");
    }

    #[test]
    fn death_anchor_hash_is_sensitive_to_position() {
        let a = death_anchor_hash(1, 1, 1000, 0, 1);
        let b = death_anchor_hash(1, 1, 1001, 0, 1); // x_mm +1
        assert_ne!(a, b, "hash must differ for distinct positions");
    }

    #[test]
    fn death_anchor_hash_is_sensitive_to_tick() {
        let a = death_anchor_hash(1, 1, 0, 0, 100);
        let b = death_anchor_hash(1, 1, 0, 0, 101);
        assert_ne!(a, b, "hash must differ for distinct ticks");
    }
}
