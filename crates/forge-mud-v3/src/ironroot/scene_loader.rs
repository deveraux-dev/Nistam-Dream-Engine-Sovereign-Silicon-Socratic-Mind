//! Scene entity physics — ported from
//! `F:\NewRepo\crates\ironroot\src\scene_loader.rs` (296 lines; triaged
//! 2026-08-13 as PORTABLE-WITH-WORK — floats there are all render/scene-
//! transform boundary). Expanded 2026-08-13 from the earlier partial port
//! (just `EntityKind`/`SceneEntity`) to the full tick loop: input → gravity
//! → move → AABB collide.
//!
//! **Scope cuts (L15, named plainly, not silent):**
//! - Renamed `LoadedScene` → [`PhysicsScene`]. The v2 name implied a
//!   `forge_render::scene::Scene` this crate doesn't have and can't spawn
//!   into (no mesh/vertex render pipeline exists in `v3` — same wall this
//!   session already hit with `world5d`'s `to_mesh()`). Every `Scene::*`
//!   call (`add_node`/`set_mesh`/`set_transform`/`update_world_transforms`)
//!   and the `glam::{Vec3,Quat}` it needed for that is cut along with it —
//!   `PhysicsScene` is entities + physics only, no render coupling.
//! - The combat hitbox system (`forge_game_systems::combat::hitbox::{Hitbox,
//!   Hurtbox}`, `SpatialGrid`, `combat_query`) is unported — a genuinely
//!   separate subsystem, not a quick pull-in. `tick()` here stops at
//!   collision; attack/hit-detection is owed, not stubbed.
//! - `load_manifest` (procedural architecture → mesh nodes) needs
//!   `forge_tile_crawler::architecture` (unported) AND a render `Scene`
//!   (doesn't exist) — cut entirely, not stubbed.
//!
//! What's ported, verbatim in spirit: the input→gravity→move→AABB-collide
//! tick, and `resolve_aabb`'s per-axis penetration resolution (the smaller-
//! overlap-axis-wins rule) — all pure integer `MilliUnit` arithmetic.

use forge_core_v3::soul::SoulId;

/// What kind of thing a scene entity is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    /// A walkable platform surface.
    Ground,
    /// The player.
    Player,
    /// A thin one-way platform (a wall from below, walkable from above).
    Wall,
    /// A non-player character.
    Npc,
    /// Shadow dummy — stands in place, takes hits, tracks the player's patterns.
    Dummy,
}

/// A spawned entity's physics state. Position is MilliUnit throughout — no
/// float anywhere in this type. `node_id` is carried for a future render
/// scene to key off of; nothing here reads or writes it.
#[derive(Debug, Clone)]
pub struct SceneEntity {
    /// What this entity is.
    pub kind: EntityKind,
    /// A future render scene's node id — unused by physics, carried for later.
    pub node_id: u32,
    /// Position, MilliUnit, X.
    pub x_mm: i64,
    /// Position, MilliUnit, Y.
    pub y_mm: i64,
    /// Velocity, MilliUnit per tick, X.
    pub vx_mm: i64,
    /// Velocity, MilliUnit per tick, Y.
    pub vy_mm: i64,
    /// Half-width, MilliUnit.
    pub half_w_mm: i64,
    /// Half-height, MilliUnit.
    pub half_h_mm: i64,
    /// Whether gravity acts on this entity.
    pub has_gravity: bool,
    /// Whether this entity currently rests on solid ground.
    pub grounded: bool,
    /// The animacy-gate handle [super::trit_grammar::TritReading::for_entity]
    /// reads. None = grammatically inanimate (a rock); Some = this
    /// entity has identity/lineage worth tracking, regardless of biology.
    pub soul: Option<SoulId>,
}

/// Gravity acceleration, MilliUnit per tick squared.
pub const GRAVITY_MM: i64 = 3;
/// Terminal fall speed, MilliUnit per tick.
pub const TERMINAL_VY: i64 = 200;

/// Input bit for "move left".
pub const INPUT_LEFT: u16 = 0x01;
/// Input bit for "move right".
pub const INPUT_RIGHT: u16 = 0x02;
/// Input bit for "jump".
pub const INPUT_JUMP: u16 = 0x10;

/// A set of physics entities and their live tick loop — the render-free
/// heart of `LoadedScene` (v2 name), now [`PhysicsScene`]: no scene graph,
/// no combat, just input→gravity→move→AABB-collide.
pub struct PhysicsScene {
    /// Every entity in the scene.
    pub entities: Vec<SceneEntity>,
    /// Index of the player entity.
    pub player_idx: usize,
    /// Index of the shadow dummy entity, if one was spawned.
    pub dummy_idx: Option<usize>,
}

impl PhysicsScene {
    /// A minimal test scene: ground + player + shadow dummy 3m to the right.
    pub fn spawn_test() -> Self {
        let mut entities = Vec::new();

        entities.push(SceneEntity {
            kind: EntityKind::Ground,
            node_id: 0,
            x_mm: 0,
            y_mm: 0,
            vx_mm: 0,
            vy_mm: 0,
            half_w_mm: 10_000,
            soul: None,
            half_h_mm: 500,
            has_gravity: false,
            grounded: true,
        });

        let player_idx = entities.len();
        entities.push(SceneEntity {
            kind: EntityKind::Player,
            soul: None,
            node_id: 0,
            x_mm: 0,
            y_mm: 2000,
            vx_mm: 0,
            vy_mm: 0,
            half_w_mm: 500,
            half_h_mm: 1000,
            has_gravity: true,
            grounded: false,
        });

        let dummy_idx = entities.len();
        entities.push(SceneEntity {
            kind: EntityKind::Dummy,
            soul: None,
            node_id: 0,
            x_mm: 3000,
            y_mm: 2000,
            vx_mm: 0,
            vy_mm: 0,
            half_w_mm: 500,
            half_h_mm: 1000,
            has_gravity: true,
            grounded: false,
        });

        PhysicsScene { entities, player_idx, dummy_idx: Some(dummy_idx) }
    }

