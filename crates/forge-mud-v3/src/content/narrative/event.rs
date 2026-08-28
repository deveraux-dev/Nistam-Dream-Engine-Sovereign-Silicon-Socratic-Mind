//! Event Architecture — the central integration point.
//!
//! An event = site + actors + factions + environment + spirit_layer + Shadow_pressure + prior_flags.
//! Events replace bosses. They have multiple angles and resolution modes.

use crate::faction_mind::Faction;
use super::state::{PlayerState, WorldState, ShadowTier};

// ── Enums ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Narrative perspectives or approaches to understanding an event.
pub enum EventAngle {
    /// Physical combat angle.
    Combat,
    /// Social/political interaction angle.
    Social,
    /// Environmental/spatial angle.
    Environmental,
    /// Spiritual/metaphysical angle.
    Spirit,
    /// Ledger/administrative angle.
    Ledger,
    /// Shadow/dark power angle.
    Shadow,
    /// Faction interest angle.
    Faction,
    /// Mercy/compassion angle.
    Mercy,
    /// Void/nihilistic angle.
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Ways an event can be resolved (final outcomes).
pub enum ResolutionMode {
    /// Destroy the antagonist.
    Kill,
    /// Show mercy; let them live.
    Spare,
    /// Reveal the truth to the world.
    Expose,
    /// Bind the antagonist to a pact.
    Bind,
    /// Erase them from existence.
    Erase,
    /// Inherit their burden/role.
    Inherit,
    /// Walk away; ignore the problem.
    Abandon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Clues or pieces of evidence that reveal story elements (12 per zone).
pub enum DiscoveryTell {
    /// Scratch marks (violence indicator).
    ScratchMarks,
    /// Open ledger entry (administrative trace).
    OpenLedger,
    /// Embedded weapon (tool trace).
    EmbeddedWeapon,
    /// Ash residue (thermal/magical trace).
    AshResidue,
    /// Cloth knot (fiber/binding trace).
    ClothKnot,
    /// Wall debt (structural/time trace).
    WallDebt,
    /// Root pulse (living/organic trace).
    RootPulse,
    /// Cold spot (spiritual/void trace).
    ColdSpot,
    /// Bell rhythm (sonic/resonance trace).
    BellRhythm,
    /// Ink stain (recording/writing trace).
    InkStain,
    /// Collapsed arch (structural failure trace).
    CollapsedArch,
    /// Body position (death/dismay trace).
    BodyPosition,
}

// ── Event State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
/// State tracking for a single narrative event.
pub struct EventState {
    /// The event's ID (typically zone index).
    pub id: u16,
    /// Volatility/danger level (0-255).
    pub volatility: u8,
    /// Which faction owns/controls this event (if any).
    pub faction_owner: Option<Faction>,
    /// Bitset of discovered event angles.
    pub discovered_angles: u16,
    /// How the event was resolved (if at all).
    pub resolved_mode: Option<ResolutionMode>,
    /// Whether the spirit-variant route has been unlocked.
    pub spirit_variant_unlocked: bool,
    /// Shadow interference level during event (0-255).
    pub shadow_interference: u8,
    /// Number of witnesses still alive.
    pub witnesses_alive: u8,
    /// Quality of evidence collected (0-255).
    pub evidence_quality: u8,
}

impl EventState {
    /// Construct a new event state.
    pub fn new(id: u16) -> Self {
        Self {
            id,
            volatility: 0,
            faction_owner: None,
            discovered_angles: 0,
            resolved_mode: None,
            spirit_variant_unlocked: false,
            shadow_interference: 0,
            witnesses_alive: 0,
            evidence_quality: 0,
        }
    }

    /// Mark an event angle as discovered.
    pub fn discover_angle(&mut self, angle: EventAngle) {
        self.discovered_angles |= 1 << (angle as u16);
    }

    /// Check if an event angle has been discovered.
    pub fn has_angle(&self, angle: EventAngle) -> bool {
        self.discovered_angles & (1 << (angle as u16)) != 0
    }

    /// Count how many angles have been discovered.
    pub fn angle_count(&self) -> u32 {
        self.discovered_angles.count_ones()
    }

