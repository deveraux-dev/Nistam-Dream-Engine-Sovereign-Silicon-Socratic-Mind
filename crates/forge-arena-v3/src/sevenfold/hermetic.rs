//! Hermetic stat & law system — a game-agnostic RPG character layer.
//!
//! Deterministic, integer-only. A single character-sheet for any actor — a
//! Pathfinder-style ability block with hermetic framing: seven registers
//! (three Active, three Passive, one cross-cutting), each governed by one of
//! the seven laws expressed as integer-only combat hooks.
//!
//! `from_legacy` folds a classic 7-stat block (str/sta/agi/dex/wis/int/cha) in.
//!
//! No float. No alloc. Copy. Sieve-safe (clean on the hot path).

// ── The 7 registers ──────────────────────────────────────────────────────────
// 8-bit registers (0..=255). Accumulation past 255 is not clamped silently —
// it is a Cataclysm (see `Cataclysm` + the per-law overflow checks).

/// The canonical ability block. Active (projective) + Passive (receptive),
/// plus Guilt as the cross-cutting Cause-&-Effect / Toll-Ledger register.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct HermeticStats {
    // ── Active (Projective / Kinetic) ──
    pub vigor: u8,         // VIG — force, HP, strike power          (≈ STR)
    pub momentum: u8,      // MOM — speed, turn priority, parry win  (≈ AGI+DEX)
    pub logic_depth: u8,   // LOG — mind, RNG-lock, Focus Mode       (≈ INT)
    // ── Passive (Receptive / Potential) ──
    pub shadow_weight: u8, // SHA — poise, absorption, stagger-resist(≈ STA/CON)
    pub tarnish: u8,       // TAR — corruption / decay track (new axis; seeded by scars)
    pub resonance: u8,     // RES — attunement, sync, charm    (≈ WIS, absorbs "CHA")
    // ── Cross-cutting (Cause & Effect / Karma) ──
    pub guilt: u8,         // GIL — the note-ledger's weight; mercy & retaliation
    /// CLA — the 8th register (Sean 2026-07-31, confirmed 08-03). WILD like
    /// guilt: it sits in neither the active nor the passive sum, so it never
    /// moves the Gender variance. No planet or metal — classical rulership
    /// has seven, and this is the eighth.
    /// Pools match `forge_items::stability` (active/passive/wild) exactly.
    pub clarity: u8,
}

impl HermeticStats {
    #[inline] pub fn active_sum(&self) -> u16 {
        self.vigor as u16 + self.momentum as u16 + self.logic_depth as u16
    }
    #[inline] pub fn passive_sum(&self) -> u16 {
        self.shadow_weight as u16 + self.tarnish as u16 + self.resonance as u16
    }
    /// |Active − Passive| — the Gender imbalance.
    #[inline] pub fn variance(&self) -> u16 { self.active_sum().abs_diff(self.passive_sum()) }
    /// `Variance >> 5` (÷32).
    #[inline] pub fn instability_index(&self) -> u16 { self.variance() >> 5 }
    /// Entity is `[UNSTABLE]` (Drift) once index > 2.
    #[inline] pub fn is_unstable(&self) -> bool { self.instability_index() > 2 }
    /// Synthesis held — physical form is stable.
    #[inline] pub fn synthesis_ok(&self) -> bool { !self.is_unstable() }

    /// Map a legacy 7-stat block onto the hermetic block.
    /// AGI+DEX fold into Momentum; WIS+CHA fold into Resonance (charm = attunement).
    /// Tarnish/Guilt start clean — they accrue from scars and the note-ledger.
    pub fn from_legacy(str_: i32, sta: i32, agi: i32, dex: i32, wis: i32, int_: i32, cha: i32) -> Self {
        let b = |v: i32| v.clamp(0, 255) as u8;
        Self {
            vigor:         b(str_),
            momentum:      b((agi + dex) / 2),
            logic_depth:   b(int_),
            shadow_weight: b(sta),
            tarnish:       0,
            resonance:     b((wis + cha) / 2), // charm-adding gear → +RES
            guilt:         0,
            clarity:       0, // wild, like guilt — earned in play, never rolled
        }
    }
}

