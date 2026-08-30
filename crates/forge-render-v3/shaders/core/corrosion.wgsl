// Corrosion visualization shader — blends clean steel → rust based on corrosion_pct.
// Feeds from creature_engine PhysicalProfile via inspection bridge.
// corrosion_pct: 0.0 = clean steel (grey, high metallic), 1.0 = fully rusted (orange, high roughness)

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_position: vec3<f32>,
};

struct Uniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    camera_pos: vec3<f32>,
    corrosion_pct: f32,
    cui_risk: f32,        // 0=low, 1=med, 2=high, 3=critical
    time: f32,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.mvp * vec4<f32>(in.position, 1.0);
    out.world_normal = (u.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.uv = in.uv;
    out.world_position = (u.model * vec4<f32>(in.position, 1.0)).xyz;
    return out;
}

// Simple hash for procedural noise
fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

// Value noise
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

// FBM noise for corrosion pattern
fn corrosion_noise(uv: vec2<f32>) -> f32 {
    var val = 0.0;
    var amp = 0.5;
    var freq = 4.0;
    for (var i = 0; i < 4; i++) {
        val += noise(uv * freq) * amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    return val;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(in.world_normal);
    let V = normalize(u.camera_pos - in.world_position);

    // Light direction (simple directional)
    let L = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let NdotL = max(dot(N, L), 0.0);
    let ambient = 0.15;

    // Corrosion pattern — spatially varying
    let corr_noise = corrosion_noise(in.uv * 8.0);
    let local_corrosion = clamp(u.corrosion_pct + (corr_noise - 0.5) * 0.3, 0.0, 1.0);

    // Clean steel: rgb(0.45, 0.45, 0.48), high metallic
    let clean_color = vec3<f32>(0.45, 0.45, 0.48);
    // Rust: rgb(0.63, 0.31, 0.16)
    let rust_color = vec3<f32>(0.63, 0.31, 0.16);
    // Deep rust: rgb(0.35, 0.15, 0.08)
    let deep_rust = vec3<f32>(0.35, 0.15, 0.08);

    // Blend based on local corrosion
    var base_color: vec3<f32>;
    if (local_corrosion < 0.5) {
        base_color = mix(clean_color, rust_color, local_corrosion * 2.0);
    } else {
        base_color = mix(rust_color, deep_rust, (local_corrosion - 0.5) * 2.0);
    }

    // Roughness increases with corrosion (affects specular)
    let roughness = mix(0.3, 0.9, local_corrosion);
    let metallic = mix(0.9, 0.1, local_corrosion);

    // Simple PBR-ish: diffuse + specular
    let H = normalize(L + V);
    let NdotH = max(dot(N, H), 0.0);
    let spec_power = mix(64.0, 4.0, roughness);
    let specular = pow(NdotH, spec_power) * metallic;

    let lit = base_color * (ambient + NdotL * 0.7) + vec3<f32>(specular);

    // CUI risk overlay: pulse red border for critical segments
    var final_color = lit;
    if (u.cui_risk >= 2.5) {
        // Critical — red pulse
        let pulse = sin(u.time * 3.0) * 0.5 + 0.5;
        let edge = smoothstep(0.0, 0.1, min(in.uv.x, min(in.uv.y, min(1.0 - in.uv.x, 1.0 - in.uv.y))));
        final_color = mix(vec3<f32>(1.0, 0.0, 0.0) * pulse, final_color, edge);
    } else if (u.cui_risk >= 1.5) {
        // High — orange tint
        final_color = mix(final_color, vec3<f32>(0.9, 0.4, 0.1), 0.15);
    }

    return vec4<f32>(final_color, 1.0);
}
