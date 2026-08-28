//! Blueprint validators — connectivity, traversal, encounter.
//! Ported from `TODO\quarry-sort\ARCHITECTS-2026-08-17\tile-crawler-architecture-newrepo\
//! blueprint_validators.rs` (v2 donor). `pp_math`→`pp_math_v3`,
//! `crate::architecture::blueprint::*`→`crate::blueprint::*`,
//! `crate::architecture::blueprint_validate::*`→`crate::blueprint_validate::*`.
//!
//! SCOPE CUT CLOSED 2026-08-25: the three GJK validators (`validate_room_overlap`/
//! `validate_spawn_safety`/`validate_door_contact`) are now ported. The old cut stood on
//! "no GJK narrow-phase in `forge-physics-v3`", which stopped being true when `gjk2d`
//! landed there 2026-08-24 (`forge-physics-v3/src/lib.rs:12-13`). No dependency grab was
//! needed and no behaviour was substituted: `gjk2d::gjk_intersects_2d` +
//! `support::ConvexPolygon2D::rect` are integer `MilliUnit` throughout.
//!
//! This is BETTER than the donor rather than verbatim, and deliberately so. The donor ran
//! its 2D test through the THREE-dimensional hull path — extruding each rect to a thin
//! z-hull via `ConvexHull::cube` with `glam::Vec3` and `as f32` casts (donor
//! blueprint_validators.rs:273-291, whose own comment admits "rects extrude to thin z-hulls
//! so box-box reduces to the 2D test"). v3 has a native integer 2D GJK, so the float
//! round-trip is dropped entirely.
//!
//! Bridge note: `ConvexPolygon2D::rect` takes raw `i64`, which is what lets this file span
//! the two `MilliUnit` types in the tree (`forge_core_v3::fixed_point:190` and
//! `pp_math_v3::fixed_point`) without converting between them or picking a winner.
//!
//! Each validator produces `ValidationIssue` entries that feed into `ValidationReport`.
//! All distance comparisons use `MilliUnit` (integer-only, no f32).

use forge_core_v3::fixed_point::MilliUnit;
use forge_physics_v3::gjk2d::gjk_intersects_2d;
use forge_physics_v3::support::ConvexPolygon2D;

use crate::blueprint::{BlueprintGraph, BlueprintNode, EdgeType, NodeBounds, NodeId, NodeType};
use crate::blueprint_validate::{IssueSeverity, ValidationIssue, ValidatorModule};

// ─── Graph Helpers ───────────────────────────────────────────────────
// Moved to forge-core-v3::zones::blueprint 2026-08-20 (E0116: BlueprintGraph
// is a foreign type here since it moved homes under L05 — an inherent impl
// must live where the type lives). nodes_of_type/node/neighbors/
// reachable_from/edges_of_type_from all live there now, re-exported below.

// ─── Connectivity Validator ──────────────────────────────────────────