    /// Check if this event has been resolved.
    pub fn is_resolved(&self) -> bool {
        self.resolved_mode.is_some()
    }
}

// ── Resolution Effects ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
/// World state changes caused by resolving an event.
pub struct ResolutionDelta {
    /// Change to public fear level.
    pub public_fear: i8,
    /// Change to root bloom (cycle acceleration).
    pub root_bloom: i8,
    /// Change to shadow pressure (escalation).
    pub shadow_pressure: i8,
    /// Change to memory integrity.
    pub memory_integrity: i8,
    /// Change to entropy debt accumulation.
    pub entropy_debt: i8,
    /// Change to spirit leak.
    pub spirit_leak: i8,
    /// Change to event volatility (unpredictability).
    pub event_volatility: i8,
    /// Change to ledger control (institutional power).
    pub ledger_control: i8,
    /// Whether faction relations shift.
    pub faction_pressure_shift: bool,
    /// Whether a new route is unlocked.
    pub route_unlock: bool,
    /// Whether ending bits should be updated.
    pub ending_mask_update: bool,
}

/// Calculate world effects of resolving an event in a given mode.
pub fn resolution_effects(mode: ResolutionMode) -> ResolutionDelta {
    match mode {
        ResolutionMode::Kill => ResolutionDelta {
            public_fear: 8,
            root_bloom: 4,
            shadow_pressure: 6,
            memory_integrity: -1,
            ..Default::default()
        },
        ResolutionMode::Spare => ResolutionDelta {
            public_fear: -2,
            shadow_pressure: 2,
            ..Default::default()
        },
        ResolutionMode::Expose => ResolutionDelta {
            ledger_control: -8,
            memory_integrity: 4,
            faction_pressure_shift: true,
            ..Default::default()
        },
        ResolutionMode::Bind => ResolutionDelta {
            entropy_debt: 3,
            route_unlock: true,
            ..Default::default()
        },
        ResolutionMode::Erase => ResolutionDelta {
            memory_integrity: -10,
            shadow_pressure: 8,
            ..Default::default()
        },
        ResolutionMode::Inherit => ResolutionDelta {
            shadow_pressure: 3,
            ending_mask_update: true,
            ..Default::default()
        },
        ResolutionMode::Abandon => ResolutionDelta {
            event_volatility: 10,
            faction_pressure_shift: true,
            ..Default::default()
        },
    }
}

// ── Event Resolver ───────────────────────────────────────────────────────────

/// Input context for resolving an event.
pub struct EventResolverInput<'a> {
    /// The event being resolved.
    pub event: &'a EventState,
    /// Current player state.
    pub player: &'a PlayerState,
    /// Current world state.
    pub world: &'a WorldState,
    /// The resolution mode chosen.
    pub resolution: ResolutionMode,
}

#[derive(Debug, Clone, Copy)]
/// Results of resolving an event.
pub struct EventResolverOutput {
    /// World state changes.
    pub delta: ResolutionDelta,
    /// Whether a new shadow tier is unlocked (escalation).
    pub shadow_tier_unlock: Option<ShadowTier>,
    /// Which ending bits are now possible.
    pub ending_bits: u32,
}

/// Determine the consequences of resolving an event.
pub fn resolve_event(input: &EventResolverInput) -> EventResolverOutput {
    let base_delta = resolution_effects(input.resolution);

    // Shadow tier escalation based on event count and resolution pattern
    let shadow_unlock = if input.player.erasure_count() >= 5 && input.world.shadow_tier == ShadowTier::Stalker {
        Some(ShadowTier::Blighted)
    } else if input.player.spirit_deaths >= 3 && input.world.shadow_tier == ShadowTier::None {
        Some(ShadowTier::Stalker)
    } else {
        None
    };

    // Ending bits based on resolution pattern
    let ending_bits = match input.resolution {
        ResolutionMode::Kill if input.player.erasure_count() > 3 => 1 << 1,  // Darkness Now
        ResolutionMode::Expose if input.event.witnesses_alive > 2 => 1 << 6, // Bell of Witnesses
        ResolutionMode::Spare if input.player.mercy_count() > 5 => 1 << 7,   // Ash Saint
        ResolutionMode::Erase => 1 << 1, // Darkness Now
        ResolutionMode::Inherit => 1 << 8, // Hollow Inheritance
        ResolutionMode::Abandon => 1 << 9, // Debt Eternal
        _ => 0,
    };

    EventResolverOutput {
        delta: base_delta,
        shadow_tier_unlock: shadow_unlock,
        ending_bits,
    }
}

/// Apply event resolution output to update world and player state.
pub fn apply_resolution(world: &mut WorldState, _player: &mut PlayerState, output: &EventResolverOutput) {
    let d = &output.delta;
    world.root_bloom = world.root_bloom.saturating_add_signed(d.root_bloom);
    world.public_fear = world.public_fear.saturating_add_signed(d.public_fear);
    world.memory_integrity = world.memory_integrity.saturating_add_signed(d.memory_integrity);
    world.entropy_debt = world.entropy_debt.saturating_add_signed(d.entropy_debt);
    world.spirit_leak = world.spirit_leak.saturating_add_signed(d.spirit_leak);
    world.event_volatility = world.event_volatility.saturating_add_signed(d.event_volatility);
    world.ledger_control = world.ledger_control.saturating_add_signed(d.ledger_control);

    if let Some(tier) = output.shadow_tier_unlock {
        world.shadow_tier = tier;
    }

    world.ending_mask |= output.ending_bits;

    // Track resolution in player flags
    // (caller sets specific mercy/erasure/witness bits based on event context)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_angle_discovery() {
        let mut evt = EventState::new(1);
        evt.discover_angle(EventAngle::Combat);
        evt.discover_angle(EventAngle::Spirit);
        assert!(evt.has_angle(EventAngle::Combat));
        assert!(evt.has_angle(EventAngle::Spirit));
        assert!(!evt.has_angle(EventAngle::Ledger));
        assert_eq!(evt.angle_count(), 2);
    }

