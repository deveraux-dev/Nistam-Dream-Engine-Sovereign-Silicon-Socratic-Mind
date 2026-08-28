//! The zone runtime state — ported verbatim from
//! `F:\NewRepo\crates\forge-zones\src\{state,volume,marker,light}.rs`.
//!
//! **A deliberate f64 wall**, same precedent as `forge-pp-lore-v3`'s
//! "Wall-and-f64" ARCH000 ruling (2026-08-13,
//! `PPMATH-FLOAT-TRANCHE-PRECISION-CONTRACT-2026-08-13.md`): this module has
//! zero dependency on `forge-core-v3`, no `SimTick`/`MilliUnit`/`Permyriad`
//! in any signature. `ZoneState` is the AUTHORING-TIME compile target of a
//! `BlueprintDocument` (`blueprint.rs`, MilliUnit-typed, integer) — a
//! one-shot compile step, not replay-deterministic runtime state, so it
//! does not cross this workspace's determinism firewall. The MilliUnit ->
//! f64 conversion happens once, at the wall boundary, in `from_blueprint.rs`.
//!
//! **Scope cut (L15, named plainly):** ported only what `zone_from_blueprint`
//! and `render_svg` actually read — `ZoneState::{new, add_volume, add_marker,
//! add_light}` plus `Volume`/`Shape`/`Marker`/`LightSource`. v2's
//! `AudioSource`/`add_audio`, `DensityUsage`/`density_budget`, and the whole
//! `traversal` module (`validate_traversal`/`check_clearance`) are cut
//! entirely — neither ported function reads them.

use serde::{Deserialize, Serialize};
use serde_json::json;

/// Shape of a zone volume.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    /// An axis-aligned box.
    Box,
    /// A vertical cylinder.
    Cylinder,
    /// A sphere.
    Sphere,
}

/// A spatial volume placed in a zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    /// Display name.
    pub name: String,
    /// Box, cylinder, or sphere.
    pub shape: Shape,
    /// World X.
    pub x: f64,
    /// World Y (vertical).
    pub y: f64,
    /// World Z.
    pub z: f64,
    /// Box width (X extent).
    pub width: f64,
    /// Vertical extent.
    pub height: f64,
    /// Box depth (Z extent).
    pub depth: f64,
    /// Cylinder/sphere radius.
    pub radius: f64,
    /// Material id, drives render color and physics defaults.
    pub material: String,
    /// Whether this volume physically blocks movement.
    pub collision: bool,
    /// Whether this volume casts a shadow.
    pub cast_shadow: bool,
    /// Surface friction coefficient.
    pub physics_friction: f64,
    /// Surface bounce coefficient.
    pub physics_bounce: f64,
    /// Whether pathfinding treats this volume as an obstacle.
    pub nav_obstacle: bool,
    /// Free-form authoring notes.
    pub notes: String,
    /// Authoring district this volume belongs to (density budgeting).
    pub district: String,
}

impl Volume {
    /// Create a new volume with sensible defaults.
    pub fn new(name: impl Into<String>, shape: Shape, x: f64, y: f64, z: f64) -> Self {
        Self {
            name: name.into(),
            shape,
            x,
            y,
            z,
            width: 0.0,
            height: 0.0,
            depth: 0.0,
            radius: 0.0,
            material: "stone".into(),
            collision: true,
            cast_shadow: true,
            physics_friction: 0.7,
            physics_bounce: 0.05,
            nav_obstacle: false,
            notes: String::new(),
            district: String::new(),
        }
    }

    /// AABB overlap test for two box volumes (top-down, ignoring Y).
    pub fn boxes_overlap(a: &Volume, b: &Volume) -> bool {
        (a.x - b.x).abs() < (a.width + b.width) / 2.0 && (a.z - b.z).abs() < (a.depth + b.depth) / 2.0
    }
}

/// A logic marker placed in the zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    /// Display name.
    pub name: String,
    /// World X.
    pub x: f64,
    /// World Y (vertical).
    pub y: f64,
    /// World Z.
    pub z: f64,
    /// What kind of marker this is (e.g. `"spawn"`, `"narrative"`).
    pub entity_type: String,
    /// Free-form authoring metadata.
    pub metadata: serde_json::Value,
}

impl Marker {
    /// Create a new marker with empty metadata.
    pub fn new(name: impl Into<String>, x: f64, y: f64, z: f64, entity_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            x,
            y,
            z,
            entity_type: entity_type.into(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

/// A dynamic light source in the zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightSource {
    /// Display name.
    pub name: String,
    /// World X.
    pub x: f64,
    /// World Y (vertical).
    pub y: f64,
    /// World Z.
    pub z: f64,
    /// e.g. `"omni"`, `"spot"`.
    pub light_type: String,
    /// Hex colour string.
    pub color: String,
    /// Light intensity.
    pub energy: f64,
    /// Falloff range in meters.
    pub range_m: f64,
}

impl LightSource {
    /// Create a new light with sensible defaults (a warm torch-like glow).
    pub fn new(name: impl Into<String>, x: f64, y: f64, z: f64, light_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            x,
            y,
            z,
            light_type: light_type.into(),
            color: "#ff8833".into(),
            energy: 1.2,
            range_m: 8.0,
        }
    }
}

/// The zone builder state. Holds all placed elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneState {
    /// Display name.
    pub name: String,
    /// Zone extent along X.
    pub width: f64,
    /// Zone extent along Z.
    pub length: f64,
    /// Lowest legal Y for placed elements.
    pub y_min: f64,
    /// Highest legal Y for placed elements.
    pub y_max: f64,
    /// Authoring origin/atlas id.
    pub origin: String,
    /// Every placed volume.
    pub volumes: Vec<Volume>,
    /// Every placed logic marker.
    pub markers: Vec<Marker>,
    /// Every placed light.
    pub lights: Vec<LightSource>,
}

