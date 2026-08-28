//! TERRAFORMA MESH P0 receipt: open a Mesh, place two cells, render, erase
//! one, render, scrub back one step, render. Three faces from one ledger.
//! `cargo run -p forge-zones-v3 --example mesh_p0 -- <out-dir>`

use std::path::Path;

use forge_zones_v3::{render_html, MeshIntent, MeshLedger, Shape, Volume};

fn main() -> Result<(), String> {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let out = Path::new(&out);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let ledger_path = out.join("mesh.jsonl");
    let _ = std::fs::remove_file(&ledger_path);

    // Volume::new only places; extents default to zero, and a zero-extent cell
    // draws as an invisible rect. Size them or the face is honestly empty.
    let mut gatehouse = Volume::new("gatehouse", Shape::Box, -8.0, 1.0, 0.0);
    gatehouse.width = 10.0;
    gatehouse.height = 4.0;
    gatehouse.depth = 6.0;
    let mut tower = Volume::new("bell-tower", Shape::Cylinder, 10.0, 1.0, 4.0);
    tower.radius = 3.5;
    tower.height = 9.0;

    let intents = vec![
        MeshIntent::Open {
            name: "Toll Gate".into(),
            width: 64.0,
            length: 64.0,
            y_min: 0.0,
            y_max: 16.0,
            origin: "mesh-p0".into(),
        },
        MeshIntent::PlaceVolume(Box::new(gatehouse)),
        MeshIntent::PlaceVolume(Box::new(tower)),
        MeshIntent::Erase { name: "gatehouse".into() },
    ];
    for intent in &intents {
        MeshLedger::append_to_file(&ledger_path, intent)?;
    }

    let ledger = MeshLedger::load_file(&ledger_path)?;
    println!("ledger lines: {}", ledger.len());

    for (label, replay) in [
        ("full", ledger.replay()),
        ("scrub1", ledger.scrubbed_back(1)),
        ("scrub2", ledger.scrubbed_back(2)),
    ] {
        let html = render_html(&replay)?;
        let path = out.join(format!("mesh-{label}.html"));
        std::fs::write(&path, &html).map_err(|e| e.to_string())?;
        println!(
            "{label}: depth {} volumes {} refused {} -> {} ({} bytes)",
            replay.depth,
            replay.zone.volumes.len(),
            replay.refused.len(),
            path.display(),
            html.len()
        );
    }
    Ok(())
}