/// Validate connectivity: exits exist, targets are valid, critical path reachable from spawn.
pub fn validate_connectivity(graph: &BlueprintGraph) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // Check: at least one spawn exists
    let spawns = graph.nodes_of_type(NodeType::Checkpoint);
    if spawns.is_empty() {
        issues.push(ValidationIssue {
            id: "CONN-001".into(),
            severity: IssueSeverity::Error,
            module: ValidatorModule::Connectivity,
            message: "No player spawn (Checkpoint node) found".into(),
            target_nodes: Vec::new(),
            constraint_id: Some("PlayerSpawnExists".into()),
            suggested_fix: Some("Add a Checkpoint node as player spawn".into()),
        });
        return issues; // Can't validate further without spawn
    }

    let spawn_id = spawns[0].id;

    // Check: at least one exit exists (TraversalGate nodes)
    let exits: Vec<&BlueprintNode> = graph.nodes.iter()
        .filter(|n| n.node_type == NodeType::TraversalGate)
        .collect();
    if exits.is_empty() {
        issues.push(ValidationIssue {
            id: "CONN-002".into(),
            severity: IssueSeverity::Error,
            module: ValidatorModule::Connectivity,
            message: "No exit (TraversalGate node) found".into(),
            target_nodes: Vec::new(),
            constraint_id: Some("ExitExists".into()),
            suggested_fix: Some("Add a TraversalGate node as level exit".into()),
        });
    }

    // Check: all rooms reachable from spawn
    let reachable = graph.reachable_from(spawn_id);
    for node in &graph.nodes {
        if node.node_type == NodeType::Room && !reachable.contains(&node.id) {
            issues.push(ValidationIssue {
                id: format!("CONN-003-{}", node.id.0),
                severity: IssueSeverity::Error,
                module: ValidatorModule::Connectivity,
                message: format!("Room {:?} not reachable from spawn", node.id),
                target_nodes: vec![node.id],
                constraint_id: Some("NoOrphanedRooms".into()),
                suggested_fix: Some("Connect this room to the main graph".into()),
            });
        }
    }

    // Check: critical path exists (spawn can reach at least one exit)
    if !exits.is_empty() {
        let exit_reachable = exits.iter().any(|e| reachable.contains(&e.id));
        if !exit_reachable {
            issues.push(ValidationIssue {
                id: "CONN-004".into(),
                severity: IssueSeverity::Error,
                module: ValidatorModule::Connectivity,
                message: "No critical path from spawn to any exit".into(),
                target_nodes: vec![spawn_id],
                constraint_id: Some("CriticalPathReachable".into()),
                suggested_fix: Some("Ensure edges connect spawn to at least one TraversalGate".into()),
            });
        }
    }

    issues
}

// ─── Traversal Validator ─────────────────────────────────────────────

/// Validate traversal: jump distances within limits, climb access valid.
pub fn validate_traversal(graph: &BlueprintGraph, max_jump: MilliUnit) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for edge in &graph.edges {
        if edge.edge_type == EdgeType::RequiresJump {
            // Calculate distance between connected nodes
            if let (Some(from_node), Some(to_node)) = (graph.node(edge.from), graph.node(edge.to)) {
                let dist = node_distance(from_node, to_node);
                if dist > max_jump {
                    issues.push(ValidationIssue {
                        id: format!("TRAV-001-{}", edge.id.0),
                        severity: IssueSeverity::Error,
                        module: ValidatorModule::Traversal,
                        message: format!(
                            "Jump from {:?} to {:?} requires {} but max is {}",
                            edge.from, edge.to, dist.0, max_jump.0
                        ),
                        target_nodes: vec![edge.from, edge.to],
                        constraint_id: Some("MaxJumpDistance".into()),
                        suggested_fix: Some("Reduce gap or add intermediate platform".into()),
                    });
                }
            }
        }
    }

    // Check: RequiresClimb edges must connect to nodes with vertical offset
    for edge in &graph.edges {
        if edge.edge_type == EdgeType::RequiresClimb {
            if let (Some(from_node), Some(to_node)) = (graph.node(edge.from), graph.node(edge.to)) {
                let vert = vertical_offset(from_node, to_node);
                if vert.0 <= 0 {
                    issues.push(ValidationIssue {
                        id: format!("TRAV-002-{}", edge.id.0),
                        severity: IssueSeverity::Warning,
                        module: ValidatorModule::Traversal,
                        message: format!("Climb edge {:?}→{:?} has no vertical offset", edge.from, edge.to),
                        target_nodes: vec![edge.from, edge.to],
                        constraint_id: None,
                        suggested_fix: Some("Verify climb direction or change edge type".into()),
                    });
                }
            }
        }
    }

    issues
}

// ─── Encounter Validator ─────────────────────────────────────────────

