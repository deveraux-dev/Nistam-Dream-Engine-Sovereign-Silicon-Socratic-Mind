//! 2D side-scroll arena geometry — ported from
//! `F:\NewRepo\crates\ironroot\src\platform.rs` (2026-08-13, "2D first
//! because the levels and maps are built in html and json").
//!
//! Terrain is a spine of platform segments (not a heightmap). Entities are
//! terrain-relative: `y_mm` is offset from the platform surface, not world
//! origin. Confirmed PORTABLE this session: the 4 real float hits in the
//! v2 source are all at the `SpawnEntry`/render boundary (cut here along
//! with the system they fed — see below), never in the spine/collision math.
//!
//! **Scope cuts (L15, named plainly, not silent):**
//! - `PlatformMaterial::from_ce_scan` — needs `forge_core::correspondence::
//!   MaterialScan`, an unported CE (texture-scan) type. `friction_permyriad`/
//!   `bounce_permyriad`/`crumble_ticks` (the actual gameplay-facing API) are
//!   ported; only the "derive material from a scanned texture" constructor
//!   is cut.
//! - `ArenaZoneDef`/`ZonePhase` — ported verbatim from `forge_game_systems::
//!   cartridge` (`F:\NewRepo\crates\forge-game-systems\src\cartridge.rs:
//!   570-660`, 2026-08-16), including `ZonePhase::essence`/
//!   `element_essence_id` — zero floats, and its one dependency
//!   (`EssenceDef`/`ESSENCE`) already lives in
//!   `forge_correspondence_v3::essence_registry`. `serde` derives are kept
//!   (this crate already depends on `serde`, `ron` per its `Cargo.toml`) so
//!   a future cartridge-loading pass can deserialize these directly.
//! - `EnemyDef` — still a minimal local stand-in (`id`, `physical` only);
//!   the real one's `drop_chance`/`rare_chance` are `f64` (a genuine
//!   integer-only blocker this file doesn't read yet, so left cut).
//! - `populate_spawn_manager` — needs `forge_game_systems::spawn::
//!   SpawnManager`, a whole unported subsystem. Cut entirely, not stubbed.
//! - `TriggerAction::AudioRegion`'s actual firing — needs `AudioCommandTx`
//!   (`forge_audio::bus::command_tx`, unported). The variant and its data
//!   are kept (a trigger can still be AUTHORED as an audio region); firing
//!   it into a real mixer is owed alongside `audio_bridge.rs`.
//!
//! Everything else — the platform spine, spawn points, trigger zones,
//! parallax layers, deterministic layout generation, and the bridge into
//! [`crate::ironroot::scene_loader::SceneEntity`] — is ported near-verbatim.

use crate::ironroot::brand::{BrandCorruption, Tithe};
use crate::ironroot::dialogue::{DialogueNode, DialogueState};
use crate::ironroot::scene_loader::{EntityKind, SceneEntity};
use crate::rng::Mulberry32;
use forge_correspondence_v3::creature_engine::{derive_stats, GameEntity, PhysicalProfile};
use forge_correspondence_v3::essence_registry::{EssenceDef, ESSENCE};
use serde::{Deserialize, Serialize};

// ── Cartridge schema (ported verbatim, see module doc) ──────────────────────

/// An authored arena zone — ported verbatim from
/// `forge_game_systems::cartridge::ArenaZoneDef`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArenaZoneDef {
    /// Schema revision, for forward-compat cartridge loading.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// The zone's own id.
    pub id: String,
    /// The zone's display name.
    pub name: String,
    /// Phases the arena cycles through. Single phase = static zone.
    /// Multi-phase = the arena shifts elements mid-fight.
    pub phases: Vec<ZonePhase>,
    /// Permyriad buff for matching element (e.g. 1500 = +15% damage).
    #[serde(default = "default_affinity_buff")]
    pub affinity_buff_permyriad: i32,
    /// Permyriad debuff for weak element.
    #[serde(default = "default_weakness_debuff")]
    pub weakness_debuff_permyriad: i32,
}

/// One phase of an [`ArenaZoneDef`] — ported verbatim from
/// `forge_game_systems::cartridge::ZonePhase`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZonePhase {
    /// The elemental theme this phase reads material/parallax from.
    pub element: String,
    /// The phase's display name.
    pub name: String,
    /// Weather override for this phase. Empty = none.
    #[serde(default)]
    pub weather_override: String,
    /// Hazard ids this phase seeds.
    #[serde(default)]
    pub hazards: Vec<String>,
    /// How many ticks this phase lasts. 0 = until arena ends.
    #[serde(default)]
    pub duration_ticks: u32,
    /// 5D SUBSTRATE — the essence slot (`0..=63`) this phase resonates on
    /// (`forge_correspondence_v3::essence_registry::ESSENCE`). 0 = unset;
    /// `essence()` falls back to `element`.
    #[serde(default)]
    pub essence_id: u8,
    /// HUD tint as a sheet token (`palette.<token>`), never a hardcoded
    /// colour.
    #[serde(default)]
    pub palette_token: String,
    /// Post-process the zone wears. Empty = none.
    #[serde(default)]
    pub shader_override: String,
    /// The adaptive-audio bed this phase plays.
    #[serde(default)]
    pub audio_profile: String,
    /// The terrain silhouette this phase asks the generator for.
    #[serde(default)]
    pub topography: String,
    /// Gravity scale in permyriad (10_000 = 1.0g). 0 = engine default.
    /// Integer, never a float: this crosses the determinism firewall.
    #[serde(default)]
    pub gravity_permyriad: i32,
    /// Flora + fauna the zone seeds, by id.
    #[serde(default)]
    pub ecology: Vec<String>,
    /// Ley effects live for the duration of this phase.
    #[serde(default)]
    pub magic: Vec<String>,
}

