//! D1 Release: Interactive star chart HTML with astrolabe 16-star catalog.
//! Run: `cargo run --example star_chart_d1_release --manifest-path crates/forge-vix-v3/Cargo.toml`

use forge_core_v3::astrolabe::CATALOG_16;
use forge_vix_v3::emit_html::{page_interactive, IPC_BRIDGE_SCRIPT, ASTROLABE_ORGAN_HOOK_SCRIPT, esc};

fn main() {
    let title = "13Forge Astrolabe — Live Star Chart";

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
    width: 100%;
    height: 100vh;
    background: radial-gradient(ellipse at center, #0f1219 0%, #0a0706 100%);
    overflow: hidden;
    box-shadow: inset 0 0 40px rgba(0, 0, 0, 0.8);
}

.star-container {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    perspective: 1000px;
}

.star {
    position: absolute;
    border: 1px solid rgba(195, 162, 86, 0.4);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.3s ease-out;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 8px;
    font-size: 11px;
    line-height: 1.3;
    overflow: hidden;
    background: conic-gradient(from 61.80deg, #c3a256 0, transparent 34%, #c3a256 68%, transparent 100%);
    box-shadow: inset 0 0 4px rgba(195, 162, 86, 0.2);
}

.star:hover {
    background: conic-gradient(from 61.80deg, #e8c547 0, transparent 34%, #e8c547 68%, transparent 100%) !important;
    border-color: rgba(232, 197, 71, 0.9) !important;
    box-shadow: 0 0 16px rgba(232, 197, 71, 0.5), inset 0 0 8px rgba(195, 162, 86, 0.4);
    z-index: 1000 !important;
    transform: scale(1.1);
}

.star-label {
    display: block;
    font-size: 10px;
    font-weight: 600;
    color: #2a2420;
    text-shadow: 0 0 2px rgba(232, 197, 71, 0.3);
    pointer-events: none;
}

.star-coords {
    display: block;
    font-size: 9px;
    color: #a8a8a8;
    opacity: 0.7;
}

.title-box {
    position: absolute;
    top: 40px;
    left: 40px;
    font-size: 28px;
    font-weight: bold;
    color: #c3a256;
    text-shadow: 0 0 12px rgba(195, 162, 86, 0.4);
    z-index: 10;
    pointer-events: none;
    letter-spacing: 2px;
}

.legend-box {
    position: absolute;
    bottom: 40px;
    right: 40px;
    font-size: 12px;
    color: #a8a8a8;
    text-align: right;
    z-index: 10;
    pointer-events: none;
    line-height: 1.6;
    background: rgba(10, 7, 6, 0.7);
    padding: 16px;
    border: 1px solid rgba(195, 162, 86, 0.2);
    border-radius: 4px;
}

.legend-title {
    color: #c3a256;
    font-weight: bold;
    margin-bottom: 8px;
}

.live-indicator {
    position: absolute;
    top: 40px;
    right: 40px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #92d450;
    z-index: 10;
    pointer-events: none;
}

.pulse {
    width: 8px;
    height: 8px;
    background: #92d450;
    border-radius: 50%;
    animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
}
"#;

    let mut body = String::new();
    body.push_str(r#"<div id="vp"><div class="star-container">"#);

    let vp_width = 1280;
    let vp_height = 720;

    for (idx, star) in CATALOG_16.iter().enumerate() {
        let ra_norm = (star.ra_cdeg as f32 / 36000.0).clamp(0.0, 1.0);
        let dec_norm = ((star.dec_cdeg as f32 + 9000.0) / 18000.0).clamp(0.0, 1.0);

        let star_size = 70;
        let x = (ra_norm * (vp_width - star_size) as f32) as i32;
        let y = (dec_norm * (vp_height - star_size) as f32) as i32;

        let label = star.name.replace(' ', " ");
        let coords = format!(
            "RA: {:.1}° | Dec: {:.1}°",
            star.ra_cdeg as f32 / 100.0,
            star.dec_cdeg as f32 / 100.0
        );

        body.push_str(&format!(
            r#"<div class="star" style="left:{}px;top:{}px;width:{}px;height:{}px;z-index:{}" data-star="{}" title="{} ({})"><span class="star-label">{}</span><span class="star-coords">{}</span></div>"#,
            x, y, star_size, star_size, idx,
            esc(&label), esc(&label), esc(&coords),
            esc(&label), esc(&coords)
        ));
    }

    body.push_str(
        r#"</div>
<div class="title-box">⟡ 13FORGE ASTROLABE ⟡</div>
<div class="live-indicator"><div class="pulse"></div><span>LIVE</span></div>
<div class="legend-box">
<div class="legend-title">STELLAR CATALOG</div>
<div>16 Fixed Stars</div>
<div>Stereographic Projection</div>
<div style="margin-top: 8px; font-size: 10px; color: #888;">Click for coordinates</div>
</div>
</div>"#,
    );

    let html = page_interactive(title, css, &body, true);
    println!("{}", html);
}
