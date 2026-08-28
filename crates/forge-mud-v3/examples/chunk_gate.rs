//! Chunk gate — raymarch the live `World5D` lattice to a big 3D image.
//! CPU-only, sampling the lattice the same way the sim collides with it.
//! Emits discriminator stats + fail-loud exit 2 if the frame is uniform or land-starved.
//!
//! cargo run -q -p forge-mud-v3 --example chunk_gate

use forge_core_v3::zones::sky_irradiance;
use forge_mud_v3::world5d::{World5D, WORLD_HALF};
use forge_mud_v3::zone::{Zone, Domain, Island, CELL_MILLI};
use forge_sieve_v3::prime_seed::prime_seed;

const W: u32 = 1280;
const H: u32 = 720;
const OUT_DIR: &str = "F:/v3/.forge/photons";
/// Steps of one third of a cell — fine enough that a one-cell shore is not skipped.
const STEP_MU: i64 = 333;
const MAX_STEPS: i32 = 1800; // doubled for 1280x720 (1280/960 = 1.33x)

fn main() {
    // Sung world: seed formula from cdk::word_world_line (per task spec).
    let seed = prime_seed("thorn", 64);
    let zone = Zone::new(Domain::Water)
        .with_water_level(0)
        .with_island(Island::new(4 + (seed % 12) as i64, 8 + ((seed >> 8) % 24) as i64));
    let world = World5D::island(&zone, seed);

    // Camera: backed off the isle, y ~6 cells up, yaw 225°, pitch ~-18°.
    let eye_x_mm = -22 * CELL_MILLI;
    let eye_y_mm = 6 * CELL_MILLI;
    let eye_z_mm = -22 * CELL_MILLI;
    let yaw = 225.0f32.to_radians();
    let pitch = -18.0f32.to_radians();
    let waterline_mu = zone.water_level_cells * 2 * CELL_MILLI;

    let (fwd, right, up) = basis(yaw, pitch);
    let aspect = W as f32 / H as f32;
    let half_fov = (75.0f32.to_radians() / 2.0).tan();

    let mut rgba = vec![0u8; (W * H * 4) as usize];
    for py in 0..H {
        for px in 0..W {
            let sx = (2.0 * (px as f32 + 0.5) / W as f32 - 1.0) * half_fov * aspect;
            let sy = (1.0 - 2.0 * (py as f32 + 0.5) / H as f32) * half_fov;
            let dir = [
                fwd[0] + right[0] * sx + up[0] * sy,
                fwd[1] + right[1] * sx + up[1] * sy,
                fwd[2] + right[2] * sx + up[2] * sy,
            ];
            let (px_rgba, hit) = march(&world, &zone, (eye_x_mm, eye_y_mm, eye_z_mm), dir, waterline_mu);
            let i = ((py * W + px) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&px_rgba);
            // The march's hit verdict IS the land marker the discriminators
            // read (alpha 254 = sky/sea, normalized back to 255 pre-PNG).
            rgba[i + 3] = if hit { 255 } else { 254 };
        }
    }

    // Discriminators: uniform flag, land hit count, and spread of luminance.
    // Discriminators, drowned-world honest: solid hits carry alpha 255 (the
    // march's marker), sky/sea carry 254. Land = solid-hit pixels — warm OR
    // submerged-blue — and the lit spread is measured over those same pixels.
    let uniform = rgba.chunks_exact(4).all(|p| p == &rgba[..4]);
    let land_hits = rgba.chunks_exact(4).filter(|p| p[3] == 255).count();
    let lum: Vec<u32> = rgba
        .chunks_exact(4)
        .filter(|p| p[3] == 255)
        .map(|p| p[0] as u32 + p[1] as u32 + p[2] as u32)
        .collect();
    let spread = lum.iter().max().copied().unwrap_or(0) - lum.iter().min().copied().unwrap_or(0);
    let green = !uniform && land_hits > (W * H / 40) as usize && spread > 60;
    // Normalize the marker back to full alpha before the PNG leaves the gate.
    for px in rgba.chunks_exact_mut(4) {
        px[3] = 255;
    }

    std::fs::create_dir_all(OUT_DIR).ok();
    let name = if green { "chunk_gate.png" } else { "chunk_gate.FAILED.png" };
    let path = format!("{OUT_DIR}/{name}");

    // [APERTURE] NOT wired to `shell/src/effects.rs`'s CPU-twin grade/
    // bloom chain (`cpu_grade`/`cpu_threshold`/`cpu_blur_h`/`cpu_blur_v`/
    // `cpu_vibe`/`cpu_third`): `studio-shell` has no `[lib]` target (bin-
    // only crate, its own excluded workspace — `shell/Cargo.toml`'s own
    // module doc names it a deliberately firewalled window frame), so
    // nothing outside that binary can import those functions, and
    // `forge-mud-v3` cannot add it as a dependency without inventing a
    // cross-crate edge this plan didn't scope. The raymarch-side bridge
    // (`shell/src/compose.rs::chiaroscuro_layer_plane`) is the real, wired
    // path to that chain for `render_chiaroscuro_composite`'s output; this
    // example's raw PNG stays raw pending either a shared LUT/bloom crate
    // or a `[lib]` target on `studio-shell` — named blocker, not silently
    // skipped (L15).
    // Encode and write PNG using the png crate.
    let mut png_buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_buf, W, H);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("create PNG encoder");
        writer.write_image_data(&rgba).expect("write PNG data");
        writer.finish().expect("finish PNG encoding");
    }
    std::fs::write(&path, png_buf).expect("write chunk png");

    println!("[chunk_gate] {path}");
    println!(
        "[chunk_gate] uniform={uniform} warm_land_px={land_hits} of {} lit_spread={spread}",
        W * H
    );
    if !green {
        eprintln!("[chunk_gate] FAIL-LOUD: the chunk did not reach pixels lit");
        std::process::exit(2);
    }
    println!("[chunk_gate] GREEN — the lattice reached pixels, lit and solid");
}