// ── The 7 Laws as combat-modifier hooks ───────────────────────────────────────
/// Every formula here is integer-only (add / sub / shift / xor). No RNG inside —
/// callers feed the RNG byte. These mirror the engine hooks 1:1.
pub mod law {
    /// I. Mentalism — Focus Mode RNG lock. Guarantees a floor from the mind stat.
    #[inline] pub fn focus_hit_roll(rng_base: u8, logic_depth: u8) -> u8 {
        (rng_base & logic_depth) | 128
    }
    /// II. Correspondence — damage scales with dungeon depth (as above, so below).
    #[inline] pub fn correspondence_dmg(base: u32, dungeon_depth_byte: u8) -> u32 {
        base + (dungeon_depth_byte as u32 & 255)
    }
    /// II. overflow → Spatial Tear (coords invert + fall damage).
    #[inline] pub fn correspondence_overflow(base: u32, depth_byte: u8) -> bool {
        base + (depth_byte as u32 & 255) > 255
    }
    /// III. Vibration — armor penetration via frequency XOR.
    #[inline] pub fn resonance_delta(att_freq: u8, def_freq: u8) -> u8 { att_freq ^ def_freq }
    /// III. delta < 16 → ignore the defender's Shadow-Weight entirely.
    #[inline] pub fn ignores_armor(delta: u8) -> bool { delta < 16 }
    /// IV. Polarity — bonus from alignment difference (>>1).
    #[inline] pub fn polarity_bonus(att_align: u8, def_align: u8) -> u8 {
        att_align.abs_diff(def_align) >> 1
    }
    /// V. Rhythm — the global turn phase (0..7). 0–3 Crest, 4–7 Trough.
    #[inline] pub fn rhythm_phase(global_turn: u8) -> u8 { global_turn & 7 }
    /// V. Crest doubles damage; Trough halves it.
    #[inline] pub fn rhythm_scale(dmg: u32, phase: u8) -> u32 {
        if phase < 4 { dmg << 1 } else { dmg >> 1 }
    }
    /// VI. Cause & Effect — the Retaliation Buffer stores half of damage taken.
    #[inline] pub fn stored_force(dmg_taken: u32) -> u32 { dmg_taken >> 1 }
    /// VI. buffer > 255 → Karmic Loop (internal detonation).
    #[inline] pub fn karmic_loop(stored_force: u32) -> bool { stored_force > 255 }
    /// VII. Gender — fusion power: one Active item + one Passive item, averaged.
    #[inline] pub fn fuse_power(active_byte: u8, passive_byte: u8) -> u16 {
        (active_byte as u16 + passive_byte as u16) >> 1
    }
}

// ── Cataclysm overflow states (the "Cataclysm State" of each Law) ─────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cataclysm {
    MindFracture,             // Mentalism overflow
    SpatialTear,              // Correspondence overflow
    ShatterState,             // Vibration overflow
    PolarityCollapse,         // Polarity overflow
    Arrhythmia,               // Rhythm overflow
    KarmicLoop,               // Cause & Effect overflow
    HermaphroditicAnnihilation, // Gender (same-dominance fusion)
}

impl Cataclysm {
    /// The Void-Tech color the entity is stained with when this fires.
    pub const fn color_hex(self) -> u32 {
        match self {
            Cataclysm::MindFracture                => 0x050505, // Pitch Black
            Cataclysm::SpatialTear                 => 0x1A0024, // Void Purple
            Cataclysm::ShatterState                => 0x5C6B73, // Cold Iron
            Cataclysm::PolarityCollapse            => 0xFFD300, // Caustic Yellow
            Cataclysm::Arrhythmia                  => 0xB2BEB5, // Ash Gray
            Cataclysm::KarmicLoop                  => 0xF9F6EE, // Bone White
            Cataclysm::HermaphroditicAnnihilation  => 0x4A4A4A, // Tarnish
        }
    }
}

// ── The 7 Principles + 10 alchemical base elements ────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Principle {
    Mentalism, Correspondence, Vibration, Polarity, Rhythm, CauseEffect, Gender,
}

/// The 10 alchemical base elements — the *substrate* layer (frequency + material).
/// Surface damage-typing (Fire/Water/…) stays in the game's own damage layer;
/// these drive crafting, hazards, and the Vibration XOR via `frequency_byte`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reagent {
    Pitch, Salt, Ash, Brass, Brine, Quicksilver, Ichor, Marrow, Sulfur, Lead,
}