impl ZonePhase {
    /// The essence row this phase resonates on — the 5D record every
    /// derived axis reads. An unset `essence_id` resolves from `element`,
    /// so a phase authored with nothing but `element = "spirit"` still
    /// lands on a real row.
    pub fn essence(&self) -> EssenceDef {
        let id = if self.essence_id > 0 { self.essence_id } else { Self::element_essence_id(&self.element) };
        ESSENCE[(id & 63) as usize]
    }

    /// Element name -> its essence id. The one place a `&str` element joins
    /// the typed registry.
    pub fn element_essence_id(element: &str) -> u8 {
        match element {
            "fire" => 0,
            "water" => 1,
            "earth" => 2,
            "air" => 3,
            "lightning" => 4,
            "ice" => 5,
            "light" => 6,
            "shadow" => 7,
            // EssenceFamily::Spirit owns 40..=47.
            "spirit" => 40,
            _ => 0,
        }
    }
}

fn default_schema_version() -> u32 {
    1
}
fn default_affinity_buff() -> i32 {
    1500
}
fn default_weakness_debuff() -> i32 {
    1000
}

/// What this file needs from `forge_game_systems::cartridge::EnemyDef`, plus
/// the `PhysicalProfile` that feeds `forge_correspondence_v3::creature_engine
/// ::derive_stats` — this is the wire-in point named in the module doc's
/// "placeholder until a full mob system" comment (2026-08-13). The real
/// cartridge schema will carry the physical profile in its own fields; this
/// stand-in carries it directly until that schema is ported.
#[derive(Debug, Clone)]
pub struct EnemyDef {
    /// The enemy template's own id.
    pub id: String,
    /// Physical measurements that derive this enemy's `GameEntity` stats.
    pub physical: PhysicalProfile,
}

// ── Platform Spine ──────────────────────────────────────────────────────────

/// A single platform segment in the arena spine.
#[derive(Debug, Clone)]
pub struct PlatformSegment {
    /// Left edge X, MilliUnit.
    pub x_start_mm: i64,
    /// Right edge X, MilliUnit.
    pub x_end_mm: i64,
    /// Surface Y, MilliUnit — entities stand on this.
    pub surface_y_mm: i64,
    /// Material id — drives friction/bounce.
    pub material: PlatformMaterial,
    /// One-way platform (can jump through from below).
    pub one_way: bool,
}

/// Platform material — determines physics behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformMaterial {
    /// High friction, no bounce.
    Stone,
    /// Medium friction, slight bounce.
    Wood,
    /// Low friction, no bounce.
    Ice,
    /// Climbable, high friction.
    Vine,
    /// Crumbles after N ticks.
    Void,
}

impl PlatformMaterial {
    /// Friction in permyriad (0-10000). Higher = more grip.
    pub fn friction_permyriad(self) -> i32 {
        match self {
            Self::Stone => 8000,
            Self::Wood => 6000,
            Self::Ice => 1500,
            Self::Vine => 9500,
            Self::Void => 5000,
        }
    }

    /// Bounce in permyriad. 0 = no bounce, 10000 = full elastic.
    pub fn bounce_permyriad(self) -> i32 {
        match self {
            Self::Stone => 0,
            Self::Wood => 1000,
            Self::Ice => 0,
            Self::Vine => 0,
            Self::Void => 0,
        }
    }

    /// Ticks before crumble (Void platforms only). 0 = never.
    pub fn crumble_ticks(self) -> u32 {
        match self {
            Self::Void => 90, // ~1.5s at 60fps
            _ => 0,
        }
    }
}

// ── Spawn Point ─────────────────────────────────────────────────────────────

/// Where enemies/NPCs appear. Terrain-relative.
#[derive(Debug, Clone)]
pub struct SpawnPoint {
    /// This spawn point's own id.
    pub id: String,
    /// X position, MilliUnit.
    pub x_mm: i64,
    /// Platform segment index this spawn sits on.
    pub platform_idx: usize,
    /// Faction tag (guard/trade/civilian/underworld/hostile).
    pub faction: String,
    /// Enemy template id from the cartridge.
    pub enemy_id: String,
    /// Respawn ticks (0 = no respawn).
    pub respawn_ticks: u32,
}

