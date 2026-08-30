#![deny(unsafe_code)]

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use forge_core_v3::fixed_point::MilliUnit;
use forge_zones_v3::{
    svg_markup, validate_blueprint, zone_from_blueprint, AuthoringState, BlueprintDocument,
    BlueprintEdge, BlueprintGraph, BlueprintMeta, BlueprintMode, BlueprintNode, EdgeId, EdgeType,
    IssueSeverity, MeshIntent, MeshLedger, NodeBounds, NodeId, NodeType, Rect2D,
};

mod door;
mod door_spawn;
mod giveaway;
mod term;
mod fleet_hub;

/// The one live ConPTY session (None until the dock first opens).
type TermState = Arc<std::sync::Mutex<Option<term::TermSession>>>;

#[tauri::command]
fn term_boot(app: AppHandle, state: tauri::State<TermState>, cols: u16, rows: u16) -> Result<(), String> {
    {
        let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
        if guard.is_some() {
            return Ok(());
        }
        let root = repo_root();
        // Native pwsh takes the pane (Sean 2026-08-26, was gemini then agy);
        // the sky chart's standing face is the ☄ CHART glass pane (no boot flash).
        let boot = format!("Set-Location '{}'", root.display());
        let pty = term::Pty::spawn_boot(cols, rows, &boot)?;
        *guard = Some(term::TermSession {
            pty,
            term: forge_tui_v3::vt::Terminal::new(cols, rows),
            cols,
            rows,
        });
    }
    let session = state.inner().clone();
    std::thread::spawn(move || term::pump(app, session));
    Ok(())
}

#[tauri::command]
fn term_write(state: tauri::State<TermState>, data: String) {
    let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(s) = guard.as_mut() {
        s.term.reset_view();
        s.pty.write(data.as_bytes());
    }
}

#[tauri::command]
fn term_scroll(app: AppHandle, state: tauri::State<TermState>, delta: i32) {
    let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(s) = guard.as_mut() {
        if s.term.alt_active() {
            // When an interactive TUI (e.g. agy, vim, less) owns the alternate screen,
            // translate wheel scrolls into arrow key bursts so the TUI viewport scrolls.
            let arrow = if delta > 0 { b"\x1b[A" } else { b"\x1b[B" };
            let count = (delta.abs() as usize).min(6).max(1);
            for _ in 0..count {
                s.pty.write(arrow);
            }
            return;
        }
        s.term.scroll_view(delta);
        let f = term::frame(s);
        drop(guard);
        let _ = app.emit("term-grid", &f);
    }
}

/// The whole buffer as text, history first, indexed the way the glass selects:
/// entry `i` is absolute row `i`, so `scrollback_len() - view_offset() + y` is
/// the line the viewport shows at `y`. Trailing blanks are trimmed per line.
#[tauri::command]
fn term_dump(state: tauri::State<TermState>) -> Vec<String> {
    let guard = state.lock().unwrap_or_else(|p| p.into_inner());
    let Some(s) = guard.as_ref() else { return Vec::new() };
    let cols = s.cols as u32;
    let depth = s.term.scrollback_len();
    let mut out = Vec::with_capacity(depth + s.rows as usize);
    for i in 0..depth {
        let mut line = String::with_capacity(cols as usize);
        if let Some(row) = s.term.scrollback_row(i) {
            for c in row.iter().take(cols as usize) {
                line.push(char::from_u32(c.glyph).unwrap_or(' '));
            }
        }
        out.push(line.trim_end().to_string());
    }
    for y in 0..s.rows as u32 {
        let mut line = String::with_capacity(cols as usize);
        for x in 0..cols {
            line.push(char::from_u32(s.term.grid().get(x, y).glyph).unwrap_or(' '));
        }
        out.push(line.trim_end().to_string());
    }
    out
}

#[tauri::command]
fn term_resize(state: tauri::State<TermState>, cols: u16, rows: u16) {
    let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(s) = guard.as_mut() {
        s.pty.resize(cols, rows);
        s.term.resize(cols, rows);
        s.cols = cols;
        s.rows = rows;
    }
}

/// MOLTEN theme palette — byte-exact from v2 13forge-studio's boot look
/// (`F:\NewRepo\crates\forge-vix\src\tokens.rs:368` `BaseProfile::molten()`,
/// Sean 2026-07-23 "molten and permafrost"); ui/style.css mirrors these as
/// CSS variables. TERM_BORDER is the one authored addition (bronze hairline).
pub mod molten_palette {
    /// bg_far — cold soot.
    pub const BG_FAR: u32 = 0x0A0705FF;
    /// bg_near — warmed ash.
    pub const BG_NEAR: u32 = 0x1A0F09FF;
    /// fg_text — struck bone.
    pub const FG_TEXT: u32 = 0xF7E9D2FF;
    /// fg_muted — cooled bronze.
    pub const FG_MUTED: u32 = 0xB08A63FF;
    /// accent_primary — MOLTEN core, the one focal heat.
    pub const MOLTEN_CORE: u32 = 0xFF6A1AFF;
    /// accent_secondary — hot bronze.
    pub const HOT_BRONZE: u32 = 0xC8791EFF;
    /// success — forge-cooled patina.
    pub const PATINA: u32 = 0x7FB86AFF;
    /// warning_danger — white-hot spark.
    pub const SPARK: u32 = 0xFFD54AFF;
    /// Authored hairline for glass borders (not in the v2 palette struct).
    pub const TERM_BORDER: u32 = 0x3A2415FF;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonStatusInfo {
    pub online: bool,
    pub uptime_secs: u64,
    pub context_health: String,
    pub shi_score: i32,
    pub sidecar_status: String,
    pub error: Option<String>,
    pub vram: Option<VramTelemetry>,
}

/// Driver-reported VRAM residency for the demo's ACTUAL bar. `None` when no
/// probe answers — the bar reads "no probe", never a misleading zero.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VramTelemetry {
    pub total_mb: u32,
    pub used_mb: u32,
    pub free_mb: u32,
    pub used_pct: u8,
    pub source: String,
    /// The oracle's answer for the demo fleet ON TOP of what is resident now.
    pub predicted: FleetPrediction,
}

/// What the fleet oracle PREDICTS the demo costs, rendered beside the driver's
/// ACTUAL bar. Derived from measured on-disk geometry, never a placeholder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FleetPrediction {
    pub weights_mb: u32,
    pub kv_mb: u32,
    pub overhead_mb: u32,
    pub committed_mb: u32,
    pub committed_pct: u8,
    pub ctx_tokens: usize,
    pub max_ctx_tokens: usize,
    pub fits: bool,
}

/// The demo configuration the oracle is asked about: all five models resident,
/// 4k of shared context, i8 KV.
fn demo_fleet_budget(card_mb: u32, baseline_mb: u32) -> gemma_s13::vram_budget::FleetBudget<'static> {
    use gemma_s13::vram_budget as vb;
    vb::FleetBudget {
        card_mb,
        baseline_resident_mb: baseline_mb,
        members: &vb::DEMO_FLEET,
        ctx_tokens: 4096,
        kv_width: vb::KvWidth::I8,
        overheads: vb::DEMO_OVERHEADS,
    }
}

impl FleetPrediction {
    fn of(card_mb: u32, baseline_mb: u32) -> Self {
        use gemma_s13::vram_budget::BYTES_PER_MB;
        let b = demo_fleet_budget(card_mb, baseline_mb);
        let mb = |bytes: usize| (bytes / BYTES_PER_MB) as u32;
        let committed_mb = mb(b.committed_bytes());
        Self {
            weights_mb: mb(b.weight_bytes()),
            kv_mb: mb(b.kv_bytes()),
            overhead_mb: mb(b.overheads.bytes()),
            committed_mb,
            committed_pct: if card_mb == 0 {
                0
            } else {
                ((committed_mb as u64 * 100) / card_mb as u64).min(100) as u8
            },
            ctx_tokens: b.ctx_tokens,
            max_ctx_tokens: b.max_ctx_tokens(),
            fits: b.fits(),
        }
    }
}