    #[test]
    fn kill_resolution_raises_fear() {
        let d = resolution_effects(ResolutionMode::Kill);
        assert_eq!(d.public_fear, 8);
        assert_eq!(d.shadow_pressure, 6);
    }

    #[test]
    fn spare_resolution_lowers_fear() {
        let d = resolution_effects(ResolutionMode::Spare);
        assert_eq!(d.public_fear, -2);
    }

    #[test]
    fn expose_shifts_factions() {
        let d = resolution_effects(ResolutionMode::Expose);
        assert!(d.faction_pressure_shift);
        assert_eq!(d.ledger_control, -8);
    }

    #[test]
    fn resolver_escalates_shadow() {
        let evt = EventState::new(1);
        let mut player = PlayerState::default();
        player.spirit_deaths = 4;
        let world = WorldState::default();
        let input = EventResolverInput {
            event: &evt,
            player: &player,
            world: &world,
            resolution: ResolutionMode::Kill,
        };
        let output = resolve_event(&input);
        assert_eq!(output.shadow_tier_unlock, Some(ShadowTier::Stalker));
    }

    #[test]
    fn apply_resolution_mutates_world() {
        let mut world = WorldState { memory_integrity: 100, ..Default::default() };
        let mut player = PlayerState::default();
        let output = EventResolverOutput {
            delta: resolution_effects(ResolutionMode::Erase),
            shadow_tier_unlock: None,
            ending_bits: 1 << 1,
        };
        apply_resolution(&mut world, &mut player, &output);
        assert_eq!(world.memory_integrity, 90);
        assert!(world.has_ending_bit(1));
    }

    #[test]
    fn abandon_increases_volatility() {
        let d = resolution_effects(ResolutionMode::Abandon);
        assert_eq!(d.event_volatility, 10);
        assert!(d.faction_pressure_shift);
    }

    #[test]
    fn bind_unlocks_route() {
        let d = resolution_effects(ResolutionMode::Bind);
        assert!(d.route_unlock);
        assert_eq!(d.entropy_debt, 3);
    }
}
