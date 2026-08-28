//! essence_registry.rs — the 64-slot SEMANTIC palette, the second leg of
//! `colourid = materialid = essence-hash`. Palette 1 (`material_registry`) is
//! PHYSICAL: what a vixel is MADE of. This palette is SEMANTIC: what it MEANS /
//! does. An entity (or VibeBuffer cell) carries `material_id` (6 bits) + an
//! `essence_id` (6 bits); physical stats derive from the material, RPG/semantic
//! stats from the essence. 64 essences = 8 families × 8 = contiguous SoA, no holes.
//! Built from Sean's two-palette design 2026-06-05.
//!
//! Axes (all Permyriad, 0..=10000):
//!   Potency    — raw power magnitude.
//!   Volatility — stable (0) ↔ chaotic (10000).
//!   Polarity   — entropy/death (0) ↔ order/life (10000).
//!   Affinity   — physical (0) ↔ spiritual (10000).
//!   Spread     — coupling / propagation reach.
//!   Tier       — common (0) ↔ legendary (10000).
//!
//! `essence_atom` resolves a slot into a runtime [`EssenceAtom`](crate::essence_registry::EssenceAtom); `rpg_stats`
//! derives six RPG stats from the six axes — the semantic mirror of
//! `MaterialAtom::physical_stats` (derive, don't hand-author). The whole-entity
//! stat block is `physical_stats(material) + rpg_stats(essence)`.

/// The eight essence families (each owns a contiguous run of 8 ids).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EssenceFamily {
    /// Classic elements (0–7).
    Primal,
    /// Compound elements (8–15).
    Extended,
    /// Decay / corrosion (16–23).
    Caustic,
    /// Life ↔ death (24–31).
    Vital,
    /// Cognition / emotion (32–39).
    Mind,
    /// Soul / faith (40–47).
    Spirit,
    /// Abstract forces (48–55).
    Cosmic,
    /// Bodies + fundamental forces (56–63).
    Celestial,
}

impl EssenceFamily {
    /// The family owning essence id 0..=63 (id / 8). Out-of-range (id/8 >= 7) folds
    /// into the last family — consistent with `essence_def`'s clamp to slot 63.
    pub const fn from_id(id: u8) -> Self {
        match id / 8 {
            0 => Self::Primal,
            1 => Self::Extended,
            2 => Self::Caustic,
            3 => Self::Vital,
            4 => Self::Mind,
            5 => Self::Spirit,
            6 => Self::Cosmic,
            _ => Self::Celestial,
        }
    }

    /// Get the family name as a string.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Primal => "Primal",
            Self::Extended => "Extended",
            Self::Caustic => "Caustic",
            Self::Vital => "Vital",
            Self::Mind => "Mind",
            Self::Spirit => "Spirit",
            Self::Cosmic => "Cosmic",
            Self::Celestial => "Celestial",
        }
    }
}

/// One essence slot — six integer semantic axes + a name. The semantic mirror of
/// [`crate::material_registry::MaterialDef`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EssenceDef {
    /// Essence name.
    pub name: &'static str,
    /// Raw power magnitude (Permyriad 0–10000).
    pub potency: u16,
    /// Stable (0) ↔ chaotic (10000).
    pub volatility: u16,
    /// Entropy/death (0) ↔ order/life (10000).
    pub polarity: u16,
    /// Physical (0) ↔ spiritual (10000).
    pub affinity: u16,
    /// Coupling / propagation reach (Permyriad 0–10000).
    pub spread: u16,
    /// Common (0) ↔ legendary (10000).
    pub tier: u16,
}

const fn e(
    name: &'static str,
    potency: u16,
    volatility: u16,
    polarity: u16,
    affinity: u16,
    spread: u16,
    tier: u16,
) -> EssenceDef {
    EssenceDef { name, potency, volatility, polarity, affinity, spread, tier }
}

