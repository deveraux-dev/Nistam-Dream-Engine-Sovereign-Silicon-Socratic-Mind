// ENB Crepuscular God-Rays & Dithered Star Bloom Shader (WGSL)
// Adapted for 13Forge v3 5D Pentaract & Celestial Star Dome Engine.
// Transpiled/normalized from ENB .fx / HLSL radial ray-march pipeline.

struct GodRayUniforms {
    light_pos_ss: vec2<f32>,     // Screen-space light origin [0..1, 0..1]
    density: f32,                // Step spacing multiplier
    weight: f32,                 // Sample weight per step
    decay: f32,                  // Exponential attenuation factor
    exposure: f32,               // Final light shaft exposure
    num_samples: u32,            // Ray-march iteration count (default 32..64)
    glaze_intensity: f32,        // Permyriad glaze intensity [0.0..1.0]
    haze_color: vec4<f32>,       // Atmospheric haze / bio-film tint
};

@group(0) @binding(0) var<uniform> u_godrays: GodRayUniforms;
@group(0) @binding(1) var t_occlusion: texture_2d<f32>;
@group(0) @binding(2) var s_occlusion: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle generation from vertex_index (0, 1, 2)
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vertex_index << 1u) & 2) * 2.0 - 1.0;
    let y = f32(i32(vertex_index & 2u) * -2) + 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

// 8x8 Bayer ordered dithering threshold lookup (values normalized to 0.0..1.0)
fn bayer8_threshold(pixel_coord: vec2<u32>) -> f32 {
    let bayer = array<f32, 64>(
         0.0/64.0, 32.0/64.0,  8.0/64.0, 40.0/64.0,  2.0/64.0, 34.0/64.0, 10.0/64.0, 42.0/64.0,
        48.0/64.0, 16.0/64.0, 56.0/64.0, 24.0/64.0, 50.0/64.0, 18.0/64.0, 58.0/64.0, 26.0/64.0,
        12.0/64.0, 44.0/64.0,  4.0/64.0, 36.0/64.0, 14.0/64.0, 46.0/64.0,  6.0/64.0, 38.0/64.0,
        60.0/64.0, 28.0/64.0, 52.0/64.0, 20.0/64.0, 62.0/64.0, 30.0/64.0, 54.0/64.0, 22.0/64.0,
         3.0/64.0, 35.0/64.0, 11.0/64.0, 43.0/64.0,  1.0/64.0, 33.0/64.0,  9.0/64.0, 41.0/64.0,
        51.0/64.0, 19.0/64.0, 59.0/64.0, 27.0/64.0, 49.0/64.0, 17.0/64.0, 57.0/64.0, 25.0/64.0,
        15.0/64.0, 47.0/64.0,  7.0/64.0, 39.0/64.0, 13.0/64.0, 45.0/64.0,  5.0/64.0, 37.0/64.0,
        63.0/64.0, 31.0/64.0, 55.0/64.0, 23.0/64.0, 61.0/64.0, 29.0/64.0, 53.0/64.0, 21.0/64.0
    );
    let idx = (pixel_coord.y % 8u) * 8u + (pixel_coord.x % 8u);
    return bayer[idx];
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var text_coord = in.uv;
    let delta_tex_coord = (text_coord - u_godrays.light_pos_ss) * (1.0 / f32(u_godrays.num_samples)) * u_godrays.density;
    
    var color = textureSample(t_occlusion, s_occlusion, text_coord);
    var illumination_decay = 1.0;
    var accumulated_rays = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    for (var i = 0u; i < u_godrays.num_samples; i = i + 1u) {
        text_coord = text_coord - delta_tex_coord;
        var sample_val = textureSample(t_occlusion, s_occlusion, text_coord);
        sample_val = sample_val * illumination_decay * u_godrays.weight;
        accumulated_rays = accumulated_rays + sample_val;
        illumination_decay = illumination_decay * u_godrays.decay;

        // Transmittance convergence: exit early when contribution falls below perceptual threshold
        // Phase 1.4 proof: prevents warp divergence & thread stalls on low-density regions
        if (illumination_decay < 0.001) {
            break;
        }
    }

    // Multiply by exposure and atmospheric haze tint
    var final_rays = accumulated_rays * u_godrays.exposure * u_godrays.haze_color;

    // 8x8 Bayer Dither Quantization to match 13Forge aesthetic
    let pixel_pos = vec2<u32>(u32(in.position.x), u32(in.position.y));
    let dither_threshold = bayer8_threshold(pixel_pos);
    
    let ray_lum = dot(final_rays.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let dither_mask = select(0.0, 1.0, (ray_lum * u_godrays.glaze_intensity) > (dither_threshold * 0.4));
    
    // Blend final god-ray shafts with base occlusion frame
    let composite = color + final_rays * (0.6 + 0.4 * dither_mask);
    return vec4<f32>(composite.rgb, color.a);
}
