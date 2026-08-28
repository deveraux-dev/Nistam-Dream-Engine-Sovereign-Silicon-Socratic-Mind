//! Baked-zone rows and their cart packing — the one home (L05) for the
//! `BakedZone` shape both shells read: studio-shell's world_builder bake
//! and studio-tauri's world-builder viewer. Zone-JSON parsing and the
//! engine feed stay shell-side (they carry serde_json/forge-core edges
//! this crate's serde/ron/blake3 footprint deliberately excludes).

/// One placement row as it lives inside a baked cart's `npe_cart` slot.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BakedPlacement {
    /// Free-text object kind, taken verbatim from source content
    /// (e.g. `"ground"`, `"stepping_stone"`, `"enemy_corrupted_wolf"`).
    /// Open vocabulary by design — a closed enum would either reject real
    /// rows or force a lossy remap.
    pub kind: String,
    /// Raw source-JSON grid units (already grid-aligned in source content —
    /// NOT permyriad/`MilliUnit` scale; real Ironroot content ranges 0-960
    /// in a 960x270 room, matching builder-grid cells 1:1).
    pub x: i64,
    /// See [`BakedPlacement::x`].
    pub y: i64,
    /// See [`BakedPlacement::x`].
    pub w: i64,
    /// See [`BakedPlacement::x`].
    pub h: i64,
}

/// One scene transition, straight from a zone JSON's `exits` object.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BakedExit {
    /// Which edge of the room this exit sits on (`"left"`/`"right"`/...).
    pub edge: String,
    /// The zone this exit leads to.
    pub target_zone: String,
    /// The room within that zone.
    pub target_room: i64,
    /// Arrival x in the target room, milli-units.
    pub x: i64,
    /// Arrival y in the target room, milli-units.
    pub y: i64,
}

/// A parsed Ironroot zone: everything `bake_ironroot_cart` needs, before
/// it's packed into a `CartBody`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BakedZone {
    /// Zone/room name, from the source `meta` block.
    pub name: String,
    /// Room height in the same raw grid units `BakedPlacement.y` uses —
    /// needed to flip Y-down source content (canvas convention) into the
    /// builder engine's Y-up convention (ground is low Y, sky is high Y).
    pub height: i64,
    /// Every platform/prop/spawn row, flattened to one vocabulary.
    pub placements: Vec<BakedPlacement>,
    /// Every authored exit.
    pub exits: Vec<BakedExit>,
}

/// Bake a parsed Ironroot zone into a [`crate::CartBody`] — through the
/// landed RON seal/load pipeline (`crate::seal`/`crate::load`), never a
/// second serialization scheme.
pub fn bake_ironroot_cart(zone: &BakedZone) -> Result<crate::CartBody, String> {
    let ron_str = ron::to_string(zone).map_err(|e| e.to_string())?;
    let value: ron::Value = ron::from_str(&ron_str).map_err(|e| e.to_string())?;
    Ok(crate::CartBody {
        npe_cart: Some(value),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone() -> BakedZone {
        BakedZone {
            name: String::from("Thorngate Creek"),
            height: 270,
            placements: vec![
                BakedPlacement { kind: String::from("ground"), x: 0, y: 254, w: 300, h: 16 },
                BakedPlacement { kind: String::from("enemy_corrupted_wolf"), x: 550, y: 240, w: 0, h: 0 },
            ],
            exits: vec![BakedExit {
                edge: String::from("left"),
                target_zone: String::from("thorngate_forest"),
                target_room: 3,
                x: 16,
                y: 240,
            }],
        }
    }

    #[test]
    fn baked_zone_round_trips_through_the_real_seal_load_pipeline() {
        let z = zone();
        let body = bake_ironroot_cart(&z).expect("bake must succeed");
        let sealed = crate::seal(&body).expect("seal must succeed");
        let loaded = crate::load(&sealed).expect("load must succeed");
        assert_eq!(loaded, body, "seal/load must be a bijection (L07)");
        let recovered: BakedZone = loaded
            .npe_cart
            .unwrap()
            .into_rust()
            .expect("npe_cart must deserialize back to BakedZone");
        assert_eq!(recovered, z, "the baked cart must carry the zone's rows, not a stub count");
    }
}