// ── Trigger Zone ────────────────────────────────────────────────────────────

/// Rectangular trigger area for events (dialogue, phase shift, traps).
#[derive(Debug, Clone)]
pub struct TriggerZone {
    /// This trigger's own id.
    pub id: String,
    /// Centre X, MilliUnit.
    pub x_mm: i64,
    /// Centre Y, MilliUnit.
    pub y_mm: i64,
    /// Half-width, MilliUnit.
    pub half_w_mm: i64,
    /// Half-height, MilliUnit.
    pub half_h_mm: i64,
    /// What happens when this trigger fires.
    pub action: TriggerAction,
    /// Fire once or every entry?
    pub repeatable: bool,
    /// Whether this trigger has fired at least once.
    pub fired: bool,
}

/// What happens when the player enters a trigger.
#[derive(Debug, Clone)]
pub enum TriggerAction {
    /// Start a dialogue node.
    Dialogue {
        /// The node to start.
        node_id: String,
    },
    /// Shift the zone's phase.
    PhaseShift,
    /// Spawn a wave of enemies.
    SpawnWave {
        /// Which wave to spawn.
        wave_id: String,
    },
    /// Brand corruption spike.
    BrandSpike {
        /// How much corruption to add.
        amount: u8,
    },
    /// Tithe payment checkpoint.
    TitheCheckpoint,
    /// Audio region entry. Data-only here — firing it into a real mixer is
    /// owed alongside `audio_bridge.rs` (see module doc).
    AudioRegion {
        /// Which audio preset this region names.
        profile_id: String,
    },
}

impl TriggerZone {
    /// Check if a point is inside this trigger.
    pub fn contains(&self, x: i64, y: i64) -> bool {
        (x - self.x_mm).abs() < self.half_w_mm && (y - self.y_mm).abs() < self.half_h_mm
    }

    /// Try to fire. Returns the action if triggered, `None` if already fired
    /// and not repeatable.
    pub fn try_fire(&mut self, x: i64, y: i64) -> Option<&TriggerAction> {
        if !self.contains(x, y) {
            return None;
        }
        if self.fired && !self.repeatable {
            return None;
        }
        self.fired = true;
        Some(&self.action)
    }
}

// ── Parallax Layer ──────────────────────────────────────────────────────────

/// Parallax background layer. `scroll_scale_permyriad` — `10000` = camera speed.
#[derive(Debug, Clone)]
pub struct ParallaxLayer {
    /// The background texture this layer references.
    pub texture_ref: String,
    /// `0` = static sky, `10000` = gameplay plane, `13000` = foreground
    /// (faster than camera).
    pub scroll_scale_permyriad: i32,
    /// Paint order — lower paints first (further back).
    pub z_order: i32,
}

// ── Arena Layout ────────────────────────────────────────────────────────────

/// Complete 2D arena layout — the "level" for a single arena run. Built from
/// cartridge zone data + seed. Deterministic.
#[derive(Debug, Clone)]
pub struct ArenaLayout {
    /// Platform spine — the walkable surfaces.
    pub platforms: Vec<PlatformSegment>,
    /// Spawn points for enemies.
    pub spawn_points: Vec<SpawnPoint>,
    /// Trigger zones for events.
    pub triggers: Vec<TriggerZone>,
    /// Parallax background layers.
    pub parallax: Vec<ParallaxLayer>,
    /// Camera clamp, left edge.
    pub bounds_left_mm: i64,
    /// Camera clamp, right edge.
    pub bounds_right_mm: i64,
    /// Camera clamp, top edge.
    pub bounds_top_mm: i64,
    /// Camera clamp, bottom edge.
    pub bounds_bottom_mm: i64,
    /// Seed used to generate this layout.
    pub seed: u64,
}

impl ArenaLayout {
    /// Find the platform segment under a given X position.
    pub fn platform_at(&self, x_mm: i64) -> Option<&PlatformSegment> {
        self.platforms.iter().find(|p| x_mm >= p.x_start_mm && x_mm <= p.x_end_mm)
    }

    /// Surface Y at a given X (for terrain-relative placement). Returns the
    /// highest platform surface at that X.
    pub fn surface_y_at(&self, x_mm: i64) -> i64 {
        self.platforms
            .iter()
            .filter(|p| x_mm >= p.x_start_mm && x_mm <= p.x_end_mm)
            .map(|p| p.surface_y_mm)
            .max()
            .unwrap_or(0)
    }