/// Validate encounters: spawns in bounds, boss arenas lockable.
pub fn validate_encounter(graph: &BlueprintGraph) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // Check: EncounterZone nodes must have at least one edge (patrol or adjacent)
    let encounter_zones = graph.nodes_of_type(NodeType::EncounterZone);
    for zone in &encounter_zones {
        let connections = graph.neighbors(zone.id);
        if connections.is_empty() {
            issues.push(ValidationIssue {
                id: format!("ENC-001-{}", zone.id.0),
                severity: IssueSeverity::Warning,
                module: ValidatorModule::Encounter,
                message: format!("EncounterZone {:?} has no connections (isolated)", zone.id),
                target_nodes: vec![zone.id],
                constraint_id: Some("EnemySpawnsInBounds".into()),
                suggested_fix: Some("Connect encounter zone to playable space".into()),
            });
        }
    }

    // Check: Arena nodes must have BossLock edges on all exits
    let arenas = graph.nodes_of_type(NodeType::Arena);
    for arena in &arenas {
        let all_exits: Vec<NodeId> = graph.edges.iter()
            .filter(|e| e.from == arena.id || e.to == arena.id)
            .filter(|e| e.edge_type != EdgeType::BossLock && e.edge_type != EdgeType::PatrolPath)
            .map(|e| if e.from == arena.id { e.to } else { e.from })
            .collect();

        let locked_exits: Vec<NodeId> = graph.edges.iter()
            .filter(|e| (e.from == arena.id || e.to == arena.id) && e.edge_type == EdgeType::BossLock)
            .map(|e| if e.from == arena.id { e.to } else { e.from })
            .collect();

        for exit in &all_exits {
            if !locked_exits.contains(exit) {
                issues.push(ValidationIssue {
                    id: format!("ENC-002-{}-{}", arena.id.0, exit.0),
                    severity: IssueSeverity::Error,
                    module: ValidatorModule::Encounter,
                    message: format!("Arena {:?} exit to {:?} not locked during boss fight", arena.id, exit),
                    target_nodes: vec![arena.id, *exit],
                    constraint_id: Some("BossArenaLockable".into()),
                    suggested_fix: Some("Add BossLock edge to this exit".into()),
                });
            }
        }
    }

    issues
}

// ─── Geometry Helpers (integer-only) ─────────────────────────────────

/// Manhattan distance between two nodes based on their bounds center.
fn node_distance(a: &BlueprintNode, b: &BlueprintNode) -> MilliUnit {
    let (ax, ay) = node_center(a);
    let (bx, by) = node_center(b);
    let dx = (ax.0 - bx.0).unsigned_abs() as i64;
    let dy = (ay.0 - by.0).unsigned_abs() as i64;
    MilliUnit(dx + dy)
}

/// Vertical offset between two nodes (positive = b is above a).
fn vertical_offset(a: &BlueprintNode, b: &BlueprintNode) -> MilliUnit {
    let (_, ay) = node_center(a);
    let (_, by) = node_center(b);
    MilliUnit(by.0 - ay.0)
}

/// Get center position of a node from its bounds.
fn node_center(node: &BlueprintNode) -> (MilliUnit, MilliUnit) {
    match node.bounds {
        NodeBounds::Rect(r) => (
            MilliUnit(r.x.0 + r.w.0 / 2),
            MilliUnit(r.y.0 + r.h.0 / 2),
        ),
        NodeBounds::Volume(b) => (
            MilliUnit(b.origin.x.0 + b.size.x.0 / 2),
            MilliUnit(b.origin.y.0 + b.size.y.0 / 2),
        ),
    }
}

// ─── GJK Bridge ──────────────────────────────────────────────────────
// Integer narrow-phase over forge-physics-v3's native 2D GJK. Cold path
// (bake/editor), so the allocation per polygon is not a hot-path concern.

/// A node's footprint as an axis-aligned convex polygon, centre + half-extents.
/// `Volume` bounds are read on their x/y face — the blueprint graph is planned
/// in plan view, exactly as the donor treated them.
fn node_to_polygon(node: &BlueprintNode) -> ConvexPolygon2D {
    match node.bounds {
        NodeBounds::Rect(r) => {
            ConvexPolygon2D::rect(r.x.0 + r.w.0 / 2, r.y.0 + r.h.0 / 2, r.w.0 / 2, r.h.0 / 2)
        }
        NodeBounds::Volume(b) => ConvexPolygon2D::rect(
            b.origin.x.0 + b.size.x.0 / 2,
            b.origin.y.0 + b.size.y.0 / 2,
            b.size.x.0 / 2,
            b.size.y.0 / 2,
        ),
    }
}

