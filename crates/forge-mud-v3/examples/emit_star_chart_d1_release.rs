//! D1 Release Artifact: Emit interactive HTML star chart with astrolabe engine.
//! Run: `cargo run --example emit_star_chart_d1_release --manifest-path crates/forge-vix-v3/Cargo.toml > dist/star-chart-release.html`

use forge_core_v3::astrolabe::{Astrolabe, CATALOG_16};
use forge_vix_v3::emit_html::{
    page_interactive, IPC_BRIDGE_SCRIPT, ASTROLABE_ORGAN_HOOK_SCRIPT, esc,
};

fn main() {
    let title = "13Forge Astrolabe — Live Star Chart";

    let vp = IrRect {
        min_x: 0,
        min_y: 0,
        max_x: 1280_000,
        max_y: 720_000,
    };

    let mut ui = LoweredUi::default();

    let mut astrolabe = Astrolabe::new(5354);
    astrolabe.rotate_rete(2250);

    for (idx, star) in CATALOG_16.iter().enumerate() {
        let key = format!("astrolabe.star.{}", star.name.replace(' ', "_"));
        let stable_key = StableKey(key);

        let ra_norm = (star.ra_cdeg as f32 / 36000.0).clamp(0.0, 1.0);
        let dec_norm = ((star.dec_cdeg as f32 + 9000.0) / 18000.0).clamp(0.0, 1.0);

        let w = 60_000;
        let h = 60_000;
        let x = (ra_norm * (vp.max_x - w as i64)) as i64;
        let y = (dec_norm * (vp.max_y - h as i64)) as i64;

        ui.layout.push(LayoutBox {
            widget_id: WidgetId(idx as u32),
            stable_key: stable_key.clone(),
            rect: IrRect {
                min_x: x,
                min_y: y,
                max_x: x + w as i64,
                max_y: y + h as i64,
            },
            z: idx as i32,
            clip_id: None,
            scroll_id: None,
            baseline: None,
            layout_version: 1,
        });

        ui.text_literals.push((
            stable_key.0.clone(),
            format!(
                "{}\nRA: {:.1}° Dec: {:.1}°",
                star.name,
                star.ra_cdeg as f32 / 100.0,
                star.dec_cdeg as f32 / 100.0
            ),
        ));
    }

    let css = r#"
body {
    margin: 0;
    padding: 0;
    background: #0a0706;
    font-family: 'IBM Plex Mono', ui-monospace, monospace;
    color: #e8e0d4;
}

#vp {
    position: relative;
    width: 1280px;
    height: 720px;
    background: radial-gradient(ellipse at center, #0f1219 0%, #0a0706 100%);
    overflow: hidden;
    box-shadow: inset 0 0 40px rgba(0, 0, 0, 0.8);
}

#vp > div {
    position: absolute;
    box-sizing: border-box;
    border: 1px solid rgba(195, 162, 86, 0.3);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease-out;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 4px;
    font-size: 10px;
    line-height: 1.2;
    overflow: hidden;
}

#vp > div:hover {
    background: rgba(195, 162, 86, 0.15) !important;
    border-color: rgba(195, 162, 86, 0.8) !important;
    box-shadow: 0 0 12px rgba(195, 162, 86, 0.4);
    z-index: 1000 !important;
}

#vp > div .t {
    display: block;
    padding: 2px 4px;
    font: 10px/1.35 'IBM Plex Mono', ui-monospace, monospace;
    color: #e8e0d4;
    pointer-events: none;
    white-space: pre-wrap;
}

.star-title {
    position: absolute;
    top: 20px;
    left: 20px;
    font-size: 24px;
    font-weight: bold;
    color: #c3a256;
    text-shadow: 0 0 8px rgba(195, 162, 86, 0.3);
    z-index: 10;
    pointer-events: none;
}

.star-legend {
    position: absolute;
    bottom: 20px;
    right: 20px;
    font-size: 12px;
    color: #a8a8a8;
    text-align: right;
    z-index: 10;
    pointer-events: none;
    line-height: 1.5;
}

canvas {
    display: block;
}

organ[type="astrolabe"],
.astrolabe-organ {
    width: 100%;
    height: 100%;
}
"#;

    let body = format!(
        r#"<div id="vp" data-title="{}">
{}
<div class="star-title">13FORGE ASTROLABE</div>
<div class="star-legend">
<div>16 Catalog Stars</div>
<div>RA/Dec Projection</div>
<div style="margin-top: 4px; font-size: 11px;">Click for details</div>
</div>
</div>"#,
        esc(title),
        ui.layout
            .iter()
            .zip(ui.text_literals.iter())
            .map(|(lb, (key, label))| {
                let (x, y, w, h) = (
                    lb.rect.min_x / 1000,
                    lb.rect.min_y / 1000,
                    (lb.rect.max_x - lb.rect.min_x) / 1000,
                    (lb.rect.max_y - lb.rect.min_y) / 1000,
                );
                format!(
                    r#"<div style="left:{}px;top:{}px;width:{}px;height:{}px;z-index:{};background:conic-gradient(from 61.80deg,#c3a256 0,transparent 34%,#c3a256 68%,transparent 100%)" data-vixi-id="{}" data-token="{}"><span class="t">{}</span></div>"#,
                    x, y, w, h, lb.z, esc(key), 0, esc(label)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    );

    let html = page_interactive(title, css, &body, true);

    println!("{}", html);
}