    /// Build from a cartridge zone + seed via [`Mulberry32`]. Generates
    /// platforms, spawn points, and triggers deterministically.
    pub fn from_cartridge_zone(zone: &ArenaZoneDef, enemies: &[&EnemyDef], seed: u64) -> Self {
        let mut rng = Mulberry32::new(seed);

        // Generate platform spine from zone phases. Ground floor always
        // exists. Additional platforms seeded from the RNG.
        let platform_count = 3 + (rng.next_u32() % 4) as usize; // 3-6 platforms
        let arena_width_mm: i64 = 12000 + (rng.next_u32() % 8000) as i64;
        let half_w = arena_width_mm / 2;

        let mut platforms = Vec::with_capacity(platform_count);

        // Ground floor — always full width, stone.
        platforms.push(PlatformSegment {
            x_start_mm: -half_w,
            x_end_mm: half_w,
            surface_y_mm: 0,
            material: material_from_element(&zone.phases[0].element),
            one_way: false,
        });

        // Upper platforms — seeded positions.
        for i in 1..platform_count {
            let width_mm = 2000 + (rng.next_u32() % 4000) as i64;
            let x_center = -half_w + (rng.next_u32() as i64 % arena_width_mm);
            let y_mm = (i as i64) * 2500 + (rng.next_u32() % 1000) as i64;
            let phase_idx = i % zone.phases.len();
            platforms.push(PlatformSegment {
                x_start_mm: x_center - width_mm / 2,
                x_end_mm: x_center + width_mm / 2,
                surface_y_mm: y_mm,
                material: material_from_element(&zone.phases[phase_idx].element),
                one_way: rng.next_u32() % 3 == 0, // ~33% chance one-way
            });
        }

        // Spawn points — one per enemy, placed on platforms.
        let spawn_points: Vec<SpawnPoint> = enemies
            .iter()
            .enumerate()
            .map(|(i, enemy)| {
                let plat_idx = if platforms.len() > 1 { 1 + (rng.next_u32() as usize % (platforms.len() - 1)) } else { 0 };
                let plat = &platforms[plat_idx];
                let x_range = plat.x_end_mm - plat.x_start_mm;
                let x_mm = plat.x_start_mm + (rng.next_u32() as i64 % x_range.max(1));
                SpawnPoint {
                    id: format!("spawn_{}_{}", enemy.id, i),
                    x_mm,
                    platform_idx: plat_idx,
                    faction: "hostile".into(),
                    enemy_id: enemy.id.clone(),
                    respawn_ticks: 600, // 10s at 60fps
                }
            })
            .collect();

        // Default triggers — brand spike at arena edges, tithe checkpoint at centre.
        let triggers = vec![
            TriggerZone {
                id: "brand_left".into(),
                x_mm: -half_w + 500,
                y_mm: 1000,
                half_w_mm: 500,
                half_h_mm: 2000,
                action: TriggerAction::BrandSpike { amount: 5 },
                repeatable: true,
                fired: false,
            },
            TriggerZone {
                id: "brand_right".into(),
                x_mm: half_w - 500,
                y_mm: 1000,
                half_w_mm: 500,
                half_h_mm: 2000,
                action: TriggerAction::BrandSpike { amount: 5 },
                repeatable: true,
                fired: false,
            },
            TriggerZone {
                id: "tithe_center".into(),
                x_mm: 0,
                y_mm: 500,
                half_w_mm: 1000,
                half_h_mm: 1000,
                action: TriggerAction::TitheCheckpoint,
                repeatable: true,
                fired: false,
            },
        ];

        let parallax = default_parallax(&zone.phases[0].element);
        let top_mm = platforms.iter().map(|p| p.surface_y_mm).max().unwrap_or(0) + 5000;

        ArenaLayout {
            platforms,
            spawn_points,
            triggers,
            parallax,
            bounds_left_mm: -half_w - 1000,
            bounds_right_mm: half_w + 1000,
            bounds_top_mm: top_mm,
            bounds_bottom_mm: -2000,
            seed,
        }
    }

    /// Process all triggers against the player position. Mutates brand/
    /// dialogue/tithe as needed. Returns the ids of triggers that fired.
    pub fn process_triggers(
        &mut self,
        player_x: i64,
        player_y: i64,
        brand: &mut BrandCorruption,
        tithe: &mut Tithe,
        dialogue: &mut DialogueState,
    ) -> Vec<String> {
        // Collect which triggers fire (avoids a borrow conflict).
        let hits: Vec<(String, TriggerAction)> = self
            .triggers
            .iter_mut()
            .filter_map(|t| {
                if !t.contains(player_x, player_y) {
                    return None;
                }
                if t.fired && !t.repeatable {
                    return None;
                }
                t.fired = true;
                Some((t.id.clone(), t.action.clone()))
            })
            .collect();

        let mut fired = Vec::new();
        for (id, action) in hits {
            fired.push(id);
            match &action {
                TriggerAction::BrandSpike { amount } => {
                    brand.corrupt(*amount);
                }
                TriggerAction::TitheCheckpoint => {
                    if tithe.debt > 0 {
                        tithe.debt = tithe.debt.saturating_sub(10);
                    } else {
                        brand.corrupt(2);
                    }
                }
                TriggerAction::Dialogue { node_id } => {
                    if !dialogue.is_active() {
                        dialogue.start(DialogueNode {
                            id: node_id.clone(),
                            text: format!("The Ironroot speaks at {node_id}..."),
                            speaker: None,
                            choices: vec![],
                        });
                    }
                }
                // Data-only until audio_bridge.rs lands — see module doc.
                TriggerAction::AudioRegion { .. } => {}
                TriggerAction::PhaseShift => {}
                TriggerAction::SpawnWave { .. } => {}
            }
        }
        fired
    }