fn footprints_touch(a: &BlueprintNode, b: &BlueprintNode) -> bool {
    gjk_intersects_2d(&node_to_polygon(a), &node_to_polygon(b))
}

// ─── Room Overlap Validator (GJK) ────────────────────────────────────

/// Validate that no two Room nodes overlap.
pub fn validate_room_overlap(graph: &BlueprintGraph) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let rooms = graph.nodes_of_type(NodeType::Room);

    for i in 0..rooms.len() {
        for j in (i + 1)..rooms.len() {
            if footprints_touch(rooms[i], rooms[j]) {
                issues.push(ValidationIssue {
                    id: format!("ROOM-001-{}-{}", rooms[i].id.0, rooms[j].id.0),
                    severity: IssueSeverity::Error,
                    module: ValidatorModule::Construction,
                    message: format!("Room {:?} overlaps Room {:?}", rooms[i].id, rooms[j].id),
                    target_nodes: vec![rooms[i].id, rooms[j].id],
                    constraint_id: Some("NoOverlappingBlockers".into()),
                    suggested_fix: Some("Move rooms apart or merge into one".into()),
                });
            }
        }
    }
    issues
}

// ─── Spawn Safety Validator (GJK) ────────────────────────────────────

/// Validate that spawn (Checkpoint) nodes do not overlap hazard (Pit) nodes.
pub fn validate_spawn_safety(graph: &BlueprintGraph) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let spawns = graph.nodes_of_type(NodeType::Checkpoint);
    let hazards = graph.nodes_of_type(NodeType::Pit);

    for spawn in &spawns {
        for hazard in &hazards {
            if footprints_touch(spawn, hazard) {
                issues.push(ValidationIssue {
                    id: format!("SPAWN-001-{}-{}", spawn.id.0, hazard.id.0),
                    severity: IssueSeverity::Error,
                    module: ValidatorModule::Construction,
                    message: format!("Spawn {:?} overlaps hazard {:?}", spawn.id, hazard.id),
                    target_nodes: vec![spawn.id, hazard.id],
                    constraint_id: Some("SpawnNotInHazard".into()),
                    suggested_fix: Some("Move spawn away from hazard area".into()),
                });
            }
        }
    }
    issues
}

// ─── Door Contact Validator ──────────────────────────────────────────

/// Validate that TraversalGate nodes physically touch at least one node they
/// connect to. A door floating in empty space is invalid.
pub fn validate_door_contact(graph: &BlueprintGraph) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for door in graph.nodes_of_type(NodeType::TraversalGate) {
        let neighbors = graph.neighbors(door.id);
        if neighbors.is_empty() {
            continue;
        }
        let touches_any = neighbors
            .iter()
            .filter_map(|&nid| graph.node(nid))
            .any(|neighbor| footprints_touch(door, neighbor));

        if !touches_any {
            issues.push(ValidationIssue {
                id: format!("DOOR-001-{}", door.id.0),
                severity: IssueSeverity::Warning,
                module: ValidatorModule::Construction,
                message: format!(
                    "Door {:?} does not physically contact any adjacent room",
                    door.id
                ),
                target_nodes: vec![door.id],
                constraint_id: None,
                suggested_fix: Some("Move door to touch room boundary".into()),
            });
        }
    }
    issues
}

// ─── Pipeline Integration ────────────────────────────────────────────

