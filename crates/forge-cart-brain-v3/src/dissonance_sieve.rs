//! Dissonance Sieve — the universal gate for all gameplay interactions.
//!
//! Every system submits an interaction through this gate before resolving:
//! combat, theft, dialogue, terrain — all pass the same verdict path. Stateless,
//! deterministic, integer-only, zero-alloc — safe to call from the 120Hz tick.
//!
//! Ported by TRANSLATION from the quarry `ironroot-edict` (pure module, no engine
//! edge) — the elemental/authority verdict primitive the cart brain owns.

// ── Types ────────────────────────────────────────────────────────────────────

/// Classical elements — part of the harmonic body's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClassicalElement {
    /// The Fire element.
    Fire,
    /// The Air element.
    Air,
    /// The Water element.
    Water,
    /// The Earth element.
    Earth,
}

/// Alchemical progression tiers for harmonic bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlchemicalTier {
    /// The base, unrefined tier.
    Nigredo,
    /// The purification tier.
    Albedo,
    /// The yellowing/awakening tier.
    Citrinitas,
    /// The final, completed tier.
    Rubedo,
}

/// Aspect geometry — the geometric relationship between two harmonic bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AspectGeometry {
    /// Same position — reinforcing.
    Conjunction,
    /// 60-degree relationship — harmonious.
    Sextile,
    /// 90-degree relationship — tense.
    Square,
    /// 120-degree relationship — flowing.
    Trine,
    /// 150-degree relationship — adjustment needed.
    Quincunx,
    /// 180-degree relationship — opposing.
    Opposition,
    /// Compound tension geometry across three points.
    Yod,
}

/// Temperament of an authority context — whether it favors the attacker or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Temperament {
    /// Favors a positive outcome.
    Benefic,
    /// Favors a negative outcome.
    Malefic,
    /// Favors neither side.
    Neutral,
}

/// Outcome of an authority resolution verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthorityOutcome {
    /// No effect resolved.
    None,
    /// A beneficial effect resolved.
    Bonification,
    /// A harmful effect resolved.
    Maltreatment,
    /// A reducing effect resolved.
    Mitigation,
    /// A conflicting, unresolved tension.
    Clash,
}

/// A harmonic body — the resonant identity of any actor in conflict.
#[derive(Debug, Clone, Copy)]
pub struct HarmonicBody {
    /// Classical element of this body.
    pub element: ClassicalElement,
    /// Alchemical tier of this body.
    pub tier: AlchemicalTier,
    /// Resonance frequency in Hertz.
    pub resonance_hz: i16,
    /// Whether this body's resonance is inverted.
    pub inverse: bool,
    /// Mass in arbitrary units (affinity scaled to 1000).
    pub mass_q: i32,
}

/// Context for authority resolution — the environmental and relational factors that affect outcome.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuthorityContext {
    /// Initiative advantage (positive favors attacker).
    pub initiative_delta: i16,
    /// Height advantage in millimetres.
    pub elevation_delta_mm: i32,
    /// Legal/social standing delta.
    pub legal_delta: i16,
    /// Witness/observability delta.
    pub witness_delta: i16,
    /// Route/path control delta.
    pub route_delta: i16,
    /// Visual/camera advantage delta.
    pub camera_delta: i16,
    /// Artifact/equipment power delta.
    pub artifact_delta: i16,
    /// Death scar accumulation delta.
    pub death_scar_delta: i16,
    /// Charge head momentum delta.
    pub charge_head_delta: i16,
    /// Overall temperament modifying the verdict.
    pub temperament: Temperament,
}

impl Default for Temperament {
    fn default() -> Self { Temperament::Neutral }
}

// ── Authority Resolution ─────────────────────────────────────────────────────

/// Compute the total authority score from a context.
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

/// Resolve an authority context into an outcome verdict.
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

// ── Elemental Modifier ───────────────────────────────────────────────────────

/// Returns Permyriad modifier (1000 = neutral, >1000 = advantage, <1000 = disadvantage)
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

// ── Quincunx ─────────────────────────────────────────────────────────────────

/// Mechanical pressure demand — the type of strain an encounter places on the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MechanicalDemand {
    /// Demands offensive pressure.
    Aggression,
    /// Demands holding back.
    Patience,
    /// Demands movement.
    Mobility,
    /// Demands holding position while channeling.
    StationaryChannel,
    /// Demands careful inventory management.
    InventoryPrecision,
    /// Demands precise parry timing.
    ParryTiming,
    /// Demands social negotiation.
    Diplomacy,
    /// Demands crafting skill.
    Crafting,
    /// Demands stealth/theft skill.
    Theft,
    /// Demands building social witness/reputation.
    WitnessBuilding,
    /// Demands navigating toward death.
    DeathRoute,
    /// Demands declining the encounter.
    Refusal,
}

/// A double-bind pressure — two conflicting demands and their combined severity.
#[derive(Debug, Clone, Copy)]
pub struct QuincunxPressure {
    /// The first mechanical demand.
    pub demand_a: MechanicalDemand,
    /// The second mechanical demand.
    pub demand_b: MechanicalDemand,
    /// Severity of the bind (Permyriad scale).
    pub severity: u16,
}

// ── Verdict ──────────────────────────────────────────────────────────────────

/// The complete verdict from the dissonance sieve — authority, power, and cost.
#[derive(Debug, Clone, Copy)]
pub struct DissonanceVerdict {
    /// The authority outcome.
    pub authority: AuthorityOutcome,
    /// Power modifier (Permyriad scale).
    pub power_modifier_q: i32,
    /// Dissonance pressure that accumulates from the interaction.
    pub dissonance_pressure: u16,
    /// Entropy cost (energy budget) for this interaction.
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

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_authority_gives_neutral_power() {
        let body = HarmonicBody { element: ClassicalElement::Fire, tier: AlchemicalTier::Albedo, resonance_hz: 432, inverse: false, mass_q: 1000 };
        let ctx = AuthorityContext::default();
        let v = evaluate(&body, &body, &ctx, None);
        assert_eq!(v.authority, AuthorityOutcome::Clash);
    }

    #[test]
    fn superior_initiative_gives_bonification() {
        let body = HarmonicBody { element: ClassicalElement::Fire, tier: AlchemicalTier::Rubedo, resonance_hz: 800, inverse: false, mass_q: 250 };
        let ctx = AuthorityContext { initiative_delta: 5, temperament: Temperament::Benefic, ..Default::default() };
        let v = evaluate(&body, &body, &ctx, None);
        assert_eq!(v.authority, AuthorityOutcome::Bonification);
        assert!(v.power_modifier_q > 1000);
    }

    #[test]
    fn fire_beats_air() {
        let fire = HarmonicBody { element: ClassicalElement::Fire, tier: AlchemicalTier::Albedo, resonance_hz: 432, inverse: false, mass_q: 1000 };
        let air = HarmonicBody { element: ClassicalElement::Air, tier: AlchemicalTier::Albedo, resonance_hz: 432, inverse: false, mass_q: 1000 };
        let ctx = AuthorityContext::default();
        let v = evaluate(&fire, &air, &ctx, None);
        assert!(v.power_modifier_q > 750); // fire has advantage over air
    }
}
