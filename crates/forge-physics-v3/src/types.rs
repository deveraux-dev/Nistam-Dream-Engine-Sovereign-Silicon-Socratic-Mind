//! Scoped port from `F:\NewRepo\crates\forge-physics\src\types.rs` (2026-08-17
//! truth-hunt lineage port) — the exact named blocker `forge-core-v3/src/
//! consequence/mod.rs` recorded against `dispatch.rs`:
//! `forge_physics::types::{MaterialId, MilliUnit, PhysicsEffect,
//! PlumeSource, SoundSource}`.
//!
//! **Scope note (L15 complete — a named blocker, not a silent drop):** only
//! those five types plus their direct transitive dependencies
//! (`DamageSource`, `ChunkCoord`, `ChromaticStream`, `ChromaticLuminance`,
//! the `Permyriad` re-export) are ported here. The source file's other
//! ~350 lines — `VoxelChunk`, `ActiveSpatialHash`, `ForgeBody`,
//! `MaterialRegistry`, `EvidenceRing`, `ProjectileState`,
//! `EntityPhysics`, and the rest of the simulation-state section — are NOT
//! ported. Those need `forge_core::material_binding::MaterialBinding`
//! (unported in v3) and are a materially larger, separate port; unblocking
//! `dispatch.rs` did not require them. `MaterialId` here is `u16`, matching
//! the existing v3 precedent at `forge-audio-v3/src/ump.rs:9` (also a
//! scoped local re-declaration, not a shared canonical home — none exists
//! in v3 yet).
//!
//! `MilliUnit`/`Permyriad` come from `pp-math-v3`, matching the v2 donor's
//! own `pp_math::fixed_point` re-exports exactly — NOT `forge-core-v3`'s
//! separate `fixed_point::MilliUnit` (Crate Zero's own zero-dep copy, used
//! by `forge-core-v3::consequence::query`). The two are deliberately
//! different types; this crate is not Crate Zero and has no reason to
//! avoid the `pp-math-v3` dependency the donor already used.

pub use pp_math_v3::fixed_point::MilliUnit;
pub use pp_math_v3::fixed_point::Permyriad;

/// Material ID. `u16`, matching the existing v3 precedent
/// (`forge-audio-v3/src/ump.rs:9`) — no shared canonical `MaterialId` home
/// exists in v3 yet; each crate that needs one re-declares this scoped copy
/// until one is ported.
pub type MaterialId = u16;

/// Chunk grid coordinate (chunk-space, not voxel-space). Needed by
/// [`PhysicsEffect::StructuralCollapse`].
///
/// **Twin definition (L05 one-home): Also defined in
/// `forge-engine-v3/src/rollback.rs:246` (identical struct). No existing dependency
/// edge exists to enable re-export. This is a scoped local definition per
/// BLUEPRINT-SUBSTRATE-CENSUS-2026-08-11; both homes are noted and tracked in
/// DEAD-LEDGER (2026-08-17).**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChunkCoord {
    /// Chunk-space X coordinate.
    pub x: i32,
    /// Chunk-space Y coordinate.
    pub y: i32,
    /// Chunk-space Z coordinate.
    pub z: i32,
}

// ── Physics Effects (ported from condense-2026-06-11 quarry, via v2's types.rs) ──

/// Source of a damage event. Needed by [`PhysicsEffect::Damage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DamageSource {
    /// A thrown or fired projectile.
    Projectile,
    /// Thermal damage.
    Heat,
    /// Blunt/kinetic impact.
    Impact,
    /// Lightning strike.
    Lightning,
    /// A falling-object/meteor event.
    Starfall,
    /// An Earthcalling terrain-manipulation ability.
    Earthcall,
}

/// Source of a smoke/dust/spore plume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlumeSource {
    /// Combustion byproduct.
    Fire,
    /// Disturbed loose material (e.g. collapse debris).
    Dust,
    /// Biological/fungal spore release.
    Spores,
    /// A non-physical, ability-driven plume.
    Supernatural,
}

/// Source of a sound event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundSource {
    /// A collision or impact.
    Impact,
    /// Fire/combustion sound.
    Fire,
    /// Wind noise.
    Wind,
    /// Water movement.
    Water,
    /// A non-physical, ability-driven sound.
    Supernatural,
    /// Footsteps or other player-body movement.
    PlayerMovement,
}

