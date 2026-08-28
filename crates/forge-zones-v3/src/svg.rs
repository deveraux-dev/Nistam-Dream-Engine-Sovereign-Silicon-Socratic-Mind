//! Zone SVG compilation — ported verbatim from
//! `F:\NewRepo\crates\forge-zones\src\compiler.rs` (`render_svg` only; the
//! file's other scene/ECS compilation helpers are unread and uncut, not
//! ported here — a real, named gap, not a silent one).

use std::path::Path;

use serde_json::json;

use crate::zone_state::{Shape, ZoneState};

/// Render a top-down SVG blueprint of the zone. Matches v2's output exactly
/// (viewBox, CSS classes, dark background).
pub fn render_svg(zone: &ZoneState, output_path: Option<&str>) -> Result<String, String> {
    if zone.name.is_empty() {
        return Ok(json!({
            "status": "ERROR",
            "message": "No zone initialized",
        })
        .to_string());
    }

    let default_path = format!("F:/output/blueprints/{}_top_down.svg", zone.name.to_lowercase());
    let path = output_path.unwrap_or(&default_path);
    let svg_content = svg_markup(zone)?;

    let out_path = Path::new(path);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create output directory: {e}"))?;
    }
    std::fs::write(out_path, &svg_content).map_err(|e| format!("Failed to write SVG: {e}"))?;

    Ok(json!({
        "status": "SUCCESS",
        "path": path,
        "view": "top-down",
        "volumes_rendered": zone.volumes.len(),
    })
    .to_string())
}

/// The same top-down markup `render_svg` writes, returned instead of written.
/// One generator, two deliveries: `render_svg` keeps its v2 write-and-report
/// contract, faces that need the picture inline call this.
pub fn svg_markup(zone: &ZoneState) -> Result<String, String> {
    if zone.name.is_empty() {
        return Err("No zone initialized".into());
    }

    let half_w = zone.width / 2.0 + 10.0;
    let half_l = zone.length / 2.0 + 10.0;

    let mut svg_lines = Vec::new();

    svg_lines.push(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" \
         width=\"1200\" height=\"1200\" style=\"background:#1a1510\">",
        -half_w,
        -half_l,
        half_w * 2.0,
        half_l * 2.0
    ));
    svg_lines.push("<style>".into());
    svg_lines.push("  .wall { fill: #5a5040; stroke: #8b7d6b; stroke-width: 0.3; }".into());
    svg_lines.push("  .building { fill: #4a3520; stroke: #6b5030; stroke-width: 0.2; }".into());
    svg_lines.push("  .tower { fill: #6b5a4a; stroke: #8b7d6b; stroke-width: 0.3; }".into());
    svg_lines.push("  .torch { fill: #ff8833; }".into());
    svg_lines.push("  .marker { fill: #ff4444; opacity: 0.5; }".into());
    svg_lines.push("  text { font-family: monospace; font-size: 2px; fill: #d4c9a8; }".into());
    svg_lines.push("</style>".into());

    for v in &zone.volumes {
        match v.shape {
            Shape::Box => {
                svg_lines.push(format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" class=\"wall\">\
                     <title>{}</title></rect>",
                    v.x - v.width / 2.0,
                    -(v.z + v.depth / 2.0),
                    v.width,
                    v.depth,
                    v.name
                ));
            }
            Shape::Cylinder | Shape::Sphere => {
                svg_lines.push(format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" class=\"tower\">\
                     <title>{}</title></circle>",
                    v.x, -v.z, v.radius, v.name
                ));
            }
        }
    }

    for l in &zone.lights {
        svg_lines.push(format!("<circle cx=\"{}\" cy=\"{}\" r=\"0.5\" class=\"torch\"/>", l.x, -l.z));
    }

    for m in &zone.markers {
        svg_lines.push(format!("<circle cx=\"{}\" cy=\"{}\" r=\"0.7\" class=\"marker\"/>", m.x, -m.z));
        svg_lines.push(format!("<text x=\"{}\" y=\"{}\">{}</text>", m.x + 1.0, -m.z, m.name));
    }

    svg_lines.push("</svg>".into());
    Ok(svg_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone_state::Volume;

    #[test]
    fn empty_zone_name_is_a_named_error_not_a_panic() {
        let zone = ZoneState::new("", 8.0, 8.0, 0.0, 8.0, "nowhere");
        let out = render_svg(&zone, Some("F:/v3/.forge/_scratch/svg_test_should_not_land.svg")).unwrap();
        assert!(out.contains("\"status\":\"ERROR\"") || out.contains("No zone initialized"));
    }

    #[test]
    fn a_real_zone_writes_a_real_svg_with_its_volume() {
        let dir = std::env::temp_dir().join("forge_zones_v3_svg_test");
        let path = dir.join("test_zone_top_down.svg");
        let mut zone = ZoneState::new("Test Zone", 16.0, 16.0, 0.0, 8.0, "test");
        let v = Volume::new("entry_hall_vol", Shape::Box, 0.0, 1.5, 0.0);
        zone.add_volume(v).unwrap();

        let out = render_svg(&zone, Some(path.to_str().unwrap())).unwrap();
        assert!(out.contains("\"status\":\"SUCCESS\""));

        let content = std::fs::read_to_string(&path).expect("svg file must exist");
        assert!(content.contains("<svg"));
        assert!(content.contains("entry_hall_vol"));
        std::fs::remove_file(&path).ok();
    }
}