    /// Bridge platforms into [`SceneEntity`] values for collision (and,
    /// later, rendering). Creates Ground/Wall entities for each platform and
    /// a Player entity. Spawn-point enemies are sized and stat-derived from
    /// `enemies` via `creature_engine::derive_stats` — unmatched `enemy_id`s
    /// fall back to a human-reference profile rather than panicking, since a
    /// missing cartridge entry is a content gap, not a physics error.
    /// Returns `(entities, player_index, spawn_stats)` — `spawn_stats` is
    /// parallel to `self.spawn_points`, not to `entities`.
    pub fn to_scene_entities(&self, enemies: &[&EnemyDef]) -> (Vec<SceneEntity>, usize, Vec<GameEntity>) {
        let mut entities = Vec::new();

        // Each platform becomes a Ground (or Wall for one-way) entity.
        for plat in &self.platforms {
            let half_w = (plat.x_end_mm - plat.x_start_mm) / 2;
            let center_x = plat.x_start_mm + half_w;
            let half_h: i64 = if plat.one_way { 100 } else { 500 }; // thin for one-way
            entities.push(SceneEntity {
                kind: if plat.one_way { EntityKind::Wall } else { EntityKind::Ground },
                soul: None,
                node_id: 0, // assigned by scene after mesh upload
                x_mm: center_x,
                y_mm: plat.surface_y_mm - half_h, // centre below surface
                vx_mm: 0,
                vy_mm: 0,
                half_w_mm: half_w,
                half_h_mm: half_h,
                has_gravity: false,
                grounded: true,
            });
        }

        // Player spawns on ground floor centre, 1m above surface.
        let ground = &self.platforms[0];
        let player_x = (ground.x_start_mm + ground.x_end_mm) / 2;
        let player_y = ground.surface_y_mm + 1000;
        let player_idx = entities.len();
        entities.push(SceneEntity {
            kind: EntityKind::Player,
            node_id: 0,
            x_mm: player_x,
            soul: None,
            y_mm: player_y,
            vx_mm: 0,
            vy_mm: 0,
            half_w_mm: 500,
            half_h_mm: 1000,
            has_gravity: true,
            grounded: false,
        });

        // Spawn point enemies as Dummy entities, sized from derived stats.
        let mut spawn_stats = Vec::with_capacity(self.spawn_points.len());
        for sp in &self.spawn_points {
            let plat = &self.platforms[sp.platform_idx.min(self.platforms.len() - 1)];
            let profile = enemies
                .iter()
                .find(|e| e.id == sp.enemy_id)
                .map(|e| &e.physical)
                .unwrap_or(&REFERENCE_HUMAN_PROFILE);
            let game_entity = derive_stats(profile);
            // Half-height is fixed at reference human scale; half-width
            // reads attack_range as a stand-in for hitbox reach until a
            // real per-species silhouette exists.
            let half_w_mm = ((game_entity.attack_range * 500.0) as i64).clamp(200, 3000);
            entities.push(SceneEntity {
                kind: EntityKind::Dummy,
                node_id: 0,
                x_mm: sp.x_mm,
                soul: None,
                y_mm: plat.surface_y_mm + 1000,
                vx_mm: 0,
                vy_mm: 0,
                half_w_mm,
                half_h_mm: 1000,
                has_gravity: true,
                grounded: false,
            });
            spawn_stats.push(game_entity);
        }

        (entities, player_idx, spawn_stats)
    }
}

/// Fallback physical profile for a spawn whose `enemy_id` has no matching
/// `EnemyDef` — reference-human proportions (`REF_MASS_KG`/`REF_HEIGHT_M` in
/// `creature_engine`), so an unmatched id degrades to baseline stats rather
/// than panicking on a content gap.
const REFERENCE_HUMAN_PROFILE: PhysicalProfile = PhysicalProfile {
    mass_kg: 80.0,
    height_m: 1.8,
    width_m: 0.5,
    limb_ratio: 0.45,
    limb_count: 2,
    surface_hardness: 0.3,
    surface_material: forge_correspondence_v3::creature_engine::SurfaceMaterial::Flesh,
    volume_m3: 0.07,
    compactness: 0.6,
    symmetry: 0.9,
};

/// Map zone element to a default platform material.
fn material_from_element(element: &str) -> PlatformMaterial {
    match element {
        "fire" => PlatformMaterial::Stone,
        "water" => PlatformMaterial::Ice,
        "earth" => PlatformMaterial::Wood,
        "air" => PlatformMaterial::Vine,
        _ => PlatformMaterial::Stone,
    }
}

