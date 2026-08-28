//! Dissonance Sieve — the universal gate for all gameplay interactions.
//!
//! Ported by translation from forge-cart-brain::dissonance_sieve. Every system
//! submits an interaction through this gate before resolving: combat, theft, dialogue,
//! terrain — all pass the same verdict path. Stateless, deterministic, integer-only,
//! zero-alloc — safe to call from the 120Hz tick.

/// Classical elemental alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClassicalElement {
    /// Fire element.
    Fire,
    /// Air element.
    Air,
    /// Water element.
    Water,
    /// Earth element.
    Earth,
}

/// Alchemical tier hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlchemicalTier {
    /// First tier (Nigredo).
    Nigredo,
    /// Second tier (Albedo).
    Albedo,
    /// Third tier (Citrinitas).
    Citrinitas,
    /// Fourth tier (Rubedo).
    Rubedo,
}

/// Aspect geometric relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AspectGeometry {
    /// Conjunction aspect.
    Conjunction,
    /// Sextile aspect.
    Sextile,
    /// Square aspect.
    Square,
    /// Trine aspect.
    Trine,
    /// Quincunx aspect.
    Quincunx,
    /// Opposition aspect.
    Opposition,
    /// Yod aspect.
    Yod,
}

/// Outcome temperament (benefit/harm/neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Temperament {
    /// Benefic temperament (favorable).
    Benefic,
    /// Malefic temperament (harmful).
    Malefic,
    /// Neutral temperament.
    Neutral,
}

impl Default for Temperament {
    fn default() -> Self {
        Temperament::Neutral
    }
}

/// Authority verdict outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthorityOutcome {
    /// No authority gained.
    None,
    /// Bonification outcome.
    Bonification,
    /// Maltreatment outcome.
    Maltreatment,
    /// Mitigation outcome.
    Mitigation,
    /// Clash outcome.
    Clash,
}

/// Harmonic body — an entity's elemental and resonance properties.
#[derive(Debug, Clone, Copy)]
pub struct HarmonicBody {
    /// Elemental alignment.
    pub element: ClassicalElement,
    /// Alchemical tier.
    pub tier: AlchemicalTier,
    /// Resonance frequency (Hz).
    pub resonance_hz: i16,
    /// Inverse flag (phase inversion).
    pub inverse: bool,
    /// Mass in Permyriad.
    pub mass_q: i32,
}

/// Authority verdict context — all integer fields.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuthorityContext {
    /// Initiative difference.
    pub initiative_delta: i16,
    /// Elevation difference in MilliUnits.
    pub elevation_delta_mm: i32,
    /// Legal standing delta.
    pub legal_delta: i16,
    /// Witness/authority delta.
    pub witness_delta: i16,
    /// Route/path delta.
    pub route_delta: i16,
    /// Camera/visibility delta.
    pub camera_delta: i16,
    /// Artifact power delta.
    pub artifact_delta: i16,
    /// Death scar delta.
    pub death_scar_delta: i16,
    /// Charge head delta.
    pub charge_head_delta: i16,
    /// Temperament modifier.
    pub temperament: Temperament,
}

/// Resolve authority from context deltas. Returns the outcome based on score.
pub fn authority_score(ctx: &AuthorityContext) -> i32 {
    ctx.initiative_delta as i32
        + ctx.elevation_delta_mm / 1000
        + ctx.legal_delta as i32
        + ctx.witness_delta as i32
        + ctx.route_delta as i32
        + ctx.camera_delta as i32
        + ctx.artifact_delta as i32
        + ctx.death_scar_delta as i32
        + ctx.charge_head_delta as i32
}

/// Resolve authority verdict from context.
pub fn resolve_authority(ctx: &AuthorityContext) -> AuthorityOutcome {
    let score = authority_score(ctx);
    if score > 0 {
        match ctx.temperament {
            Temperament::Benefic => AuthorityOutcome::Bonification,
            Temperament::Malefic => AuthorityOutcome::Maltreatment,
            Temperament::Neutral => AuthorityOutcome::Mitigation,
        }
    } else if score == 0 {
        AuthorityOutcome::Clash
    } else {
        AuthorityOutcome::None
    }
}

/// Elemental modifier Permyriad (1000 = neutral, >1000 = advantage, <1000 = disadvantage).
pub fn elemental_modifier_q(attacker: &HarmonicBody, defender: &HarmonicBody) -> i32 {
    let counter = match (attacker.element, defender.element) {
        (ClassicalElement::Fire, ClassicalElement::Air) => 1200,
        (ClassicalElement::Air, ClassicalElement::Water) => 1200,
        (ClassicalElement::Water, ClassicalElement::Fire) => 1200,
        (ClassicalElement::Earth, ClassicalElement::Air) => 1200,
        (ClassicalElement::Fire, ClassicalElement::Water) => 800,
        (ClassicalElement::Air, ClassicalElement::Earth) => 800,
        (ClassicalElement::Water, ClassicalElement::Air) => 800,
        _ => 1000,
    };
    // Tier difference bonus
    let tier_bonus = (attacker.tier as i32 - defender.tier as i32) * 100;
    (counter + tier_bonus).clamp(500, 2000)
}

