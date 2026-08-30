//! Alchemical Additives — Gemstone/mineral modifiers socketed into weapons/armor.
//!
//! Each additive binds to an AlchemicalTier and modifies combat physics:
//! - Albedo (432Hz, Water): integer stability, radial knockback, neutral frame
//! - Citrinitas (Inverse Hz, Air): phase inversion, negative velocity, suction
//! - Nigredo (40Hz, Earth): mass amplification, crushing gravity
//! - Rubedo (800Hz, Fire): plasma shockwaves, rapid multi-hits
//!
//! Additives are Items with base_type 200-299, socketed via the existing socket system.
//! Stateless, deterministic, no alloc on hot path.

use serde::{Deserialize, Serialize};

// ── Additive Classification ──────────────────────────────────────────────────

/// Which alchemical tier this additive amplifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdditiveTier {
    Nigredo,
    Albedo,
    Citrinitas,
    Rubedo,
    Universal, // works across tiers (Gold)
}

/// The mineral/gemstone type. Each has unique physics behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Additive {
    // ── Albedo (Water, 432Hz) ────────────────────────────────────────────
    /// Rounds collision data to nearest integer. Eliminates f32 drift.
    Jasper,
    /// Mid-tier stability. Reduces rounding error by half.
    Agate,
    /// Maximum integer stability. 100% computational efficiency, zero energy loss.
    Diamond,

    // ── Citrinitas (Air, Inverse Hz) ─────────────────────────────────────
    /// Temporal lingering: stores impact, delivers inverse damage after N ticks.
    Bronze,
    /// Heat suction: pulls ambient thermal energy → cold-fusion AoE on release.
    Sulfur,
    /// Electrostatic accumulation via negative friction. Discharge on threshold.
    Amber,
    /// Ethereal bypass: phase-shifts to pull spectral entities into physical plane.
    Silver,

    // ── Nigredo (Earth, 40Hz) ────────────────────────────────────────────
    /// Negative velocity vector (inward suction, strong).
    Iron,
    /// Negative velocity vector (inward suction, weak).
    Tin,
    /// Caps/stabilizes phase-shifted collisions. Prevents physics overflow.
    Steel,
    /// Mid-tier stabilizer for inverse physics.
    Copper,

    // ── Rubedo (Fire, 800Hz) ─────────────────────────────────────────────
    /// Plasma ignition: converts kinetic overflow into volumetric shockwave.
    Ruby,
    /// Rapid multi-hit: splits single strike into N micro-impacts.
    Garnet,
    /// Thermal amplification: each consecutive hit increases damage tier.
    Obsidian,

    // ── Universal ────────────────────────────────────────────────────────
    /// Balances constructive/destructive wave interference.
    /// Allows toggling between positive knockback and negative suction.
    Gold,
}

// ── Static Properties ────────────────────────────────────────────────────────

/// Physics modification applied by an additive during collision resolution.
#[derive(Debug, Clone, Copy)]
pub struct AdditiveEffect {
    pub tier: AdditiveTier,
    /// Permyriad modifier to collision rounding (10000 = round to integer).
    pub integer_stability_q: i32,
    /// Permyriad modifier to knockback direction (negative = inward suction).
    pub velocity_sign_q: i32,
    /// Delay in ticks before effect applies (0 = immediate).
    pub delay_ticks: u16,
    /// Permyriad energy efficiency (10000 = zero loss).
    pub efficiency_q: i32,
    /// Whether this additive bypasses spectral/immaterial defense.
    pub ethereal_bypass: bool,
    /// Permyriad mass multiplier bonus (added to base mass_q).
    pub mass_bonus_q: i32,
}

impl Additive {
    pub const fn tier(self) -> AdditiveTier {
        match self {
            Self::Jasper | Self::Agate | Self::Diamond => AdditiveTier::Albedo,
            Self::Bronze | Self::Sulfur | Self::Amber | Self::Silver => AdditiveTier::Citrinitas,
            Self::Iron | Self::Tin | Self::Steel | Self::Copper => AdditiveTier::Nigredo,
            Self::Ruby | Self::Garnet | Self::Obsidian => AdditiveTier::Rubedo,
            Self::Gold => AdditiveTier::Universal,
        }
    }