    /// One tick: input → gravity → move → collide against Ground/Wall entities.
    pub fn tick(&mut self, input_bits: u16) {
        {
            let p = &mut self.entities[self.player_idx];
            let speed: i64 = 80;
            p.vx_mm = 0;
            if input_bits & INPUT_LEFT != 0 {
                p.vx_mm -= speed;
            }
            if input_bits & INPUT_RIGHT != 0 {
                p.vx_mm += speed;
            }
            if input_bits & INPUT_JUMP != 0 && p.grounded {
                p.vy_mm = 120;
                p.grounded = false;
            }
        }

        // Gravity.
        for e in &mut self.entities {
            if e.has_gravity && !e.grounded {
                e.vy_mm = (e.vy_mm - GRAVITY_MM).max(-TERMINAL_VY);
            }
        }

        // Move.
        for e in &mut self.entities {
            e.x_mm += e.vx_mm;
            e.y_mm += e.vy_mm;
        }

        // Collision vs ground/walls (collect solid data first to avoid a
        // borrow conflict against `self.entities`).
        let solids: Vec<(i64, i64, i64, i64)> = self
            .entities
            .iter()
            .filter(|e| e.kind == EntityKind::Ground || e.kind == EntityKind::Wall)
            .map(|e| (e.x_mm, e.y_mm, e.half_w_mm, e.half_h_mm))
            .collect();
        for idx in 0..self.entities.len() {
            if self.entities[idx].kind == EntityKind::Player || self.entities[idx].kind == EntityKind::Dummy {
                for &(gx, gy, ghw, ghh) in &solids {
                    let solid = SceneEntity {
                        kind: EntityKind::Ground,
                        node_id: 0,
                        x_mm: gx,
                        y_mm: gy,
                        soul: None,
                        vx_mm: 0,
                        vy_mm: 0,
                        half_w_mm: ghw,
                        half_h_mm: ghh,
                        has_gravity: false,
                        grounded: true,
                    };
                    resolve_aabb(&mut self.entities[idx], &solid);
                }
            }
        }
    }

    /// The player entity.
    pub fn player(&self) -> &SceneEntity {
        &self.entities[self.player_idx]
    }
}

/// Resolve an AABB overlap between `mover` and a static `solid` — the
/// smaller-overlap axis wins (a shallow vertical dip pops the mover up/down,
/// a shallow horizontal graze pushes it sideways), per-axis, so a diagonal
/// hit never resolves as a mix of both at once.
fn resolve_aabb(mover: &mut SceneEntity, solid: &SceneEntity) {
    if solid.kind != EntityKind::Ground && solid.kind != EntityKind::Wall {
        return;
    }
    let dx = (mover.x_mm - solid.x_mm).abs();
    let dy = (mover.y_mm - solid.y_mm).abs();
    let overlap_x = mover.half_w_mm + solid.half_w_mm - dx;
    let overlap_y = mover.half_h_mm + solid.half_h_mm - dy;
    if overlap_x <= 0 || overlap_y <= 0 {
        return;
    }
    if overlap_y < overlap_x {
        if mover.y_mm > solid.y_mm {
            mover.y_mm = solid.y_mm + solid.half_h_mm + mover.half_h_mm;
            if mover.vy_mm < 0 {
                mover.vy_mm = 0;
                mover.grounded = true;
            }
        } else {
            mover.y_mm = solid.y_mm - solid.half_h_mm - mover.half_h_mm;
            if mover.vy_mm > 0 {
                mover.vy_mm = 0;
            }
        }
    } else {
        if mover.x_mm > solid.x_mm {
            mover.x_mm = solid.x_mm + solid.half_w_mm + mover.half_w_mm;
        } else {
            mover.x_mm = solid.x_mm - solid.half_w_mm - mover.half_w_mm;
        }
        mover.vx_mm = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_falls_to_ground() {
        let mut scene = PhysicsScene::spawn_test();
        for _ in 0..200 {
            scene.tick(0);
        }
        let p = scene.player();
        assert!(p.grounded);
        assert_eq!(p.y_mm, 1500); // ground.half_h(500) + player.half_h(1000)
    }

    #[test]
    fn player_moves_left_right() {
        let mut scene = PhysicsScene::spawn_test();
        for _ in 0..200 {
            scene.tick(0);
        }
        let start_x = scene.player().x_mm;
        for _ in 0..10 {
            scene.tick(INPUT_RIGHT);
        }
        assert!(scene.player().x_mm > start_x);
    }

    #[test]
    fn player_jumps() {
        let mut scene = PhysicsScene::spawn_test();
        for _ in 0..200 {
            scene.tick(0);
        }
        assert!(scene.player().grounded);
        scene.tick(INPUT_JUMP);
        assert!(!scene.player().grounded);
        for _ in 0..200 {
            scene.tick(0);
        }
        assert!(scene.player().grounded);
    }

    #[test]
    fn dummy_lands_on_ground() {
        let mut scene = PhysicsScene::spawn_test();
        for _ in 0..200 {
            scene.tick(0);
        }
        let d = &scene.entities[scene.dummy_idx.unwrap()];
        assert!(d.grounded);
        assert_eq!(d.y_mm, 1500);
    }
}
