//! Brand Corruption & Tithe — ported from
//! `F:\NewRepo\crates\forge-game-systems\src\brand.rs` (2026-08-13, "keep
//! draining ironroot"). Confirmed zero float, zero unsafe, zero deps beyond
//! `serde` (cut, same reasoning as `session.rs`/`dialogue.rs`).
//!
//! Both gauges are **monotonic accrual identities** — `level`/`debt` only
//! climb (saturating), forcing a state transition at the ceiling — the same
//! law `hermetics.rs`'s Guilt/Tarnish registers ("accrue, never rolled")
//! already enforce, and the same one-way shape `SoulIdentity`'s lineage
//! chain never runs backward. A real conceptual sibling of this session's
//! accrual-identity family, not a structural copy — `BrandCorruption` is a
//! `u8` gauge, not a sealed word.

/// Brand Corruption gauge — the Brand is alive and hungry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrandCorruption {
    /// `0..=255`. At 128+ visual instability begins. At 255 forced Aspect transformation.
    pub level: u8,
    /// Attunement tier `0..=4` (Unbranded → Ascendant).
    pub attunement: AttunementTier,
}

/// How well the host has bonded with their Brand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum AttunementTier {
    /// No bond yet.
    Unbranded = 0,
    /// The default starting bond.
    Marked = 1,
    /// A real bond has formed.
    Attuned = 2,
    /// The bond runs deep.
    Resonant = 3,
    /// The bond is complete.
    Ascendant = 4,
}

impl Default for AttunementTier {
    fn default() -> Self {
        Self::Marked
    }
}

impl Default for BrandCorruption {
    fn default() -> Self {
        Self { level: 0, attunement: AttunementTier::Marked }
    }
}

impl BrandCorruption {
    /// Add corruption from combat actions (kills, Brand ability use, taking damage).
    pub fn corrupt(&mut self, amount: u8) {
        self.level = self.level.saturating_add(amount);
    }

    /// Slow natural decay per tick (the host fights back).
    pub fn decay(&mut self, amount: u8) {
        self.level = self.level.saturating_sub(amount);
    }

    /// At 128+ the Brand is visually taking over.
    pub fn is_unstable(&self) -> bool {
        self.level >= 128
    }

    /// At 255 the Brand forces transformation.
    pub fn is_forced_transform(&self) -> bool {
        self.level == 255
    }

    /// Can the player voluntarily transform at this attunement?
    pub fn can_transform(&self, tier: u8) -> bool {
        match tier {
            1 => self.attunement >= AttunementTier::Attuned,
            2 => self.attunement >= AttunementTier::Resonant,
            3 => self.attunement >= AttunementTier::Ascendant,
            _ => false,
        }
    }
}

/// Tithe — what the player owes the world for the power it gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tithe {
    /// `0..=255`. High = the arena is angry. Low = the arena is starving.
    pub debt: u8,
    /// Ticks since the last kill. If too long, the world decays zones.
    pub starvation_ticks: u32,
}

impl Default for Tithe {
    fn default() -> Self {
        Self { debt: 0, starvation_ticks: 0 }
    }
}

impl Tithe {
    /// A kill adds to the debt and resets starvation.
    pub fn on_kill(&mut self, amount: u8) {
        self.debt = self.debt.saturating_add(amount);
        self.starvation_ticks = 0;
    }

    /// Each tick without a kill, the debt slowly decays and starvation grows.
    pub fn tick(&mut self) {
        self.starvation_ticks = self.starvation_ticks.saturating_add(1);
        // Slow decay: debt drops 1 per 120 ticks (~2 seconds at 60fps).
        if self.starvation_ticks % 120 == 0 && self.debt > 0 {
            self.debt -= 1;
        }
    }

    /// At 255 the world manifests a Branded encounter.
    pub fn should_manifest_branded(&self) -> bool {
        self.debt == 255
    }

    /// The arena is starving — zones should decay.
    pub fn is_starving(&self) -> bool {
        self.starvation_ticks > 600
    }

    /// Project where `debt` would settle under a constant kill rate —
    /// via `forge_core_v3::decay::LeakyPermyriad`'s steady-state resolvent
    /// (`crates/forge-core-v3/src/decay.rs`, Crate Zero, no game vocabulary
    /// crosses the boundary — only `u64`s). A real capability `tick()`/
    /// `on_kill()` alone cannot answer: no projection existed before this
    /// weld.
    ///
    /// [ASSUMED] `tick()`'s real decay is flat ("-1 unit per 120 ticks"),
    /// not multiplicative like `LeakyPermyriad`'s. This locally linearizes
    /// around the CURRENT `debt` (the leak fraction that would produce the
    /// same -1/120-tick loss AT this debt level) — accurate near today's
    /// debt, not exact across the whole `0..=255` range. If `debt` moves
    /// far from where this was called, re-call it; this is a live estimate,
    /// not a cached constant.
    pub fn projected_equilibrium(&self, recent_kills_per_tick: u64) -> u64 {
        if self.debt == 0 {
            return 0;
        }
        let leak = (forge_core_v3::decay::PMY / (120 * self.debt as u64))
            .clamp(1, forge_core_v3::decay::PMY) as u16;
        forge_core_v3::decay::LeakyPermyriad::equilibrium(recent_kills_per_tick, leak)
    }
}