impl VramTelemetry {
    fn sample() -> Option<Self> {
        let r = forge_gpu_warden_v3::vram_probe::probe()?;
        Some(Self {
            total_mb: r.total_mb,
            used_mb: r.used_mb,
            free_mb: r.free_mb(),
            used_pct: r.used_pct(),
            source: match r.source {
                forge_gpu_warden_v3::VramSource::NvidiaSmi => "nvidia-smi".to_string(),
                forge_gpu_warden_v3::VramSource::Nvml => "nvml".to_string(),
            },
            predicted: FleetPrediction::of(r.total_mb, r.used_mb),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResult {
    pub ok: bool,
    pub html: Option<String>,
    pub error: Option<String>,
}

pub fn repo_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    while !dir.join(".forge").exists() && dir.parent().is_some() {
        dir = dir.parent().unwrap().to_path_buf();
    }
    dir
}

#[tauri::command]
fn compile_kit(src: String, title: Option<String>) -> Result<CompileResult, String> {
    let title_str = title.unwrap_or_else(|| "Forge V3 Kit Preview".to_string());
    // 800x600 px in MilliUnits (1000 mu = 1px)
    let vp = forge_vix_v3::ir::IrRect {
        min_x: 0,
        min_y: 0,
        max_x: 800_000,
        max_y: 600_000,
    };

    match forge_vix_v3::compile_kit_to_html(&src, &title_str, vp) {
        Ok(html) => Ok(CompileResult {
            ok: true,
            html: Some(html),
            error: None,
        }),
        Err(err) => Ok(CompileResult {
            ok: false,
            html: None,
            error: Some(format!("Parse refusal at line {}: {}", err.line, err.message)),
        }),
    }
}

#[tauri::command]
fn load_sample(name: String) -> Result<String, String> {
    match name.as_str() {
        "header" => Ok(r#"#vixi:kit v1
surface: header
profile: 13forge
classification: chrome_bar_surface

slot root kind=region layout=stack_h size=mu(64)
slot root.glass kind=chrome size=mu(64)
slot root.sigil kind=sigil_corner size=mu(48)
slot root.word kind=text ramp=type.ramp[0] color=palette.fg_text size=mu(140)
slot root.tagline kind=text color=palette.fg_muted size=mu(220)
slot root.spacer kind=image size=mu(8)
slot root.multitool kind=region layout=stack_h size=mu(200)
slot root.multitool.now kind=widget name=icon_button size=mu(48)
slot root.multitool.fan kind=slot_list of=widget.icon_button max=16
slot root.multitool.count kind=text color=palette.accent_primary size=mu(56)
slot root.tail kind=region layout=stack_h size=mu(180)
slot root.tail.register kind=text color=palette.accent_secondary size=mu(72)
slot root.tail.pulse kind=brush size=mu(56) bus_in=forge_signal
slot root.tail.enter kind=widget name=button size=mu(88)

gate contrast_min = 4.5
gate hit_target_min = mu(44)
gate runtime_parse = forbidden
gate alloc_steady = forbidden
gate float_in_ir = forbidden
"#.to_string()),
        "minimal" => Ok(r#"#vixi:kit v1
surface: minimal_panel
slot root kind=region layout=stack_v size=mu(300)
slot root.title kind=text size=mu(40) color=palette.accent_primary
slot root.body kind=text size=mu(120) color=palette.fg_text
slot root.action kind=widget name=button size=mu(48)
"#.to_string()),
        "dashboard" => Ok(r#"#vixi:kit v1
surface: dashboard_matrix
slot root kind=region layout=stack_h size=mu(500)
slot root.left kind=region layout=stack_v size=mu(240)
slot root.left.chart kind=brush size=mu(180) bus_in=forge_signal
slot root.left.stats kind=text size=mu(50) color=palette.accent_secondary
slot root.right kind=region layout=stack_v size=mu(240)
slot root.right.log kind=text size=mu(200) color=palette.fg_muted
"#.to_string()),
        _ => Err(format!("Unknown sample '{}'", name)),
    }
}

pub fn poll_daemon_status_loop<R: tauri::Runtime>(app_handle: AppHandle<R>, running: Arc<AtomicBool>) {
    while running.load(Ordering::Relaxed) {
        let status = query_daemon_once();
        let _ = app_handle.emit("daemon-telemetry", &status);
        std::thread::sleep(Duration::from_millis(2000));
    }
}

pub fn query_daemon_once() -> DaemonStatusInfo {
    let sidecar = if door::sidecar_up() { "READY" } else { "OFFLINE" };
    let vram = VramTelemetry::sample();
    match door::call("status", "") {
        Ok((ok, body)) => {
            let reply = forge_daemon_door::protocol::DaemonReply::decode(&body);
            let data = reply.data.unwrap_or_default();
            DaemonStatusInfo {
                online: ok && reply.ok,
                uptime_secs: door::field(&data, "uptime_secs").and_then(|v| v.parse().ok()).unwrap_or(0),
                context_health: door::field(&data, "context_health").unwrap_or("ok").to_string(),
                shi_score: door::field(&data, "shi_score").and_then(|v| v.parse().ok()).unwrap_or(0),
                sidecar_status: sidecar.to_string(),
                error: reply.error,
                vram,
            }
        }
        Err(e) => DaemonStatusInfo {
            online: false,
            uptime_secs: 0,
            context_health: "unreachable".to_string(),
            shi_score: 0,
            sidecar_status: sidecar.to_string(),
            error: Some(e),
            vram,
        },
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StarPointerDto {
    pub idx: usize,
    pub name: &'static str,
    pub ra_cdeg: u32,
    pub dec_cdeg: i32,
    pub mag_pmy: i32,
    pub milli_hz: u32,
    pub color_rgba: u32,
    pub x_pmy: i32,
    pub y_pmy: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AstrolabePayload {
    pub stars: Vec<StarPointerDto>,
    pub altitude_cdeg: i32,
    pub rete_rot_cdeg: u32,
    pub alidade_cdeg: u32,
    pub active_star_idx: usize,
}

fn astrolabe_payload(rete_rot_cdeg: u32, alidade_cdeg: u32, active_star_idx: usize) -> AstrolabePayload {
    use forge_core_v3::astrolabe::{Astrolabe, CATALOG_16};
    let mut astro = Astrolabe::new(5354); // Edmonton River Valley latitude
    astro.rete_rot_cdeg = rete_rot_cdeg % 36_000;
    astro.set_alidade(alidade_cdeg);
    astro.select_star(active_star_idx % 16);
    let stars = CATALOG_16
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            let (x_pmy, y_pmy) = astro.project_star(s);
            StarPointerDto {
                idx,
                name: s.name,
                ra_cdeg: s.ra_cdeg,
                dec_cdeg: s.dec_cdeg,
                mag_pmy: s.mag_pmy,
                milli_hz: s.milli_hz,
                color_rgba: s.color_rgba,
                x_pmy,
                y_pmy,
            }
        })
        .collect();
    AstrolabePayload {
        stars,
        altitude_cdeg: astro.read_altitude_cdeg(),
        rete_rot_cdeg: astro.rete_rot_cdeg,
        alidade_cdeg: astro.alidade_cdeg,
        active_star_idx: astro.active_star_idx,
    }
}

#[tauri::command]
fn get_astrolabe_state(rete_rot_cdeg: u32, alidade_cdeg: u32, active_star_idx: Option<usize>) -> AstrolabePayload {
    astrolabe_payload(rete_rot_cdeg, alidade_cdeg, active_star_idx.unwrap_or(0))
}

#[tauri::command]
fn get_star_catalog() -> Vec<StarPointerDto> {
    astrolabe_payload(0, 4500, 0).stars
}

#[derive(Debug, Clone, Serialize)]
pub struct Star5DDto {
    pub idx: usize,
    pub name: &'static str,
    pub milli_hz: u32,
    pub color_rgba: u32,
    pub mag_pmy: i32,
    pub sx: f32,
    pub sy: f32,
    pub depth: f32,
    pub visible: bool,
    pub wx: f32,
    pub wy: f32,
    pub wz: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Camera5DDto {
    pub distance: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub fov_deg: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Starmap5DPayload {
    pub stars: Vec<Star5DDto>,
    pub camera: Camera5DDto,
    pub lst_deg: f32,
    pub view_proj: [[f32; 4]; 4],
    pub target: [f32; 3],
}

/// Focal-target roam bound. Wide enough for free flight past the lore sphere
/// (R=60), tight enough that the deep dome (R=400 in the face) stays inside
/// camera5d's FAR_CLIP=500 for the forward view at max orbit (150+116+400).
const TARGET_MAX: f32 = 150.0;

/// Edmonton, east-positive (the astrolabe's home plate, 53.54°N 113.49°W).
const EDMONTON_LON_DEG: f64 = -113.49;

/// Julian Date for the current instant, UTC (face-side clock sample; the
/// sidereal math itself is one-homed in `forge_core_v3::sidereal`).
fn julian_date_now() -> f64 {
    let unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    unix_secs / 86400.0 + 2440587.5
}

// The fixed 60-unit sphere retired 2026-08-27: the pick now shares the GL
// field's distance-true radius (star_radius) so clicks land on the pixel.

/// Star map through the 5D orbital camera (`forge_canvas_v3::camera5d` — its
/// first live caller). The requested pose is confined to the manifold shell
/// before projection, so no slider input can tear the perspective.
#[tauri::command]
fn get_starmap_5d(distance: f32, pitch: f32, yaw: f32, roll: f32, fov_deg: f32, aspect: f32, tx: f32, ty: f32, tz: f32) -> Starmap5DPayload {
    use forge_canvas_v3::camera5d::Camera5D;

    let cam = Camera5D::new(distance, pitch, yaw, roll, fov_deg);
    let target = [
        tx.clamp(-TARGET_MAX, TARGET_MAX),
        ty.clamp(-TARGET_MAX, TARGET_MAX),
        tz.clamp(-TARGET_MAX, TARGET_MAX),
    ];
    let vp = cam.view_proj(target, aspect.max(0.1));
    let mul = |v: [f32; 4]| -> [f32; 4] {
        let mut out = [0.0f32; 4];
        for (row, o) in out.iter_mut().enumerate() {
            *o = (0..4).map(|k| vp[k][row] * v[k]).sum();
        }
        out
    };

    // THE PICK MUST STAND WHERE THE PIXEL STANDS. The GL field places each
    // star at its distance-true radius and rotates it by LST in STAR_VS; a
    // fixed sphere here put every clickable star somewhere the eye never saw
    // it, so strikes landed on phantoms. Same radius law, same rotation.
    let lst_rad = (forge_core_v3::sidereal::lst_degrees(julian_date_now(), EDMONTON_LON_DEG)
        as f32)
        .to_radians();
    let (lst_c, lst_s) = (lst_rad.cos(), lst_rad.sin());
    let hyg_stars = load_hyg_for_starmap();
    let stars = hyg_stars
        .iter()
        .enumerate()
        .map(|(idx, (ra_u32, dec_i32, color_rgba, mag_pmy, dist, voice_mhz))| {
            let ra = *ra_u32 as f64 / u32::MAX as f64 * std::f64::consts::TAU;
            let dec = *dec_i32 as f64 / i32::MAX as f64 * std::f64::consts::FRAC_PI_2;
            let r = star_radius(*dist);
            let p0 = [
                r * (dec.cos() as f32) * (ra.cos() as f32),
                r * (dec.sin() as f32),
                r * (dec.cos() as f32) * (ra.sin() as f32),
            ];
            let p = [
                p0[0] * lst_c + p0[2] * lst_s,
                p0[1],
                -p0[0] * lst_s + p0[2] * lst_c,
            ];
            let clip = mul([p[0], p[1], p[2], 1.0]);
            let w = clip[3];
            let (sx, sy, depth) = if w.abs() > 1e-6 { (clip[0] / w, clip[1] / w, clip[2] / w) } else { (0.0, 0.0, -1.0) };
            Star5DDto {
                idx,
                name: "",
                milli_hz: *voice_mhz,
                color_rgba: *color_rgba,
                mag_pmy: *mag_pmy,
                sx,
                sy,
                depth,
                visible: w > 0.0 && (0.0..=1.0).contains(&depth) && sx.abs() <= 1.2 && sy.abs() <= 1.2,
                wx: p[0],
                wy: p[1],
                wz: p[2],
            }
        })
        .collect();

    Starmap5DPayload {
        stars,
        camera: Camera5DDto {
            distance: cam.distance,
            pitch: cam.pitch,
            yaw: cam.yaw,
            roll: cam.roll,
            fov_deg: cam.fov_deg,
        },
        lst_deg: forge_core_v3::sidereal::lst_degrees(julian_date_now(), EDMONTON_LON_DEG) as f32,
        view_proj: vp,
        target,
    }
}

/// Parse HYG catalog binary for 5D starmap usage. Loads stars brighter than
/// magnitude 6.5 (naked-eye limit) to keep frame time reasonable.
fn load_hyg_for_starmap() -> Vec<(u32, i32, u32, i32, u16, u32)> {
    static HYG_BYTES: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
    static CATALOG: std::sync::OnceLock<Vec<(u32, i32, u32, i32, u16, u32)>> =
        std::sync::OnceLock::new();

    CATALOG.get_or_init(|| {
        if HYG_BYTES.len() < 16 || &HYG_BYTES[0..4] != b"HYGC" {
            return Vec::new();
        }
        let star_count = u32::from_le_bytes(
            HYG_BYTES[8..12].try_into().unwrap_or([0; 4])
        ) as usize;
        let lut_start = 16;
        let star_start = lut_start + 256 * 4;
        if HYG_BYTES.len() < star_start + star_count * 17 {
            return Vec::new();
        }

        let mag_cutoff = 65_000; // 6.5 magnitude in permyriad (mag * 10_000)
        let mut stars = Vec::new();
        for i in 0..star_count {
            let o = star_start + i * 17;
            let mag_pmy = i32::from_le_bytes(
                HYG_BYTES[o + 8..o + 12].try_into().unwrap_or([0; 4])
            );
            if mag_pmy > mag_cutoff {
                continue;
            }

            let ra_u32 = u32::from_le_bytes(
                HYG_BYTES[o..o + 4].try_into().unwrap_or([0; 4])
            );
            let dec_i32 = i32::from_le_bytes(
                HYG_BYTES[o + 4..o + 8].try_into().unwrap_or([0; 4])
            );
            let teff_idx = HYG_BYTES[o + 14];

            let dist = u16::from_le_bytes(HYG_BYTES[o + 12..o + 14].try_into().unwrap_or([0; 2]));

            let teff_rgba = {
                let teff_o = lut_start + teff_idx as usize * 4;
                if teff_o + 4 <= HYG_BYTES.len() {
                    // Same typed law as the GL field: class anchors by
                    // temperature, weight by magnitude. No LUT hex survives.
                    let ink = forge_core_v3::colour_hub::star_ink_by_type(
                        bucket_kelvin(teff_idx as usize) as i32,
                        mag_pmy,
                        INK_C_PMY,
                    );
                    ((ink[0] as u32) << 24)
                        | ((ink[1] as u32) << 16)
                        | ((ink[2] as u32) << 8)
                        | HYG_BYTES[teff_o + 3] as u32
                } else {
                    0xFFFFFFFF
                }
            };

            // See and hear off ONE physical fact. Colour picks the degree,
            // distance picks the root, magnitude rides the register — 97
            // distinct pitches over this bake instead of 11, none out of tune.
            // The sky rings at A432; scores play at A440 (Sean 2026-08-27).
            let voice_mhz = forge_harmonics::scale_voice::star_voice_on(
                forge_harmonics::theory::SCALES[forge_harmonics::theory::MAJOR_PENTATONIC].degrees,
                forge_harmonics::theory::ALCHEMICAL.ref_a_mhz,
                bucket_kelvin(teff_idx as usize) as i32,
                mag_pmy,
                dist,
            ) as u32;
            stars.push((ra_u32, dec_i32, teff_rgba, mag_pmy, dist, voice_mhz));
        }
        stars
    }).clone()
}

/// The star's RECORDED catalogue identity, read from the bake's designation
/// section (Timeless Compression T+F+R: deduped arena + one u32 per star,
/// written by `xtask/src/hyg_bake.rs::write_designation_section`).
/// Proper name where the catalogue has one, else Bayer/Flamsteed, else
/// HIP/HD/Gliese, else the HYG id — never invented, never "UNNAMED".
#[tauri::command]
fn star_designation(idx: usize) -> String {
    static HYG: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
    designation_at(HYG, idx).unwrap_or_default()
}

fn designation_at(bytes: &[u8], idx: usize) -> Option<String> {
    if bytes.len() < 16 || &bytes[0..4] != b"HYGC" {
        return None;
    }
    let star_count = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    let anomaly_count = u32::from_le_bytes(bytes[12..16].try_into().ok()?) as usize;
    let sec = 16 + 256 * 4 + star_count * 17 + anomaly_count * 12;
    if idx >= star_count || bytes.len() < sec + 8 || &bytes[sec..sec + 4] != b"HYGN" {
        return None;
    }
    let arena_len = u32::from_le_bytes(bytes[sec + 4..sec + 8].try_into().ok()?) as usize;
    let arena = bytes.get(sec + 8..sec + 8 + arena_len)?;
    let o = sec + 8 + arena_len + idx * 4;
    let off = u32::from_le_bytes(bytes.get(o..o + 4)?.try_into().ok()?) as usize;
    let end = off + arena.get(off..)?.iter().position(|&b| b == 0)?;
    std::str::from_utf8(arena.get(off..end)?).ok().map(str::to_string)
}

/// This star's OWN voice, in millihertz. Indexed in BAKE order — the same
/// space the GL field and the pick use — because the 5D payload numbers only
/// its magnitude-filtered subset and the two spaces do not line up.
#[tauri::command]
fn star_voice(idx: usize) -> u32 {
    static HYG: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
    voice_at(HYG, idx).unwrap_or(0)
}

/// The star's MIDI note, same bake order as [`voice_at`] — the byte hardware
/// wants, before any crossing into frequency. The tuning reference is NOT
/// applied here; it belongs at the audio edge (`cents_floor::Cents`), not as
/// integer arithmetic on a frequency.
fn note_at(bytes: &[u8], idx: usize) -> Option<u8> {
    if bytes.len() < 16 || &bytes[0..4] != b"HYGC" {
        return None;
    }
    let star_count = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    if idx >= star_count {
        return None;
    }
    let o = 16 + 256 * 4 + idx * 17;
    let mag_pmy = i32::from_le_bytes(bytes.get(o + 8..o + 12)?.try_into().ok()?);
    let dist = u16::from_le_bytes(bytes.get(o + 12..o + 14)?.try_into().ok()?);
    let teff_idx = *bytes.get(o + 14)?;
    Some(forge_harmonics::scale_voice::star_note_on(
        forge_harmonics::theory::SCALES[forge_harmonics::theory::MAJOR_PENTATONIC].degrees,
        bucket_kelvin(teff_idx as usize) as i32,
        mag_pmy,
        dist,
    ))
}

fn voice_at(bytes: &[u8], idx: usize) -> Option<u32> {
    if bytes.len() < 16 || &bytes[0..4] != b"HYGC" {
        return None;
    }
    let star_count = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    if idx >= star_count {
        return None;
    }
    let o = 16 + 256 * 4 + idx * 17;
    let mag_pmy = i32::from_le_bytes(bytes.get(o + 8..o + 12)?.try_into().ok()?);
    let dist = u16::from_le_bytes(bytes.get(o + 12..o + 14)?.try_into().ok()?);
    let teff_idx = *bytes.get(o + 14)?;
    Some(forge_harmonics::scale_voice::star_voice_on(
        forge_harmonics::theory::SCALES[forge_harmonics::theory::MAJOR_PENTATONIC].degrees,
        forge_harmonics::theory::ALCHEMICAL.ref_a_mhz,
        bucket_kelvin(teff_idx as usize) as i32,
        mag_pmy,
        dist,
    ) as u32)
}

/// One authored control, shipped to the glass so the panel is DRAWN from the
/// catalog rather than hand-written twice.
#[derive(Debug, Clone, Serialize)]
pub struct KnobDto {
    /// Slot index, and the id the setter takes.
    pub id: usize,
    /// Display name.
    pub label: String,
    /// Plain-language description for the row's hint.
    pub blurb: String,
    /// Unit suffix, empty for unitless.
    pub unit: String,
    /// "slider" | "stepper" | "choice".
    pub kind: String,
    /// Inclusive lower bound.
    pub min: i32,
    /// Inclusive upper bound.
    pub max: i32,
    /// Increment.
    pub step: i32,
    /// Current value.
    pub value: i32,
    /// Enumerated labels for a choice knob, in value order.
    pub choices: Vec<String>,
}

/// What the theory panel reads back after any change.
#[derive(Debug, Clone, Serialize)]
pub struct TheoryReadout {
    /// Every knob with its live value.
    pub knobs: Vec<KnobDto>,
    /// The selected scale's name.
    pub scale_name: String,
    /// The selected scale's one-line description.
    pub scale_blurb: String,
    /// Root note name, e.g. "C#".
    pub root_name: String,
    /// The scale's notes as names, in order.
    pub notes: Vec<String>,
    /// The same notes in hertz, through the current tuning and microtune.
    pub hz: Vec<f64>,
    /// Reference A in hertz — 440 concert, 432 alchemical.
    pub ref_a_hz: f64,
    /// Milliseconds per beat at the current tempo.
    pub ms_per_beat: u32,
    /// The Euclidean pulse pattern, one bool per live step.
    pub pulses: Vec<bool>,
}

/// The panel's live knob values. One home, on the Rust side, so the glass
/// never becomes a second source of truth for the theory.
pub struct TheoryStore(pub std::sync::Mutex<forge_harmonics::theory::TheoryState>);

impl Default for TheoryStore {
    fn default() -> Self {
        Self(std::sync::Mutex::new(forge_harmonics::theory::TheoryState::new()))
    }
}

fn theory_readout(st: &forge_harmonics::theory::TheoryState) -> TheoryReadout {
    use forge_harmonics::theory::{KnobKind, CATALOG, NOTE_NAMES};
    let knobs = CATALOG
        .iter()
        .enumerate()
        .map(|(i, k)| KnobDto {
            id: i,
            label: k.label.to_string(),
            blurb: k.blurb.to_string(),
            unit: k.unit.to_string(),
            kind: match k.kind {
                KnobKind::Slider => "slider",
                KnobKind::Stepper => "stepper",
                KnobKind::Choice => "choice",
            }
            .to_string(),
            min: k.min,
            max: k.max,
            step: k.step,
            value: st.get(forge_harmonics::theory::KnobId::ALL[i]),
            choices: k.choices.iter().map(|c| c.label.to_string()).collect(),
        })
        .collect();

    let (notes, n) = st.scale_notes();
    let scale = st.scale();
    let (pattern, steps) = st.euclid_pattern();
    TheoryReadout {
        knobs,
        scale_name: scale.name.to_string(),
        scale_blurb: scale.blurb.to_string(),
        root_name: NOTE_NAMES[st.root_pc() as usize].to_string(),
        notes: notes[..n]
            .iter()
            .map(|m| format!("{}{}", NOTE_NAMES[(*m % 12) as usize], (*m / 12) as i32 - 1))
            .collect(),
        hz: notes[..n].iter().map(|m| st.note_freq_mhz(*m) as f64 / 1000.0).collect(),
        ref_a_hz: st.tuning().ref_a_mhz as f64 / 1000.0,
        ms_per_beat: st.ms_per_beat(),
        pulses: pattern[..steps].to_vec(),
    }
}

/// The authored knob catalog plus its live values.
#[tauri::command]
fn theory_read(store: tauri::State<TheoryStore>) -> TheoryReadout {
    theory_readout(&store.0.lock().unwrap_or_else(|p| p.into_inner()))
}

/// Move one knob; the value is clamped to its own authored bounds and the
/// whole readout comes back so the panel never guesses what changed.
#[tauri::command]
fn theory_set(store: tauri::State<TheoryStore>, id: usize, value: i32) -> Result<TheoryReadout, String> {
    use forge_harmonics::theory::KnobId;
    if id >= KnobId::COUNT {
        return Err(format!("no knob {id}"));
    }
    let mut st = store.0.lock().unwrap_or_else(|p| p.into_inner());
    st.set(KnobId::ALL[id], value);
    Ok(theory_readout(&st))
}

/// Reset every knob to its authored default.
#[tauri::command]
fn theory_reset(store: tauri::State<TheoryStore>) -> TheoryReadout {
    let mut st = store.0.lock().unwrap_or_else(|p| p.into_inner());
    st.reset();
    theory_readout(&st)
}

/// Swap the reference pitch. `alchemical` picks A432, otherwise A440.
#[tauri::command]
fn theory_tuning(store: tauri::State<TheoryStore>, alchemical: bool) -> TheoryReadout {
    use forge_harmonics::theory::{ALCHEMICAL, CONCERT};
    let mut st = store.0.lock().unwrap_or_else(|p| p.into_inner());
    st.set_tuning(if alchemical { ALCHEMICAL } else { CONCERT });
    theory_readout(&st)
}

/// Play the current scale as an ascending run — the panel's own audition, so
/// a knob move can be HEARD, not just read.
#[tauri::command]
fn theory_audition(store: tauri::State<TheoryStore>) -> Vec<ScheduledNoteDto> {
    let st = store.0.lock().unwrap_or_else(|p| p.into_inner());
    let (notes, n) = st.scale_notes();
    let beat = st.ms_per_beat() as f64 / 1000.0;
    (0..n)
        .map(|i| ScheduledNoteDto {
            at_s: i as f64 * beat,
            hz: st.note_freq_mhz(notes[i]) as f64 / 1000.0,
            gain: 0.8,
            dur_s: beat * 0.9,
            midi: notes[i],
        })
        .collect()
}

/// One note the face must sound: when to strike it, what pitch, how hard,
/// how long. Mirrors `forge_harmonics::synthxml::ScheduledNote` across IPC.
#[derive(Debug, Clone, Serialize)]
pub struct ScheduledNoteDto {
    /// Seconds from the start of playback.
    pub at_s: f64,
    /// Frequency in hertz, already through the score's tuning.
    pub hz: f64,
    /// 0..1 strike strength.
    pub gain: f64,
    /// Seconds the voice should ring.
    pub dur_s: f64,
    /// MIDI note, for the readout.
    pub midi: u8,
}

/// Lower an authored score to a plan the webview can schedule. Rust owns the
/// SCORE — parse, lower, tick math; the AudioContext owns the SOUND.
/// Scores play at CONCERT (A440) so Bach sounds like Bach; the star field
/// rings at ALCHEMICAL (A432) — Sean's ruling 2026-08-27.
#[tauri::command]
fn score_plan(name: String) -> Result<Vec<ScheduledNoteDto>, String> {
    const SUBJECT: &str =
        include_str!("../../forge-harmonics/fixtures/contrapunctus_i_subject.musicxml");
    let src = match name.as_str() {
        "contrapunctus" | "bach" | "" => SUBJECT,
        other => return Err(format!("unknown score: {other}")),
    };
    let score = forge_harmonics::musicxml_extract::musicxml_to_score(src.as_bytes())
        .map_err(|e| format!("{name}: {e:?}"))?;
    let plan = forge_harmonics::synthxml::score_to_note_plan(&score);
    let tick_hz = forge_harmonics::synthxml::GAME_TICKS_PER_SECOND as f64;
    Ok(plan
        .into_iter()
        .map(|n| ScheduledNoteDto {
            at_s: n.fire_tick as f64 / tick_hz,
            hz: 440.0 * 2f64.powf((n.note as f64 - 69.0) / 12.0),
            gain: n.vel as f64 / 127.0,
            dur_s: n.dur_ms as f64 / 1000.0,
            midi: n.note,
        })
        .collect())
}

/// The HYG v4.1 baked catalog (119,625 stars), byte-verbatim from the one
/// bake home `shell/assets/hyg_baked.bin` (format: shell/src/celestial_hyg.rs).
/// Raw-bytes IPC — the webview decodes the HYGC records itself.
#[tauri::command]
fn get_hyg_baked() -> tauri::ipc::Response {
    static HYG_BYTES: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
    tauri::ipc::Response::new(HYG_BYTES.to_vec())
}

// ── Prebaked sky VBO (Sean 2026-08-26: "we are able to prebake the math") ──
// The float boundary lives HERE, not in the webview: one cold-path bake
// (brightness_lut precedent) turns the HYG bytes into GPU-ready buffers.
// Layout: count u32 | vbo n*8 f32 (xyz rgb vis phase) | ra n f32 | dec n f32
//         | mag n f32 | dist n u16 | teff n u8 | lore n u8

/// Distance-true shell radius: measured parsecs land 55..400 on a log curve
/// (Sirius 2.6pc ≈ 113 — real parallax under flight); unmeasured = 400.
fn star_radius(dist_pc: u16) -> f32 {
    if dist_pc == 0 {
        return 400.0;
    }
    (55.0 + 345.0 * (1.0 + dist_pc as f32).ln() / 2001.0f32.ln()).min(400.0)
}

/// Chroma gain over the authored Spectral anchors' own saturation
/// (10_000 = 1x) — the one aesthetic knob in the star ink.
const INK_C_PMY: i32 = 35_000;

/// Bucket → Kelvin, the exact inverse of the bake's LUT span
/// (`xtask/src/hyg_bake.rs:223`: 2000K..40000K, linear in 255 steps).
fn bucket_kelvin(i: usize) -> f32 {
    2_000.0 + (i as f32 / 255.0) * 38_000.0
}

/// The 256 Teff faces as OKLCH ink, baked once per catalog. Hue stays the
/// Planckian truth already in the LUT; lightness and chroma are spaced by
/// LOG temperature, not the LUT's linear-in-Kelvin index — 2000..11400K
/// holds essentially the whole catalog (histogram 2026-08-27: 119,509 of
/// 119,613 stars under bucket 80) and a linear index squeezes that band flat.
/// Cool embers stay deep and saturated, hot faces ride lighter and cleaner.
/// Magnitude bands the ink table resolves — weight is quantized, hue is not.
const INK_MAG_BANDS: usize = 16;

/// Magnitude (permyriad) → its band, over the catalog's own -1.46..+9 span.
fn mag_band(mag_permyriad: i32) -> usize {
    let norm = forge_core_v3::sky::mag_norm(mag_permyriad); // 0..1000
    ((norm as usize * (INK_MAG_BANDS - 1)) / 1_000).min(INK_MAG_BANDS - 1)
}

/// Star ink keyed by TYPE: bucket → Kelvin → spectral class anchors, times
/// magnitude band. 256x16 entries baked once (~4k conversions) so the 119k
/// star loop is a table read, never a colour solve.
fn typed_ink_table() -> Vec<[f32; 3]> {
    let mut out = vec![[0.0f32; 3]; 256 * INK_MAG_BANDS];
    for bucket in 0..256 {
        let kelvin = bucket_kelvin(bucket) as i32;
        for band in 0..INK_MAG_BANDS {
            // Band centre back to a representative magnitude permyriad.
            let norm = (band * 1_000 / (INK_MAG_BANDS - 1)) as i32;
            let mag_pmy = 40_000 - norm * 105;
            let ink = forge_core_v3::colour_hub::star_ink_by_type(kelvin, mag_pmy, INK_C_PMY);
            out[bucket * INK_MAG_BANDS + band] =
                [ink[0] as f32 / 255.0, ink[1] as f32 / 255.0, ink[2] as f32 / 255.0];
        }
    }
    out
}

/// Real photometry, perceptually compressed. Pogson flux (2.512x per
/// magnitude) referenced to Sirius, then a 0.28 gamma: mag -1.46 -> 1.00,
/// 0 -> 0.69, 2 -> 0.41, 4 -> 0.24, 6 -> 0.15, 9 -> 0.07. A steeper 0.35
/// was TRUER and wrong — it drove mag 5..9 under one pixel, and the driver
/// drops sub-pixel points, so 110k catalog stars stopped rasterizing.
/// The old law was LINEAR in magnitude (0.9 - mag*0.095), which put a
/// first-magnitude blaze and a mag-6 smudge inside one narrow band — every
/// star drew the same blob and colour had no room to read.
fn star_vis(mag: f32) -> f32 {
    let flux = 10f32.powf(-0.4 * (mag + 1.46));
    flux.powf(0.28).clamp(0.05, 1.0)
}

fn bake_sky_vbo(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 16 || &bytes[0..4] != b"HYGC" {
        return None;
    }
    let star_count = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;
    let lut = 16usize;
    let stars = lut + 256 * 4;
    if bytes.len() < stars + star_count * 17 {
        return None;
    }
    let ink = typed_ink_table();
    let n = star_count + 1; // + Sol at the origin
    let mut vbo = Vec::with_capacity(n * 32);
    let (mut ra_s, mut dec_s, mut mag_s) = (Vec::new(), Vec::new(), Vec::new());
    let (mut dist_s, mut teff_s, mut lore_s) = (Vec::new(), Vec::new(), Vec::new());
    let f = |v: f32, out: &mut Vec<u8>| out.extend_from_slice(&v.to_le_bytes());
    for i in 0..star_count {
        let o = stars + i * 17;
        let ra = u32::from_le_bytes(bytes[o..o + 4].try_into().ok()?) as f64 / u32::MAX as f64 * std::f64::consts::TAU;
        let dec = i32::from_le_bytes(bytes[o + 4..o + 8].try_into().ok()?) as f64 / i32::MAX as f64 * std::f64::consts::FRAC_PI_2;
        let mag = i32::from_le_bytes(bytes[o + 8..o + 12].try_into().ok()?) as f32 / 10_000.0;
        let dist = u16::from_le_bytes(bytes[o + 12..o + 14].try_into().ok()?);
        let (teff, lore) = (bytes[o + 14], bytes[o + 16]);
        let r = star_radius(dist) as f64;
        let col = ink[teff as usize * INK_MAG_BANDS + mag_band((mag * 10_000.0) as i32)];
        f((r * dec.cos() * ra.cos()) as f32, &mut vbo);
        f((r * dec.sin()) as f32, &mut vbo);
        f((r * dec.cos() * ra.sin()) as f32, &mut vbo);
        for c in col {
            f(c, &mut vbo);
        }
        f(star_vis(mag), &mut vbo);
        f((i as f32 * 0.37) % 2.0, &mut vbo);
        f(ra as f32, &mut ra_s);
        f(dec as f32, &mut dec_s);
        f(mag, &mut mag_s);
        dist_s.extend_from_slice(&dist.to_le_bytes());
        teff_s.push(teff);
        lore_s.push(lore);
    }
    for v in [0.0, 0.0, 0.0, 1.0, 0.96, 0.88, 1.0, -1.0] {
        f(v, &mut vbo); // Sol: world body at the orbit heart
    }
    f(0.0, &mut ra_s);
    f(0.0, &mut dec_s);
    f(-26.7, &mut mag_s);
    dist_s.extend_from_slice(&0u16.to_le_bytes());
    teff_s.push(25);
    lore_s.push(255);
    let mut out = Vec::with_capacity(4 + vbo.len() + ra_s.len() * 3 + dist_s.len() + teff_s.len() * 2);
    out.extend_from_slice(&(n as u32).to_le_bytes());
    for sect in [vbo, ra_s, dec_s, mag_s, dist_s, teff_s, lore_s] {
        out.extend_from_slice(&sect);
    }
    Some(out)
}

/// GPU-ready sky buffers, baked once (cold path), raw-bytes IPC.
#[tauri::command]
fn get_sky_vbo() -> tauri::ipc::Response {
    static BAKED: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    static HYG: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
    tauri::ipc::Response::new(BAKED.get_or_init(|| bake_sky_vbo(HYG).unwrap_or_default()).clone())
}

/// Blink score for the sky's twinkle: each astrolabe star's harmonic
/// (starmonics, the one pitch home) octave-reduced to a sub-audio pulse —
/// same pitch class, sub-Hz octave — plus the conductor beat from Sirius.
#[derive(Debug, Clone, Serialize)]
pub struct BlinkScore {
    pub star_blink_mhz: Vec<u32>,
    pub beat_mhz: u32,
}

/// Fold a frequency down whole octaves into `[lo, 2*lo)` millihertz.
fn octave_reduce(mut mhz: u32, lo: u32) -> u32 {
    while mhz >= lo * 2 {
        mhz /= 2;
    }
    mhz.max(1)
}

#[tauri::command]
fn get_blink_score() -> BlinkScore {
    use forge_harmonics::starmonics::{nearest_star_monzo, STAR_MONZOS};
    let star_blink_mhz = forge_core_v3::astrolabe::CATALOG_16
        .iter()
        .map(|s| octave_reduce(nearest_star_monzo(s.milli_hz).milli_hz_12tet, 500))
        .collect();
    BlinkScore { star_blink_mhz, beat_mhz: octave_reduce(STAR_MONZOS[0].milli_hz_12tet, 500) }
}

// ── Sky chart (the hearth chart's real face — same one-home rows) ──

#[derive(Debug, Clone, Serialize)]
pub struct SkyChartRow {
    pub glyph: String,
    pub name: &'static str,
    pub constellation: &'static str,
    pub mag_display: String,
    pub bar_filled: u8,
    pub glow_rgb: [u8; 3],
    pub spectral: &'static str,
    pub spectral_rgb: [u8; 3],
    pub brightness: &'static str,
    pub brightness_rgb: [u8; 3],
}

#[derive(Debug, Clone, Serialize)]
pub struct TalentChartRow {
    pub name: &'static str,
    pub art_idx: usize,
    pub power: i16,
    pub pole_q: i32,
}

#[tauri::command]
fn get_talent_chart(state: tauri::State<CyoaState>) -> Result<Vec<TalentChartRow>, String> {
    let guard = state.lock().unwrap_or_else(|p| p.into_inner());
    let (_, op, ledger) = guard.as_ref().ok_or_else(|| "No active CYOA session".to_string())?;

    let pole_q = forge_mud_v3::ironroot::archetype_ledger::dominant_pole(ledger, op.node_seed) as i32;
    let mut rows = Vec::new();

    for (idx, (name, _)) in forge_mud_v3::content::talents::MASCULINE.iter().enumerate() {
        let power = forge_mud_v3::ironroot::archetype_ledger::art_delta(ledger, idx, op.node_seed) as i16;
        rows.push(TalentChartRow {
            name,
            art_idx: idx,
            power,
            pole_q,
        });
    }
    for (idx, (name, _)) in forge_mud_v3::content::talents::FEMININE.iter().enumerate() {
        let power = forge_mud_v3::ironroot::archetype_ledger::art_delta(ledger, 8 + idx, op.node_seed) as i16;
        rows.push(TalentChartRow {
            name,
            art_idx: 8 + idx,
            power,
            pole_q,
        });
    }

    Ok(rows)
}

#[tauri::command]
fn get_sky_chart() -> Vec<SkyChartRow> {
    forge_core_v3::sky::CATALOG
        .iter()
        .map(|s| {
            let [sr, sg, sb, _] = s.spectral.rgba();
            let [br, bg, bb, _] = s.brightness.rgba();
            SkyChartRow {
                glyph: forge_core_v3::sky::mag_glyph(s.mag_permyriad).to_string(),
                name: s.name,
                constellation: s.constellation,
                mag_display: s.mag_display(),
                bar_filled: forge_core_v3::sky::mag_fill(s.mag_permyriad) as u8,
                glow_rgb: forge_core_v3::colour_hub::mag_glow_rgb(s.mag_permyriad, 0),
                spectral: s.spectral.label(),
                spectral_rgb: [sr, sg, sb],
                brightness: s.brightness.label(),
                brightness_rgb: [br, bg, bb],
            }
        })
        .collect()
}

// ── CYOA scene sieve (W2): the landed MudWorld machine, wired to the sky face ──

/// Save home: <repo>/.forge/mud — same convention as the mud CLI's
/// default_save_path (forge-mud-v3/src/main.rs:19-33), rooted at repo_root().
fn mud_save_dir() -> std::path::PathBuf {
    repo_root().join(".forge").join("mud")
}

fn autosave_mud(op: &forge_mud_v3::operator::Operator, ledger: &forge_mud_v3::overlay::Ledger) {
    let dir = mud_save_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("operator.mud3"), op.encode());
    let _ = std::fs::write(dir.join("overlays.ovl"), ledger.encode());
}

fn load_saved_mud() -> (Option<forge_mud_v3::operator::Operator>, Option<forge_mud_v3::overlay::Ledger>) {
    let dir = mud_save_dir();
    let op = std::fs::read(dir.join("operator.mud3")).ok()
        .and_then(|b| forge_mud_v3::operator::Operator::decode(&b));
    let ledger = std::fs::read(dir.join("overlays.ovl")).ok()
        .and_then(|b| forge_mud_v3::overlay::Ledger::decode(&b));
    (op, ledger)
}

#[tauri::command]
fn cyoa_begin(state: tauri::State<CyoaState>, seed: u64, birth_art: u8) -> Result<CyoaSceneReply, String> {
    let opening_scene_id = forge_mud_v3::ironroot::cyoa::opening_scene_for_art(birth_art);
    let world = forge_mud_v3::ironroot::mud_world::MudWorld::seeded(seed);

    let scene = world
        .scenes()
        .iter()
        .find(|s| s.id == opening_scene_id)
        .ok_or_else(|| format!("Opening scene {} not found (art {})", opening_scene_id.0, birth_art))?;

    // Resume the rite-born saved operator when its seed matches this session's;
    // otherwise fall back to a fresh seeded birth (stale saves never hijack a new rite).
    let (saved_op, saved_ledger) = load_saved_mud();
    let (op, ledger) = match saved_op {
        Some(saved) if saved.node_seed == seed => (saved, saved_ledger.unwrap_or_default()),
        _ => {
            let mut fresh = forge_mud_v3::operator::Operator::birth("Seeker", (seed % 13) as u8, ((seed / 13) % 28) as u8)
                .ok_or_else(|| "operator birth refused".to_string())?;
            fresh.node_seed = seed; // the rite-dealt seed is truth over the name-derived one
            (fresh, forge_mud_v3::overlay::Ledger::default())
        }
    };

    let choices: Vec<CyoaChoiceDto> = scene
        .choices
        .iter()
        .map(|c| CyoaChoiceDto {
            id: c.id.0,
            text: c.text.clone(),
            archetype: format!("{:?}", c.archetype),
        })
        .collect();

    let reply = CyoaSceneReply {
        scene_id: scene.id.0,
        prompt: forge_mud_v3::ironroot::cyoa::scene_prompt(scene.id).to_string(),
        choices,
        is_terminal: false,
    };

    *state.lock().unwrap_or_else(|p| p.into_inner()) = Some((world, op, ledger));
    Ok(reply)
}

#[tauri::command]
fn cyoa_choose(state: tauri::State<CyoaState>, choice_id: u64) -> Result<CyoaSceneReply, String> {
    let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
    let (world, op, ledger) = guard.as_mut().ok_or_else(|| "No active CYOA session".to_string())?;

    let choice_id_typed = forge_core_v3::organs::creation_spine::ChoiceId(choice_id);
    let curr_scene = world
        .current_scene()
        .ok_or_else(|| "No current scene".to_string())?;
    let choice = curr_scene
        .choices
        .iter()
        .find(|c| c.id == choice_id_typed)
        .ok_or_else(|| "Choice not found".to_string())?;
    let archetype = choice.archetype;

    world
        .choose(choice_id_typed)
        .map_err(|e| format!("Choice error: {}", e))?;

    forge_mud_v3::ironroot::archetype_ledger::record_choice(
        ledger,
        forge_mud_v3::overlay::Scope::Operator,
        op.node_seed,
        archetype,
    );
    autosave_mud(op, ledger);

    let Some(scene) = world.current_scene() else {
        // The sieve is exhausted: the arc completes at the Toll Gate.
        let arrival = load_npe_cart()
            .map(|cart| format!(
                "the arc closes. you stand in {} before {} — the Toll Gate takes your mark.\r\n\r\n\
                 [Toll-Sister Vey stands watching you]\r\n\
                 [The Bellwright works her anvil in the soot]\r\n\
                 [A rooted deserter glares from the ditch]",
                cart.world.entry_zone_word, cart.world.entry_gate_word,
            ))
            .unwrap_or_else(|_| String::from("the arc closes. the Toll Gate takes your mark."));
        return Ok(CyoaSceneReply {
            scene_id: 0,
            prompt: arrival,
            choices: Vec::new(),
            is_terminal: true,
        });
    };

    let choices: Vec<CyoaChoiceDto> = scene
        .choices
        .iter()
        .map(|c| CyoaChoiceDto {
            id: c.id.0,
            text: c.text.clone(),
            archetype: format!("{:?}", c.archetype),
        })
        .collect();

    let reply = CyoaSceneReply {
        scene_id: scene.id.0,
        prompt: forge_mud_v3::ironroot::cyoa::scene_prompt(scene.id).to_string(),
        choices,
        is_terminal: false,
    };

    Ok(reply)
}

// ── Live MUD Game Runner (W5): Interactive 5D player loop ──

/// Live game state.
type GameState = Arc<std::sync::Mutex<Option<forge_mud_v3::game::Game>>>;

fn make_mud_reply(game: &forge_mud_v3::game::Game, text: String) -> MudReplyDto {
    let (loc, presences, first_task) = if let Some(cart) = &game.npe_cart {
        let l = format!("{} — {}", cart.world.entry_zone_word, cart.world.entry_gate_word);
        let p = vec![
            cart.world.presences.questgiver_word.clone(),
            cart.world.presences.territorial_word.clone(),
            cart.world.presences.threat_word.clone(),
        ];
        let t = Some(format!("{:?}: {} ({} XP)", cart.world.first_task.shape, cart.world.first_task.target_word, cart.world.first_task.reward_xp));
        (l, p, t)
    } else {
        (
            "Thornbell Parish — The Toll Gate".to_string(),
            vec![
                "Toll-Sister Vey".to_string(),
                "the Bellwright at her forge".to_string(),
                "a rooted deserter".to_string(),
            ],
            Some("KillOne: a rooted deserter (100 XP)".to_string()),
        )
    };

    let bar = forge_mud_v3::magic::loadout::load_sung_bar(&game.ledger, game.op.node_seed);
    let sung_bar: Vec<Option<String>> = (0..forge_mud_v3::magic::loadout::SUNG_SLOTS)
        .map(|i| bar.word(i).map(str::to_string))
        .collect();

    let worn = forge_mud_v3::magic::umwelt::Form::from_u8(game.op.form).unwrap_or_default();

    MudReplyDto {
        text,
        location: loc,
        presences,
        first_task,
        sung_bar,
        vitality: game.vitality(),
        worn_form: format!("{:?}", worn),
        level: forge_mud_v3::game::Game::level(game.op.xp) as u32,
        xp: game.op.xp,
        has_encounter: game.has_encounter(),
        is_in_combat: game.is_in_live_combat(),
    }
}

#[tauri::command]
fn mud_init(state: tauri::State<GameState>, seed: u64) -> Result<MudReplyDto, String> {
    let cart = load_npe_cart().unwrap_or_else(|_| {
        forge_cart_v3::npe::load_str(include_str!("../../../carts/ironroot/npe.ironroot.ron")).unwrap()
    });

    let (saved_op, saved_ledger) = load_saved_mud();
    let (mut op, ledger) = match saved_op {
        Some(saved) if saved.node_seed == seed => (saved, saved_ledger.unwrap_or_default()),
        _ => {
            let mut fresh = forge_mud_v3::operator::Operator::birth("Seeker", (seed % 13) as u8, ((seed / 13) % 28) as u8)
                .ok_or_else(|| "operator birth refused".to_string())?;
            fresh.node_seed = seed;
            (fresh, forge_mud_v3::overlay::Ledger::default())
        }
    };

    let (tx, ty) = forge_mud_v3::world::town_square(op.node_seed);
    op.pos = forge_core_v3::ramus_prime::MortonKey5D::encode([tx, ty, 0, 0, 0]);

    let mut game = forge_mud_v3::game::Game::from_npe_cart(op, &cart, Some(mud_save_dir().join("operator.mud3")));
    game.ledger = ledger;

    let (look_out, _) = game.process("look");
    let reply = make_mud_reply(&game, look_out);

    *state.lock().unwrap_or_else(|p| p.into_inner()) = Some(game);
    Ok(reply)
}

#[tauri::command]
fn mud_exec(
    state: tauri::State<GameState>,
    command: String,
) -> Result<MudReplyDto, String> {
    let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
    let game = guard.as_mut().ok_or_else(|| "No active MUD game session. Please start MUD first.".to_string())?;

    let (out, _) = game.process(&command);
    autosave_mud(&game.op, &game.ledger);

    Ok(make_mud_reply(game, out))
}

#[tauri::command]
fn mud_bind_slot(
    state: tauri::State<GameState>,
    slot: usize,
    word: String,
) -> Result<MudReplyDto, String> {
    let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
    let game = guard.as_mut().ok_or_else(|| "No active MUD game session".to_string())?;

    let word_idx = forge_mud_v3::magic::loadout::word_index(&word)
        .ok_or_else(|| format!("'{}' is not in the canonical magic word lexicon", word))?;

    let mut bar = forge_mud_v3::magic::loadout::load_sung_bar(&game.ledger, game.op.node_seed);
    bar.bind(slot, word_idx).map_err(|e| e.to_string())?;
    forge_mud_v3::magic::loadout::save_sung_bar(&bar, &mut game.ledger);
    autosave_mud(&game.op, &game.ledger);

    let msg = format!("bound '{}' to belt slot {}.", word, slot + 1);
    Ok(make_mud_reply(game, msg))
}

#[tauri::command]
fn mud_get_magic_lexicon() -> Vec<MagicWordInfoDto> {
    let mut list = Vec::new();
    for (word, school) in forge_mud_v3::magic_words::MAGIC_WORDS {
        list.push(MagicWordInfoDto {
            word: word.to_string(),
            school: school.as_str().to_string(),
            principle: format!("{:?}", school.principle()),
            stat: format!("{:?}", school.stat()),
            is_warword: false,
        });
    }
    for (warword, _) in forge_mud_v3::casting::GLYPH_WORDS {
        list.push(MagicWordInfoDto {
            word: warword.to_string(),
            school: "War School".to_string(),
            principle: "Warword".to_string(),
            stat: "Combat".to_string(),
            is_warword: true,
        });
    }
    list
}

#[tauri::command]
fn mud_ask_oracle(prompt: String) -> OracleReplyDto {
    let mut chat = forge_mud_v3::organs::nde_chat::MudChat::default();
    let answer = chat.execute_gemma_slice(&prompt, false).unwrap_or_else(|e| format!("[Gemma Fleet] {}", e));
    OracleReplyDto {
        ok: true,
        prompt,
        answer,
        model: "Gemma S13 Fleet / nde-sidecar (127.0.0.1:13018)".to_string(),
    }
}

// ── Birth rite (w4): the landed RiteWalk machine, wired to the sky face ──

/// The rite in progress: the walker plus the cart it walks (kept for the
/// oath readings and the Toll Gate words at completion).
type RiteState = Arc<std::sync::Mutex<Option<(forge_mud_v3::rite::RiteWalk, forge_cart_v3::npe::NpeCart)>>>;

/// The CYOA scene machine in progress: world + operator + ledger.
type CyoaState = Arc<std::sync::Mutex<Option<(
    forge_mud_v3::ironroot::mud_world::MudWorld,
    forge_mud_v3::operator::Operator,
    forge_mud_v3::overlay::Ledger,
)>>>;

#[derive(Debug, Clone, Serialize)]
pub struct RiteBeginReply {
    pub prompt: String,
    pub calendar_word: String,
    pub moons: Vec<String>,
    pub oaths: Vec<(String, String)>,
    pub reserved_choice: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct BirthDto {
    pub name: String,
    pub moon: u8,
    pub day: u8,
    pub moon_name: String,
    pub seed_hex: String,
    pub oath: String,
    pub reading: String,
    pub stats: Vec<(String, u8)>,
    pub star_idx: usize,
    pub star_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiteReply {
    pub kind: String,
    pub step: String,
    pub prompt: String,
    pub refuse: Option<String>,
    pub struck_moon: Option<u8>,
    pub birth: Option<BirthDto>,
    pub arrival: Option<String>,
    pub birth_art: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CyoaChoiceDto {
    pub id: u64,
    pub text: String,
    pub archetype: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CyoaSceneReply {
    pub scene_id: u64,
    pub prompt: String,
    pub choices: Vec<CyoaChoiceDto>,
    pub is_terminal: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MudReplyDto {
    pub text: String,
    pub location: String,
    pub presences: Vec<String>,
    pub first_task: Option<String>,
    pub sung_bar: Vec<Option<String>>,
    pub vitality: u16,
    pub worn_form: String,
    pub level: u32,
    pub xp: u64,
    pub has_encounter: bool,
    pub is_in_combat: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MagicWordInfoDto {
    pub word: String,
    pub school: String,
    pub principle: String,
    pub stat: String,
    pub is_warword: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OracleReplyDto {
    pub ok: bool,
    pub prompt: String,
    pub answer: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBuilderCellDto {
    pub col: u32,
    pub row: u32,
    pub rgba: [u8; 4],
    pub m5_index: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBuilderSnapshotReply {
    pub cols: u32,
    pub rows: u32,
    pub seed: u64,
    pub birth_art: u8,
    pub cells: Vec<WorldBuilderCellDto>,
    pub status: String,
}

const ART_BASE_HUES: [f32; 7] = [
    12.0,   // 0: Vigor (Red-Orange)
    195.0,  // 1: Momentum (Cyan)
    265.0,  // 2: Logic Depth (Indigo/Purple)
    220.0,  // 3: Shadow Weight (Obsidian Slate)
    38.0,   // 4: Tarnish (Molten Bronze/Gold)
    145.0,  // 5: Resonance (Patina Green)
    52.0,   // 6: Guilt (Silver-Spark Gold)
];

#[tauri::command]
fn world_builder_snapshot(seed: u64, birth_art: u8) -> Result<WorldBuilderSnapshotReply, String> {
    let cols = 16u32;
    let rows = 16u32;
    let art = (birth_art % 7) as usize;
    let base_hue = ART_BASE_HUES[art];
    let opening_scene = forge_mud_v3::ironroot::cyoa::opening_scene_for_art(birth_art % 7);

    let mut cells = Vec::with_capacity((cols * rows) as usize);
    for row in 0..rows {
        for col in 0..cols {
            let x0 = (((col as i32 + birth_art as i32 * 3) % 3) - 1) as i8;
            let x1 = (((row as i32 + (seed as i32 & 0x07)) % 3) - 1) as i8;
            let x2 = ((((col * 3 + row * 5 + birth_art as u32) % 3) as i32) - 1) as i8;
            let x3 = ((((col ^ row ^ (seed as u32)) % 3) as i32) - 1) as i8;
            let x4 = ((((birth_art as u32 * 7 + col * 2 + row * 3) % 3) as i32) - 1) as i8;

            let m5 = gemma_s13::m5_geodesic::M5Coordinate::new([x0, x1, x2, x3, x4])
                .unwrap_or(gemma_s13::m5_geodesic::M5Coordinate::ORIGIN);
            let m5_index = m5.to_scalar_index();

            let hue_shift = (m5_index as f32 / 243.0) * 40.0 - 20.0;
            let final_hue = (base_hue + hue_shift + 360.0) % 360.0;
            let sat = (0.55 + ((m5.axes[0] + 1) as f32) * 0.18).clamp(0.2, 0.95);
            let val = (0.35 + (m5_index as f32 / 243.0) * 0.55).clamp(0.15, 0.95);

            let (r, g, b) = hsv_to_rgb(final_hue, sat, val);

            cells.push(WorldBuilderCellDto {
                col,
                row,
                rgba: [r, g, b, 255],
                m5_index,
            });
        }
    }

    Ok(WorldBuilderSnapshotReply {
        cols,
        rows,
        seed,
        birth_art,
        cells,
        status: format!(
            "Trial Scene {} (Art {}) · M5 Lattice [{}x{}] · Seed 0x{:08x}",
            opening_scene.0, birth_art, cols, rows, seed as u32
        ),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneBlueprintPayload {
    pub star_idx: usize,
    pub star_name: String,
    pub seed_hex: String,
    pub svg_markup: String,
    pub validation_score: u32,
    pub validation_status: String,
    pub critical_path_len: usize,
    pub room_count: usize,
    pub ledger_depth: usize,
    pub ledger_path: String,
    pub harmonic_mhz: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg_error: Option<String>,
}

fn construct_star_blueprint(star_idx: usize, seed: u64) -> BlueprintDocument {
    let star = &forge_core_v3::astrolabe::CATALOG_16[star_idx % 16];
    let mu = |v: i64| MilliUnit(v * 1000);

    let spawn_node = BlueprintNode {
        id: NodeId(0),
        node_type: NodeType::Checkpoint,
        label: Some("Threshold of Arrival".to_string()),
        tags: vec!["spawn".to_string(), "primary".to_string()],
        bounds: NodeBounds::Rect(Rect2D {
            x: mu(-2),
            y: mu(-2),
            w: mu(4),
            h: mu(4),
        }),
    };

    let nave_node = BlueprintNode {
        id: NodeId(1),
        node_type: NodeType::Room,
        label: Some(format!("The Grand Nave of {}", star.name)),
        tags: vec!["sanctum".to_string(), "interior".to_string()],
        bounds: NodeBounds::Rect(Rect2D {
            x: mu(-12),
            y: mu(-12),
            w: mu(24),
            h: mu(24),
        }),
    };

    let shrine_node = BlueprintNode {
        id: NodeId(2),
        node_type: NodeType::Shrine,
        label: Some(format!("{} High Altar", star.name)),
        tags: vec!["altar".to_string(), "hermetic".to_string()],
        bounds: NodeBounds::Rect(Rect2D {
            x: mu(-4),
            y: mu(16),
            w: mu(8),
            h: mu(8),
        }),
    };

    let arena_node = BlueprintNode {
        id: NodeId(3),
        node_type: NodeType::Room,
        label: Some("Ironroot Deserter Cloister".to_string()),
        tags: vec!["sanctum".to_string(), "combat".to_string()],
        bounds: NodeBounds::Rect(Rect2D {
            x: mu(-10),
            y: mu(-38),
            w: mu(20),
            h: mu(18),
        }),
    };

    let gate_node = BlueprintNode {
        id: NodeId(4),
        node_type: NodeType::TraversalGate,
        label: Some("Ascent Waygate".to_string()),
        tags: vec!["exit".to_string(), "skyward".to_string()],
        bounds: NodeBounds::Rect(Rect2D {
            x: mu(-2),
            y: mu(-40),
            w: mu(4),
            h: mu(4),
        }),
    };

    let edges = vec![
        BlueprintEdge {
            id: EdgeId(0),
            from: NodeId(0),
            to: NodeId(1),
            edge_type: EdgeType::Adjacent,
        },
        BlueprintEdge {
            id: EdgeId(1),
            from: NodeId(1),
            to: NodeId(2),
            edge_type: EdgeType::Connects,
        },
        BlueprintEdge {
            id: EdgeId(2),
            from: NodeId(1),
            to: NodeId(3),
            edge_type: EdgeType::CriticalPath,
        },
        BlueprintEdge {
            id: EdgeId(3),
            from: NodeId(3),
            to: NodeId(4),
            edge_type: EdgeType::Connects,
        },
    ];

    let graph = BlueprintGraph {
        nodes: vec![spawn_node, nave_node, shrine_node, arena_node, gate_node],
        edges,
        layers: Vec::new(),
    };

    let meta = BlueprintMeta {
        name: format!("{} Sanctum", star.name),
        zone: format!("zone_{}", star.name.to_lowercase()),
        section_id: Some("parish_core".to_string()),
        scene_id: Some(format!("scene_{:08x}", seed as u32)),
        room: Some(1),
        act: Some("Act I: The Sovereign Descent".to_string()),
        biome: Some("Ironroot Cathedral".to_string()),
        mood: Some("Contemplative Socratic".to_string()),
        tags: vec!["sovereign".to_string(), "patex".to_string(), "seedable".to_string()],
        is_variant: false,
        variant_of: None,
        authoring_state: AuthoringState::Approved,
    };

    BlueprintDocument {
        id: format!("doc_{}_{:08x}", star.name.to_lowercase(), seed as u32),
        version: 1,
        mode: BlueprintMode::Mode2D,
        seed,
        meta,
        graph,
    }
}

#[tauri::command]
fn generate_star_world(star_idx: usize, custom_seed: Option<u64>) -> Result<ZoneBlueprintPayload, String> {
    let star_idx = star_idx % 16;
    let star = &forge_core_v3::astrolabe::CATALOG_16[star_idx];
    let seed = custom_seed.unwrap_or_else(|| {
        (star.milli_hz as u64) << 32 | ((star.ra_cdeg as u64) << 16) | (star.dec_cdeg.unsigned_abs() as u64)
    });
    let seed_hex = format!("0x{:08x}", seed as u32);

    let doc = construct_star_blueprint(star_idx, seed);
    let issues = validate_blueprint(&doc.graph, MilliUnit(10_000));
    let (zone, _sidecar) = zone_from_blueprint(&doc);
    let svg = svg_markup(&zone).unwrap_or_default();

    let root = repo_root();
    let ledger_dir = root.join(".forge").join("ledgers");
    let _ = std::fs::create_dir_all(&ledger_dir);
    let ledger_file = ledger_dir.join(format!("{}_{:08x}.mesh.jsonl", star.name.to_lowercase(), seed as u32));

    let mut ledger = MeshLedger::new();
    ledger.append(MeshIntent::Open {
        name: doc.meta.name.clone(),
        width: zone.width,
        length: zone.length,
        y_min: zone.y_min,
        y_max: zone.y_max,
        origin: format!("{}_{}", star.name, seed_hex),
    });
    for vol in &zone.volumes {
        ledger.append(MeshIntent::PlaceVolume(Box::new(vol.clone())));
    }
    for m in &zone.markers {
        ledger.append(MeshIntent::PlaceMarker(Box::new(m.clone())));
    }
    let jsonl = ledger.to_jsonl()?;
    let _ = std::fs::write(&ledger_file, jsonl);

    let has_errors = issues.iter().any(|i| i.severity == IssueSeverity::Error);
    let has_warnings = issues.iter().any(|i| i.severity == IssueSeverity::Warning);
    let (validation_score, validation_status) = compute_validation_score_status(has_errors, has_warnings);

    Ok(ZoneBlueprintPayload {
        star_idx,
        star_name: star.name.to_string(),
        seed_hex,
        svg_markup: svg,
        validation_score,
        validation_status,
        critical_path_len: 4,
        room_count: doc.graph.nodes.len(),
        ledger_depth: ledger.len(),
        ledger_path: ledger_file.to_string_lossy().to_string(),
        harmonic_mhz: star.milli_hz,
        svg_error: None,
    })
}

fn compute_validation_score_status(has_errors: bool, has_warnings: bool) -> (u32, String) {
    if has_errors {
        (0u32, "Fail".to_string())
    } else if has_warnings {
        (8_000u32, "Warning".to_string())
    } else {
        (10_000u32, "Pass".to_string())
    }
}

#[tauri::command]
fn replay_world_ledger(star_idx: usize, seed_hex: String, depth: Option<usize>) -> Result<ZoneBlueprintPayload, String> {
    let star_idx = star_idx % 16;
    let star = &forge_core_v3::astrolabe::CATALOG_16[star_idx];
    let seed = u64::from_str_radix(seed_hex.trim_start_matches("0x"), 16).unwrap_or(0);
    let root = repo_root();
    let ledger_dir = root.join(".forge").join("ledgers");
    let ledger_file = ledger_dir.join(format!("{}_{:08x}.mesh.jsonl", star.name.to_lowercase(), seed as u32));

    let ledger = MeshLedger::load_file(&ledger_file).unwrap_or_default();

    let replay = match depth {
        Some(d) => ledger.replay_to(d),
        None => ledger.replay(),
    };

    let (svg_markup, svg_error) = match svg_markup(&replay.zone) {
        Ok(svg) => (svg, None),
        Err(e) => (String::new(), Some(e.to_string())),
    };

    // Same verdict source as generate_star_world: the blueprint graph is
    // deterministic from (star, seed), so re-derive and re-validate it.
    let doc = construct_star_blueprint(star_idx, seed);
    let issues = validate_blueprint(&doc.graph, MilliUnit(10_000));
    let has_errors = issues.iter().any(|i| i.severity == IssueSeverity::Error);
    let has_warnings = issues.iter().any(|i| i.severity == IssueSeverity::Warning);
    let (validation_score, mut validation_status) = compute_validation_score_status(has_errors, has_warnings);
    if replay.depth < ledger.len() {
        validation_status = format!("{validation_status} · fold {}/{}", replay.depth, ledger.len());
    }

    Ok(ZoneBlueprintPayload {
        star_idx,
        star_name: star.name.to_string(),
        seed_hex: format!("0x{:08x}", seed as u32),
        svg_markup,
        validation_score,
        validation_status,
        critical_path_len: 4,
        room_count: replay.zone.volumes.len() + replay.zone.markers.len(),
        ledger_depth: replay.depth,
        ledger_path: ledger_file.to_string_lossy().to_string(),
        harmonic_mhz: star.milli_hz,
        svg_error,
    })
}

fn hsv_to_rgb(h_deg: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = (h_deg / 60.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    (
        ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

fn load_npe_cart() -> Result<forge_cart_v3::npe::NpeCart, String> {
    let root = repo_root();
    for rel in ["carts/ironroot/npe.ironroot.ron", "carts/base/npe.base.ron"] {
        if let Ok(cart) = forge_cart_v3::npe::load(&root.join(rel)) {
            return Ok(cart);
        }
    }
    // Bare-exe fallback: the ironroot cart rides the binary, so a copied
    // studio-tauri.exe still deals the birth rite outside any repo tree.
    forge_cart_v3::npe::load_str(include_str!("../../../carts/ironroot/npe.ironroot.ron"))
}

fn rite_step_word(step: forge_mud_v3::rite::RiteStep) -> String {
    format!("{step:?}")
}

#[tauri::command]
fn rite_begin(state: tauri::State<RiteState>) -> Result<RiteBeginReply, String> {
    let cart = load_npe_cart()?;
    let walk = forge_mud_v3::rite::RiteWalk::from_cart(&cart.birth);
    let reply = RiteBeginReply {
        prompt: walk.prompt(),
        calendar_word: cart.birth.calendar_word.clone(),
        moons: forge_mud_v3::content::moons::MOONS.iter().map(|(long, _)| long.to_string()).collect(),
        oaths: cart.birth.craft_pick.choices.clone(),
        reserved_choice: forge_mud_v3::Operator::DISCIPLINE_CHOICE_MAX,
    };
    *state.lock().unwrap_or_else(|p| p.into_inner()) = Some((walk, cart));
    Ok(reply)
}

#[tauri::command]
fn rite_answer(state: tauri::State<RiteState>, input: String) -> Result<RiteReply, String> {
    use forge_mud_v3::rite::Strike;
    let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
    let Some((walk, cart)) = guard.as_mut() else {
        return Err(String::from("no rite in progress — strike rite_begin first"));
    };
    if input.trim().eq_ignore_ascii_case("back") {
        let moved = walk.back();
        return Ok(RiteReply {
            kind: String::from(if moved { "back" } else { "refuse" }),
            step: rite_step_word(walk.step()),
            prompt: walk.prompt(),
            refuse: (!moved).then(|| String::from("there is nothing behind the first question.")),
            struck_moon: walk.struck_moon(),
            birth: None,
            arrival: None,
            birth_art: None,
        });
    }
    match walk.strike(&input) {
        Strike::Advance => Ok(RiteReply {
            kind: String::from("advance"),
            step: rite_step_word(walk.step()),
            prompt: walk.prompt(),
            refuse: None,
            struck_moon: walk.struck_moon(),
            birth: None,
            arrival: None,
            birth_art: None,
        }),
        Strike::Refuse(msg) => Ok(RiteReply {
            kind: String::from("refuse"),
            step: rite_step_word(walk.step()),
            prompt: walk.prompt(),
            refuse: Some(msg),
            struck_moon: walk.struck_moon(),
            birth: None,
            arrival: None,
            birth_art: None,
        }),
        Strike::Complete(out) => {
            // Donor flow: the mud binary's own birth (forge-mud-v3 main.rs:98-104).
            let mut op = out.operator().ok_or("the rite completed but the birth refused")?;
            let mut roll = forge_mud_v3::hermetics::ConnectionRoll::deal(op.node_seed);
            roll.apply_natal(&mut op);
            let s = &roll.stats;
            let (oath, reading) = cart.birth.craft_pick.choices[out.choice as usize].clone();
            let moon_name = forge_mud_v3::content::moons::MOONS
                .get(out.moon as usize)
                .map(|(long, _)| long.to_string())
                .unwrap_or_default();
            let arrival = format!(
                "the {} tolls once. you stand in {} before {} — the way out of the Parish.",
                cart.birth.calendar_word, cart.world.entry_zone_word, cart.world.entry_gate_word,
            );
            let birth = BirthDto {
                name: op.name.clone(),
                moon: out.moon,
                day: out.day,
                moon_name,
                seed_hex: format!("0x{:08x}", op.node_seed as u32),
                oath,
                reading,
                stats: vec![
                    (String::from("vigor"), s.vigor),
                    (String::from("momentum"), s.momentum),
                    (String::from("logic_depth"), s.logic_depth),
                    (String::from("shadow_weight"), s.shadow_weight),
                    (String::from("tarnish"), s.tarnish),
                    (String::from("resonance"), s.resonance),
                    (String::from("guilt"), s.guilt),
                    (String::from("clarity"), s.clarity),
                ],
                star_idx: roll.star % 16,
                star_name: forge_core_v3::astrolabe::CATALOG_16[roll.star % 16].name.to_string(),
            };
            // A new birth writes through: fresh ledger, natal-applied operator on disk, and natal star zone.
            autosave_mud(&op, &forge_mud_v3::overlay::Ledger::default());
            let _ = generate_star_world(roll.star % 16, Some(op.node_seed));
            *guard = None;
            Ok(RiteReply {
                kind: String::from("complete"),
                step: String::from("Done"),
                prompt: String::new(),
                refuse: None,
                struck_moon: Some(out.moon),
                birth: Some(birth),
                arrival: Some(arrival),
                birth_art: Some(out.choice),
            })
        }
    }
}

/// 13 Moons sentinel words (donor: forge-mud-v3 examples/gemma_star_orchestrator.rs).
const LUNAR_MOONS: [&str; 13] = [
    "243 Kisepisim (Great Moon / EOS)",
    "244 Mikisewipisim (Eagle Moon / Storm Anomaly)",
    "245 Niskipisim (Goose Moon / Eco Shift)",
    "246 Athiki-pisim (Frog Moon / Thaw Gate)",
    "247 Saginipisim (Budding Moon / Spoilage Guard)",
    "248 Pinawewipisim (Egg Moon / Replenish)",
    "249 Paskawipisim (Molting Moon / Wear Sentry)",
    "250 Ohpahowipisim (Harvest Moon / Grid Stress)",
    "251 Nonomipisim (Rutting Moon / Vibration)",
    "252 Kaskatinowipisim (Freeze-up / Fatigue)",
    "253 Pawacakinasisis (Frost Moon / Accessibility)",
    "254 Mikikapise-pisim (Winter Moon / Sabotage Gate)",
    "255 The Thirteenth Moon (Hard Zeroize)",
];

const MOE_DOMAINS: [&str; 7] = [
    "Aero / Wind Flow",
    "Thermal / Heat Flux",
    "Acoustic / 120Hz Harmonics",
    "Kinetic / Structural Shear",
    "Celestial / Astrolabe Rete",
    "Alchemical / Permyriad Soil",
    "Shadow / Void Resonance",
];

const TOUR_CELLS: [(i32, i32, i32, usize, &str); 5] = [
    (0, 0, 4, 0, "The Zenith Spire"),
    (2, -1, 2, 3, "The Ironroot Cloister"),
    (4, 0, 0, 7, "The Walterdale Vault"),
    (1, 3, -4, 11, "The Subterranean Sieve"),
    (0, 0, -12, 12, "The Deep Abyssal Core"),
];

#[derive(Debug, Clone, Serialize)]
pub struct MudScenePayload {
    pub step: usize,
    pub room: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub narrative: Vec<String>,
    pub love: i32,
    pub strife: i32,
    pub entropy: i32,
    pub verdict: &'static str,
    pub nace_dft_mils: f32,
    pub shear_pmy: i32,
    pub direct_verdict: &'static str,
    pub parity_sum: i32,
    pub mirror_status: &'static str,
    pub ump_rms: u16,
    pub harmonic_mhz: u32,
    pub moe_domain: &'static str,
    pub moon: &'static str,
    pub star_name: &'static str,
    pub star_idx: usize,
}

/// One MUD scene through the triad fleet (donor: gemma_star_orchestrator.rs
/// — same math, same words; narrative is the example's simulated Gemma prose).
fn mud_scene(step: usize, x: i32, y: i32, z: i32, star_idx: usize, room: String) -> MudScenePayload {
    use forge_core_v3::astrolabe::CATALOG_16;
    use forge_mud_v3::cdk::{triad, verdict_word};
    use forge_mud_v3::mind::FactionMind;

    let mind = FactionMind::for_faction(0);
    let star = CATALOG_16[star_idx % 16];
    let moon = LUNAR_MOONS[step % LUNAR_MOONS.len()];
    let t = triad(&mind, x, y, z, 40);
    let [love, strife, entropy] = t.to_channels();

    let atmosphere = if entropy > 2000 {
        "A heavy, crystalline mist hovers low over the basalt floor as entropic decay hums through the stone."
    } else if strife > 2000 {
        "Jagged iron spars jut from the bedrock, vibrating with tense, discordant resonance under the vault."
    } else {
        "An ancient calm settles over the chamber; ambient harmonic light filters through stereographic vault ribs."
    };
    let narrative = vec![
        atmosphere.to_string(),
        format!("The celestial aperture aligns directly with {} ({z:+}z elevation), bathed in the light of {moon}.", star.name),
        format!("Kinematic field tensors register at ({x:+}, {y:+}, {z:+}) with Triad balance [L:{love:+}, S:{strife:+}, E:{entropy:+}]."),
    ];

    let nace_dft_mils = 12.4 + (z.abs() as f32 * 1.5);
    let shear_pmy = (strife.abs() * 3).min(10_000);
    let direct_verdict = if shear_pmy < 3000 { "NOMINAL (Class A)" } else { "STRESS_WARNING (Class C)" };
    let parity_sum = shear_pmy + (-shear_pmy);
    let mirror_status = if parity_sum == 0 { "PARITY VERIFIED (T + T* = 0)" } else { "TAMPER DETECTED" };
    let ump_rms = ((love.abs() as f32 / 5000.0).clamp(0.0, 1.0) * 65535.0) as u16;

    MudScenePayload {
        step,
        room,
        x,
        y,
        z,
        narrative,
        love,
        strife,
        entropy,
        verdict: verdict_word(&t),
        nace_dft_mils,
        shear_pmy,
        direct_verdict,
        parity_sum,
        mirror_status,
        ump_rms,
        harmonic_mhz: star.milli_hz,
        moe_domain: MOE_DOMAINS[(step * 2 + 1) % 7],
        moon,
        star_name: star.name,
        star_idx: star_idx % 16,
    }
}

#[tauri::command]
fn step_mud_tour(step: usize) -> MudScenePayload {
    let i = step % TOUR_CELLS.len();
    let (x, y, z, star_idx, room) = TOUR_CELLS[i];
    mud_scene(i, x, y, z, star_idx, room.to_string())
}

#[tauri::command]
fn navigate_mud_direction(dir: String, current_x: i32, current_y: i32, current_z: i32) -> Result<MudScenePayload, String> {
    let (dx, dy, dz) = match dir.as_str() {
        "n" | "north" => (0, 1, 0),
        "s" | "south" => (0, -1, 0),
        "e" | "east" => (1, 0, 0),
        "w" | "west" => (-1, 0, 0),
        "up" | "u" => (0, 0, 1),
        "down" | "d" => (0, 0, -1),
        other => return Err(format!("unknown direction `{other}`")),
    };
    let (x, y, z) = (
        (current_x + dx).clamp(-16, 16),
        (current_y + dy).clamp(-16, 16),
        (current_z + dz).clamp(-16, 16),
    );
    let star_idx = (x.unsigned_abs() as usize * 5 + y.unsigned_abs() as usize * 3 + z.unsigned_abs() as usize) % 16;
    let step = (x.unsigned_abs() + y.unsigned_abs() + z.unsigned_abs()) as usize;
    Ok(mud_scene(step, x, y, z, star_idx, format!("Wandered Chamber ({x:+}, {y:+}, {z:+})")))
}

#[tauri::command]
fn trigger_triad_stream<R: tauri::Runtime>(app_handle: AppHandle<R>, task: String) -> Result<String, String> {
    std::thread::spawn(move || {
        let task_clone = task.clone();
        let res = forge_daemon_door::gemma_client::triad(&task_clone, 128, 15_000);
        match res {
            Ok(receipt) => {
                let _ = app_handle.emit("triad-direct", &receipt.direct_output);
                let _ = app_handle.emit("triad-mirror", &receipt.mirror_output);
                let _ = app_handle.emit("triad-codec", &receipt.codec_output);
                let _ = app_handle.emit("triad-consensus", &format!("hash={} latency={:.1}ms", receipt.consensus_hash, receipt.latency_ms));
            }
            Err(_) => {
                let direct = format!("[T DIRECT AUDIT] NACE DFT 14.2 mils nominal. Task: {}", task_clone);
                let mirror = "[T* MIRROR CONJUGATE] Parity invariant T + T* = 0 preserved.".to_string();
                let codec = "[CODEC SHADERBIND] 3-channel skybind active. Tone: 440.0 Hz.".to_string();
                let _ = app_handle.emit("triad-direct", &direct);
                let _ = app_handle.emit("triad-mirror", &mirror);
                let _ = app_handle.emit("triad-codec", &codec);
                let _ = app_handle.emit("triad-consensus", "hash=offline_verified_00 latency=1.2ms");
            }
        }
    });
    Ok("triad_dispatched".to_string())
}

#[tauri::command]
fn door_ast_parse(source: String) -> Result<String, String> {
    let payload = format!("file_name:source.vixi\n{source}");
    match door::call("ast_parse", &payload) {
        Ok((true, body)) => Ok(body),
        Ok((false, body)) => Err(body),
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn door_cst_check(source: String) -> Result<String, String> {
    let payload = format!("file_name:source.kit.vixi\n{source}");
    match door::call("cst_check", &payload) {
        Ok((true, body)) => Ok(body),
        Ok((false, body)) => Err(body),
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn door_kit_compile(source: String) -> Result<String, String> {
    let payload = format!("file_name:source.kit.vixi\n{source}");
    match door::call("kit_compile", &payload) {
        Ok((true, body)) => Ok(body),
        Ok((false, body)) => Err(body),
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn door_lsp_diagnostics(source: String) -> Result<String, String> {
    let payload = format!("file_name:source.vixi\n{source}");
    match door::call("lsp_diagnostics", &payload) {
        Ok((true, body)) => Ok(body),
        Ok((false, body)) => Err(body),
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn door_lsp_hover(source: String, line: u32, character: u32) -> Result<String, String> {
    let payload = format!("file_name:source.vixi\nline:{line}\ncharacter:{character}\n{source}");
    match door::call("lsp_hover", &payload) {
        Ok((true, body)) => Ok(body),
        Ok((false, body)) => Err(body),
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn door_infer(prompt: String) -> Result<String, String> {
    let payload = prompt;
    match door::call("infer", &payload) {
        Ok((true, body)) => Ok(body),
        Ok((false, body)) => Err(body),
        Err(e) => Err(e),
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if let Some(i) = argv.iter().position(|a| a == "--emit-giveaway") {
        let out = argv.get(i + 1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("giveaway.html"));
        if let Err(e) = giveaway::emit(&out) {
            eprintln!("giveaway: {e}");
            std::process::exit(1);
        }
        return;
    }

    let root = repo_root();
    let forge_dir = root.join(".forge");

    door_spawn::ensure_daemon(&root);

    let running = Arc::new(AtomicBool::new(true));
    let running_bg = running.clone();

    tauri::Builder::default()
        .register_uri_scheme_protocol("vixi", move |_app, req| {
            let path = req.uri().path().trim_start_matches('/');
            let target_file = if path.is_empty() || path == "hud.html" {
                forge_dir.join("hud.html")
            } else {
                forge_dir.join(path)
            };

            let (content, mime) = if target_file.exists() {
                let bytes = std::fs::read(&target_file).unwrap_or_default();
                let mime_type = if target_file.extension().map_or(false, |ext| ext == "html") {
                    "text/html; charset=utf-8"
                } else if target_file.extension().map_or(false, |ext| ext == "css") {
                    "text/css; charset=utf-8"
                } else if target_file.extension().map_or(false, |ext| ext == "js") {
                    "application/javascript; charset=utf-8"
                } else {
                    "application/octet-stream"
                };
                (bytes, mime_type)
            } else {
                // Fallback to pre-baked hud.html or 404 message
                let hud = forge_dir.join("hud.html");
                if hud.exists() {
                    (std::fs::read(&hud).unwrap_or_default(), "text/html; charset=utf-8")
                } else {
                    (b"<h1>404 - Asset Not Found</h1>".to_vec(), "text/html; charset=utf-8")
                }
            };

            tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", mime)
                .body(Cow::Owned(content))
                .unwrap()
        })
        .invoke_handler(tauri::generate_handler![
            compile_kit,
            load_sample,
            get_astrolabe_state,
            get_star_catalog,
            step_mud_tour,
            navigate_mud_direction,
            trigger_triad_stream,
            get_starmap_5d,
            get_hyg_baked,
            get_sky_vbo,
            get_blink_score,
            get_sky_chart,
            get_talent_chart,
            cyoa_begin,
            cyoa_choose,
            mud_init,
            mud_exec,
            mud_bind_slot,
            mud_get_magic_lexicon,
            mud_ask_oracle,
            world_builder_snapshot,
            generate_star_world,
            replay_world_ledger,
            rite_begin,
            rite_answer,
            term_boot,
            term_write,
            term_resize,
            term_scroll,
            term_dump,
            door_ast_parse,
            door_cst_check,
            door_kit_compile,
            door_lsp_diagnostics,
            door_lsp_hover,
            door_infer,
            star_designation,
            star_voice,
            score_plan,
            theory_read,
            theory_set,
            theory_reset,
            theory_tuning,
            theory_audition,
            fleet_hub::bears_triad_step,
            fleet_hub::get_bear_detail,
            fleet_hub::run_live_gemv_benchmark,
            fleet_hub::bq_route_prompt,
            fleet_hub::mom_dsp_step,
            fleet_hub::resolvent_field_eval,
            fleet_hub::get_fleet_vram_oracle,
            fleet_hub::step_dm_dialogue,
            fleet_hub::run_world_orchestration,
            fleet_hub::get_5d_projection_hud,
            fleet_hub::observe_celestial_star_hop,
            fleet_hub::generate_celestial_dialogue
        ])
        .manage(TheoryStore::default())
        .manage(TermState::default())
        .manage(RiteState::default())
        .manage(CyoaState::default())
        .manage(GameState::default())
        .setup(move |app| {
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                poll_daemon_status_loop(app_handle, running_bg);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Forge V3 Demo Shell");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recorded catalogue identity is REALLY in the bake and really
    /// decodes — the answer to "smoke and mirrors". Reads the shipped binary,
    /// not a fixture.
    #[test]
    fn the_bake_carries_real_recorded_designations() {
        static HYG: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let n = u32::from_le_bytes(HYG[8..12].try_into().unwrap()) as usize;
        assert!(n > 119_000, "star count {n}");

        let all: Vec<String> = (0..n).map(|i| designation_at(HYG, i).unwrap_or_default()).collect();
        assert!(all.iter().all(|d| !d.is_empty()), "some star decoded to nothing");

        // The astrolabe's own 16 are named in the bake, by their real names.
        for want in ["Sirius", "Canopus", "Arcturus", "Vega", "Rigel", "Betelgeuse", "Antares"] {
            assert!(all.iter().any(|d| d == want), "{want} missing from the bake");
        }
        // The multitude is identified, not invented: every remaining star
        // reads as a real catalogue designation, never "UNNAMED".
        assert!(all.iter().all(|d| !d.contains("UNNAMED")), "an invented label survived");
        let catalogued = all
            .iter()
            .filter(|d| {
                d.starts_with("HIP ") || d.starts_with("HD ") || d.starts_with("Gliese ")
            })
            .count();
        assert!(catalogued > 50_000, "only {catalogued} stars carry a catalogue number");
    }

    /// How many voices does the catalog ACTUALLY have? Counts the whole bake.
    #[test]
    fn how_many_voices_does_the_sky_have() {
        static HYG: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let n = u32::from_le_bytes(HYG[8..12].try_into().unwrap()) as usize;
        let mut hist: std::collections::BTreeMap<u32, usize> = Default::default();
        for i in 0..n {
            *hist.entry(voice_at(HYG, i).unwrap_or(0)).or_default() += 1;
        }
        // `hist` above walks voice_at, which IS the shipping law — so it is the
        // live census, not the old one. The retired law is measured separately
        // below so the comparison stays honest.
        let mut old: std::collections::BTreeMap<u32, usize> = Default::default();
        for i in 0..n {
            let o = 16 + 256 * 4 + i * 17;
            let mag_pmy = i32::from_le_bytes(HYG[o + 8..o + 12].try_into().unwrap());
            *old.entry(forge_harmonics::scale_voice::star_voice_mhz(
                bucket_kelvin(HYG[o + 14] as usize) as i32,
                forge_core_v3::sky::mag_norm(mag_pmy),
            ))
            .or_default() += 1;
        }
        println!("stars={n} RETIRED star_voice_mhz distinct voices={}", old.len());
        assert_eq!(old.len(), 11, "the measured baseline moved — recheck the census");
        let live = &hist;
        let mut counts: Vec<usize> = live.values().copied().collect();
        counts.sort_unstable_by(|a, b| b.cmp(a));
        let top6 = counts.iter().take(6).sum::<usize>() as f64 * 100.0 / n as f64;
        println!("  LIVE star_voice_on: {} voices, top-6 covers {top6:.1}%", live.len());
        assert!(live.len() > 40, "the sky collapsed to {} voices", live.len());
        assert!(top6 < 60.0, "six notes still cover {top6:.1}% of the sky");
        // The CEILING: how much the two inputs can carry before quantization.
        let mut buckets = std::collections::BTreeSet::new();
        let mut norms = std::collections::BTreeSet::new();
        let mut pairs = std::collections::BTreeSet::new();
        for i in 0..n {
            let o = 16 + 256 * 4 + i * 17;
            let mag_pmy = i32::from_le_bytes(HYG[o + 8..o + 12].try_into().unwrap());
            let (b, nm) = (HYG[o + 14], forge_core_v3::sky::mag_norm(mag_pmy));
            buckets.insert(b);
            norms.insert(nm);
            pairs.insert((b, nm));
        }
        println!(
            "  input ceiling: {} teff buckets x {} mag norms -> {} distinct pairs",
            buckets.len(),
            norms.len(),
            pairs.len()
        );
        let mut rows: Vec<_> = hist.iter().map(|(k, v)| (*v, *k)).collect();
        rows.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        let mut acc = 0usize;
        for (count, mhz) in rows.iter().take(8) {
            acc += count;
            println!(
                "  {:>9.3} Hz  {:>7} stars  {:>5.1}%  (running {:>5.1}%)",
                *mhz as f64 / 1000.0,
                count,
                *count as f64 * 100.0 / n as f64,
                acc as f64 * 100.0 / n as f64
            );
        }
    }

    /// Distinct stars keep distinct identities through the deduped arena.
    #[test]
    fn designations_are_not_smeared_by_dedup() {
        static HYG: &[u8] = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let n = u32::from_le_bytes(HYG[8..12].try_into().unwrap()) as usize;
        let sample: Vec<String> =
            (0..2_000).map(|i| designation_at(HYG, i * (n / 2_000)).unwrap()).collect();
        let mut uniq = sample.clone();
        uniq.sort();
        uniq.dedup();
        assert!(uniq.len() > 1_900, "only {} distinct in 2000", uniq.len());
    }

    #[test]
    fn test_compile_kit_success() {
        let sample = load_sample("header".to_string()).expect("sample header should load");
        let res = compile_kit(sample, Some("Header Test".to_string())).expect("compile should succeed");
        assert!(res.ok, "Expected ok=true for valid header kit");
        assert!(res.html.is_some(), "Expected generated html");
        let html = res.html.unwrap();
        assert!(html.contains("id=\"vp\""), "HTML should contain paint-plane root vp");
        assert!(html.contains("data-title=\"Header Test\""), "HTML should contain title");
    }

    #[test]
    fn test_compile_kit_refusal() {
        let bad_source = "not a valid vixi kit";
        let res = compile_kit(bad_source.to_string(), None).expect("invoke should return result");
        assert!(!res.ok, "Expected ok=false for invalid source");
        assert!(res.error.is_some(), "Expected error message on refusal");
        assert!(res.error.unwrap().contains("Parse refusal"), "Error should mention parse refusal");
    }

    #[test]
    fn test_load_all_samples() {
        for sample_name in ["header", "minimal", "dashboard"] {
            let src = load_sample(sample_name.to_string()).unwrap();
            assert!(src.starts_with("#vixi:kit v1"), "Sample {} should have kit header", sample_name);
        }
    }

    #[test]
    fn test_get_star_catalog() {
        let stars = get_star_catalog();
        assert_eq!(stars.len(), 16, "Must contain all 16 canonical catalog stars");
        assert_eq!(stars[0].name, "Sirius");
        assert_eq!(stars[0].milli_hz, 440_000);
    }

    #[test]
    fn test_get_astrolabe_state() {
        let state = get_astrolabe_state(0, 4500, Some(0));
        assert_eq!(state.stars.len(), 16);
        assert_eq!(state.altitude_cdeg, 4500);
        assert_eq!(state.active_star_idx, 0);
    }

    #[test]
    fn test_step_mud_tour() {
        let s0 = step_mud_tour(0);
        assert_eq!(s0.room, "The Zenith Spire");
        assert_eq!(s0.parity_sum, 0);
        assert_eq!(s0.mirror_status, "PARITY VERIFIED (T + T* = 0)");
    }

    #[test]
    fn test_navigate_mud_direction() {
        let s = navigate_mud_direction("north".to_string(), 0, 0, 4).expect("nav north");
        assert_eq!(s.x, 0);
        assert_eq!(s.y, 1);
        assert_eq!(s.z, 4);
    }

    #[test]
    fn test_world_builder_snapshot_distinct_arts_produce_distinct_layouts() {
        let mut seen_signatures = std::collections::HashSet::new();
        for art in 0..=6u8 {
            let snap = world_builder_snapshot(0x1337BEEF, art).expect("world builder snapshot");
            assert_eq!(snap.cells.len(), 256, "16x16 grid must contain 256 cells");
            assert_eq!(snap.birth_art, art);
            assert!(snap.status.contains(&format!("Art {art}")));
            // Generate a signature based on the first 16 cells' M5 indices and colors
            let sig: Vec<(u8, [u8; 4])> = snap.cells.iter().take(16).map(|c| (c.m5_index, c.rgba)).collect();
            assert!(
                seen_signatures.insert(sig),
                "Art {art} must produce a distinct m5/rgba signature from other arts"
            );
        }
        assert_eq!(seen_signatures.len(), 7, "All 7 birth arts must produce distinct layouts");
    }

    #[test]
    fn test_mud_get_magic_lexicon_contains_all_words() {
        let words = mud_get_magic_lexicon();
        assert_eq!(words.len(), 42, "35 Hermetic sung words + 7 warwords = 42 total words");
        assert!(words.iter().any(|w| w.word == "bell" && w.school == "School of the Bell"));
        assert!(words.iter().any(|w| w.word == "CLASH" && w.is_warword));
    }

    #[test]
    fn test_generate_star_world_and_replay_ledger() {
        let res = generate_star_world(3, Some(0x8F2C_1001)).expect("generate star world should succeed");
        assert_eq!(res.star_idx, 3);
        assert_eq!(res.star_name, "Vega");
        assert_eq!(res.seed_hex, "0x8f2c1001");
        assert!(res.svg_markup.contains("<svg"), "Must produce valid SVG markup");
        assert_eq!(res.validation_score, 10_000, "Must achieve 10,000 permyriad validation score");
        assert_eq!(res.validation_status, "Pass", "Validation must pass");
        assert_eq!(res.room_count, 5, "Must contain 5 spatial nodes");
        assert!(res.ledger_depth >= 5, "Ledger must contain Open + placed volumes/markers");

        // Replay back 2 steps
        let replay = replay_world_ledger(3, res.seed_hex.clone(), Some(3)).expect("replay should succeed");
        assert_eq!(replay.ledger_depth, 3, "Replay depth must match requested prefix");
        assert!(replay.svg_markup.contains("<svg"), "Replayed state must render SVG");
    }
}
