//! Combat arbiter — deterministic damage resolution.
//! Ported from AKGAME: scripts/core/CombatController.gd (COM-001)
//!
//! Formula: ((base + vigor/10) + momentum/20) / (resisted ? 2 : 1), clamped u8.
//! Proc density cap: max 1 new on-hit effect per attack (BH-006).
//!
//! Serde stripped — pure integer logic, no serialization dep (WASM-clean).

/// Elemental attribute resistance table (COM-001).
/// Key resists Value: Fire resists Water, etc.
pub const ATTRIBUTE_RESISTANCE: &[(&str, &str)] = &[
    ("Fire",      "Water"),
    ("Poison",    "Earth"),
    ("Water",     "Electric"),
    ("Light",     "Darkness"),
    ("Electric",  "Earth"),
    ("Blood",     "Fire"),
    ("Earth",     "Fire"),
    ("Darkness",  "Light"),
];

/// Minimal actor archetype for combat resolution.
#[derive(Debug, Clone)]
pub struct ActorStats {
    /// Actor name (informational, not used in logic).
    pub name:      &'static str,
    /// Elemental attribute (e.g. "Fire", "Water") for resistance matching.
    pub attribute: &'static str,
    /// Material type (e.g. "IRON", "REFRACTION_GOLD") for resonance shifts.
    pub material:  &'static str,
    /// Vigor stat (divided by 10 to modify damage).
    pub vigor:     i32,
    /// Momentum stat (divided by 20 to modify damage).
    pub momentum:  i32,
}

/// On-hit micro-modifier (proc effect). Fixed-size; no Vec in the hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MicroMod {
    /// Effect name (e.g. "BLEED", "BURN") for proc grouping.
    pub effect: &'static str,
}

/// Result of a single attack resolution.
#[derive(Debug, Clone, Copy)]
pub struct CombatResult {
    /// Computed damage (u8 clamped).
    pub damage:           u8,
    /// True if attacker's attribute resists defender's attribute.
    pub is_resisted:      bool,
    /// True if damage exceeds 100 (triggers visceral animation).
    pub visceral_trigger: bool,
    /// Resonance shift applied (0 or 1, from material type).
    pub resonance_shift:  i32,
    /// The primary depth-2 proc effect name (empty = no proc).
    pub depth2_proc:      &'static str,
    /// Magnitude of the proc (count of identical effects).
    pub proc_magnitude:   u32,
}

/// Resolve an attack deterministically. `micro_mods` must already be sorted
/// by effect name so this call is pure and no allocation is needed (BH-006).
///
/// Callers that hold a `[MicroMod; N]` sort it before calling:
/// `mods.sort_by_key(|m| m.effect);`
pub fn resolve_attack(
    attacker: &ActorStats,
    defender: &ActorStats,
    base_power: i32,
    micro_mods: &[MicroMod],
) -> CombatResult {
    let vigor_mod    = attacker.vigor    / 10;
    let momentum_mod = attacker.momentum / 20;
    let raw          = base_power + vigor_mod + momentum_mod;

    let is_resisted = ATTRIBUTE_RESISTANCE.iter()
        .any(|&(a, r)| a == attacker.attribute && r == defender.attribute);

    let raw_after = if is_resisted { raw / 2 } else { raw };
    let damage    = raw_after.clamp(0, 255) as u8;

    // BH-006: deterministic proc — caller pre-sorts; pick first unique group.
    let (depth2_proc, proc_magnitude) = if micro_mods.is_empty() {
        ("", 0)
    } else {
        let primary = micro_mods[0].effect;
        let count   = micro_mods.iter().filter(|m| m.effect == primary).count() as u32;
        (primary, count)
    };

    CombatResult {
        damage,
        is_resisted,
        visceral_trigger: damage > 100,
        resonance_shift: if attacker.material == "REFRACTION_GOLD" { 1 } else { 0 },
        depth2_proc,
        proc_magnitude,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(name: &'static str, attr: &'static str, vigor: i32, momentum: i32) -> ActorStats {
        ActorStats { name, attribute: attr, material: "IRON", vigor, momentum }
    }

    #[test]
    fn basic_damage_formula() {
        let a = actor("A", "Fire", 100, 200);
        let d = actor("D", "Earth", 0, 0);
        let r = resolve_attack(&a, &d, 50, &[]);
        // vigor/10=10, momentum/20=10, raw=70, not resisted
        assert_eq!(r.damage, 70);
        assert!(!r.is_resisted);
    }

    #[test]
    fn resistance_halves_damage() {
        let a = actor("A", "Fire", 0, 0);
        let d = actor("D", "Water", 0, 0); // Fire → Water → resisted
        let r = resolve_attack(&a, &d, 100, &[]);
        assert!(r.is_resisted);
        assert_eq!(r.damage, 50);
    }

    #[test]
    fn damage_clamped_to_u8() {
        let a = actor("A", "Fire", 10_000, 10_000);
        let d = actor("D", "Earth", 0, 0);
        let r = resolve_attack(&a, &d, 200, &[]);
        assert_eq!(r.damage, 255);
    }

    #[test]
    fn visceral_trigger_above_100() {
        let a = actor("A", "Fire", 0, 0);
        let d = actor("D", "Earth", 0, 0);
        let r = resolve_attack(&a, &d, 101, &[]);
        assert!(r.visceral_trigger);
    }

    #[test]
    fn proc_picks_first_after_sort() {
        let a = actor("A", "Fire", 0, 0);
        let d = actor("D", "Earth", 0, 0);
        // Pre-sorted alphabetically (BLEED before BURN).
        let mods = [
            MicroMod { effect: "BLEED" },
            MicroMod { effect: "BURN" },
            MicroMod { effect: "BURN" },
        ];
        let r = resolve_attack(&a, &d, 10, &mods);
        assert_eq!(r.depth2_proc, "BLEED"); // alphabetically first
        assert_eq!(r.proc_magnitude, 1);
    }

    #[test]
    fn resonance_shift_on_refraction_gold() {
        let mut a = actor("A", "Fire", 0, 0);
        a.material = "REFRACTION_GOLD";
        let d = actor("D", "Earth", 0, 0);
        let r = resolve_attack(&a, &d, 10, &[]);
        assert_eq!(r.resonance_shift, 1);
    }
}
