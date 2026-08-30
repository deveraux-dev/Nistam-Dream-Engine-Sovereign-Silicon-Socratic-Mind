//! Single-file offline build: inlines the star lane as gzip+base64 blobs and
//! swaps the Tauri IPC bridge for `ui/offline-shim.js`.
//! Entry: `studio-tauri --emit-giveaway <out.html>`.

use std::io::Write;
use std::path::Path;

use base64::Engine as _;

static HYG: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
static INDEX_HTML: &str = include_str!("../ui/index.html");
static APP_JS: &str = include_str!("../ui/app.js");
static STYLE_CSS: &str = include_str!("../ui/style.css");
static SHIM_JS: &str = include_str!("../ui/offline-shim.js");
static MILKYWAY: &[u8] = include_bytes!("../ui/milkyway.jpg");

const MAG_CUTOFF: i32 = 65_000;
const HDR: usize = 16;
const LUT: usize = 256 * 4;
const REC: usize = 17;

fn gz(bytes: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    e.write_all(bytes).expect("gzip write");
    e.finish().expect("gzip finish")
}

fn blob(bytes: &[u8]) -> (String, usize, usize) {
    let packed = gz(bytes);
    let raw = bytes.len();
    let gzl = packed.len();
    (base64::engine::general_purpose::STANDARD.encode(&packed), raw, gzl)
}

fn star_count() -> usize {
    u32::from_le_bytes(HYG[8..12].try_into().unwrap_or([0; 4])) as usize
}

fn mag_pmy_at(i: usize) -> i32 {
    let o = HDR + LUT + i * REC + 8;
    i32::from_le_bytes(HYG[o..o + 4].try_into().unwrap_or([0; 4]))
}

/// RGB triples for the 256x16 typed ink table, in `bake_sky_vbo` order.
fn ink_blob() -> Vec<u8> {
    crate::typed_ink_table()
        .iter()
        .flat_map(|c| [
            (c[0] * 255.0).round().clamp(0.0, 255.0) as u8,
            (c[1] * 255.0).round().clamp(0.0, 255.0) as u8,
            (c[2] * 255.0).round().clamp(0.0, 255.0) as u8,
        ])
        .collect()
}

/// One 12-byte row per magnitude-filtered star: full bake index, packed
/// RGBA, voice in millihertz. Row order IS the 5D payload's `idx` space.
fn subset_blob() -> Result<Vec<u8>, String> {
    let stars = crate::load_hyg_for_starmap();
    let full: Vec<usize> = (0..star_count()).filter(|&i| mag_pmy_at(i) <= MAG_CUTOFF).collect();
    if full.len() != stars.len() {
        return Err(format!("subset drift: {} filtered vs {} payload rows", full.len(), stars.len()));
    }
    let mut out = Vec::with_capacity(stars.len() * 12);
    for (idx, (_, _, color_rgba, _, _, voice_mhz)) in full.iter().zip(stars.iter()) {
        out.extend_from_slice(&(*idx as u32).to_le_bytes());
        out.extend_from_slice(&color_rgba.to_le_bytes());
        out.extend_from_slice(&voice_mhz.to_le_bytes());
    }
    Ok(out)
}

/// One MIDI note byte per star in BAKE order — the space `star_voice` is
/// indexed in. Hardware is a discrete 12-TET engine, so the byte IS the pitch;
/// the crossing into frequency happens once, at the audio edge, in the browser.
/// One quarter the size of the millihertz it replaces, and not rounded by an
/// integer retune on the way out.
fn note_blob() -> Vec<u8> {
    let n = star_count();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(crate::note_at(HYG, i).unwrap_or(0));
    }
    out
}

const CUT_OPEN: &str = "<!-- giveaway:cut -->";
const CUT_CLOSE: &str = "<!-- /giveaway:cut -->";

