//! Authorship bridge: `BlueprintDocument` -> `ZoneState` — ported verbatim
//! from `F:\NewRepo\crates\forge-zones\src\from_blueprint.rs`. Translates
//! the integer `MilliUnit` authoring substrate (`blueprint.rs`) into the f64
//! runtime zone substrate (`zone_state.rs`, a deliberate wall — see its own
//! doc comment). This is the ONE place `MilliUnit -> f64` conversion happens.
//!
//! ### Coordinate convention
//! Blueprint top-down `x -> world x`, `y -> world z`; world y is vertical.
//! Nodes are translated so the bounding-box center sits on world origin.

use crate::blueprint::{BlueprintDocument, BlueprintNode, NodeBounds, NodeId, NodeType};
use crate::zone_state::{LightSource, Marker, Shape, Volume, ZoneState};

/// Companion to `ZoneState` — carries blueprint->zone provenance, keyed by
/// `NodeId`.
#[derive(Debug, Clone, Default)]
pub struct ZoneLowerSidecar {
    /// `(NodeId, tag)` pairs — e.g. `"volume:room"`, `"marker:checkpoint"`,
    /// `"light:lighting_zone"`.
    pub provenance: Vec<(NodeId, String)>,
}

/// Translate a `BlueprintDocument` into a runtime `ZoneState` + sidecar.
///
/// Bounds-rejected volumes (`Ok("WARNING")` from `ZoneState::add_volume`) are
/// discarded silently in v0; the translator does not surface placement
/// errors, matching v2.
pub fn zone_from_blueprint(doc: &BlueprintDocument) -> (ZoneState, ZoneLowerSidecar) {
    let (center_x, center_z, width, length, y_min, y_max) = compute_zone_bounds(&doc.graph.nodes);

    let mut zone = ZoneState::new(doc.meta.name.clone(), width, length, y_min, y_max, doc.meta.zone.clone());
    let mut sidecar = ZoneLowerSidecar::default();

    for node in &doc.graph.nodes {
        let (cx_raw, cy, cz_raw) = node_center(&node.bounds);
        let cx = cx_raw - center_x;
        let cz = cz_raw - center_z;

        match node.node_type {
            NodeType::Checkpoint => {
                let m = Marker::new(node_name(node, "checkpoint"), cx, 0.0, cz, "spawn");
                zone.add_marker(m);
                sidecar.provenance.push((node.id, "marker:checkpoint".to_string()));
            }
            NodeType::NarrativeBeat => {
                let m = Marker::new(node_name(node, "beat"), cx, 0.0, cz, "narrative");
                zone.add_marker(m);
                sidecar.provenance.push((node.id, "marker:narrative".to_string()));
            }
            NodeType::TraversalGate => {
                let m = Marker::new(node_name(node, "gate"), cx, 0.0, cz, "gate");
                zone.add_marker(m);
                sidecar.provenance.push((node.id, "marker:gate".to_string()));
            }
            NodeType::Shrine => {
                let m = Marker::new(node_name(node, "shrine"), cx, 0.0, cz, "shrine");
                zone.add_marker(m);
                sidecar.provenance.push((node.id, "marker:shrine".to_string()));
            }
            NodeType::LightingZone => {
                let light_y = if cy > 0.0 { cy } else { 2.5 };
                let l = LightSource::new(node_name(node, "light"), cx, light_y, cz, "omni");
                let _ = zone.add_light(l);
                sidecar.provenance.push((node.id, "light:lighting_zone".to_string()));
            }
            NodeType::Room
            | NodeType::Platform
            | NodeType::Wall
            | NodeType::Arena
            | NodeType::EncounterZone
            | NodeType::BiomePatch
            | NodeType::Volume
            | NodeType::Corridor => {
                let (w, h, d) = node_dims(&node.bounds);
                let mut v = Volume::new(node_name(node, "vol"), Shape::Box, cx, cy, cz);
                v.width = w;
                v.height = h;
                v.depth = d;
                let _ = zone.add_volume(v);
                let tag = format!("volume:{}", node_type_tag(&node.node_type));
                sidecar.provenance.push((node.id, tag));
            }
            NodeType::Pit => {
                let (w, h, d) = node_dims(&node.bounds);
                let mut v = Volume::new(node_name(node, "vol"), Shape::Box, cx, cy, cz);
                v.width = w;
                v.height = h;
                v.depth = d;
                v.collision = false;
                v.nav_obstacle = true;
                let _ = zone.add_volume(v);
                let tag = format!("volume:{}", node_type_tag(&node.node_type));
                sidecar.provenance.push((node.id, tag));
            }
        }
    }

    (zone, sidecar)
}

fn node_name(node: &BlueprintNode, kind: &str) -> String {
    node.label.clone().unwrap_or_else(|| format!("{}_{}", kind, node.id.0))
}

/// MilliUnit i64 (1000 = 1 unit) -> f64. The one conversion at the wall.
fn mu(v: i64) -> f64 {
    v as f64 / 1000.0
}

fn node_center(b: &NodeBounds) -> (f64, f64, f64) {
    match b {
        NodeBounds::Rect(r) => (mu(r.x.0 + r.w.0 / 2), 0.0, mu(r.y.0 + r.h.0 / 2)),
        NodeBounds::Volume(v) => (
            mu(v.origin.x.0 + v.size.x.0 / 2),
            mu(v.origin.y.0 + v.size.y.0 / 2),
            mu(v.origin.z.0 + v.size.z.0 / 2),
        ),
    }
}

