// 5D Pentaract Raymarching Compute Kernel (WGSL)
// Phase 3.1 GPU Implementation: Trit-cell quantization, absence mask early-out,
// corrected 3D POM vector step, transmittance convergence gate.

struct M5Params {
    absence_mask: array<vec4<u32>, 2>, // 256-bit O(1) existence mask [8 x u32]
    sun_dir_5d: vec4<f32>,             // (X, Y, Z, T)
    scale_dim_s: f32,                  // Semantic scale axis S
    t_zero: f32,                       // Hyper-plane slice T0
    s_zero: f32,                       // Hyper-plane slice S0
    step_size: f32,                    // Raymarch delta lambda
};

@group(0) @binding(0) var<uniform> params: M5Params;
@group(0) @binding(1) var heightmap_tex: texture_2d<f32>;
@group(0) @binding(2) var heightmap_samp: sampler;
@group(0) @binding(3) var out_color: texture_storage_2d<rgba8unorm, write>;

fn check_absence_5d(cell_idx: u32) -> bool {
    if (cell_idx >= 243u) {
        return false;
    }
    let word_idx = cell_idx >> 5u;  // cell_idx / 32
    let bit_idx = cell_idx & 31u;   // cell_idx % 32
    let vec_idx = word_idx >> 2u;   // word_idx / 4
    let comp_idx = word_idx & 3u;   // word_idx % 4

    return (params.absence_mask[vec_idx][comp_idx] & (1u << bit_idx)) != 0u;
}

fn quantize_trit(val: f32) -> i32 {
    return clamp(i32(floor(val * 3.0)), -1, 1) + 1;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(out_color);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    let uv = (vec2<f32>(global_id.xy) + 0.5) / vec2<f32>(dims);
    let ray_dir_4d = normalize(vec4<f32>(uv * 2.0 - 1.0, 1.0, params.sun_dir_5d.w));

    var curr_pos_4d = vec4<f32>(uv, 0.0, params.t_zero);
    var curr_pos_s = params.s_zero;

    var ssgi_radiance = vec3<f32>(0.0);
    var godrays_accum = 0.0;
    var transmittance = 1.0;

    // Corrected 3D-Slice POM Vector Step: Δ(u,v) = [D_u, D_v] / |D_h| · Δh
    let d_h = max(abs(ray_dir_4d.z), 0.0001);
    let delta_h = params.step_size;
    let uv_step = (ray_dir_4d.xy / d_h) * delta_h;

    for (var i: u32 = 0u; i < 64u; i = i + 1u) {
        // Transmittance convergence gate: Phase 1.4 proof
        if (transmittance < 0.001) {
            break;
        }

        // Quantize 5D position to linear cell index I in [0..242]
        let tx = quantize_trit(curr_pos_4d.x);
        let ty = quantize_trit(curr_pos_4d.y);
        let tz = quantize_trit(curr_pos_4d.z);
        let tt = quantize_trit(curr_pos_4d.w);
        let ts = quantize_trit(curr_pos_s);
        let cell_idx = u32(tx + ty * 3 + tz * 9 + tt * 27 + ts * 81);

        // O(1) Absence Index Check: skip empty pentaract cells in one cycle
        if (!check_absence_5d(cell_idx)) {
            curr_pos_4d = curr_pos_4d + ray_dir_4d * delta_h;
            curr_pos_s = curr_pos_s + params.scale_dim_s * delta_h;
            continue;
        }

        // Heightmap Surface Intersection (3D spatial component)
        let h_sample = textureSampleLevel(heightmap_tex, heightmap_samp, curr_pos_4d.xy, 0.0).r;
        if (curr_pos_4d.z < h_sample) {
            let N = vec3<f32>(0.0, 0.0, 1.0);
            let n_dot_l = max(dot(N, params.sun_dir_5d.xyz), 0.0);
            ssgi_radiance = vec3<f32>(0.85, 0.75, 0.55) * n_dot_l * transmittance;
            break;
        }

        // Atmospheric Extinction & Godray Scatter
        let density = 0.04 * exp(-curr_pos_4d.z * 2.0);
        let shadow = select(1.0, 0.1, h_sample > curr_pos_4d.z);
        godrays_accum = godrays_accum + density * shadow * transmittance;
        transmittance = transmittance * exp(-density * delta_h);

        // March forward across hyper-ray
        curr_pos_4d.x = curr_pos_4d.x + uv_step.x;
        curr_pos_4d.y = curr_pos_4d.y + uv_step.y;
        curr_pos_4d.z = curr_pos_4d.z + delta_h;
        curr_pos_4d.w = curr_pos_4d.w + ray_dir_4d.w * delta_h;
        curr_pos_s = curr_pos_s + params.scale_dim_s * delta_h;
    }

    let final_color = vec4<f32>(ssgi_radiance + vec3<f32>(godrays_accum * 0.35), 1.0);
    textureStore(out_color, global_id.xy, final_color);
}