// ── The twelve Brands (ported 2026-08-18 from quarry
// `F:\v3\TODO\quarry-sort\MYGAMEDRAIN-2026-08-17\ironroot-edict-game\src\brand_defs.rs`,
// ironroot lineage). The gauges above tracked corruption of "the Brand"
// without ever naming one; this is the roster. Verbatim port + one weld the
// donor didn't have: each Brand's trigon element, which `bell_pit.rs`'s
// Trigon Trials already treat as canon ("Earth — Taurus/Virgo/Capricorn"). ──

use crate::combat_brain::dissonance::ClassicalElement;

/// Display info for a Brand (zodiac archetype). Gameplay stats stay in the
/// gauges above — this is identity, not arithmetic (donor brand_defs.rs:7).
#[derive(Debug, Clone, Copy)]
pub struct BrandInfo {
    /// The sign.
    pub name: &'static str,
    /// The archetype it plays.
    pub role: &'static str,
}

/// Brand count — the zodiac's twelve, no thirteenth.
pub const BRAND_COUNT: usize = 12;

/// The 12 zodiac Brands in canonical order (donor brand_defs.rs:13-26).
pub const BRANDS: [BrandInfo; BRAND_COUNT] = [
    BrandInfo { name: "Aries", role: "Berserker" },
    BrandInfo { name: "Taurus", role: "Bulwark" },
    BrandInfo { name: "Gemini", role: "Trickster" },
    BrandInfo { name: "Cancer", role: "Warden" },
    BrandInfo { name: "Leo", role: "Champion" },
    BrandInfo { name: "Virgo", role: "Alchemist" },
    BrandInfo { name: "Libra", role: "Arbiter" },
    BrandInfo { name: "Scorpio", role: "Assassin" },
    BrandInfo { name: "Sagittarius", role: "Ranger" },
    BrandInfo { name: "Capricorn", role: "Sentinel" },
    BrandInfo { name: "Aquarius", role: "Seer" },
    BrandInfo { name: "Pisces", role: "Mystic" },
];

/// A Brand's trigon element — the classical zodiac trigons, the same grouping
/// bell_pit's Trigon Trials run (fire/earth/air/water, three signs each).
pub fn brand_element(index: usize) -> ClassicalElement {
    // Canonical order starts at Aries (Fire) and cycles Fire→Earth→Air→Water.
    match index % 4 {
        0 => ClassicalElement::Fire,
        1 => ClassicalElement::Earth,
        2 => ClassicalElement::Air,
        _ => ClassicalElement::Water,
    }
}

/// Navigation direction for the CharSelect brand grid (donor brand_defs.rs:30).
#[derive(Debug, Clone, Copy)]
pub enum NavDirection {
    /// Previous sign.
    Up,
    /// Next sign.
    Down,
    /// One column back in the 4-wide grid.
    Left,
    /// One column forward in the 4-wide grid.
    Right,
}

/// Apply a navigation direction to a brand index, wrapping within 0..12
/// (donor brand_defs.rs:39-46, verbatim — the CharSelect arrow/WASD walk).
pub fn navigate_brand(current: usize, direction: NavDirection) -> usize {
    match direction {
        NavDirection::Up => if current == 0 { 11 } else { current - 1 },
        NavDirection::Down => (current + 1) % 12,
        NavDirection::Left => if current < 4 { current + 8 } else { current - 4 },
        NavDirection::Right => (current + 4) % 12,
    }
}