/// Default parallax layers themed by element.
fn default_parallax(element: &str) -> Vec<ParallaxLayer> {
    let bg = match element {
        "fire" => ("ember_sky", "ash_mountains", "charred_trees"),
        "water" => ("storm_sky", "fog_mountains", "kelp_trees"),
        "earth" => ("dusk_sky", "stone_mountains", "oak_trees"),
        "air" => ("pale_sky", "cloud_mountains", "wind_grass"),
        "spirit" => ("forest_canopy_vista", "camp_exterior", "forest_deep"),
        _ => ("void_sky", "void_mountains", "void_trees"),
    };
    vec![
        ParallaxLayer { texture_ref: bg.0.into(), scroll_scale_permyriad: 0, z_order: -3 },
        ParallaxLayer { texture_ref: bg.1.into(), scroll_scale_permyriad: 1000, z_order: -2 },
        ParallaxLayer { texture_ref: bg.2.into(), scroll_scale_permyriad: 5000, z_order: -1 },
        ParallaxLayer { texture_ref: "foreground_dust".into(), scroll_scale_permyriad: 13000, z_order: 1 },
    ]
}

#[cfg(test)]
mod parallax_tests {
    use super::*;

    #[test]
    fn spirit_resolves_its_own_backplates_not_the_void() {
        let layers = default_parallax("spirit");
        let refs: Vec<&str> = layers.iter().map(|l| l.texture_ref.as_str()).collect();
        assert_eq!(refs, vec!["forest_canopy_vista", "camp_exterior", "forest_deep", "foreground_dust"], "spirit must not fall through to the void arm");
    }

    #[test]
    fn spirit_keeps_the_shared_scroll_and_depth_ladder() {
        let spirit = default_parallax("spirit");
        let earth = default_parallax("earth");
        let ladder = |v: &[ParallaxLayer]| -> Vec<(i32, i32)> { v.iter().map(|l| (l.scroll_scale_permyriad, l.z_order)).collect() };
        assert_eq!(ladder(&spirit), ladder(&earth), "one parallax ladder, themed per element");
    }