    pub const fn effect(self) -> AdditiveEffect {
        match self {
            // Albedo: integer stability, neutral frame, radial knockback
            Self::Jasper => AdditiveEffect {
                tier: AdditiveTier::Albedo,
                integer_stability_q: 7500,  // rounds most collision data
                velocity_sign_q: 10000,     // standard outward (radial)
                delay_ticks: 0,
                efficiency_q: 8500,
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },
            Self::Agate => AdditiveEffect {
                tier: AdditiveTier::Albedo,
                integer_stability_q: 5000,
                velocity_sign_q: 10000,
                delay_ticks: 0,
                efficiency_q: 7000,
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },
            Self::Diamond => AdditiveEffect {
                tier: AdditiveTier::Albedo,
                integer_stability_q: 10000, // perfect integer rounding
                velocity_sign_q: 10000,     // perfect radial symmetry
                delay_ticks: 0,
                efficiency_q: 10000,        // zero energy loss
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },

            // Citrinitas: phase inversion, negative velocity, temporal tricks
            Self::Bronze => AdditiveEffect {
                tier: AdditiveTier::Citrinitas,
                integer_stability_q: 5000,
                velocity_sign_q: -10000,    // full inversion (delayed)
                delay_ticks: 36,            // 300ms at 120Hz
                efficiency_q: 6000,
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },
            Self::Sulfur => AdditiveEffect {
                tier: AdditiveTier::Citrinitas,
                integer_stability_q: 3000,
                velocity_sign_q: -7500,     // strong suction
                delay_ticks: 0,
                efficiency_q: 5000,
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },
            Self::Amber => AdditiveEffect {
                tier: AdditiveTier::Citrinitas,
                integer_stability_q: 4000,
                velocity_sign_q: -5000,     // moderate suction
                delay_ticks: 0,
                efficiency_q: 6000,
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },
            Self::Silver => AdditiveEffect {
                tier: AdditiveTier::Citrinitas,
                integer_stability_q: 6000,
                velocity_sign_q: -8000,     // strong pull
                delay_ticks: 0,
                efficiency_q: 7000,
                ethereal_bypass: true,      // pulls ghosts to physical
                mass_bonus_q: 0,
            },

            // Nigredo: mass, gravity, crushing
            Self::Iron => AdditiveEffect {
                tier: AdditiveTier::Nigredo,
                integer_stability_q: 3000,
                velocity_sign_q: -6000,     // inward suction
                delay_ticks: 0,
                efficiency_q: 5000,
                ethereal_bypass: false,
                mass_bonus_q: 3000,         // +30% mass
            },
            Self::Tin => AdditiveEffect {
                tier: AdditiveTier::Nigredo,
                integer_stability_q: 2000,
                velocity_sign_q: -3000,     // weak suction
                delay_ticks: 0,
                efficiency_q: 4000,
                ethereal_bypass: false,
                mass_bonus_q: 1500,         // +15% mass
            },
            Self::Steel => AdditiveEffect {
                tier: AdditiveTier::Nigredo,
                integer_stability_q: 8000,  // stabilizer
                velocity_sign_q: 0,         // neutral (caps overflow)
                delay_ticks: 0,
                efficiency_q: 9000,
                ethereal_bypass: false,
                mass_bonus_q: 2000,
            },
            Self::Copper => AdditiveEffect {
                tier: AdditiveTier::Nigredo,
                integer_stability_q: 6000,
                velocity_sign_q: 0,         // neutral stabilizer
                delay_ticks: 0,
                efficiency_q: 7000,
                ethereal_bypass: false,
                mass_bonus_q: 1000,
            },

            // Rubedo: plasma, multi-hit, thermal escalation
            Self::Ruby => AdditiveEffect {
                tier: AdditiveTier::Rubedo,
                integer_stability_q: 2000,
                velocity_sign_q: 15000,     // amplified outward (shockwave)
                delay_ticks: 0,
                efficiency_q: 4000,         // volatile, lossy
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },
            Self::Garnet => AdditiveEffect {
                tier: AdditiveTier::Rubedo,
                integer_stability_q: 3000,
                velocity_sign_q: 10000,
                delay_ticks: 0,
                efficiency_q: 5000,
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },
            Self::Obsidian => AdditiveEffect {
                tier: AdditiveTier::Rubedo,
                integer_stability_q: 1000,
                velocity_sign_q: 12000,     // escalating outward
                delay_ticks: 0,
                efficiency_q: 3000,         // very volatile
                ethereal_bypass: false,
                mass_bonus_q: 500,
            },

            // Universal
            Self::Gold => AdditiveEffect {
                tier: AdditiveTier::Universal,
                integer_stability_q: 10000, // perfect balance
                velocity_sign_q: 0,         // TOGGLEABLE (handled by combat logic)
                delay_ticks: 0,
                efficiency_q: 10000,        // perfect
                ethereal_bypass: false,
                mass_bonus_q: 0,
            },
        }
    }

    /// Item base_type ID for inventory system integration.
    pub const fn item_base_type(self) -> u16 {
        match self {
            Self::Jasper => 200,
            Self::Agate => 201,
            Self::Diamond => 202,
            Self::Bronze => 210,
            Self::Sulfur => 211,
            Self::Amber => 212,
            Self::Silver => 213,
            Self::Iron => 220,
            Self::Tin => 221,
            Self::Steel => 222,
            Self::Copper => 223,
            Self::Ruby => 230,
            Self::Garnet => 231,
            Self::Obsidian => 232,
            Self::Gold => 240,
        }
    }