/// The 64 essences, indexed by `essence_id` (0..=63). Eight families of eight.
/// Columns: potency · volatility · polarity · affinity · spread · tier.
pub const ESSENCE: [EssenceDef; 64] = [
    // ── Primal (0–7): the classic elements — potent, physical, common ──
    e("Fire",       7000, 8000, 2000, 2000, 8000, 2000),
    e("Water",      6000, 4000, 6000, 3000, 7000, 2000),
    e("Earth",      6500, 1000, 5500, 1500, 3000, 1500),
    e("Air",        4500, 6000, 5000, 4000, 9000, 1500),
    e("Lightning",  8000, 9500, 3000, 3500, 7000, 3000),
    e("Ice",        5500, 2000, 4000, 2500, 5000, 2000),
    e("Light",      7000, 3000, 9000, 7000, 8000, 3500),
    e("Shadow",     6500, 5000, 1000, 6500, 7000, 3500),
    // ── Extended (8–15): compound elements ──
    e("Magma",      8000, 6000, 2000, 1500, 5000, 3000),
    e("Steam",      4500, 7000, 5000, 2500, 8500, 2500),
    e("Frost",      6000, 2500, 3500, 3000, 6000, 3000),
    e("Storm",      8500, 9000, 3500, 4000, 9000, 4000),
    e("Sand",       4000, 4000, 4500, 1000, 6500, 2000),
    e("Crystal",    6000, 1000, 7000, 5000, 2000, 4000),
    e("MetalChi",   7000, 2000, 6500, 4500, 3000, 3500),
    e("WoodChi",    5500, 3000, 7000, 4500, 6000, 3000),
    // ── Caustic (16–23): decay / corrosion — entropic, spreading ──
    e("Poison",     6000, 5000, 1500, 2500, 7500, 3000),
    e("Acid",       6500, 6000, 1500, 1500, 6000, 3000),
    e("Radiation",  8500, 7000, 1000, 3500, 9000, 5000),
    e("Plague",     7000, 6000,  800, 3000, 9500, 4500),
    e("Decay",      5000, 3000,  500, 3000, 7000, 3000),
    e("Spore",      4000, 4500, 3000, 2500, 9000, 3000),
    e("Smoke",      3500, 6000, 2500, 3000, 8500, 2000),
    e("Cinder",     4500, 5000, 1800, 2000, 6000, 2500),
    // ── Vital (24–31): life ↔ death — polarity at the extremes ──
    e("Life",       8000, 3000, 10000, 6000, 7000, 5000),
    e("Blood",      6500, 5000, 6000, 4000, 5000, 3500),
    e("Growth",     6000, 2500, 8500, 5000, 8000, 3500),
    e("Healing",    6500, 2000, 9500, 7000, 6000, 4000),
    e("Fertility",  6000, 3000, 9000, 5500, 7500, 3500),
    e("Hunger",     5500, 6000, 2000, 4000, 7000, 3500),
    e("Death",      8500, 3500,    0, 6500, 6500, 5500),
    e("Undeath",    7000, 4500,  500, 7000, 6000, 5000),
    // ── Mind (32–39): cognition / emotion — high affinity, internal ──
    e("Logic",      6000,  500, 9000, 8000, 3000, 4000),
    e("Memory",     5000, 2000, 7000, 8500, 4000, 4000),
    e("Dream",      5500, 7000, 6000, 9000, 6000, 4500),
    e("Madness",    6500, 9500, 1500, 8000, 7000, 4500),
    e("Fear",       6000, 7000, 2000, 7000, 8000, 3500),
    e("Rage",       7500, 8500, 2500, 6000, 6500, 3500),
    e("Calm",       4500,  500, 8000, 8000, 5000, 3500),
    e("Will",       7000, 2000, 7500, 7500, 3500, 4500),
    // ── Spirit (40–47): soul / faith — most spiritual, hope ↔ despair ──
    e("Soul",       7500, 3000, 7000, 10000, 5000, 5500),
    e("Echo",       4000, 4000, 5000, 9000, 7000, 4000),
    e("Hope",       6000, 3000, 9500, 9000, 8000, 4500),
    e("Despair",    6000, 5000,  500, 9000, 8000, 4500),
    e("Faith",      7000, 2500, 9000, 9500, 7000, 5000),
    e("Curse",      7000, 6000,  800, 8500, 7500, 5000),
    e("Blessing",   7000, 2500, 9500, 9500, 7000, 5000),
    e("Wraith",     6000, 6000, 1000, 9000, 6000, 4500),
    // ── Cosmic (48–55): abstract forces — legendary, order ↔ chaos ──
    e("Time",       8500, 4000, 6000, 8000, 9000, 7000),
    e("Space",      8500, 3500, 6500, 7500, 9500, 7000),
    e("Order",      8000,    0, 10000, 7000, 6000, 7000),
    e("Chaos",      8000, 10000,   0, 7000, 8000, 7000),
    e("Fate",       8000, 2000, 7000, 8500, 7000, 7500),
    e("Luck",       5000, 8000, 6000, 7000, 6000, 6000),
    e("Entropy",    7500, 7000,  200, 6000, 9000, 7000),
    e("Creation",   9500, 5000, 9500, 8000, 8500, 8500),
    // ── Celestial / Force (56–63): bodies + fundamental forces; Null = absence ──
    e("Solar",      9000, 5000, 8000, 6000, 9000, 7000),
    e("Lunar",      6500, 4000, 6000, 7500, 8000, 6500),
    e("Stellar",    9000, 4500, 7000, 7000, 9500, 7500),
    e("Gravity",    8500, 1500, 6500, 4000, 9500, 7000),
    e("Magnetism",  7000, 3000, 6000, 3500, 8000, 6000),
    e("Resonance",  6500, 4000, 6500, 6000, 9000, 6000),
    e("Aether",     8000, 3000, 7500, 9500, 8500, 8000),
    // #63 Null — the absence essence (semantic mirror of material Void): every
    // axis zero, so every derived RPG stat is zero. The clean nothing-slot.
    e("Null",          0,    0,    0,    0,    0,    0),
];