/// Mechanical demand archetype (Quincunx).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MechanicalDemand {
    /// Aggression demand.
    Aggression,
    /// Patience demand.
    Patience,
    /// Mobility demand.
    Mobility,
    /// Stationary channel demand.
    StationaryChannel,
    /// Inventory precision demand.
    InventoryPrecision,
    /// Parry timing demand.
    ParryTiming,
    /// Diplomacy demand.
    Diplomacy,
    /// Crafting demand.
    Crafting,
    /// Theft demand.
    Theft,
    /// Witness building demand.
    WitnessBuilding,
    /// Death route demand.
    DeathRoute,
    /// Refusal demand.
    Refusal,
}

/// Quincunx pressure constraint.
#[derive(Debug, Clone, Copy)]
pub struct QuincunxPressure {
    /// First mechanical demand.
    pub demand_a: MechanicalDemand,
    /// Second mechanical demand.
    pub demand_b: MechanicalDemand,
    /// Severity of the pressure.
    pub severity: u16,
}

/// Dissonance verdict — the output of the universal gate.
#[derive(Debug, Clone, Copy)]
pub struct DissonanceVerdict {
    /// Authority outcome verdict.
    pub authority: AuthorityOutcome,
    /// Power modifier in Permyriad.
    pub power_modifier_q: i32,
    /// Dissonance pressure applied.
    pub dissonance_pressure: u16,
    /// Entropy cost of this action.
    pub entropy_cost: u32,
}

/// The universal gate. Every system calls this before resolving outcomes.
pub fn evaluate(
    attacker_body: &HarmonicBody,
    defender_body: &HarmonicBody,
    authority: &AuthorityContext,
    quincunx: Option<&QuincunxPressure>,
) -> DissonanceVerdict {
    let auth = resolve_authority(authority);
    let elem_q = elemental_modifier_q(attacker_body, defender_body);

    let auth_q = match auth {
        AuthorityOutcome::Bonification => 1500,
        AuthorityOutcome::Maltreatment => 2000,
        AuthorityOutcome::Mitigation => 1250,
        AuthorityOutcome::Clash => 750,
        AuthorityOutcome::None => 1000,
    };

    let dissonance = quincunx.map(|q| q.severity).unwrap_or(0);

    DissonanceVerdict {
        authority: auth,
        power_modifier_q: (elem_q * auth_q) / 1000,
        dissonance_pressure: dissonance,
        entropy_cost: dissonance as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_authority_gives_clash() {
        let body = HarmonicBody {
            element: ClassicalElement::Fire,
            tier: AlchemicalTier::Albedo,
            resonance_hz: 432,
            inverse: false,
            mass_q: 1000,
        };
        let ctx = AuthorityContext::default();
        let v = evaluate(&body, &body, &ctx, None);
        assert_eq!(v.authority, AuthorityOutcome::Clash);
    }

    #[test]
    fn superior_initiative_gives_bonification() {
        let body = HarmonicBody {
            element: ClassicalElement::Fire,
            tier: AlchemicalTier::Rubedo,
            resonance_hz: 800,
            inverse: false,
            mass_q: 250,
        };
        let ctx = AuthorityContext { initiative_delta: 5, temperament: Temperament::Benefic, ..Default::default() };
        let v = evaluate(&body, &body, &ctx, None);
        assert_eq!(v.authority, AuthorityOutcome::Bonification);
        assert!(v.power_modifier_q > 1000);
    }

    #[test]
    fn fire_beats_air() {
        let fire = HarmonicBody {
            element: ClassicalElement::Fire,
            tier: AlchemicalTier::Albedo,
            resonance_hz: 432,
            inverse: false,
            mass_q: 1000,
        };
        let air = HarmonicBody {
            element: ClassicalElement::Air,
            tier: AlchemicalTier::Albedo,
            resonance_hz: 432,
            inverse: false,
            mass_q: 1000,
        };
        let ctx = AuthorityContext::default();
        let v = evaluate(&fire, &air, &ctx, None);
        assert!(v.power_modifier_q > 750);
    }

    #[test]
    fn authority_score_sums_deltas() {
        let ctx = AuthorityContext {
            initiative_delta: 10,
            elevation_delta_mm: 5000,
            legal_delta: 3,
            witness_delta: 2,
            ..Default::default()
        };
        let score = authority_score(&ctx);
        assert_eq!(score, 10 + 5 + 3 + 2, "score sums deltas; elevation divided by 1000");
    }

    #[test]
    fn elemental_modifier_has_bounds() {
        let body1 = HarmonicBody {
            element: ClassicalElement::Fire,
            tier: AlchemicalTier::Rubedo,
            resonance_hz: 800,
            inverse: false,
            mass_q: 1000,
        };
        let body2 = HarmonicBody {
            element: ClassicalElement::Water,
            tier: AlchemicalTier::Nigredo,
            resonance_hz: 100,
            inverse: false,
            mass_q: 1000,
        };
        let modifier = elemental_modifier_q(&body1, &body2);
        assert!(modifier >= 500, "modifier >= 500");
        assert!(modifier <= 2000, "modifier <= 2000");
    }
}
