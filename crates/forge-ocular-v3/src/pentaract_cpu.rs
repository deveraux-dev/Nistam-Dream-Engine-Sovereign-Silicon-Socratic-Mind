//! CPU reference for pentaract_march_5d.wgsl.
//!
//! Direct port of the WGSL kernel's logic: trit-quantized 5D raymarching with
//! O(1) absence-mask cell skip and transmittance early-exit.

use crate::pentaract_params::M5Params;
use forge_core_v3::atom::TritCell5D;

/// Render a 5D pentaract slice as RGBA8 pixels.
///
/// width, height: output resolution.
/// params: raymarching parameters (absence_mask, sun_dir_5d, scale_dim_s, t_zero, s_zero, step_size).
/// heightmap_fn: closure to sample heightmap at (u, v) — returns z height at that point.
///
/// Returns RGBA8 buffer (width * height * 4 bytes).
pub fn render_pentaract_cpu<F>(
    width: u32,
    height: u32,
    params: &M5Params,
    mut heightmap_fn: F,
) -> Vec<u8>
where
    F: FnMut(u32, u32) -> f32,
{
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    for py in 0..height {
        for px in 0..width {
            let uv_x = px as f32 / width as f32;
            let uv_y = py as f32 / height as f32;

            let pixel = march_ray(params, uv_x, uv_y, &mut heightmap_fn);
            let i = ((py * width + px) * 4) as usize;
            rgba[i] = pixel[0];
            rgba[i + 1] = pixel[1];
            rgba[i + 2] = pixel[2];
            rgba[i + 3] = pixel[3];
        }
    }

    rgba
}

fn march_ray<F>(params: &M5Params, uv_x: f32, uv_y: f32, heightmap_fn: &mut F) -> [u8; 4]
where
    F: FnMut(u32, u32) -> f32,
{
    let mut radiance = [0.0f32; 3];
    let mut transmittance = 1.0f32;

    const MAX_STEPS: i32 = 64;

    for step in 0..MAX_STEPS {
        if transmittance < 0.001 {
            break;
        }

        let t = step as f32;
        let s = params.s_zero + t * params.step_size;

        let pos_x = params.t_zero + uv_x * params.scale_dim_s;
        let pos_y = params.t_zero + uv_y * params.scale_dim_s;

        let trits = [
            quantize_trit((pos_x + t * 0.1) as i32),
            quantize_trit((pos_y + t * 0.1) as i32),
            quantize_trit((s * 0.1) as i32),
            0i8,
            0i8,
        ];

        let cell = TritCell5D::from_trits(trits);
        if cell.is_sentinel() {
            continue;
        }

        if !params.absence_mask.is_empty() && check_absence_5d(params, cell) {
            continue;
        }

        let z = heightmap_fn((pos_x * 0.5) as u32, (pos_y * 0.5) as u32);

        if s > z {
            let normal = [0.0, 1.0, 0.0];
            let illum = dot(normal, params.sun_dir_5d);
            let light_contrib = illum.max(0.0) * transmittance;
            radiance[0] += 0.5 * light_contrib;
            radiance[1] += 0.5 * light_contrib;
            radiance[2] += 0.5 * light_contrib;

            transmittance *= 0.8;
        }
    }

    let sky = [0.3, 0.5, 0.8];
    let final_color = [
        ((radiance[0] + sky[0] * transmittance).clamp(0.0, 1.0) * 255.0) as u8,
        ((radiance[1] + sky[1] * transmittance).clamp(0.0, 1.0) * 255.0) as u8,
        ((radiance[2] + sky[2] * transmittance).clamp(0.0, 1.0) * 255.0) as u8,
        255,
    ];

    final_color
}

fn quantize_trit(v: i32) -> i8 {
    if v < -1 {
        -1
    } else if v > 1 {
        1
    } else {
        v as i8
    }
}

fn check_absence_5d(params: &M5Params, cell: TritCell5D) -> bool {
    if params.absence_mask.len() < 2 {
        return false;
    }
    let idx = (cell.0 as usize) / 32;
    let bit = (cell.0 as usize) % 32;
    if idx >= params.absence_mask.len() {
        false
    } else {
        (params.absence_mask[idx] & (1u32 << bit)) != 0
    }
}

fn dot(a: [f32; 3], b: [f32; 4]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