/// Look up an essence slot (0..=63). Out-of-range clamps to the last slot.
/// Derive the resonance-glow susceptibility (Permyriad) from two essence axes:
/// `potency × affinity`. Spiritual potency halos; brute-physical mass does not.
/// ONE formula, shared by both [`EssenceAtom::luminance_q`] and the compile-time
/// GPU LUT [`ESSENCE_LUMINANCE`], so the two can never drift.
const fn lum_q(potency: u16, affinity: u16) -> u16 {
    // `if` clamp, not `u16::min` — `Ord::min` is not const-stable, and this fn
    // must stay const for the compile-time `ESSENCE_LUMINANCE` LUT.
    let p = (if potency > 10_000 { 10_000 } else { potency }) as u32;
    let a = (if affinity > 10_000 { 10_000 } else { affinity }) as u32;
    (p * a / 10_000) as u16
}

/// The 64-entry resonance-glow LUT, derived from the essence axes at COMPILE
/// time (`potency × affinity`). Bind it as a GPU uniform beside the material /
/// essence tables; the canvas shader does `vibe.glow × ESSENCE_LUMINANCE[essence_id]`
/// so *what a cell MEANS* decides how brightly it answers the vibe/aura field.
/// This is the resonance-RESPONSE glow — NO OVERLAP with material `emission_pmy`
/// (authored static emission); they compose as
/// `final_glow = material_emission + vibe_glow × essence_luminance`. Physical
/// essences (Earth ~975) stay dark; spiritual ones (Light ~4900, the Spirit
/// family) blaze. Derived, never hand-authored — the 7th derive off the six axes.
pub const ESSENCE_LUMINANCE: [u16; 64] = {
    let mut t = [0u16; 64];
    let mut i = 0;
    while i < 64 {
        t[i] = lum_q(ESSENCE[i].potency, ESSENCE[i].affinity);
        i += 1;
    }
    t
};

/// Look up an essence definition by ID (0–63, clamped).
pub fn essence_def(id: u8) -> &'static EssenceDef {
    &ESSENCE[(id as usize).min(63)]
}

/// Resolve an essence slot into a runtime [`EssenceAtom`] (axes + family). The
/// semantic mirror of [`crate::material_registry::material_atom`].
pub fn essence_atom(id: u8) -> EssenceAtom {
    let d = essence_def(id);
    EssenceAtom {
        family: EssenceFamily::from_id(id.min(63)),
        potency: d.potency,
        volatility: d.volatility,
        polarity: d.polarity,
        affinity: d.affinity,
        spread: d.spread,
        tier: d.tier,
    }
}