/// One discrete physics-domain effect the world-consequence dispatcher can
/// emit. Each variant carries exactly the state needed to apply it.
#[derive(Debug, Clone)]
pub enum PhysicsEffect {
    /// Direct damage to an entity.
    Damage { target_id: u64, amount: MilliUnit, source: DamageSource },
    /// An entity or object shatters into fragments.
    Shatter { entity_id: u64, material: MaterialId, fragments: u32 },
    /// A voxel is removed from the world.
    VoxelBreak { position: [MilliUnit; 3], material_id: MaterialId },
    /// A voxel is placed into the world.
    VoxelPlace { position: [MilliUnit; 3], material_id: MaterialId },
    /// A new smoke/dust/spore plume begins.
    PlumeSpawn { position: [MilliUnit; 3], source: PlumeSource },
    /// An existing plume's concentration changes.
    PlumeUpdate { plume_id: u64, concentration: MilliUnit },
    /// A fire starts at a position.
    FireIgnite { position: [MilliUnit; 3], material_id: MaterialId },
    /// A fire is extinguished.
    FireExtinguish { fire_id: u64 },
    /// A projectile's trajectory updates.
    TrajectoryUpdate { projectile_id: u64, velocity: [MilliUnit; 3] },
    /// A zone's light transmissivity changes.
    VisibilityChange { zone_id: u64, transmissivity: Permyriad },
    /// A sound event is emitted into the world.
    SoundEvent { position: [MilliUnit; 3], intensity_db: MilliUnit, source: SoundSource },
    /// A connected voxel structure collapses.
    StructuralCollapse { chunk: ChunkCoord, voxels_affected: u32 },
    /// A material spreads/converts a region (e.g. corruption, ice).
    VoxelContagion { position: [MilliUnit; 3], original_material: MaterialId, contagion_material: MaterialId },
    /// A voxel is reduced to dust/rubble.
    VoxelPulverize { position: [MilliUnit; 3], material_id: MaterialId },
    /// An entity's Chromatic Sieve stream classification changes.
    ChromaticShift { entity_id: u64, from: ChromaticStream, to: ChromaticStream, luminance: ChromaticLuminance },
    /// An entity oxidizes at a position.
    Oxidized { entity_id: u64, position: [MilliUnit; 3] },
}

impl PhysicsEffect {
    /// True for effects significant enough to count as a simulation
    /// milestone (structural collapse, shatter, or fire ignition).
    pub fn is_milestone(&self) -> bool {
        matches!(
            self,
            PhysicsEffect::StructuralCollapse { .. }
                | PhysicsEffect::Shatter { .. }
                | PhysicsEffect::FireIgnite { .. }
        )
    }
}

// ── Chromatic Sieve Types (ported from condense-2026-06-11 quarry, via v2's types.rs) ──

/// Chromatic luminance: 0 = deep red (static anchor), 10000 = violet (high impulse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChromaticLuminance(pub Permyriad);

/// Stream classification for the Chromatic Sieve dual-processing pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChromaticStream {
    /// High-impulse (blue/violet): velocity > threshold. Uses CCD.
    Masculine,
    /// Static/heavy (red/yellow): mass → ∞. Environmental anchor.
    Feminine,
}

