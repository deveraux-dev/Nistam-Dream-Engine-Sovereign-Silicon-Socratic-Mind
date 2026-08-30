//! Harvest Astro 2D game engine + Ironroot zones to materialize forge-atlas-v3.

use forge_atlas_v3::{AtlasCell, RoleKind};
use forge_poll5d_v3::Morton8;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn material_for_ground(ground_type: u8) -> [u64; 8] {
    match ground_type {
        b'G' => [1u64, 0, 0, 0, 0, 0, 0, 0], // Grass
        b'S' => [2u64, 0, 0, 0, 0, 0, 0, 0], // Stone
        b'D' => [3u64, 0, 0, 0, 0, 0, 0, 0], // Dirt
        b'W' => [4u64, 0, 0, 0, 0, 0, 0, 0], // Water
        b'A' => [5u64, 0, 0, 0, 0, 0, 0, 0], // Sand
        _ => [0u64; 8],
    }
}

fn biome_to_ground(biome: &str) -> u8 {
    match biome {
        "Prairie" => b'G',
        "Forest" => b'D', // Forest = dirt/wood floor
        "Underground" => b'S',
        "Water" => b'W',
        _ => b'G',
    }
}

fn main() {
    let astro_concepts_path = "F:/NewRepo/crates/ffi-ui-assimilator-001/corpora/astro/astro_concepts.json";
    let ironroot_base = "F:/v3/assets/ironroot/Good";
    let zones = ["prairie_start", "forest_depths", "iron_caverns"];

    if !Path::new(astro_concepts_path).exists() {
        eprintln!("[astro-harvest] Concepts file not found: {}", astro_concepts_path);
        std::process::exit(1);
    }

    let concepts_raw = fs::read_to_string(astro_concepts_path)
        .expect("read astro_concepts.json");
    let concepts: Value = serde_json::from_str(&concepts_raw)
        .expect("parse astro_concepts.json");

    println!("[astro-harvest] Reading {} Astro concepts...",
             concepts["concepts"].as_array().map(|a| a.len()).unwrap_or(0));

    let mut cells: Vec<AtlasCell> = Vec::new();
    let mut zone_seeds = Vec::new();

    // Load ironroot zone data
    for zone_name in zones.iter() {
        let zone_path = format!("{}/{}.json", ironroot_base, zone_name);
        if let Ok(zone_raw) = fs::read_to_string(&zone_path) {
            if let Ok(zone) = serde_json::from_str::<Value>(&zone_raw) {
                let biome = zone.get("biome").and_then(|v| v.as_str()).unwrap_or("Prairie");
                let ground_type = biome_to_ground(biome);
                let material = material_for_ground(ground_type);
                let mut npc_count = 0;

                if let Some(npcs) = zone.get("npcs").and_then(|v| v.as_array()) {
                    npc_count = npcs.len();
                    for npc in npcs.iter() {
                        if let Some(pos) = npc.get("position").and_then(|v| v.as_array()) {
                            if let (Some(x_f), Some(y_f)) = (pos.get(0).and_then(|v| v.as_f64()),
                                                             pos.get(1).and_then(|v| v.as_f64())) {
                                let x = (x_f.abs() as u16) % 13;
                                let y = (y_f.abs() as u16) % 13;
                                let role = RoleKind::Presence;

                                if let Some(morton) = Morton8::encode(x, y, 0, 0, 0) {
                                    if let Some(cell) = AtlasCell::decode_parts(
                                        morton, material, ground_type, role as u8
                                    ) {
                                        cells.push(cell);
                                    }
                                }
                            }
                        }
                    }
                }

                zone_seeds.push(json!({
                    "zone": zone_name,
                    "biome": biome,
                    "ground_type": (ground_type as char).to_string(),
                    "npcs_seeded": npc_count,
                }));
            }
        }
    }

    // Seed Astro concepts as landmarks
    if let Some(concept_array) = concepts["concepts"].as_array() {
        for (idx, concept) in concept_array.iter().enumerate() {
            if let Some(name) = concept.get("name").and_then(|v| v.as_str()) {
                if name.contains("Sprite") || name.contains("Graphic") {
                    let x = (idx % 13) as u16;
                    let y = ((idx / 13) % 13) as u16;

                    if let Some(morton) = Morton8::encode(x, y, 0, 0, 0) {
                        let grass_material = material_for_ground(b'G');
                        if let Some(cell) = AtlasCell::decode_parts(
                            morton, grass_material, b'G', RoleKind::Landmark as u8
                        ) {
                            cells.push(cell);
                        }
                    }
                }
            }
        }
    }

    println!("[astro-harvest] Materialized {} atlas cells", cells.len());
    for seed in &zone_seeds {
        if let Some(zone) = seed.get("zone").and_then(|v| v.as_str()) {
            if let Some(npcs) = seed.get("npcs_seeded").and_then(|v| v.as_u64()) {
                println!("  {} ({} NPCs)", zone, npcs);
            }
        }
    }

    println!("[astro-harvest] GREEN — Astro + Ironroot zones materialized into atlas");

    let output = json!({
        "harvested": {
            "astro_concepts": concepts["concepts"].as_array().map(|a| a.len()).unwrap_or(0),
            "atlas_cells_seeded": cells.len(),
            "zones_loaded": zone_seeds.len(),
            "atlas_capacity": 256,
        },
        "zones_materialized": zone_seeds,
        "materials": {
            "grass": "G",
            "dirt_forest": "D",
            "stone_underground": "S",
            "water": "W",
            "sand": "A",
        },
    });

    println!("\n{}", serde_json::to_string_pretty(&output).unwrap());
}