/// Camera basis, Y-up, forward = -Z at yaw 0 (the BDO rig's convention).
fn basis(yaw: f32, pitch: f32) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let fwd = [-sy * cp, sp, -cy * cp];
    let right = [cy, 0.0, -sy];
    let up = [
        right[1] * fwd[2] - right[2] * fwd[1],
        right[2] * fwd[0] - right[0] * fwd[2],
        right[0] * fwd[1] - right[1] * fwd[0],
    ];
    (fwd, right, up)
}

/// Surface normal from the lattice itself: every air neighbour pushes the normal
/// outward, so a flat top reads +Y and a shoulder reads diagonal. No stored normals.
fn lattice_normal(world: &World5D, cell: forge_mud_v3::zone::Cell) -> [f32; 3] {
    let mut n = [0.0f32; 3];
    for (axis, d) in [(0usize, 1i64), (1, 1), (2, 1)] {
        let plus = match axis {
            0 => forge_mud_v3::zone::Cell::spatial(cell.x + d, cell.y, cell.z),
            1 => forge_mud_v3::zone::Cell::spatial(cell.x, cell.y + d, cell.z),
            _ => forge_mud_v3::zone::Cell::spatial(cell.x, cell.y, cell.z + d),
        };
        let minus = match axis {
            0 => forge_mud_v3::zone::Cell::spatial(cell.x - d, cell.y, cell.z),
            1 => forge_mud_v3::zone::Cell::spatial(cell.x, cell.y - d, cell.z),
            _ => forge_mud_v3::zone::Cell::spatial(cell.x, cell.y, cell.z - d),
        };
        n[axis] += if world.is_solid(plus) { 0.0 } else { 1.0 };
        n[axis] -= if world.is_solid(minus) { 0.0 } else { 1.0 };
    }
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len < 1e-6 {
        [0.0, 1.0, 0.0]
    } else {
        [n[0] / len, n[1] / len, n[2] / len]
    }
}