/// The runtime semantic atom for one essence identity (`essence_id`). Carries the
/// six axes plus the resolved [`EssenceFamily`] — the SEMANTIC counterpart to
/// [`crate::material_binding::MaterialAtom`]. All axes Permyriad (10000 = 1.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EssenceAtom {
    /// Essence family.
    pub family: EssenceFamily,
    /// Raw power magnitude (Permyriad 0–10000).
    pub potency: u16,
    /// Stable (0) ↔ chaotic (10000).
    pub volatility: u16,
    /// Entropy/death (0) ↔ order/life (10000).
    pub polarity: u16,
    /// Physical (0) ↔ spiritual (10000).
    pub affinity: u16,
    /// Coupling / propagation reach (Permyriad 0–10000).
    pub spread: u16,
    /// Common (0) ↔ legendary (10000).
    pub tier: u16,
}

impl EssenceAtom {
    /// Six derived RPG/semantic stats, computed from the six essence axes — NOT
    /// hand-authored. The semantic mirror of `MaterialAtom::physical_stats`; each
    /// is Permyriad (0..=10000). Whole-entity stats =
    /// `physical_stats(material) + rpg_stats(essence)`.
    ///
    ///   Vigor      = potency × (1 − affinity)   — physical might (potent + bodily)
    ///   Spirit     = potency × affinity         — spiritual might (potent + soulful)
    ///   Logic      = polarity × affinity        — ordered + spiritual = reason
    ///   Momentum   = volatility × (1 − affinity)— chaotic + physical = kinetic drive
    ///   Resilience = (1 − volatility) × potency — stable + potent = endurance
    ///   Shadow     = (1 − polarity) × spread    — entropic + spreading = corruption
    pub fn rpg_stats(&self) -> RpgStats {
        let pot = self.potency.min(10_000) as u32;
        let vol = self.volatility.min(10_000) as u32;
        let pol = self.polarity.min(10_000) as u32;
        let aff = self.affinity.min(10_000) as u32;
        let spread = self.spread.min(10_000) as u32;
        let inv_aff = 10_000 - aff;
        let inv_vol = 10_000 - vol;
        let inv_pol = 10_000 - pol;
        RpgStats {
            vigor: (pot * inv_aff / 10_000) as u16,
            spirit: (pot * aff / 10_000) as u16,
            logic: (pol * aff / 10_000) as u16,
            momentum: (vol * inv_aff / 10_000) as u16,
            resilience: (inv_vol * pot / 10_000) as u16,
            shadow: (inv_pol * spread / 10_000) as u16,
        }
    }

    /// Resonance-response glow susceptibility (Permyriad): how brightly this
    /// essence HALOS when struck by the vibe/aura field. Derived (not authored)
    /// = `potency × affinity` — the same 2-axis-product shape as [`Self::rpg_stats`],
    /// and the semantic twin of material `emission_pmy`. NO OVERLAP: material owns
    /// literal/static emission, essence owns reactive spiritual luminance, so
    /// `final_glow = material_emission + vibe_glow × essence_luminance`. Matches
    /// [`ESSENCE_LUMINANCE`] for every slot (shared `lum_q`, cannot drift).
    pub const fn luminance_q(&self) -> u16 {
        lum_q(self.potency, self.affinity)
    }
}

/// Six derived RPG/semantic stats (Permyriad 0..=10000), each a pure function of
/// an [`EssenceAtom`]'s six axes — the semantic-layer twin of
/// [`crate::material_binding::PhysicalStats`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RpgStats {
    /// Physical might (potent + bodily).
    pub vigor: u16,
    /// Spiritual might (potent + soulful).
    pub spirit: u16,
    /// Ordered + spiritual = reason.
    pub logic: u16,
    /// Chaotic + physical = kinetic drive.
    pub momentum: u16,
    /// Stable + potent = endurance.
    pub resilience: u16,
    /// Entropic + spreading = corruption.
    pub shadow: u16,
}

/// ADR-0012 §4 composition: physical(material_id) + rpg(essence_id), both derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WholeEntityStats {
    /// Physical stats from material.
    pub physical: crate::material_binding::PhysicalStats,
    /// RPG stats from essence.
    pub rpg: RpgStats,
}