fn node_dims(b: &NodeBounds) -> (f64, f64, f64) {
    match b {
        // 2D nodes default to 3 m vertical extent (room height).
        NodeBounds::Rect(r) => (mu(r.w.0), 3.0, mu(r.h.0)),
        NodeBounds::Volume(v) => (mu(v.size.x.0), mu(v.size.y.0), mu(v.size.z.0)),
    }
}

/// Compute `(center_x, center_z, width, length, y_min, y_max)` for the
/// bounding box around all nodes.
fn compute_zone_bounds(nodes: &[BlueprintNode]) -> (f64, f64, f64, f64, f64, f64) {
    if nodes.is_empty() {
        return (0.0, 0.0, 32.0, 32.0, 0.0, 8.0);
    }

    let mut min_x = i64::MAX;
    let mut max_x = i64::MIN;
    let mut min_z = i64::MAX;
    let mut max_z = i64::MIN;
    let mut min_y = 0_i64;
    let mut max_y = 0_i64;

    for n in nodes {
        match &n.bounds {
            NodeBounds::Rect(r) => {
                min_x = min_x.min(r.x.0);
                max_x = max_x.max(r.x.0 + r.w.0);
                min_z = min_z.min(r.y.0);
                max_z = max_z.max(r.y.0 + r.h.0);
            }
            NodeBounds::Volume(v) => {
                min_x = min_x.min(v.origin.x.0);
                max_x = max_x.max(v.origin.x.0 + v.size.x.0);
                min_z = min_z.min(v.origin.z.0);
                max_z = max_z.max(v.origin.z.0 + v.size.z.0);
                min_y = min_y.min(v.origin.y.0);
                max_y = max_y.max(v.origin.y.0 + v.size.y.0);
            }
        }
    }

    let center_x = mu((min_x + max_x) / 2);
    let center_z = mu((min_z + max_z) / 2);

    let width = (mu(max_x - min_x) + 4.0).max(8.0);
    let length = (mu(max_z - min_z) + 4.0).max(8.0);
    let y_min = mu(min_y) - 2.0;
    let y_max = (mu(max_y) + 4.0).max(y_min + 6.0);

    (center_x, center_z, width, length, y_min, y_max)
}

fn node_type_tag(t: &NodeType) -> &'static str {
    match t {
        NodeType::Room => "room",
        NodeType::Platform => "platform",
        NodeType::Pit => "pit",
        NodeType::Wall => "wall",
        NodeType::Arena => "arena",
        NodeType::Checkpoint => "checkpoint",
        NodeType::Shrine => "shrine",
        NodeType::EncounterZone => "encounter_zone",
        NodeType::TraversalGate => "traversal_gate",
        NodeType::NarrativeBeat => "narrative_beat",
        NodeType::BiomePatch => "biome_patch",
        NodeType::LightingZone => "lighting_zone",
        NodeType::Volume => "volume",
        NodeType::Corridor => "corridor",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint::{
        AuthoringState, BlueprintGraph, BlueprintMeta, BlueprintMode, Rect2D,
    };

    fn doc_with_one_room() -> BlueprintDocument {
        BlueprintDocument {
            id: "test".into(),
            version: 1,
            mode: BlueprintMode::Mode2D,
            seed: 0,
            meta: BlueprintMeta {
                name: "Test Zone".into(),
                zone: "test_zone".into(),
                section_id: None,
                scene_id: None,
                room: None,
                act: None,
                biome: None,
                mood: None,
                tags: vec![],
                is_variant: false,
                variant_of: None,
                authoring_state: AuthoringState::Generated,
            },
            graph: BlueprintGraph {
                nodes: vec![BlueprintNode {
                    id: NodeId(1),
                    node_type: NodeType::Room,
                    label: Some("entry_hall".into()),
                    tags: vec![],
                    bounds: NodeBounds::Rect(Rect2D {
                        x: forge_core_v3::fixed_point::MilliUnit(0),
                        y: forge_core_v3::fixed_point::MilliUnit(0),
                        w: forge_core_v3::fixed_point::MilliUnit(4000),
                        h: forge_core_v3::fixed_point::MilliUnit(4000),
                    }),
                }],
                edges: vec![],
                layers: vec![],
            },
        }
    }

    /// [BOARD: WELD-workstream2] the compiler bridge round-trips a real
    /// BlueprintDocument into a ZoneState with the room's real dims (4m x 4m,
    /// MilliUnit(4000) -> 4.0), not a placeholder.
    #[test]
    fn a_room_node_becomes_a_box_volume_with_real_dims() {
        let doc = doc_with_one_room();
        let (zone, sidecar) = zone_from_blueprint(&doc);

        assert_eq!(zone.name, "Test Zone");
        assert_eq!(zone.volumes.len(), 1, "the room became exactly one volume");
        assert_eq!(zone.volumes[0].width, 4.0);
        assert_eq!(zone.volumes[0].depth, 4.0);
        assert_eq!(sidecar.provenance.len(), 1);
        assert_eq!(sidecar.provenance[0].1, "volume:room");
    }

    #[test]
    fn pit_nodes_are_non_colliding_nav_obstacles() {
        let mut doc = doc_with_one_room();
        doc.graph.nodes[0].node_type = NodeType::Pit;
        let (zone, _) = zone_from_blueprint(&doc);
        assert!(!zone.volumes[0].collision, "a pit has no collision");
        assert!(zone.volumes[0].nav_obstacle, "a pit blocks navigation");
    }

    #[test]
    fn empty_graph_falls_back_to_default_bounds() {
        let mut doc = doc_with_one_room();
        doc.graph.nodes.clear();
        let (zone, sidecar) = zone_from_blueprint(&doc);
        assert_eq!(zone.width, 32.0);
        assert_eq!(zone.length, 32.0);
        assert!(sidecar.provenance.is_empty());
    }
}