/// Fraction of `budget` cells directly above `cell` that are clear (not
/// solid) — `1.0` fully open to sky, `0.0` capped immediately by an
/// overhang. Same bounded-march discipline as `forge_core_v3::zones::
/// sky_irradiance::sky_visibility`, over `World5D::is_solid` instead of
/// `SparseChunkGrid::get` — the two worlds use different storage, the sky
/// math this feeds does not.
fn sky_visibility_over_world(world: &World5D, cell: forge_mud_v3::zone::Cell, budget: i64) -> f32 {
    if budget == 0 {
        return 1.0;
    }
    let mut clear = 0i64;
    for step in 1..=budget {
        let above = forge_mud_v3::zone::Cell::spatial(cell.x, cell.y + step, cell.z);
        if world.is_solid(above) {
            break;
        }
        clear += 1;
    }
    clear as f32 / budget as f32
}

/// One ray through the lattice. Returns the shaded pixel and whether it hit solid.
fn march(
    world: &World5D,
    _zone: &Zone,
    origin: (i64, i64, i64),
    dir: [f32; 3],
    waterline_mu: i64,
) -> ([u8; 4], bool) {
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-6);
    let d = [dir[0] / len, dir[1] / len, dir[2] / len];
    let mut crossed_water = false;

    for step in 1..=MAX_STEPS {
        let t = (step as i64) * STEP_MU;
        let p = (
            origin.0 + (d[0] * t as f32) as i64,
            origin.1 + (d[1] * t as f32) as i64,
            origin.2 + (d[2] * t as f32) as i64,
        );
        let cell = forge_mud_v3::zone::Cell::spatial(
            p.0.div_euclid(CELL_MILLI),
            p.1.div_euclid(CELL_MILLI),
            p.2.div_euclid(CELL_MILLI),
        );
        if cell.x.abs() > WORLD_HALF || cell.y.abs() > WORLD_HALF || cell.z.abs() > WORLD_HALF {
            break;
        }
        if p.1 <= waterline_mu {
            crossed_water = true;
        }
        if world.is_solid(cell) {
            // Fixed sun direction: facing southeast, mid-high angle.
            let sun_dir = (0.707f32, 0.5, 0.5); // normalized
            let normal = lattice_normal(world, cell);
            let normal_t = (normal[0], normal[1], normal[2]);
            let sun_dot = (normal[0] * sun_dir.0 + normal[1] * sun_dir.1 + normal[2] * sun_dir.2).max(0.0);

            // Occlusion-gated sky irradiance (same fix as `raymarch_5d.rs`'s
            // chiaroscuro pass, this repo's own `sky_visibility` discipline
            // applied over World5D's own `is_solid` instead of
            // `SparseChunkGrid` — the two storage backends differ, the sky
            // math (`forge_core_v3::zones::sky_irradiance`) does not) —
            // replaces the old flat `0.3 + 0.7*sun_dot` constant-ambient
            // formula, so front faces get real sky fill and anything under
            // an overhang stays dark instead of matching open sky.
            let visibility = sky_visibility_over_world(world, cell, 12);
            let sky = sky_irradiance::hemispheric_irradiance(
                normal_t, sun_dir, (1.0, 1.0, 1.0), 1.0, 2.0, visibility,
            );
            let lit = sun_dot + 0.5 * (sky.0 + sky.1 + sky.2) / 3.0;

            // Material colors: stone, soil, etc. Simplified.
            let (r, g, b) = if crossed_water {
                // Underwater: blue-grey
                ((60.0 * lit).clamp(0.0, 255.0) as u8, (80.0 * lit).clamp(0.0, 255.0) as u8, (120.0 * lit).clamp(0.0, 255.0) as u8)
            } else {
                // Land: brown-tan
                ((150.0 * lit).clamp(0.0, 255.0) as u8, (130.0 * lit).clamp(0.0, 255.0) as u8, (100.0 * lit).clamp(0.0, 255.0) as u8)
            };
            return ([r, g, b, 255], true);
        }
    }

    // Sky: simple vertical gradient. Below waterline the ray read as sea.
    let sky = if crossed_water {
        [12, 34, 68, 255]
    } else {
        // Vertical gradient: sky blue at horizon, darker blue at zenith.
        let sky_brightness = (0.5 + 0.5 * d[1].max(-0.2).min(1.0)) as u8;
        let base = (100 + sky_brightness) as u8;
        [base, base + 50, 180 + sky_brightness, 255]
    };
    (sky, false)
}