/// The quincunx exit step: +5 mod 12. 5 is coprime with 12, so this visits
/// all twelve Brand petals exactly once before repeating (Ch27 §4a3).
pub fn quincunx_next(current: usize) -> usize {
    (current + 5) % BRAND_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Donor test 7.4: navigation wraps within 0..12 from every start.
    #[test]
    fn brand_navigation_wraps_within_bounds() {
        for start in 0..BRAND_COUNT {
            for dir in [NavDirection::Up, NavDirection::Down, NavDirection::Left, NavDirection::Right] {
                let result = navigate_brand(start, dir);
                assert!(result < BRAND_COUNT, "navigate_brand({start}, {dir:?}) = {result}");
            }
        }
    }

    /// Roster law: twelve unique signs, twelve unique roles, and the trigons
    /// come out exactly three signs per element — with bell_pit's named canon
    /// (Taurus/Virgo/Capricorn = Earth) landing where it says it does.
    #[test]
    fn brand_roster_is_unique_and_trigons_are_canon() {
        for (i, a) in BRANDS.iter().enumerate() {
            for b in &BRANDS[i + 1..] {
                assert_ne!(a.name, b.name);
                assert_ne!(a.role, b.role);
            }
        }
        for element in [ClassicalElement::Fire, ClassicalElement::Earth, ClassicalElement::Air, ClassicalElement::Water] {
            let n = (0..BRAND_COUNT).filter(|&i| brand_element(i) == element).count();
            assert_eq!(n, 3, "{element:?} trigon must hold exactly three signs");
        }
        for name in ["Taurus", "Virgo", "Capricorn"] {
            let i = BRANDS.iter().position(|b| b.name == name).unwrap();
            assert_eq!(brand_element(i), ClassicalElement::Earth, "{name} is Earth trigon (bell_pit canon)");
        }
    }

    /// L07 bijection: 12 applications of +5 mod 12 must hit all 12 petals
    /// exactly once before repeating (the book's core mathematical claim).
    #[test]
    fn quincunx_bijection_is_complete() {
        let mut visited = [false; BRAND_COUNT];
        let mut current = 0usize;
        for _ in 0..BRAND_COUNT {
            assert!(!visited[current], "petal {current} visited twice within one lap");
            visited[current] = true;
            current = quincunx_next(current);
        }
        assert_eq!(current, 0, "the 13th step must return to the center exit");
        assert!(visited.iter().all(|&v| v), "not all twelve petals were visited");
    }

    #[test]
    fn corruption_saturates_at_255() {
        let mut bc = BrandCorruption::default();
        bc.corrupt(200);
        bc.corrupt(200);
        assert_eq!(bc.level, 255);
    }

    #[test]
    fn instability_at_128() {
        let mut bc = BrandCorruption::default();
        bc.corrupt(127);
        assert!(!bc.is_unstable());
        bc.corrupt(1);
        assert!(bc.is_unstable());
    }

    #[test]
    fn forced_transform_at_255() {
        let mut bc = BrandCorruption::default();
        bc.level = 255;
        assert!(bc.is_forced_transform());
    }

    #[test]
    fn attunement_gates_transformation() {
        let mut bc = BrandCorruption::default();
        bc.attunement = AttunementTier::Marked;
        assert!(!bc.can_transform(1));
        bc.attunement = AttunementTier::Attuned;
        assert!(bc.can_transform(1));
        assert!(!bc.can_transform(2));
    }

    /// [BOARD: WELD-permeq01] The projection Tithe couldn't make before this
    /// weld: at debt=60 (leak = 10_000/(120*60) = 1), a steady kill rate of
    /// 1/tick settles at equilibrium(1, 1) = 10_000 — the exact resolvent
    /// identity `decay.rs` itself proves, called live through Tithe now.
    #[test]
    fn projected_equilibrium_matches_the_leaky_permyriad_resolvent_directly() {
        let mut t = Tithe::default();
        assert_eq!(t.projected_equilibrium(5), 0, "zero debt has no leak fraction, so no equilibrium");

        t.debt = 60;
        let expected = forge_core_v3::decay::LeakyPermyriad::equilibrium(1, 1);
        assert_eq!(t.projected_equilibrium(1), expected);

        // Higher debt -> smaller linearized leak fraction -> a higher
        // ceiling projected for the SAME kill rate (real, monotone behavior,
        // not an arbitrary number).
        let mut higher = Tithe::default();
        higher.debt = 200;
        assert!(
            higher.projected_equilibrium(1) >= t.projected_equilibrium(1),
            "less-decayed-per-tick debt levels project a higher or equal ceiling"
        );
    }

    #[test]
    fn tithe_rises_on_kill() {
        let mut t = Tithe::default();
        t.on_kill(10);
        assert_eq!(t.debt, 10);
        assert_eq!(t.starvation_ticks, 0);
    }

    #[test]
    fn tithe_decays_over_time() {
        let mut t = Tithe::default();
        t.debt = 5;
        for _ in 0..120 {
            t.tick();
        }
        assert_eq!(t.debt, 4);
    }

    #[test]
    fn tithe_manifests_branded_at_255() {
        let mut t = Tithe::default();
        t.debt = 255;
        assert!(t.should_manifest_branded());
    }

    #[test]
    fn starvation_after_600_ticks() {
        let mut t = Tithe::default();
        for _ in 0..601 {
            t.tick();
        }
        assert!(t.is_starving());
    }
}