/// Run all six blueprint validators and collect issues.
pub fn validate_blueprint(graph: &BlueprintGraph, max_jump: MilliUnit) -> Vec<ValidationIssue> {
    let mut all = Vec::new();
    all.extend(validate_connectivity(graph));
    all.extend(validate_traversal(graph, max_jump));
    all.extend(validate_encounter(graph));
    all.extend(validate_room_overlap(graph));
    all.extend(validate_spawn_safety(graph));
    all.extend(validate_door_contact(graph));
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint::*;

    fn make_node(id: u32, nt: NodeType, x: i64, y: i64, w: i64, h: i64) -> BlueprintNode {
        BlueprintNode {
            id: NodeId(id),
            node_type: nt,
            label: None,
            tags: Vec::new(),
            bounds: NodeBounds::Rect(Rect2D {
                x: MilliUnit(x), y: MilliUnit(y),
                w: MilliUnit(w), h: MilliUnit(h),
            }),
        }
    }

    fn make_edge(id: u32, from: u32, to: u32, et: EdgeType) -> BlueprintEdge {
        BlueprintEdge { id: EdgeId(id), from: NodeId(from), to: NodeId(to), edge_type: et }
    }

    // ── The three GJK validators (scope cut closed 2026-08-25) ──────────

    #[test]
    fn overlapping_rooms_are_an_error() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Room, 0, 0, 100, 100),
                make_node(1, NodeType::Room, 50, 50, 100, 100),
            ],
            edges: vec![],
            layers: vec![],
        };
        let issues = validate_room_overlap(&graph);
        assert_eq!(issues.len(), 1, "one overlapping pair, one issue");
        assert_eq!(issues[0].id, "ROOM-001-0-1");
        assert_eq!(issues[0].severity, IssueSeverity::Error);
    }

    #[test]
    fn rooms_set_apart_raise_nothing() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Room, 0, 0, 100, 100),
                make_node(1, NodeType::Room, 900, 900, 100, 100),
            ],
            edges: vec![],
            layers: vec![],
        };
        assert!(validate_room_overlap(&graph).is_empty());
    }

    #[test]
    fn a_spawn_sitting_in_a_pit_is_an_error() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Checkpoint, 0, 0, 100, 100),
                make_node(1, NodeType::Pit, 20, 20, 100, 100),
                make_node(2, NodeType::Pit, 800, 800, 100, 100),
            ],
            edges: vec![],
            layers: vec![],
        };
        let issues = validate_spawn_safety(&graph);
        assert_eq!(issues.len(), 1, "only the pit under the spawn counts");
        assert_eq!(issues[0].id, "SPAWN-001-0-1");
        assert_eq!(issues[0].constraint_id.as_deref(), Some("SpawnNotInHazard"));
    }

    #[test]
    fn a_door_floating_clear_of_its_rooms_warns() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Room, 0, 0, 100, 100),
                make_node(1, NodeType::TraversalGate, 5_000, 5_000, 10, 10),
            ],
            edges: vec![make_edge(0, 0, 1, EdgeType::Connects)],
            layers: vec![],
        };
        let issues = validate_door_contact(&graph);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "DOOR-001-1");
        assert_eq!(issues[0].severity, IssueSeverity::Warning, "a floating door is a warning, not an error");
    }

    #[test]
    fn a_door_touching_its_room_is_clean() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Room, 0, 0, 100, 100),
                make_node(1, NodeType::TraversalGate, 90, 40, 20, 20),
            ],
            edges: vec![make_edge(0, 0, 1, EdgeType::Connects)],
            layers: vec![],
        };
        assert!(validate_door_contact(&graph).is_empty());
    }

    /// A door with no edges at all is the connectivity validator's problem,
    /// not this one's — it must not double-report.
    #[test]
    fn a_door_with_no_edges_is_left_to_the_connectivity_validator() {
        let graph = BlueprintGraph {
            nodes: vec![make_node(1, NodeType::TraversalGate, 5_000, 5_000, 10, 10)],
            edges: vec![],
            layers: vec![],
        };
        assert!(validate_door_contact(&graph).is_empty());
    }

    #[test]
    fn the_pipeline_now_runs_all_six_validators() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Room, 0, 0, 100, 100),
                make_node(1, NodeType::Room, 50, 50, 100, 100),
            ],
            edges: vec![],
            layers: vec![],
        };
        let all = validate_blueprint(&graph, MilliUnit(1_000));
        assert!(
            all.iter().any(|i| i.id.starts_with("ROOM-001")),
            "the room-overlap validator must be reachable through the pipeline, not just directly"
        );
    }

    #[test]
    fn connectivity_no_spawn_errors() {
        let graph = BlueprintGraph { nodes: vec![], edges: vec![], layers: vec![] };
        let issues = validate_connectivity(&graph);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "CONN-001");
    }

    #[test]
    fn connectivity_no_exit_errors() {
        let graph = BlueprintGraph {
            nodes: vec![make_node(0, NodeType::Checkpoint, 0, 0, 100, 100)],
            edges: vec![],
            layers: vec![],
        };
        let issues = validate_connectivity(&graph);
        assert!(issues.iter().any(|i| i.id == "CONN-002"));
    }

    #[test]
    fn connectivity_valid_path() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Checkpoint, 0, 0, 100, 100),
                make_node(1, NodeType::Room, 200, 0, 100, 100),
                make_node(2, NodeType::TraversalGate, 400, 0, 100, 100),
            ],
            edges: vec![
                make_edge(0, 0, 1, EdgeType::Adjacent),
                make_edge(1, 1, 2, EdgeType::Connects),
            ],
            layers: vec![],
        };
        let issues = validate_connectivity(&graph);
        assert!(issues.is_empty());
    }

    #[test]
    fn connectivity_orphaned_room() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Checkpoint, 0, 0, 100, 100),
                make_node(1, NodeType::TraversalGate, 200, 0, 100, 100),
                make_node(2, NodeType::Room, 500, 500, 100, 100), // orphaned
            ],
            edges: vec![
                make_edge(0, 0, 1, EdgeType::Adjacent),
            ],
            layers: vec![],
        };
        let issues = validate_connectivity(&graph);
        assert!(issues.iter().any(|i| i.id == "CONN-003-2"));
    }

    #[test]
    fn traversal_jump_too_far() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Platform, 0, 0, 100, 100),
                make_node(1, NodeType::Platform, 10000, 0, 100, 100),
            ],
            edges: vec![
                make_edge(0, 0, 1, EdgeType::RequiresJump),
            ],
            layers: vec![],
        };
        let issues = validate_traversal(&graph, MilliUnit(5000));
        assert_eq!(issues.len(), 1);
        assert!(issues[0].id.starts_with("TRAV-001"));
    }

    #[test]
    fn traversal_jump_within_range() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Platform, 0, 0, 100, 100),
                make_node(1, NodeType::Platform, 2000, 0, 100, 100),
            ],
            edges: vec![
                make_edge(0, 0, 1, EdgeType::RequiresJump),
            ],
            layers: vec![],
        };
        let issues = validate_traversal(&graph, MilliUnit(5000));
        assert!(issues.is_empty());
    }

    #[test]
    fn encounter_arena_unlocked_exit() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Arena, 0, 0, 500, 500),
                make_node(1, NodeType::Room, 600, 0, 100, 100),
            ],
            edges: vec![
                make_edge(0, 0, 1, EdgeType::Adjacent), // not locked!
            ],
            layers: vec![],
        };
        let issues = validate_encounter(&graph);
        assert!(issues.iter().any(|i| i.id.starts_with("ENC-002")));
    }

    #[test]
    fn encounter_arena_locked_exit_passes() {
        let graph = BlueprintGraph {
            nodes: vec![
                make_node(0, NodeType::Arena, 0, 0, 500, 500),
                make_node(1, NodeType::Room, 600, 0, 100, 100),
            ],
            edges: vec![
                make_edge(0, 0, 1, EdgeType::BossLock),
            ],
            layers: vec![],
        };
        let issues = validate_encounter(&graph);
        // BossLock edges are excluded from "all_exits" check, so no issue
        assert!(issues.iter().all(|i| !i.id.starts_with("ENC-002")));
    }
}