impl Reagent {
    /// The hidden frequency byte (drives Vibration armor-pen).
    pub const fn frequency_byte(self) -> u8 {
        match self {
            Reagent::Pitch => 0,    Reagent::Salt => 16,   Reagent::Ash => 32,
            Reagent::Brass => 64,   Reagent::Brine => 96,  Reagent::Quicksilver => 128,
            Reagent::Ichor => 170,  Reagent::Marrow => 192, Reagent::Sulfur => 223,
            Reagent::Lead => 255,
        }
    }
    pub const fn principle(self) -> Principle {
        match self {
            Reagent::Pitch => Principle::Mentalism,
            Reagent::Salt => Principle::Gender,        // Passive
            Reagent::Ash => Principle::Polarity,
            Reagent::Brass => Principle::Correspondence,
            Reagent::Brine => Principle::Rhythm,
            Reagent::Quicksilver => Principle::Vibration,
            Reagent::Ichor => Principle::CauseEffect,
            Reagent::Marrow => Principle::CauseEffect,
            Reagent::Sulfur => Principle::Gender,      // Active
            Reagent::Lead => Principle::Rhythm,
        }
    }
    pub const fn color_hex(self) -> u32 {
        match self {
            Reagent::Pitch => 0x050505, Reagent::Salt => 0xF9F6EE, Reagent::Ash => 0xB2BEB5,
            Reagent::Brass => 0xB5A642, Reagent::Brine => 0x004F4D, Reagent::Quicksilver => 0x39FF14,
            Reagent::Ichor => 0x7E0000, Reagent::Marrow => 0xEAE0C8, Reagent::Sulfur => 0xFFD300,
            Reagent::Lead => 0x5C6B73,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::law::*;

    #[test]
    fn synthesis_drift_at_index_over_2() {
        // Active 100 vs Passive 4 → variance 96 → 96>>5 = 3 > 2 → unstable.
        let s = HermeticStats { vigor: 100, momentum: 0, logic_depth: 0,
                                shadow_weight: 4, tarnish: 0, resonance: 0, guilt: 0,
                                clarity: 0 };
        assert_eq!(s.variance(), 96);
        assert_eq!(s.instability_index(), 3);
        assert!(s.is_unstable());
    }

    #[test]
    fn balanced_pools_are_stable() {
        // guilt AND clarity are wild — neither enters a pool sum, so a maxed
        // clarity must not tip the Gender variance any more than guilt does.
        let s = HermeticStats { vigor: 30, momentum: 30, logic_depth: 30,
                                shadow_weight: 30, tarnish: 30, resonance: 30, guilt: 99,
                                clarity: 255 };
        assert!(s.synthesis_ok());
        assert_eq!(s.variance(), 0, "a wild register moved a pool sum");
    }

    #[test]
    fn law_formulas_are_integer_exact() {
        assert!(focus_hit_roll(0, 200) >= 128);            // Mentalism floor
        assert_eq!(correspondence_dmg(10, 5), 15);          // Correspondence
        assert!(ignores_armor(resonance_delta(0b1_0000, 0b1_0001))); // Vibration: delta 1 < 16
        assert!(!ignores_armor(resonance_delta(0, 64)));    // delta 64 → armor holds
        assert_eq!(polarity_bonus(200, 100), 50);           // Polarity (100>>1)
        assert_eq!(rhythm_phase(10), 2);                    // Rhythm
        assert_eq!(rhythm_scale(10, 2), 20);                //   crest doubles
        assert_eq!(rhythm_scale(10, 5), 5);                 //   trough halves
        assert_eq!(stored_force(40), 20);                   // Cause & Effect
        assert_eq!(fuse_power(200, 100), 150);              // Gender fusion
    }

    #[test]
    fn reagent_table_matches_lore() {
        assert_eq!(Reagent::Quicksilver.frequency_byte(), 128);
        assert_eq!(Reagent::Quicksilver.color_hex(), 0x39FF14);
        assert_eq!(Reagent::Lead.frequency_byte(), 255);
        assert_eq!(Reagent::Pitch.principle(), Principle::Mentalism);
    }

    #[test]
    fn legacy_stats_fold_in() {
        // STR 18, STA 14, AGI 12, DEX 10, WIS 12, INT 16, CHA 14
        let h = HermeticStats::from_legacy(18, 14, 12, 10, 12, 16, 14);
        assert_eq!(h.vigor, 18);
        assert_eq!(h.momentum, 11);     // (12+10)/2
        assert_eq!(h.logic_depth, 16);
        assert_eq!(h.shadow_weight, 14);
        assert_eq!(h.resonance, 13);    // (12+14)/2  ← WIS + CHA
        assert_eq!(h.guilt, 0);
    }

    #[test]
    fn cataclysm_colors_present() {
        assert_eq!(Cataclysm::MindFracture.color_hex(), 0x050505);
        assert_eq!(Cataclysm::HermaphroditicAnnihilation.color_hex(), 0x4A4A4A);
    }
}