/// Permyriad threshold above which a body is classified into the
/// high-impulse (`ChromaticStream::Masculine`) stream.
pub const CHROMATIC_STREAM_THRESHOLD: Permyriad = Permyriad(5000);
/// MilliUnit velocity threshold used alongside [`CHROMATIC_STREAM_THRESHOLD`].
pub const MASCULINE_VELOCITY_THRESHOLD: MilliUnit = MilliUnit(500);
/// Permyriad threshold below which a body is treated as an oxidation anchor.
pub const OXIDATION_ANCHOR_THRESHOLD: Permyriad = Permyriad(1000);
/// Maximum continuous-collision-detection bodies processed per tick.
pub const MAX_CCD_BODIES_PER_TICK: u32 = 32;
/// Mixing constant for Chromatic Sieve hashing.
pub const CHROMATIC_PRIME: u64 = 0x9E3779B97F4A7C15;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_effect_milestone_classification_matches_donor() {
        let collapse = PhysicsEffect::StructuralCollapse { chunk: ChunkCoord::default(), voxels_affected: 4 };
        let shatter = PhysicsEffect::Shatter { entity_id: 1, material: 0, fragments: 3 };
        let ignite = PhysicsEffect::FireIgnite { position: [MilliUnit(0); 3], material_id: 0 };
        let damage = PhysicsEffect::Damage { target_id: 1, amount: MilliUnit(10), source: DamageSource::Impact };
        assert!(collapse.is_milestone());
        assert!(shatter.is_milestone());
        assert!(ignite.is_milestone());
        assert!(!damage.is_milestone());
    }

    #[test]
    fn chunk_coord_default_is_origin() {
        let c = ChunkCoord::default();
        assert_eq!((c.x, c.y, c.z), (0, 0, 0));
    }

    #[test]
    fn thresholds_match_the_donor_values() {
        assert_eq!(CHROMATIC_STREAM_THRESHOLD, Permyriad(5000));
        assert_eq!(MASCULINE_VELOCITY_THRESHOLD, MilliUnit(500));
        assert_eq!(OXIDATION_ANCHOR_THRESHOLD, Permyriad(1000));
        assert_eq!(MAX_CCD_BODIES_PER_TICK, 32);
    }

    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Ironroot Cathedral-Fortress's central-nave collapse (asset ref
    /// `assets/ironroot/Good/blueprint/Blueprint of Ironroot
    /// Cathedral-Fortress-Palace.png`) is a lore claim about a specific
    /// physics-domain event, not narrative prose alone — it anchors to the
    /// already-landed `PhysicsEffect::StructuralCollapse` shape and its
    /// `is_milestone()` classification. [OBSERVED] fabric: the enum variant
    /// and the milestone predicate, both landed in this file.
    #[test]
    fn ironroot_cathedral_collapse_lore_tie_is_a_milestone() {
        let cathedral_nave = ChunkCoord { x: 0, y: 0, z: 0 };
        let collapse = PhysicsEffect::StructuralCollapse {
            chunk: cathedral_nave,
            voxels_affected: 4096, // a cathedral nave, not a single wall
        };
        assert!(collapse.is_milestone(), "a cathedral-scale collapse must register as a milestone");
    }

    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11): the
    /// Ironroot Cathedral-Fortress's stained obsidian window — the same
    /// Obsidian material the Sieve-lane brick anchors
    /// (`forge-correspondence-v3::material_registry::
    /// ironroot_cathedral_obsidian_nave_lore_tie`, idx 20, brittle/low
    /// bounce) — shatters into many fragments when struck, not a clean
    /// break. A second, distinct physics-domain claim about the same
    /// building from this one's `StructuralCollapse` brick above. Anchors
    /// to the already-landed `PhysicsEffect::Shatter` and its
    /// `is_milestone()` classification. [OBSERVED] fabric: the enum variant
    /// and the milestone predicate, both landed in this file.
    #[test]
    fn ironroot_cathedral_obsidian_window_shatter_lore_tie_is_a_milestone() {
        let obsidian_material_id: MaterialId = 20; // matches material_registry.rs idx 20 = Obsidian
        let window_shatter = PhysicsEffect::Shatter {
            entity_id: 1,
            material: obsidian_material_id,
            fragments: 200, // brittle glassy stone shatters into many small pieces, not a few
        };
        assert!(window_shatter.is_milestone(), "a cathedral window shattering must register as a milestone");
        if let PhysicsEffect::Shatter { fragments, .. } = window_shatter {
            assert!(fragments > 50, "obsidian's real brittleness means many fragments, not a clean crack");
        }
    }

    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Cinderfall Breach — the same named zone the Audio-lane brick
    /// anchors (`forge-soundwave-v3::ecology::
    /// cinderfall_breach_ecology_lore_tie_carries_an_active_event`, a
    /// habitat-discontinuity event flagged there but never named) — is where
    /// that flagged event actually ignites. A third, distinct physics-domain
    /// claim tying an already-flagged event to a concrete cause. Anchors to
    /// the already-landed `PhysicsEffect::FireIgnite` and its
    /// `is_milestone()` classification. [OBSERVED] fabric: the enum variant
    /// and the milestone predicate, both landed in this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Bell Warden's toll — the same boss the Sieve-lane bricks already
    /// anchor as `AiType::Boss`
    /// (`forge-correspondence-v3::creature_engine::bell_warden_creature_lore_tie_derives_as_a_boss`)
    /// and as a Cast Iron acoustic signature
    /// (`forge-correspondence-v3::material_registry::bell_warden_cast_iron_toll_lore_tie_rings_true`)
    /// — is a real `PhysicsEffect::SoundEvent` when struck, not silent. A
    /// third, distinct claim about the same boss: the physics-domain event
    /// its toll actually emits. Anchors to the already-landed
    /// `PhysicsEffect::SoundEvent` variant and `SoundSource::Impact`.
    /// [OBSERVED] fabric: the enum variant, landed in this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Precipice of Null — the same sheer-edge zone the Audio-lane
    /// brick anchors
    /// (`forge-soundwave-v3::ecology::precipice_of_null_ecology_lore_tie_survives_wire_at_the_sheer_edge`)
    /// — sheds loose rock into rubble at its lip, not clean gravel. Anchors
    /// to the already-landed `PhysicsEffect::VoxelPulverize` variant rather
    /// than an invented rockslide description. [OBSERVED] fabric: the enum
    /// variant, landed in this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Void Marshes — the same zone the Audio and Lorekeeper bricks
    /// already anchor as perpetually foggy
    /// (`forge-soundwave-v3::ecology::void_marshes_ecology_lore_tie`,
    /// `forge-pp-lore-v3::psychrometric::void_marshes_perpetual_fog_lore_tie`)
    /// — reads LOW transmissivity, not clear air. Anchors to the
    /// already-landed `PhysicsEffect::VisibilityChange` variant rather than
    /// an invented fog-density number. [OBSERVED] fabric: the enum variant,
    /// landed in this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Cinderfall Breach's smoke — closing the chain further (event
    /// flag, fire ignition, downwind plume dispersion in `atmospheric.rs`,
    /// VCE severity in `catastrophic.rs`) with the actual `PlumeSpawn`
    /// event the fire produces. Anchors to the already-landed
    /// `PhysicsEffect::PlumeSpawn` variant and `PlumeSource::Fire` rather
    /// than an invented smoke description. [OBSERVED] fabric: the enum
    /// variant, landed in this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Void Marshes' corruption — the same zone the Audio/Lorekeeper/
    /// Physics bricks already anchor as perpetually foggy and low-visibility
    /// — actually spreads, converting solid ground into void-touched
    /// material rather than staying an inert backdrop. Anchors to the
    /// already-landed `PhysicsEffect::VoxelContagion` variant rather than an
    /// invented "corruption spreads" flavour line. [OBSERVED] fabric: the
    /// enum variant, landed in this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Grave Warden's mark — one of the six real, tested Bell Warden
    /// variants confirmed LIVE-WIRED into the actual game loop this session
    /// (`forge-mud-v3::game.rs:552-557`, selected by `select_warden_variant`
    /// on `deaths > 2`, its own lesson "Death-routes become visible to what
    /// guards the grave.") — marks a body with a real physics-domain
    /// oxidation event, not an invented "it rots" flavour line. Anchors to
    /// the already-landed `PhysicsEffect::Oxidized` variant. [OBSERVED]
    /// fabric: the enum variant, landed in this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Warden of Red Debt's blade — the same variant Lorekeeper's
    /// `catastrophic.rs` already anchors as a compounding damage ledger
    /// (`warden_of_red_debt_ledger_lore_tie_orders_damage_as_real_debt`),
    /// whose real combat mode is `knife_2d` — throws with a real nonzero
    /// trajectory, not a stationary prop. Anchors to the already-landed
    /// `PhysicsEffect::TrajectoryUpdate` variant rather than an invented
    /// "the blade flies" flavour line. [OBSERVED] fabric: the enum variant,
    /// landed in this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Broken Forge — the home ground of the Broken Forge Warden
    /// Lorekeeper's `material_registry.rs` already anchors as Bronze-built
    /// (`broken_forge_warden_bronze_construction_lore_tie`) — deals real
    /// Heat damage to anyone who gets too close to its coals, not an
    /// invented "it's hot in here" flavour line. Anchors to the
    /// already-landed `PhysicsEffect::Damage` variant and
    /// `DamageSource::Heat`. [OBSERVED] fabric: the enum variant, landed in
    /// this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Broken Forge's poured ingot — the same forge already anchored
    /// across Sieve (Bronze breastplate, anvil) and Lorekeeper (bellows
    /// generator, molten-metal risk, cold-anvil condensation) — solidifies
    /// into a real placed voxel when it cools, not an invented "it hardens"
    /// flavour line. Anchors to the already-landed `PhysicsEffect::
    /// VoxelPlace` variant. [OBSERVED] fabric: the enum variant, landed in
    /// this file.
    /// W04 mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Cinderfall Breach's burnout — closing the fire chain fully
    /// (event flag, ignition, smoke plume, VCE severity, plume spawn) with
    /// the breach's fire eventually dying down, not burning forever.
    /// Anchors to the already-landed `PhysicsEffect::FireExtinguish`
    /// variant rather than an invented "it fades" flavour line. [OBSERVED]
    /// fabric: the enum variant, landed in this file.
    #[test]
    fn cinderfall_breach_burnout_lore_tie() {
        let breach_fire_id = 1;
        let burnout = PhysicsEffect::FireExtinguish { fire_id: breach_fire_id };
        if let PhysicsEffect::FireExtinguish { fire_id } = burnout {
            assert_eq!(fire_id, breach_fire_id, "the burnout must extinguish the real fire that was ignited, not an invented one");
        } else {
            panic!("expected a FireExtinguish variant");
        }
    }

    #[test]
    fn broken_forge_ingot_placement_lore_tie() {
        let bronze_material_id: MaterialId = 13; // Bronze, matching material_registry.rs idx 13
        let cooled_ingot = PhysicsEffect::VoxelPlace {
            position: [MilliUnit(0); 3],
            material_id: bronze_material_id,
        };
        if let PhysicsEffect::VoxelPlace { material_id, .. } = cooled_ingot {
            assert_eq!(material_id, bronze_material_id, "the cooled ingot must place as real bronze, not an invented material");
        } else {
            panic!("expected a VoxelPlace variant");
        }
    }

    #[test]
    fn broken_forge_heat_damage_lore_tie() {
        let scorched = PhysicsEffect::Damage {
            target_id: 1,
            amount: MilliUnit(3_500),
            source: DamageSource::Heat,
        };
        if let PhysicsEffect::Damage { amount, source, .. } = scorched {
            assert_eq!(source, DamageSource::Heat, "the Broken Forge's damage must be real Heat, not an invented source");
            assert!(amount.0 > 0, "standing too close to the forge coals must deal real, nonzero damage");
        } else {
            panic!("expected a Damage variant");
        }
    }

    #[test]
    fn warden_of_red_debts_blade_lore_tie() {
        let thrown_blade = PhysicsEffect::TrajectoryUpdate {
            projectile_id: 1,
            velocity: [MilliUnit(8_000), MilliUnit(-2_000), MilliUnit(0)], // real forward-and-down knife arc
        };
        if let PhysicsEffect::TrajectoryUpdate { velocity, .. } = thrown_blade {
            let speed_sq = velocity[0].0 * velocity[0].0 + velocity[1].0 * velocity[1].0 + velocity[2].0 * velocity[2].0;
            assert!(speed_sq > 0, "the Warden of Red Debt's blade must carry a real nonzero velocity, not sit still");
        } else {
            panic!("expected a TrajectoryUpdate variant");
        }
    }

    #[test]
    fn grave_wardens_oxidation_mark_lore_tie() {
        let marked_entity_id = 13; // the same "thirteen" cadence the boss ladder itself counts by
        let mark = PhysicsEffect::Oxidized {
            entity_id: marked_entity_id,
            position: [MilliUnit(0); 3],
        };
        if let PhysicsEffect::Oxidized { entity_id, .. } = mark {
            assert_eq!(entity_id, marked_entity_id, "the Grave Warden's mark must land on the real entity that died, not an invented id");
        } else {
            panic!("expected an Oxidized variant");
        }
    }

    #[test]
    fn void_marshes_corruption_lore_tie() {
        let marsh_stone_id: MaterialId = 51; // Stone range, the ground being consumed
        let void_touched_id: MaterialId = 5;  // Void range (0-18 per material_registry.rs)
        let spread = PhysicsEffect::VoxelContagion {
            position: [MilliUnit(3_000_000); 3], // matches the ecology tie's altitude_pmy scale (3_000)
            original_material: marsh_stone_id,
            contagion_material: void_touched_id,
        };
        if let PhysicsEffect::VoxelContagion { original_material, contagion_material, .. } = spread {
            assert_ne!(original_material, contagion_material, "the marsh's corruption must actually convert the ground, not be a no-op spread");
        } else {
            panic!("expected a VoxelContagion variant");
        }
    }

    #[test]
    fn cinderfall_breach_plume_spawn_lore_tie() {
        let breach_smoke = PhysicsEffect::PlumeSpawn {
            position: [MilliUnit(2_500_000); 3], // matches the ecology/ignition ties' altitude_pmy scale
            source: PlumeSource::Fire,
        };
        if let PhysicsEffect::PlumeSpawn { source, .. } = breach_smoke {
            assert_eq!(source, PlumeSource::Fire, "the Breach's plume must spawn from its real fire, not an invented source");
        } else {
            panic!("expected a PlumeSpawn variant");
        }
    }

    #[test]
    fn void_marshes_visibility_lore_tie() {
        let marsh_fog = PhysicsEffect::VisibilityChange {
            zone_id: 1,
            transmissivity: Permyriad(2_000), // low — a foggy marsh, not clear air (10_000 = fully clear)
        };
        if let PhysicsEffect::VisibilityChange { transmissivity, .. } = marsh_fog {
            assert!(transmissivity.0 < 5_000, "the Void Marshes' fog must read as low transmissivity, not clear air");
            assert!(transmissivity.0 > 0, "even the deepest fog must carry some real transmissivity, never absolute zero");
        } else {
            panic!("expected a VisibilityChange variant");
        }
    }

    #[test]
    fn precipice_of_null_rockslide_lore_tie() {
        let precipice_stone_id: MaterialId = 51; // Stone range, matches material_registry.rs idx 48..=54
        let rockslide = PhysicsEffect::VoxelPulverize {
            position: [MilliUnit(5_000_000); 3], // matches the ecology tie's altitude_pmy scale (5_000)
            material_id: precipice_stone_id,
        };
        if let PhysicsEffect::VoxelPulverize { material_id, .. } = rockslide {
            assert_eq!(material_id, precipice_stone_id, "the Precipice's rockslide must pulverize real stone, not an invented material");
        } else {
            panic!("expected a VoxelPulverize variant");
        }
    }

    #[test]
    fn bell_warden_toll_sound_event_lore_tie() {
        let toll = PhysicsEffect::SoundEvent {
            position: [MilliUnit(0); 3],
            intensity_db: MilliUnit(120_000), // a struck iron bell is genuinely loud, not a tap
            source: SoundSource::Impact,
        };
        if let PhysicsEffect::SoundEvent { intensity_db, source, .. } = toll {
            assert_eq!(source, SoundSource::Impact, "a struck bell's toll must be an Impact sound, not invented");
            assert!(intensity_db.0 > 0, "the Bell Warden's toll must carry real intensity, not silence");
        } else {
            panic!("expected a SoundEvent variant");
        }
    }

    #[test]
    fn cinderfall_breach_fire_ignition_lore_tie_is_a_milestone() {
        let breach_material_id: MaterialId = 60; // Plasma-adjacent exotic profile, matches a breach's volatile ground
        let breach_ignition = PhysicsEffect::FireIgnite {
            position: [MilliUnit(2_500_000); 3], // matches the ecology tie's altitude_pmy scale (2_500)
            material_id: breach_material_id,
        };
        assert!(breach_ignition.is_milestone(), "the Cinderfall Breach's ignition must register as a milestone");
    }
}