/// Drop every `giveaway:cut` region — the desktop-only faces (FLEET TRIAD
/// button, its panel, the VRAM strip) that have no live command in a browser.
fn cut_regions(html: &str) -> Result<(String, usize), String> {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut cuts = 0;
    while let Some(open) = rest.find(CUT_OPEN) {
        let close = rest[open..]
            .find(CUT_CLOSE)
            .ok_or_else(|| format!("unbalanced {CUT_OPEN} in index.html"))?;
        out.push_str(&rest[..open]);
        rest = &rest[open + close + CUT_CLOSE.len()..];
        cuts += 1;
    }
    out.push_str(rest);
    if out.contains(CUT_CLOSE) {
        return Err(format!("stray {CUT_CLOSE} in index.html"));
    }
    Ok((out, cuts))
}

fn strip_external_links(html: &str) -> String {
    html.lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("<link") && (t.contains("favicon") || t.contains("apple-touch-icon") || t.contains("site.webmanifest")))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn emit(out_path: &Path) -> Result<(), String> {
    let n = star_count();
    if n == 0 || &HYG[0..4] != b"HYGC" {
        return Err("hyg_baked.bin is not a HYGC catalog".into());
    }

    let (hyg_b64, hyg_raw, hyg_gz) = blob(HYG);
    let (ink_b64, ink_raw, ink_gz) = blob(&ink_blob());
    let (sub_b64, sub_raw, sub_gz) = blob(&subset_blob()?);
    let (note_b64, note_raw, note_gz) = blob(&note_blob());
    let chart = serde_json::to_string(&crate::get_sky_chart()).map_err(|e| e.to_string())?;

    let milkyway_uri = format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(MILKYWAY)
    );
    let app_js = APP_JS.replace("'milkyway.jpg'", &format!("'{milkyway_uri}'"));

    let payload = format!(
        "window.__FORGE_GIVEAWAY__={{hyg:\"{hyg_b64}\",ink:\"{ink_b64}\",subset:\"{sub_b64}\",notes:\"{note_b64}\",refAMhz:{ref_a},chart:{chart}}};",
        ref_a = forge_harmonics::theory::ALCHEMICAL.ref_a_mhz
    );

    let (cut_html, cuts) = cut_regions(INDEX_HTML)?;
    if cuts == 0 {
        return Err("index.html carries no giveaway:cut regions".into());
    }
    let head = strip_external_links(&cut_html)
        .replace(
            "<link rel=\"stylesheet\" href=\"style.css\" />",
            &format!("<style>\n{STYLE_CSS}\n</style>"),
        )
        .replace(
            "<script src=\"app.js\"></script>",
            &format!("<script>\n{payload}\n</script>\n<script>\n{SHIM_JS}\n</script>\n<script>\n{app_js}\n</script>"),
        );

    if head.contains("href=\"style.css\"") || head.contains("src=\"app.js\"") {
        return Err("index.html no longer matches the inline anchors".into());
    }

    std::fs::write(out_path, &head).map_err(|e| format!("write {}: {e}", out_path.display()))?;

    let mb = |b: usize| b as f64 / (1024.0 * 1024.0);
    println!("giveaway: {}", out_path.display());
    println!("  stars      {n}");
    println!("  cut        {cuts} desktop-only regions");
    println!("  hyg        {hyg_raw:>10} raw  {hyg_gz:>10} gz  {:>7.2} MB b64", mb(hyg_b64.len()));
    println!("  ink        {ink_raw:>10} raw  {ink_gz:>10} gz  {:>7.2} MB b64", mb(ink_b64.len()));
    println!("  subset     {sub_raw:>10} raw  {sub_gz:>10} gz  {:>7.2} MB b64", mb(sub_b64.len()));
    println!("  notes      {note_raw:>10} raw  {note_gz:>10} gz  {:>7.2} MB b64", mb(note_b64.len()));
    println!("  TOTAL HTML {:>10} B  {:>7.2} MB", head.len(), mb(head.len()));
    Ok(())
}