/// whole = physical_stats(material_id) + rpg_stats(essence_id) (ADR-0012 §4; 6-bit slots).
pub fn whole_entity_stats(material_id: u8, essence_id: u8) -> WholeEntityStats {
    WholeEntityStats {
        physical: crate::material_registry::material_atom(material_id).physical_stats(),
        rpg: essence_atom(essence_id).rpg_stats(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_entity_stats_composes_both_palettes() {
        let w = whole_entity_stats(0, 6); // Gold material + Light essence
        assert!(w.physical.heft > 8000);
        assert!(w.rpg.spirit > 3000);
        let z = whole_entity_stats(62, 63); // Void material + Null essence
        assert_eq!(z.rpg, RpgStats::default());
        assert_eq!(z.physical.heft, 0);
    }

    #[test]
    fn essence_luminance_is_spiritual_potency_not_physical_mass() {
        // Resonance-response glow = potency × affinity. Spiritual essences halo;
        // brute-physical ones stay dark. (This is NOT material emission.)
        let light = essence_atom(6); // Light: potency 7000 × affinity 7000
        let earth = essence_atom(2); // Earth: potency 6500 × affinity 1500
        assert_eq!(light.luminance_q(), 4900);
        assert_eq!(earth.luminance_q(), 975);
        assert!(
            light.luminance_q() > earth.luminance_q() * 4,
            "spiritual Light must blaze far brighter than physical Earth"
        );
    }

    #[test]
    fn essence_luminance_lut_matches_the_method_for_all_64() {
        // The compile-time LUT and the runtime method share lum_q — prove no drift
        // across the whole 64-slot palette.
        for id in 0u8..64 {
            assert_eq!(
                ESSENCE_LUMINANCE[id as usize],
                essence_atom(id).luminance_q(),
                "slot {id}: LUT/method drift"
            );
        }
    }

    #[test]
    fn registry_is_exactly_64_dense_unique_slots() {
        assert_eq!(ESSENCE.len(), 64);
        assert!(ESSENCE.iter().all(|d| !d.name.is_empty()), "no holes");
        // Contiguous SoA law: no duplicate slots.
        for i in 0..ESSENCE.len() {
            for j in (i + 1)..ESSENCE.len() {
                assert_ne!(ESSENCE[i].name, ESSENCE[j].name, "duplicate essence");
            }
        }
    }

    #[test]
    fn families_partition_the_64_in_runs_of_eight() {
        assert_eq!(EssenceFamily::from_id(0), EssenceFamily::Primal);
        assert_eq!(EssenceFamily::from_id(7), EssenceFamily::Primal);
        assert_eq!(EssenceFamily::from_id(8), EssenceFamily::Extended);
        assert_eq!(EssenceFamily::from_id(16), EssenceFamily::Caustic);
        assert_eq!(EssenceFamily::from_id(24), EssenceFamily::Vital);
        assert_eq!(EssenceFamily::from_id(32), EssenceFamily::Mind);
        assert_eq!(EssenceFamily::from_id(40), EssenceFamily::Spirit);
        assert_eq!(EssenceFamily::from_id(48), EssenceFamily::Cosmic);
        assert_eq!(EssenceFamily::from_id(56), EssenceFamily::Celestial);
        assert_eq!(EssenceFamily::from_id(63), EssenceFamily::Celestial);
    }

    #[test]
    fn fire_is_physical_might_soul_is_spiritual() {
        // Fire (0): potent + physical + chaotic → vigor & momentum, little spirit.
        let fire = essence_atom(0).rpg_stats();
        assert!(fire.vigor > fire.spirit, "fire is bodily, not soulful");
        assert!(fire.momentum > 5000, "fire is kinetic");
        // Soul (40): potent + max-spiritual → all spirit, no vigor.
        let soul = essence_atom(40).rpg_stats();
        assert_eq!(essence_def(40).name, "Soul");
        assert!(soul.spirit > soul.vigor, "soul is spiritual");
        assert!(soul.vigor < 500, "max affinity drains physical vigor");
    }

    #[test]
    fn logic_is_the_ordered_mind() {
        // Logic (32): high polarity (order) × high affinity (mental) → high logic,
        // near-zero momentum (very stable, very non-physical).
        let logic = essence_atom(32).rpg_stats();
        assert_eq!(essence_def(32).name, "Logic");
        assert!(logic.logic > 6000, "ordered + spiritual = reason");
        assert!(logic.momentum < logic.logic, "logic is not kinetic");
    }

    #[test]
    fn order_endures_chaos_corrupts() {
        let order = essence_atom(50).rpg_stats(); // vol 0, pol 10000
        let chaos = essence_atom(51).rpg_stats(); // vol 10000, pol 0
        assert!(order.resilience > chaos.resilience, "stable order endures");
        assert_eq!(chaos.resilience, 0, "max volatility = no endurance");
        assert!(chaos.shadow > order.shadow, "entropy corrupts");
        assert_eq!(order.shadow, 0, "max polarity (order) casts no shadow");
        assert!(order.logic > 0, "order is reasoned");
    }

    #[test]
    fn null_is_the_zero_essence() {
        // Slot 63 = the absence essence: every axis 0 → every stat 0.
        let d = essence_def(63);
        assert_eq!(d.name, "Null");
        assert_eq!((d.potency, d.volatility, d.polarity, d.affinity, d.spread, d.tier), (0, 0, 0, 0, 0, 0));
        let s = essence_atom(63).rpg_stats();
        assert_eq!(s, RpgStats::default(), "Null derives all-zero stats");
    }

    #[test]
    fn tier_rises_from_primal_to_cosmic() {
        let avg = |range: std::ops::Range<usize>| -> u32 {
            let r = range.clone();
            ESSENCE[range].iter().map(|d| d.tier as u32).sum::<u32>() / (r.len() as u32)
        };
        assert!(avg(48..56) > avg(0..8), "Cosmic essences are rarer than Primal");
        assert!(essence_def(55).tier > 8000, "Creation is top-tier");
        assert!(essence_def(62).tier >= 8000, "Aether is top-tier");
    }

    #[test]
    fn all_axes_and_stats_are_permyriad_bounded() {
        for id in 0u8..64 {
            let d = essence_def(id);
            for ax in [d.potency, d.volatility, d.polarity, d.affinity, d.spread, d.tier] {
                assert!(ax <= 10_000, "axis out of Permyriad range at {id}");
            }
            let s = essence_atom(id).rpg_stats();
            for st in [s.vigor, s.spirit, s.logic, s.momentum, s.resilience, s.shadow] {
                assert!(st <= 10_000, "stat out of Permyriad range at {id}");
            }
        }
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11): the
    /// Sunken Choir — a drowned reliquary in the abyss whose relics are bound
    /// to the Wraith essence (id 47, Spirit family) — is a lore claim about a
    /// specific semantic-atom's derived stats, not narrative prose alone. It
    /// anchors to the already-landed `essence_atom`/`rpg_stats` derivation:
    /// a Wraith is more spirit than flesh, and its high affinity/low polarity
    /// axes must derive that shape, not just assert it. [OBSERVED] fabric:
    /// `EssenceAtom::rpg_stats`, both landed in this file.
    #[test]
    fn sunken_choir_wraith_relic_lore_tie_is_spirit_not_flesh() {
        assert_eq!(essence_def(47).name, "Wraith");
        let wraith = essence_atom(47).rpg_stats();
        assert!(wraith.spirit > wraith.vigor, "a Wraith relic sings spirit, not muscle");
        assert!(wraith.shadow > 3000, "the Sunken Choir's low polarity must carry real corruption");
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11):
    /// the Cinderfall Breach's flame — the same named zone the Audio,
    /// Physics, and Lorekeeper bricks already anchor (event flag, fire
    /// ignition, smoke plume, VCE severity) — carries a Fire-essence
    /// signature: physical and chaotic, not spiritual. Anchors to the
    /// already-landed `essence_atom`/`rpg_stats` derivation. [OBSERVED]
    /// fabric: `EssenceAtom::rpg_stats`, already tested generically above
    /// (`fire_is_physical_might_soul_is_spiritual`).
    #[test]
    fn cinderfall_breach_fire_essence_lore_tie_is_physical_not_spiritual() {
        assert_eq!(essence_def(0).name, "Fire");
        let breach_flame = essence_atom(0).rpg_stats();
        assert!(breach_flame.vigor > breach_flame.spirit, "the Breach's flame must be bodily, not soulful");
        assert!(breach_flame.momentum > 5000, "the Breach's flame must be kinetic, matching its real fire-ignite physics");
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11):
    /// the Skyreach Pinnacle's lightning — the same summit the Audio and
    /// Lorekeeper bricks already anchor (the altitude ceiling, and the real
    /// Faraday-induction lightning rod:
    /// `forge-pp-lore-v3::electrical::skyreach_pinnacle_lightning_rod_lore_tie`)
    /// — carries a Storm-essence signature: volatile and far-reaching, not
    /// stable. Anchors to the already-landed `essence_atom`/`rpg_stats`
    /// derivation. [OBSERVED] fabric: `EssenceAtom::rpg_stats`, already
    /// tested generically above.
    #[test]
    fn skyreach_pinnacle_storm_essence_lore_tie_is_volatile_and_far_reaching() {
        assert_eq!(essence_def(11).name, "Storm");
        let storm = essence_atom(11);
        let stats = storm.rpg_stats();
        assert!(stats.momentum > 5000, "the Pinnacle's storm must be genuinely kinetic, not calm");
        assert!(storm.spread > 8000, "a lightning strike's reach must be real, near-maximal spread");
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11):
    /// the Thirteen Bells Warden — one of the six real, tested Bell Warden
    /// variants confirmed LIVE-WIRED into the actual game loop this session
    /// (`forge-mud-v3::game.rs:552-557`, selected by `select_warden_variant`
    /// on `perfect_parries > 4`, its own lesson "The bell can be answered")
    /// — carries a Resonance-essence signature: coupled and far-reaching,
    /// matching its own bell-chain combat mode. Anchors to the
    /// already-landed `essence_atom`/`rpg_stats` derivation rather than an
    /// invented "it resonates" flavour line. [OBSERVED] fabric:
    /// `EssenceAtom::rpg_stats`, already tested generically above.
    #[test]
    fn thirteen_bells_warden_resonance_essence_lore_tie() {
        assert_eq!(essence_def(61).name, "Resonance");
        let resonance = essence_atom(61);
        assert!(resonance.spread > 8000, "the Thirteen Bells Warden's chain must carry near-maximal coupling reach");
        let stats = resonance.rpg_stats();
        assert!(stats.logic > 3000, "an answerable bell must carry real ordered structure, not chaos alone");
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11):
    /// the Broken Forge's craft-discipline — the same forge already
    /// anchored across Physics/Sieve/Lorekeeper — carries a MetalChi
    /// signature: stable and enduring, not chaotic, matching a controlled
    /// smith's craft rather than wild destruction. Anchors to the
    /// already-landed `essence_atom`/`rpg_stats` derivation. [OBSERVED]
    /// fabric: `EssenceAtom::rpg_stats`, already tested generically above.
    #[test]
    fn broken_forge_metalchi_essence_lore_tie_is_stable_not_chaotic() {
        assert_eq!(essence_def(14).name, "MetalChi");
        let metalchi = essence_atom(14);
        let stats = metalchi.rpg_stats();
        assert!(stats.resilience > 4000, "the forge's craft-discipline must endure, not falter — low volatility means real resilience");
        assert!(stats.momentum < 3000, "a disciplined smith's chi is controlled, not wild kinetic chaos");
    }

    #[test]
    fn two_palettes_compose_on_one_entity() {
        // The VibeBuffer headline: one cell = material_id + essence_id; physical
        // stats come from the material table, RPG stats from the essence table —
        // independent axes read off two parallel 64-slot palettes.
        use crate::material_registry::material_atom;
        let phys = material_atom(20).physical_stats(); // Obsidian (physical)
        let rpg = essence_atom(7).rpg_stats(); // Shadow (semantic)
        assert!(phys.brittleness > 4000, "Obsidian shatters (from MATERIALS)");
        assert!(rpg.shadow > 4000, "Shadow corrupts (from ESSENCE)");
        // The two are independent: the essence doesn't touch physical brittleness.
        assert_eq!(phys.brittleness, material_atom(20).physical_stats().brittleness);
    }
}
