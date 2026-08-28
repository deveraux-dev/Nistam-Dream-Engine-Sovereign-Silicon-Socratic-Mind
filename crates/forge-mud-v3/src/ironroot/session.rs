//! Session state for an active run — ported from
//! `F:\NewRepo\crates\ironroot\src\session.rs` (2026-08-13, "keep draining
//! ironroot").
//!
//! **Scope cut (L15, named plainly):** the v2 source also `impl Session for
//! IronrootSession` — a trait from `forge_game_systems::session`, part of a
//! polymorphic multi-game host abstraction (dispatch across several
//! playable games behind one interface). `forge-mud-v3` is its own single
//! game, not a multi-game host — that trait and its `InputBits` dependency
//! are unported and genuinely not needed here. Only the struct and its own
//! inherent methods (`new`/`tick`/`rng`) are ported.
//!
//! The RNG uses [`crate::rng::Mulberry32`] — this crate's one home for that
//! generator (see `rng.rs`'s own doc comment) — not a second copy.
//!
//! **Also cut:** the v2 source derived `Serialize`/`Deserialize` (serde) and
//! had a save/load round-trip test. `serde` is not a dependency of this
//! crate (this workspace's own convention elsewhere is serde stripped at
//! the boundary, e.g. `ghostmoon.rs`'s doc comment) and nothing here yet
//! calls into a real save path — that's `persist.rs`'s job, unported.
//! Adding the dependency now, before a real caller needs it, would be
//! exactly the "just in case" entropy spend C10 forbids. Revisit when
//! `persist.rs` lands.

use crate::rng::Mulberry32;

/// State for an active run. The RNG is derived on demand from `master_seed`
/// + `tick_count` so the session stays `Clone` — `Mulberry32` itself is not
/// stored directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IronrootSession {
    /// The run's own seed — every deterministic draw traces back to this.
    pub master_seed: u64,
    /// Which wave/encounter the run is on.
    pub wave_number: u32,
    /// Ticks elapsed since the run started.
    pub tick_count: u64,
    /// Whether the run is still live — `tick()` is a no-op once `false`.
    pub active: bool,
}

impl IronrootSession {
    /// Start a new run from a seed.
    pub fn new(seed: u64) -> Self {
        Self { master_seed: seed, wave_number: 0, tick_count: 0, active: true }
    }

    /// Advance one tick. A no-op once the session is inactive.
    pub fn tick(&mut self) {
        if !self.active {
            return;
        }
        self.tick_count = self.tick_count.saturating_add(1);
    }

    /// Derive a fresh deterministic RNG pinned to this session's current tick.
    pub fn rng(&self) -> Mulberry32 {
        Mulberry32::new(self.master_seed.wrapping_add(self.tick_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_sets_seed_and_active() {
        let s = IronrootSession::new(42);
        assert_eq!(s.master_seed, 42);
        assert_eq!(s.wave_number, 0);
        assert_eq!(s.tick_count, 0);
        assert!(s.active);
    }

    #[test]
    fn rng_is_deterministic_per_tick() {
        let s = IronrootSession::new(42);
        let mut a = s.rng();
        let mut b = s.rng();
        assert_eq!(a.next_u32(), b.next_u32());
    }

    #[test]
    fn tick_advances_count_when_active() {
        let mut s = IronrootSession::new(1);
        s.tick();
        s.tick();
        assert_eq!(s.tick_count, 2);
    }

    #[test]
    fn tick_is_noop_when_inactive() {
        let mut s = IronrootSession::new(1);
        s.active = false;
        s.tick();
        assert_eq!(s.tick_count, 0);
    }

    #[test]
    fn cloning_preserves_state() {
        let mut s = IronrootSession::new(7);
        s.tick();
        let cloned = s.clone();
        assert_eq!(cloned, s);
    }
}