    /// Look up additive from item base_type.
    pub const fn from_base_type(bt: u16) -> Option<Self> {
        match bt {
            200 => Some(Self::Jasper),
            201 => Some(Self::Agate),
            202 => Some(Self::Diamond),
            210 => Some(Self::Bronze),
            211 => Some(Self::Sulfur),
            212 => Some(Self::Amber),
            213 => Some(Self::Silver),
            220 => Some(Self::Iron),
            221 => Some(Self::Tin),
            222 => Some(Self::Steel),
            223 => Some(Self::Copper),
            230 => Some(Self::Ruby),
            231 => Some(Self::Garnet),
            232 => Some(Self::Obsidian),
            240 => Some(Self::Gold),
            _ => None,
        }
    }
}

// ── Combat Integration ───────────────────────────────────────────────────────

/// Resolve the combined additive effect from all socketed gems on a weapon.
/// Sums integer_stability and efficiency, takes strongest velocity_sign,
/// longest delay, and OR's ethereal_bypass.
pub fn resolve_additives(additives: &[Additive]) -> AdditiveEffect {
    let mut result = AdditiveEffect {
        tier: AdditiveTier::Universal,
        integer_stability_q: 0,
        velocity_sign_q: 10000, // default: standard outward
        delay_ticks: 0,
        efficiency_q: 5000,     // default: 50% baseline
        ethereal_bypass: false,
        mass_bonus_q: 0,
    };

    if additives.is_empty() {
        return result;
    }

    let mut stability_sum: i64 = 0;
    let mut efficiency_sum: i64 = 0;
    let mut strongest_velocity: i32 = 10000;
    let mut strongest_abs: i32 = 10000;

    for &a in additives {
        let e = a.effect();
        stability_sum += e.integer_stability_q as i64;
        efficiency_sum += e.efficiency_q as i64;
        result.mass_bonus_q = result.mass_bonus_q.saturating_add(e.mass_bonus_q);
        result.ethereal_bypass |= e.ethereal_bypass;

        if e.delay_ticks > result.delay_ticks {
            result.delay_ticks = e.delay_ticks;
        }

        // Strongest velocity wins (by absolute magnitude)
        let abs_v = e.velocity_sign_q.unsigned_abs();
        if abs_v > strongest_abs as u32 {
            strongest_abs = abs_v as i32;
            strongest_velocity = e.velocity_sign_q;
        }
    }

    let n = additives.len() as i64;
    result.integer_stability_q = (stability_sum / n).clamp(0, 10000) as i32;
    result.efficiency_q = (efficiency_sum / n).clamp(0, 10000) as i32;
    result.velocity_sign_q = strongest_velocity;

    result
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diamond_gives_perfect_stability() {
        let e = Additive::Diamond.effect();
        assert_eq!(e.integer_stability_q, 10000);
        assert_eq!(e.efficiency_q, 10000);
        assert_eq!(e.velocity_sign_q, 10000); // radial outward
    }

    #[test]
    fn silver_has_ethereal_bypass() {
        let e = Additive::Silver.effect();
        assert!(e.ethereal_bypass);
        assert!(e.velocity_sign_q < 0); // suction
    }

    #[test]
    fn bronze_has_delay() {
        let e = Additive::Bronze.effect();
        assert_eq!(e.delay_ticks, 36);
        assert!(e.velocity_sign_q < 0);
    }

    #[test]
    fn gold_is_universal_perfect() {
        let e = Additive::Gold.effect();
        assert_eq!(e.tier, AdditiveTier::Universal);
        assert_eq!(e.integer_stability_q, 10000);
        assert_eq!(e.efficiency_q, 10000);
        assert_eq!(e.velocity_sign_q, 0); // toggleable
    }

    #[test]
    fn resolve_mixed_additives() {
        let combo = [Additive::Diamond, Additive::Silver];
        let r = resolve_additives(&combo);
        // Silver has ethereal bypass
        assert!(r.ethereal_bypass);
        // Diamond + Silver stability averaged
        assert_eq!(r.integer_stability_q, 8000); // (10000+6000)/2
        // Diamond's outward (abs 10000) beats Silver's suction (abs 8000)
        assert_eq!(r.velocity_sign_q, 10000);
    }

    #[test]
    fn iron_adds_mass() {
        let e = Additive::Iron.effect();
        assert_eq!(e.mass_bonus_q, 3000);
    }

    #[test]
    fn ruby_amplifies_outward() {
        let e = Additive::Ruby.effect();
        assert!(e.velocity_sign_q > 10000); // amplified shockwave
    }

    #[test]
    fn base_type_roundtrip() {
        for a in [Additive::Jasper, Additive::Gold, Additive::Silver, Additive::Ruby] {
            assert_eq!(Additive::from_base_type(a.item_base_type()), Some(a));
        }
    }
}