impl ZoneState {
    /// Initialize a new zone grid.
    pub fn new(
        name: impl Into<String>,
        width: f64,
        length: f64,
        y_min: f64,
        y_max: f64,
        origin: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            length,
            y_min,
            y_max,
            origin: origin.into(),
            volumes: Vec::new(),
            markers: Vec::new(),
            lights: Vec::new(),
        }
    }

    /// Add a volume to the zone. Checks bounds and box overlap.
    /// Returns a JSON status string (v2's own API shape, kept verbatim).
    pub fn add_volume(&mut self, vol: Volume) -> Result<String, String> {
        let half_w = self.width / 2.0;
        let half_l = self.length / 2.0;

        if vol.x.abs() > half_w + 5.0 || vol.z.abs() > half_l + 5.0 {
            return Ok(json!({
                "status": "WARNING",
                "message": format!("Volume '{}' is outside zone bounds", vol.name),
            })
            .to_string());
        }
        if vol.y < self.y_min - 2.0 || vol.y > self.y_max + 5.0 {
            return Ok(json!({
                "status": "WARNING",
                "message": format!("Volume '{}' Y={} is outside vertical range", vol.name, vol.y),
            })
            .to_string());
        }

        if vol.shape == Shape::Box {
            for v in &self.volumes {
                if v.shape == Shape::Box
                    && Volume::boxes_overlap(&vol, v)
                    && (vol.y - v.y).abs() < vol.height.max(v.height)
                {
                    let msg = json!({
                        "status": "WARNING",
                        "message": format!(
                            "Volume '{}' overlaps with '{}'. Proceeding but flagged.",
                            vol.name, v.name
                        ),
                    })
                    .to_string();
                    self.volumes.push(vol);
                    return Ok(msg);
                }
            }
        }

        let name = vol.name.clone();
        let position = format!("({}, {}, {})", vol.x, vol.y, vol.z);
        let size = if vol.shape == Shape::Box {
            format!("{}×{}×{}", vol.width, vol.height, vol.depth)
        } else {
            format!("r={} h={}", vol.radius, vol.height)
        };

        self.volumes.push(vol);

        Ok(json!({
            "status": "SUCCESS",
            "volume": name,
            "position": position,
            "size": size,
            "total_volumes": self.volumes.len(),
        })
        .to_string())
    }

    /// Place a logic marker in the zone.
    pub fn add_marker(&mut self, marker: Marker) -> String {
        let name = marker.name.clone();
        let entity_type = marker.entity_type.clone();
        let position = format!("({}, {}, {})", marker.x, marker.y, marker.z);

        self.markers.push(marker);

        json!({
            "status": "SUCCESS",
            "marker": name,
            "type": entity_type,
            "position": position,
            "total_markers": self.markers.len(),
        })
        .to_string()
    }

    /// Place a dynamic light. Rejects if >= 4 dynamic lights within 15m.
    pub fn add_light(&mut self, light: LightSource) -> Result<String, String> {
        let nearby_lights = self
            .lights
            .iter()
            .filter(|l| ((light.x - l.x).powi(2) + (light.z - l.z).powi(2)).sqrt() < 15.0)
            .count();

        if nearby_lights >= 4 {
            return Ok(json!({
                "status": "REJECTED",
                "message": format!(
                    "Too many dynamic lights near ({},{}). {} within 15m. Max 4.",
                    light.x, light.z, nearby_lights
                ),
            })
            .to_string());
        }

        let name = light.name.clone();
        let position = format!("({}, {}, {})", light.x, light.y, light.z);
        let color = light.color.clone();
        let energy = light.energy;
        let range = light.range_m;

        self.lights.push(light);

        Ok(json!({
            "status": "SUCCESS",
            "light": name,
            "position": position,
            "color": color,
            "energy": energy,
            "range": range,
            "nearby_count": nearby_lights + 1,
            "total_lights": self.lights.len(),
        })
        .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(name: &str, x: f64, z: f64, w: f64, d: f64) -> Volume {
        let mut v = Volume::new(name, Shape::Box, x, 0.0, z);
        v.width = w;
        v.depth = d;
        v
    }

    #[test]
    fn boxes_overlap_when_close() {
        let a = make_box("a", 0.0, 0.0, 4.0, 4.0);
        let b = make_box("b", 1.0, 1.0, 4.0, 4.0);
        assert!(Volume::boxes_overlap(&a, &b));
    }

    #[test]
    fn boxes_do_not_overlap_when_far() {
        let a = make_box("a", 0.0, 0.0, 4.0, 4.0);
        let b = make_box("b", 20.0, 20.0, 4.0, 4.0);
        assert!(!Volume::boxes_overlap(&a, &b));
    }
}