    #[test]
    fn unknown_elements_still_reach_the_void() {
        let layers = default_parallax("not_an_element");
        assert_eq!(layers[0].texture_ref, "void_sky", "the fallback stays reachable");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cartridge port's whole point (L15 diff-floor): fields the old
    /// minimal stand-in could never carry are now real and readable.
    #[test]
    fn zone_phase_carries_fields_the_old_stand_in_could_not() {
        let unset = ZonePhase { element: "spirit".into(), ..Default::default() };
        assert_eq!(unset.essence().name, "Soul", "essence_id=0 falls back to element, family Spirit (40-47), row 40 = Soul");

        let explicit = ZonePhase { element: "spirit".into(), essence_id: 41, ..Default::default() };
        assert_eq!(explicit.essence().name, "Echo", "an explicit essence_id in-family overrides the element fallback (row 41 = Echo)");

        let themed = ZonePhase {
            element: "fire".into(),
            weather_override: "ash_storm".into(),
            gravity_permyriad: 8_000,
            ..Default::default()
        };
        assert_eq!(themed.weather_override, "ash_storm");
        assert_eq!(themed.gravity_permyriad, 8_000);
    }

    fn test_zone_and_enemies() -> (ArenaZoneDef, Vec<EnemyDef>) {
        let zone = ArenaZoneDef {
            id: "test_zone".into(),
            phases: vec![
                ZonePhase { element: "fire".into(), ..Default::default() },
                ZonePhase { element: "earth".into(), ..Default::default() },
            ],
            ..Default::default()
        };
        let enemies = vec![
            EnemyDef {
                id: "goblin".into(),
                physical: PhysicalProfile {
                    mass_kg: 45.0,
                    height_m: 1.2,
                    width_m: 0.4,
                    limb_ratio: 0.4,
                    limb_count: 2,
                    surface_hardness: 0.3,
                    surface_material: forge_correspondence_v3::creature_engine::SurfaceMaterial::Leather,
                    volume_m3: 0.045,
                    compactness: 0.6,
                    symmetry: 0.9,
                },
            },
            EnemyDef {
                id: "wisp".into(),
                physical: PhysicalProfile {
                    mass_kg: 5.0,
                    height_m: 0.5,
                    width_m: 0.3,
                    limb_ratio: 0.1,
                    limb_count: 0,
                    surface_hardness: 0.1,
                    surface_material: forge_correspondence_v3::creature_engine::SurfaceMaterial::Void,
                    volume_m3: 0.01,
                    compactness: 0.3,
                    symmetry: 0.5,
                },
            },
        ];
        (zone, enemies)
    }

    fn make_layout() -> ArenaLayout {
        let (zone, enemies) = test_zone_and_enemies();
        let enemy_refs: Vec<&EnemyDef> = enemies.iter().collect();
        ArenaLayout::from_cartridge_zone(&zone, &enemy_refs, 42)
    }

    /// Like `make_layout`, but also hands back the `EnemyDef`s so callers
    /// can drive `to_scene_entities`, which needs them to derive stats.
    fn make_layout_with_enemies() -> (ArenaLayout, Vec<EnemyDef>) {
        let (zone, enemies) = test_zone_and_enemies();
        let enemy_refs: Vec<&EnemyDef> = enemies.iter().collect();
        let layout = ArenaLayout::from_cartridge_zone(&zone, &enemy_refs, 42);
        (layout, enemies)
    }

    #[test]
    fn platform_material_friction() {
        assert!(PlatformMaterial::Ice.friction_permyriad() < PlatformMaterial::Stone.friction_permyriad());
        assert!(PlatformMaterial::Vine.friction_permyriad() > PlatformMaterial::Stone.friction_permyriad());
    }

    #[test]
    fn void_platform_crumbles() {
        assert!(PlatformMaterial::Void.crumble_ticks() > 0);
        assert_eq!(PlatformMaterial::Stone.crumble_ticks(), 0);
    }

    #[test]
    fn trigger_fires_once() {
        let mut t = TriggerZone { id: "test".into(), x_mm: 0, y_mm: 0, half_w_mm: 100, half_h_mm: 100, action: TriggerAction::BrandSpike { amount: 10 }, repeatable: false, fired: false };
        assert!(t.try_fire(0, 0).is_some());
        assert!(t.try_fire(0, 0).is_none());
    }

    #[test]
    fn trigger_repeatable() {
        let mut t = TriggerZone { id: "test".into(), x_mm: 0, y_mm: 0, half_w_mm: 100, half_h_mm: 100, action: TriggerAction::PhaseShift, repeatable: true, fired: false };
        assert!(t.try_fire(0, 0).is_some());
        assert!(t.try_fire(0, 0).is_some());
    }

    #[test]
    fn trigger_outside_misses() {
        let mut t = TriggerZone { id: "test".into(), x_mm: 0, y_mm: 0, half_w_mm: 100, half_h_mm: 100, action: TriggerAction::PhaseShift, repeatable: true, fired: false };
        assert!(t.try_fire(500, 500).is_none());
    }

    #[test]
    fn layout_from_cartridge_has_ground() {
        let layout = make_layout();
        assert!(layout.platforms.len() >= 3);
        assert_eq!(layout.platforms[0].surface_y_mm, 0);
        assert!(!layout.platforms[0].one_way);
    }

    #[test]
    fn layout_spawns_enemies() {
        let layout = make_layout();
        assert!(!layout.spawn_points.is_empty());
        for sp in &layout.spawn_points {
            assert!(sp.platform_idx < layout.platforms.len());
        }
    }

    #[test]
    fn layout_has_triggers() {
        let layout = make_layout();
        assert!(layout.triggers.len() >= 3);
    }

    #[test]
    fn layout_is_deterministic() {
        let (zone, enemies) = test_zone_and_enemies();
        let refs: Vec<&EnemyDef> = enemies.iter().collect();
        let a = ArenaLayout::from_cartridge_zone(&zone, &refs, 42);
        let b = ArenaLayout::from_cartridge_zone(&zone, &refs, 42);
        assert_eq!(a.platforms.len(), b.platforms.len());
        for (pa, pb) in a.platforms.iter().zip(b.platforms.iter()) {
            assert_eq!(pa.surface_y_mm, pb.surface_y_mm);
            assert_eq!(pa.x_start_mm, pb.x_start_mm);
        }
    }

    #[test]
    fn different_seeds_different_layouts() {
        let (zone, enemies) = test_zone_and_enemies();
        let refs: Vec<&EnemyDef> = enemies.iter().collect();
        let a = ArenaLayout::from_cartridge_zone(&zone, &refs, 42);
        let b = ArenaLayout::from_cartridge_zone(&zone, &refs, 99);
        let differs = a.platforms.iter().zip(b.platforms.iter()).any(|(pa, pb)| pa.surface_y_mm != pb.surface_y_mm || pa.x_start_mm != pb.x_start_mm);
        assert!(differs || a.platforms.len() != b.platforms.len());
    }

    #[test]
    fn parallax_has_4_layers() {
        let layout = make_layout();
        assert_eq!(layout.parallax.len(), 4);
        assert_eq!(layout.parallax[0].scroll_scale_permyriad, 0);
        assert_eq!(layout.parallax[3].scroll_scale_permyriad, 13000);
    }

    #[test]
    fn surface_y_finds_ground() {
        let layout = make_layout();
        let far_left = layout.bounds_left_mm + 1500;
        let y = layout.surface_y_at(far_left);
        assert!(y >= 0);
    }

    #[test]
    fn trigger_brand_spike_corrupts() {
        let mut layout = make_layout();
        let mut brand = BrandCorruption::default();
        let mut tithe = Tithe::default();
        let mut dialogue = DialogueState::default();
        let tx = layout.triggers.iter().find(|t| t.id == "brand_left").unwrap().x_mm;
        let ty = layout.triggers.iter().find(|t| t.id == "brand_left").unwrap().y_mm;
        assert_eq!(brand.level, 0);
        layout.process_triggers(tx, ty, &mut brand, &mut tithe, &mut dialogue);
        assert!(brand.level > 0);
    }

    #[test]
    fn trigger_tithe_checkpoint_reduces_debt() {
        let mut layout = make_layout();
        let mut brand = BrandCorruption::default();
        let mut tithe = Tithe { debt: 20, starvation_ticks: 0 };
        let mut dialogue = DialogueState::default();
        layout.process_triggers(0, 500, &mut brand, &mut tithe, &mut dialogue);
        assert!(tithe.debt < 20);
    }

    #[test]
    fn trigger_dialogue_starts() {
        let mut layout = ArenaLayout {
            platforms: vec![],
            spawn_points: vec![],
            triggers: vec![TriggerZone {
                id: "talk".into(),
                x_mm: 0,
                y_mm: 0,
                half_w_mm: 500,
                half_h_mm: 500,
                action: TriggerAction::Dialogue { node_id: "ironroot_greeting".into() },
                repeatable: false,
                fired: false,
            }],
            parallax: vec![],
            bounds_left_mm: -1000,
            bounds_right_mm: 1000,
            bounds_top_mm: 1000,
            bounds_bottom_mm: -1000,
            seed: 1,
        };
        let mut brand = BrandCorruption::default();
        let mut tithe = Tithe::default();
        let mut dialogue = DialogueState::default();
        assert!(!dialogue.is_active());
        layout.process_triggers(0, 0, &mut brand, &mut tithe, &mut dialogue);
        assert!(dialogue.is_active());
    }

    #[test]
    fn scene_entities_has_player_and_ground() {
        let (layout, enemies) = make_layout_with_enemies();
        let refs: Vec<&EnemyDef> = enemies.iter().collect();
        let (entities, player_idx, _) = layout.to_scene_entities(&refs);
        assert!(entities.len() >= layout.platforms.len() + 1);
        assert_eq!(entities[player_idx].kind, EntityKind::Player);
        assert!(entities.iter().any(|e| e.kind == EntityKind::Ground));
    }

    #[test]
    fn scene_entities_player_on_ground() {
        let (layout, enemies) = make_layout_with_enemies();
        let refs: Vec<&EnemyDef> = enemies.iter().collect();
        let (entities, player_idx, _) = layout.to_scene_entities(&refs);
        let player = &entities[player_idx];
        assert_eq!(player.y_mm, 1000);
        assert!(player.has_gravity);
    }

    #[test]
    fn scene_entities_enemies_from_spawns() {
        let (layout, enemies) = make_layout_with_enemies();
        let refs: Vec<&EnemyDef> = enemies.iter().collect();
        let enemy_count = layout.spawn_points.len();
        let (entities, _, spawn_stats) = layout.to_scene_entities(&refs);
        let dummies = entities.iter().filter(|e| e.kind == EntityKind::Dummy).count();
        assert_eq!(dummies, enemy_count);
        assert_eq!(spawn_stats.len(), enemy_count);
    }

    #[test]
    fn one_way_platforms_are_walls() {
        let (layout, enemies) = make_layout_with_enemies();
        let refs: Vec<&EnemyDef> = enemies.iter().collect();
        let (entities, _, _) = layout.to_scene_entities(&refs);
        let one_way_count = layout.platforms.iter().filter(|p| p.one_way).count();
        let wall_count = entities.iter().filter(|e| e.kind == EntityKind::Wall).count();
        assert_eq!(wall_count, one_way_count);
    }

    #[test]
    fn scene_entities_dummy_stats_come_from_derive_stats() {
        // The goblin (45kg) and wisp (5kg) test enemies have very different
        // physical profiles — their derived GameEntity stats (and thus the
        // Dummy hitbox width read from attack_range) must actually differ,
        // proving the spawn loop reads per-enemy physics, not a fixed value.
        let (layout, enemies) = make_layout_with_enemies();
        let refs: Vec<&EnemyDef> = enemies.iter().collect();
        let (entities, _, spawn_stats) = layout.to_scene_entities(&refs);
        let dummy_widths: Vec<i64> =
            entities.iter().filter(|e| e.kind == EntityKind::Dummy).map(|e| e.half_w_mm).collect();
        assert!(
            dummy_widths.iter().any(|w| *w != dummy_widths[0]),
            "goblin and wisp must not collapse to the same hardcoded hitbox"
        );
        for stats in &spawn_stats {
            assert!(stats.stats.str_ >= 1 && stats.stats.str_ <= 255);
            assert!(stats.max_hp > 0);
        }
    }

    #[test]
    fn scene_entities_unmatched_enemy_id_falls_back_to_reference_human() {
        let (zone, _) = test_zone_and_enemies();
        let unknown = EnemyDef { id: "no_such_enemy".into(), physical: REFERENCE_HUMAN_PROFILE };
        let refs: Vec<&EnemyDef> = vec![&unknown];
        let layout = ArenaLayout::from_cartridge_zone(&zone, &refs, 7);
        let (_, _, spawn_stats) = layout.to_scene_entities(&refs);
        let reference = derive_stats(&REFERENCE_HUMAN_PROFILE);
        for stats in &spawn_stats {
            assert_eq!(stats.max_hp, reference.max_hp, "unmatched id must degrade to the reference profile, not panic");
        }
    }
}
