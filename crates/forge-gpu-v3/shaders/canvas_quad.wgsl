struct Uniforms {
    vp_w: f32,
    vp_h: f32,
    time: f32,
    vibe_glow: f32,
    vibe_shake: f32,
    vibe_chromatic: f32,
    vibe_pulse: f32,
    _pad: f32,
    // NO audio_* here. Four f32 (rms, beat_phase, sub_bass, spectrum_high) were declared
    // and forwarded every frame with ZERO reads in this shader — declared, never exercised
    // (root#rank). Audio reaches pixels ONLY through the derived vibe_* scalars above
    // (root#audio-inert). Re-adding a raw audio uniform requires a WGSL read in the same
    // commit, or vibe_bus_frozen.rs goes RED.
    smithy_tex_enabled: u32,
    glass_tilt: f32,
    glass_opacity_q: u32,
    _pad4: u32,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

struct MaterialParams {
    albedo_packed: u32,
    roughness_pmy: u32,  // u16 pair: roughness_pmy | refraction_pmy
    emission_pmy: u32,   // u16 pair: emission_pmy | _reserved
    _pad: u32,
}
@group(1) @binding(0) var<storage, read> material_palette: array<MaterialParams>;
@group(2) @binding(0) var screen_texture: texture_2d<f32>;

// DODRegistries: packed material data from .forge_reg + dispatch descriptor.
// Mirrors forge_core::forge_reg::GpuMaterialEntry (64 bytes). Canvas shader reads
// only the first 4 fields (offsets 0-15); PBR shader reads all. The shader switches
// on `kind` (data) — there is NO hardcoded `mat_idx == N` branch. `params` low
// byte = Smithy array layer for KIND_TEXTURED.
struct GpuRegistryEntry {
    colour: u32,
    roughness_metallic: u32,
    kind: u32,
    params: u32,
    // ── PBR fields (canvas ignores, stride must match) ───────────────────
    texture_index: u32,
    normal_index: u32,
    ao_index: u32,
    flags: u32,
    emission: u32,
    detail_lut: u32,
    ring_freq_attack: u32,
    hardness_mass: u32,
    flammability_flags: u32,
    destruction_charpy: u32,
    _reserved0: u32,
    _reserved1: u32,
}
@group(3) @binding(0) var<storage, read> dod_registry: array<GpuRegistryEntry>;

// Material dispatch kinds — mirror of forge_core::forge_reg::KIND_* (append only).
const KIND_DEFAULT: u32 = 0u;
const KIND_GUNMETAL: u32 = 1u;
const KIND_GLASS: u32 = 2u;
const KIND_HOLOGRAM: u32 = 3u;
const KIND_TEXTURED: u32 = 4u;

// Smithy material atlas — texture_2d_array folded into group 3 (was a separate
// @group(4), which exceeded wgpu's max_bind_groups=4 and crashed shader-module
// creation). One ARRAY LAYER per material slot; the substrate scales to N
// materials by adding layers, NOT new bind groups. This is the first brick of
// the unified vixel substrate (1 pixel = 1 voxel = 1 vixel): 2D chrome quads and
// 3D mesh materials index the SAME array-indexed material storage.
// Layers: 0=parchment 1=bronze 2=wood 3=vellum 4=cobblestone.
// When the smithy_tex_enabled bit for a slot is 0 the shader uses the procedural
// apply_*_proc fallback and never samples the array.
@group(3) @binding(1) var t_smithy: texture_2d_array<f32>;
@group(3) @binding(2) var s_smithy: sampler;

// ESSENCE_LUMINANCE LUT (forge_core essence_registry): one u32 (Permyriad) per
// essence slot. Indexed by the per-quad essence_id (one-based in packed_flags,
// 0 = inert). Drives resonance-RESPONSE glow — what a cell MEANS sets how
// brightly it answers the vibe/aura field (§6). Folded into group 3 (no @group(4)).
@group(3) @binding(3) var<storage, read> essence_luminance: array<u32>;

struct Instance {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: u32,
    // color_bottom occupies offset 20 in QuadInstance (gradient bottom colour).
    // The quad shader renders solid colour only, but this MUST be declared so the
    // vertex attribute offsets stay aligned with the Rust struct — without it,
    // radius/outline/rot all read 4 bytes early and quads render invisible.
    @location(3) color_bottom: u32,
    @location(4) radius: f32,
    @location(5) outline: f32,
    @location(6) thickness: f32,
    @location(7) packed_flags: u32,
    @location(8) rot: vec2<f32>,
}

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) quad_color: vec4<f32>,
    @location(2) corner_radius: f32,
    @location(3) is_outline: f32,
    @location(4) stroke_thickness: f32,
    @location(5) quad_size: vec2<f32>,
    @location(6) @interpolate(flat) packed_flags: u32,
}

// sRGB → LINEAR, and this is the whole bug (2026-07-30). The live surface is forced
// to an *_Srgb format (main.rs:2906) precisely so the hardware re-encodes on store,
// and `ClearColour::resolve` decodes its palette to match. This function did not: it
// divided by 255 and handed the shader sRGB bytes pretending to be linear, so every
// quad, glyph and gradient got encoded once too many while the clear got it right.
// An authored #0A0705 ground measured #382E26 on glass — +0x2E, uniform, on every
// channel. Rust twin + the transfer function's one home: forge_core::correspondence.
//
// ALPHA IS NOT DECODED. It is coverage, not light; running it through the curve
// would darken every soft edge and antialiased glyph in the engine.
fn srgb_to_linear_ch(s: f32) -> f32 {
    if s <= 0.04045 {
        return s / 12.92;
    }
    return pow((s + 0.055) / 1.055, 2.4);
}

fn unpack(c: u32) -> vec4<f32> {
    return vec4<f32>(
        srgb_to_linear_ch(f32((c >> 24u) & 0xFFu) / 255.0),
        srgb_to_linear_ch(f32((c >> 16u) & 0xFFu) / 255.0),
        srgb_to_linear_ch(f32((c >>  8u) & 0xFFu) / 255.0),
        f32( c         & 0xFFu) / 255.0,
    );
}

var<private> QUAD: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VOut {
    let q = QUAD[vi];
    // Rotate the corner about the quad center by inst.rot (cos, sin). Identity
    // (1,0) reproduces the axis-aligned position exactly — rects/circles unchanged;
    // a thin quad with rot = direction unit-vector draws an oriented line.
    let half = inst.size * 0.5;
    let local = q * inst.size - half;
    let rotated = vec2<f32>(
        local.x * inst.rot.x - local.y * inst.rot.y,
        local.x * inst.rot.y + local.y * inst.rot.x,
    );
    let px = inst.pos + half + rotated;
    let ndc_x = px.x / u.vp_w * 2.0 - 1.0;
    let ndc_y = -(px.y / u.vp_h * 2.0 - 1.0);

    // Perspective shear for KIND_GLASS quads: horizontal lean proportional to NDC-Y.
    // select() avoids the banned "if mat_idx == N" pattern — geometry-only, not dispatch.
    let tilt_ndc = select(0.0, u.glass_tilt * ndc_y, (inst.packed_flags & 0xFFu) == KIND_GLASS);
    var out: VOut;
    out.clip = vec4<f32>(ndc_x + tilt_ndc, ndc_y, 0.0, 1.0);
    out.local_uv = q;
    out.quad_color = unpack(inst.color);
    out.corner_radius = inst.radius;
    out.is_outline = inst.outline;
    out.stroke_thickness = inst.thickness;
    out.quad_size = inst.size;
    out.packed_flags = inst.packed_flags;
    return out;
}

// ── Material branches ────────────────────────────────────────────────

// ── Procedural noise (shared) ────────────────────────────────────────

fn hash_f(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn noise_2d(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u_smooth = f * f * (3.0 - 2.0 * f);
    let a = hash_f(i);
    let b = hash_f(i + vec2<f32>(1.0, 0.0));
    let c = hash_f(i + vec2<f32>(0.0, 1.0));
    let d = hash_f(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u_smooth.x), mix(c, d, u_smooth.x), u_smooth.y);
}

fn apply_gunmetal(base: vec4<f32>, roughness_pmy_raw: u32, uv: vec2<f32>, quad_size: vec2<f32>) -> vec4<f32> {
    let roughness = f32(roughness_pmy_raw & 0xFFFFu) / 10000.0;
    let light_dir = normalize(vec3<f32>(0.3, -0.7, 0.5));
    let normal = vec3<f32>(0.0, 0.0, 1.0);
    let half_vec = normalize(light_dir + vec3<f32>(0.0, 0.0, 1.0));
    let ndoth = max(dot(normal, half_vec), 0.0);
    let spec_power = mix(128.0, 4.0, roughness);
    let specular = pow(ndoth, spec_power) * (1.0 - roughness);

    // Brushed metal grain (horizontal directional noise, tiled by quad size)
    let grain_uv = uv * quad_size * 0.02; // scale to physical pixels
    let grain = noise_2d(vec2<f32>(grain_uv.x * 8.0, grain_uv.y * 0.5)) * 0.04 - 0.02;

    // Subtle concentric machining marks (radial from center)
    let center_dist = length(uv - 0.5) * 2.0;
    let machining = sin(center_dist * 60.0) * 0.008 * (1.0 - roughness);

    let detail = grain + machining;
    return vec4<f32>(base.rgb + specular * vec3<f32>(0.8, 0.85, 0.9) + detail, base.a);
}

fn apply_glass(base: vec4<f32>, screen_pos: vec2<f32>, refraction_pmy_raw: u32, uv: vec2<f32>) -> vec4<f32> {
    // Fresnel rim: 1.0 at quad edges, 0.0 at center.
    // glass_opacity_q gates the global strength (0=sand phase, 10000=crystallised).
    let edge_dist = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let fres_edge = 1.0 - clamp(edge_dist * 4.0, 0.0, 1.0);
    let fres = fres_edge * (f32(u.glass_opacity_q) / 10000.0);

    // Scene behind the glass: refract proportional to Fresnel rim
    let ref_str = f32((refraction_pmy_raw >> 16u) & 0xFFFFu) / 100.0;
    let i_uv = vec2<i32>(screen_pos) + vec2<i32>(i32(fres * ref_str), 0);
    let scene = textureLoad(screen_texture, i_uv, 0).rgb;
    let lum = dot(scene, vec3<f32>(0.2126, 0.7152, 0.0722));

    // Derive prism (chromatic rim shift) and facet (scene ambient bleed)
    let tint = base.rgb;
    let prism = vec3<f32>(tint.r * 1.1, tint.g, tint.b * 0.95);
    let facet = scene * 0.04;

    // 6-line glass recipe: Fresnel-rim + lum-keeps-text-solid
    let glass = tint * (0.04 + fres * 0.5)
              + prism * (fres * 0.55)
              + facet;
    let alpha = clamp(0.12 + fres * 0.55 + lum * 0.97, 0.0, 1.0);

    // Bloom: additive edge glow driven by vibe_glow (artifact_glow proxy)
    let bloom_lum = dot(glass, vec3<f32>(0.2126, 0.7152, 0.0722));
    let bloom = max(0.0, bloom_lum - 0.8) * u.vibe_glow * 2.0;

    return vec4<f32>(clamp(glass + glass * bloom, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}

fn apply_hologram(base: vec4<f32>, uv: vec2<f32>, emission_pmy_raw: u32) -> vec4<f32> {
    let emission = f32(emission_pmy_raw & 0xFFFFu) / 10000.0;
    let scanline_freq = 80.0;
    let scanline = sin(uv.y * scanline_freq * 3.14159) * 0.5 + 0.5;
    let scanline_mask = smoothstep(0.3, 0.7, scanline);

    // Chromatic split (RGB offset by scanline phase)
    let r_shift = sin(uv.y * scanline_freq * 3.14159 + 0.3) * 0.5 + 0.5;
    let b_shift = sin(uv.y * scanline_freq * 3.14159 - 0.3) * 0.5 + 0.5;
    let chroma = vec3<f32>(base.r * r_shift, base.g * scanline_mask, base.b * b_shift);

    // Flicker (time-driven subtle intensity variation)
    let flicker = 0.9 + 0.1 * sin(u.time * 17.0 + uv.y * 5.0);

    let bloom_color = chroma * emission * flicker;
    let holo_rgb = base.rgb * 0.5 + bloom_color;
    return vec4<f32>(holo_rgb, base.a * (0.4 + 0.6 * scanline_mask));
}

// ── VibeMatrix injection (post-material) ─────────────────────────────

fn apply_parchment_proc(uv: vec2<f32>) -> vec3<f32> {
    // Smithy log/journal surface — warm bone, 24-cell paper grain, edge-darkening.
    // Amplitude tier: chrome_floating 5% max chroma drift.
    let base = vec3<f32>(0.902, 0.831, 0.706); // #E6D4B4
    let cell_scale = 24.0;
    let cell_uv = uv * cell_scale;
    let cell_id = floor(cell_uv);
    let cell_fract = fract(cell_uv);

    // Per-cell hash for faint grain variation (max 5% = 0.05 drift)
    let h = hash_f(cell_id);
    let jitter = (h - 0.5) * 0.05; // range [-0.025, +0.025]

    // Soft edge-darkening: feather within 0.04 of cell edge
    let border = min(min(cell_fract.x, 1.0 - cell_fract.x), min(cell_fract.y, 1.0 - cell_fract.y));
    let mortar = smoothstep(0.0, 0.04, border);

    // Faint surface noise for paper texture
    let surface_noise = noise_2d(cell_uv * 2.5) * 0.018 - 0.009;

    let paper = base + jitter + surface_noise;
    return clamp(paper * mortar * 0.88 + paper * 0.12, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_bronze_proc(uv: vec2<f32>) -> vec3<f32> {
    // Smithy active focal surface — hammered bronze, 12-cell dent pattern, ember underglow.
    // Amplitude tier: audio_reactive_peak 12% max chroma drift.
    let base = vec3<f32>(0.659, 0.439, 0.267); // #A87044
    let cell_scale = 12.0;
    let cell_uv = uv * cell_scale;
    let cell_id = floor(cell_uv);
    let cell_fract = fract(cell_uv);

    // Per-cell hash for hammer-dent highlight variation (max 5% per chrome_floating tier)
    let h = hash_f(cell_id);
    let jitter = (h - 0.5) * 0.10; // range [-0.05, +0.05]

    // Hammer-dent border: wider mortar border for visible dent seams
    let border = min(min(cell_fract.x, 1.0 - cell_fract.x), min(cell_fract.y, 1.0 - cell_fract.y));
    let mortar = smoothstep(0.0, 0.10, border);

    // Ember underglow: warm red-orange tint at cell centers
    let center_dist = length(cell_fract - vec2<f32>(0.5));
    let ember = max(0.0, 0.3 - center_dist) * 0.08;
    let ember_tint = vec3<f32>(ember, ember * 0.4, 0.0);

    // Per-cell surface noise
    let surface_noise = noise_2d(cell_uv * 4.0) * 0.025 - 0.012;

    let metal = base + jitter + surface_noise + ember_tint;
    return clamp(metal * mortar * 0.85 + metal * 0.15, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_wood_proc(uv: vec2<f32>) -> vec3<f32> {
    // Smithy resting backdrop — worn workshop bench, anisotropic woodgrain, oil-sheen knots.
    // Amplitude tier: chrome_floating 5% max chroma drift.
    let base = vec3<f32>(0.361, 0.227, 0.133); // #5C3A22
    // Anisotropic grain: 6 cells/unit horizontal, ~1 cell vertical
    let cell_scale_x = 6.0;
    let cell_scale_y = 1.0;
    let cell_uv = uv * vec2<f32>(cell_scale_x, cell_scale_y);
    let cell_id = floor(cell_uv);
    let cell_fract = fract(cell_uv);

    // Per-cell hash for stripe variation (max 5% drift)
    let h = hash_f(cell_id);
    let jitter = (h - 0.5) * 0.08; // range [-0.04, +0.04]

    // Grain stripe: darken along horizontal grain lines
    let grain_border = min(cell_fract.y, 1.0 - cell_fract.y);
    let grain = smoothstep(0.0, 0.12, grain_border);

    // Oil sheen at knot intersections: small bright spots at cell corners
    let knot_dist = length(cell_fract - vec2<f32>(0.0));
    let knot_dist2 = length(cell_fract - vec2<f32>(1.0, 0.0));
    let sheen = max(0.0, 0.15 - min(knot_dist, knot_dist2)) * 0.25;

    // Per-cell surface noise for grain texture
    let surface_noise = noise_2d(cell_uv * 3.5) * 0.020 - 0.010;

    let plank = base + jitter + surface_noise + sheen;
    return clamp(plank * grain * 0.80 + plank * 0.20, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_vellum_proc(uv: vec2<f32>) -> vec3<f32> {
    // Smithy semi-transparent overlay — off-white fiber grain, translucency desaturation hint.
    // Amplitude tier: chrome_floating 5% max chroma drift.
    let base = vec3<f32>(0.941, 0.910, 0.847); // #F0E8D8
    let cell_scale = 18.0;
    let cell_uv = uv * cell_scale;
    let cell_id = floor(cell_uv);
    let cell_fract = fract(cell_uv);

    // Per-cell hash for faint fiber variation (max 5% drift)
    let h = hash_f(cell_id);
    let jitter = (h - 0.5) * 0.05; // range [-0.025, +0.025]

    // Soft fiber edge
    let border = min(min(cell_fract.x, 1.0 - cell_fract.x), min(cell_fract.y, 1.0 - cell_fract.y));
    let mortar = smoothstep(0.0, 0.05, border);

    // Faint surface noise
    let surface_noise = noise_2d(cell_uv * 2.0) * 0.015 - 0.007;

    // Translucency hint: slight desaturation toward neutral (simulate light bleed)
    let grey = (base.r + base.g + base.b) / 3.0;
    let desaturated = mix(base, vec3<f32>(grey), 0.15);

    let vellum = desaturated + jitter + surface_noise;
    return clamp(vellum * mortar * 0.90 + vellum * 0.10, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_cobblestone_proc(uv: vec2<f32>) -> vec3<f32> {
    // Workshop floor cobblestone — warm grey-brown, max 5% chroma drift per cell.
    // 16x16 cell grid with per-cell hue jitter and darkened mortar borders.
    let base = vec3<f32>(0.32, 0.28, 0.24);
    let cell_scale = 16.0;
    let cell_uv = uv * cell_scale;
    let cell_id = floor(cell_uv);
    let cell_fract = fract(cell_uv);

    // Per-cell hash for hue jitter (max 5% = 0.05 chroma drift)
    let h = hash_f(cell_id);
    let jitter = (h - 0.5) * 0.10; // range [-0.05, +0.05]

    // Mortar border: darken within 0.06 of cell edge
    let border = min(min(cell_fract.x, 1.0 - cell_fract.x), min(cell_fract.y, 1.0 - cell_fract.y));
    let mortar = smoothstep(0.0, 0.06, border);

    // Per-cell surface noise for slight texture variation
    let surface_noise = noise_2d(cell_uv * 3.0) * 0.025 - 0.012;

    let stone = base + jitter + surface_noise;
    return clamp(stone * mortar * 0.85 + stone * 0.15, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_vibe_matrix(color: vec4<f32>, vibe_mask: u32) -> vec4<f32> {
    if vibe_mask == 0u {
        return color;
    }
    var result = color;
    // Bit 0: GLOW — driven by artifact_glow via VibeVector
    if (vibe_mask & 0x01u) != 0u {
        let glow_t = sin(u.time * 3.0) * 0.5 + 0.5;
        result = vec4<f32>(result.rgb + result.rgb * glow_t * u.vibe_glow, result.a);
    }
    // Bit 1: SHAKE — driven by distortion_level via VibeVector
    if (vibe_mask & 0x02u) != 0u {
        let noise = fract(sin(u.time * 43758.5453) * 2.0);
        result = vec4<f32>(result.rgb * (1.0 - u.vibe_shake * noise), result.a);
    }
    // Bit 2: CHROMATIC — driven by chromatic_aberration via VibeVector
    if (vibe_mask & 0x04u) != 0u {
        let shift = u.vibe_chromatic * 0.20;
        result = vec4<f32>(result.r * (1.0 + shift), result.g, result.b * (1.0 - shift), result.a);
    }
    // Bit 3: PULSE — driven by particle_density via VibeVector
    if (vibe_mask & 0x08u) != 0u {
        let pulse_t = sin(u.time * 6.0) * 0.5 + 0.5;
        result = vec4<f32>(result.rgb * (1.0 + pulse_t * u.vibe_pulse * 0.3), result.a);
    }
    return result;
}

// Resonance-response glow (§6): a cell's essence_id selects how brightly it HALOS
// when struck by the vibe/aura field. essence_id is one-based in packed_flags
// bits[16..23] — 0 = inert (no resonance, e.g. UI chrome). The lift is ADDITIVE and
// SEPARATE from authored material emission:
//   final_glow = material_emission + vibe_glow × ESSENCE_LUMINANCE[essence_id-1]
// At-rest contrast gate holds for free: u.vibe_glow == 0 (silent audio + idle UI +
// no aura) -> nothing added. Bounded by the same ceiling as vibe_glow (<= 12%).
fn apply_resonance_glow(color: vec4<f32>, packed_flags: u32) -> vec4<f32> {
    let ess = (packed_flags >> 16u) & 0x7Fu;
    if ess == 0u {
        return color; // inert: no essence assigned
    }
    let lum = f32(essence_luminance[ess - 1u]) / 10000.0; // Permyriad -> 0..1
    return vec4<f32>(color.rgb + color.rgb * (u.vibe_glow * lum), color.a);
}

// Hard pixel coverage for square-cornered quads (r == 0): 1.0 inside the SDF
// boundary, 0.0 outside, no AA ramp. Required so abutting integer-aligned cells
// tile without the dark canvas bleeding through their shared edge as a seam.
fn coverage_hard(d: f32) -> f32 {
    return select(0.0, 1.0, d <= 0.0);
}

// Gaussian SDF box-shadow (M9.1): the same signed distance that masks the quad
// drives its drop shadow, so the halo stays continuous through rounded corners
// with no second geometry pass. `softness` is the blur radius in pixels; a
// non-positive radius means no shadow. Returns 0..1 shadow coverage at `d`.
fn sdf_box_shadow(d: f32, softness: f32) -> f32 {
    if softness <= 0.0 {
        return 0.0;
    }
    let sigma = softness * 0.5;
    let x = max(d, 0.0) / sigma;
    return exp(-0.5 * x * x);
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let uv = in.local_uv;
    let p = (uv - 0.5) * in.quad_size;
    let r = in.corner_radius;

    // DROP SHADOW (bit 23): the instance arrives inflated by `apron` on every
    // side (QuadBatcher::push_shadow, apron = blur * 2.0) so fragments exist
    // where the halo falls; the SDF evaluates against the TRUE rect by
    // deflating half_size by that same apron. Two sites, one contract.
    // Bit 23 = DROP SHADOW, bit 22 = GLOW. Both arrive inflated by `apron`
    // (blur * 2.0, QuadBatcher::push_shadow / push_glow) so fragments exist where
    // the falloff lands, and both deflate half_size by that same apron so the SDF
    // measures the TRUE rect. Two sites, one contract — now shared by two flags.
    var half_size = in.quad_size * 0.5;
    // SHADOW (bit 23, 0x800000) | GLOW (bit 24, 0x1000000). GLOW moved off bit 22
    // on 2026-07-31 — it sat inside the essence field [16..22].
    let is_falloff = (in.packed_flags & 0x1800000u) != 0u && in.is_outline <= 0.5;
    if is_falloff {
        half_size = half_size - vec2<f32>(in.stroke_thickness * 2.0);
    }

    // Rounded rect SDF
    let q = abs(p) - half_size + r;
    let d = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
    // Square corners (r == 0) get HARD coverage so integer-aligned pixel cells,
    // grid lines, and 1px borders tile seamlessly. The old smoothstep ramped
    // every cell edge's alpha, bleeding the dark canvas through abutting cells
    // as a false AA seam (CLARITY-PASS-001: atmosphere must not distort canvas).
    let aa = select(coverage_hard(d), 1.0 - smoothstep(-1.0, 1.0, d), r > 0.0);

    // Base color with SDF masking
    var base_color: vec4<f32>;
    if in.is_outline > 0.5 {
        let inner_d = d + in.stroke_thickness;
        let inner_aa = select(coverage_hard(inner_d), 1.0 - smoothstep(-1.0, 1.0, inner_d), r > 0.0);
        let stroke = aa - inner_aa;
        base_color = vec4<f32>(in.quad_color.rgb, in.quad_color.a * stroke);
    } else {
        base_color = vec4<f32>(in.quad_color.rgb, in.quad_color.a * aa);
    }

    // packed_flags bit 23 = DROP SHADOW. stroke_thickness carries the blur radius
    // for a shadowed fill (an outline quad spends that field on its stroke and
    // never sets the bit). The halo is masked to OUTSIDE the shape (1.0 - aa) so
    // the quad's own colour is never darkened.
    if (in.packed_flags & 0x800000u) != 0u && in.is_outline <= 0.5 {
        let halo = sdf_box_shadow(d, in.stroke_thickness) * (1.0 - aa);
        base_color = vec4<f32>(
            mix(base_color.rgb, vec3<f32>(0.0), halo),
            max(base_color.a, halo * 0.5),
        );
    }

    // GLOW (bit 24): the shadow's geometry with the opposite intent. A shadow mixes
    // the ground toward black; a glow keeps its authored colour and spends the SDF
    // on ALPHA alone, then rides the additive pipeline so it sums into the ground
    // instead of veiling it. Unmasked by `1.0 - aa` on purpose — a halo is brightest
    // where the shape is, and gets its shape from the falloff, not from a cutout.
    if (in.packed_flags & 0x1000000u) != 0u && in.is_outline <= 0.5 {
        let field = sdf_box_shadow(d, in.stroke_thickness);
        base_color = vec4<f32>(in.quad_color.rgb, in.quad_color.a * field);
    }

    // Unpack material index and vibe mask
    let mat_idx = in.packed_flags & 0xFFu;
    let vibe_mask = (in.packed_flags >> 8u) & 0xFFu;

    // ── Data-driven material dispatch ────────────────────────────────────────
    // The per-quad mat_idx is an INDEX into the registry; the shader switches on
    // the entry's `kind` field (data), never a hardcoded `mat_idx == N` branch.
    // Add a material = a new registry row (+ an array layer for textured) — no
    // shader edit. `params` low byte selects the Smithy array layer.
    let entry = dod_registry[mat_idx];
    let kind = entry.kind;
    let layer = entry.params & 0xFFu;

    // KIND_DEFAULT (incl. mat_idx 0): global mood vibe only, early out — but still
    // answer the resonance field if the cell carries an essence.
    //
    // THE MASK IS THE INSTANCE'S OWN (ᒥ vibe-mask-unhardcode, 2026-07-30). This line
    // passed a hardcoded `0x0Fu` — GLOW|SHAKE|CHROMATIC|PULSE, every channel forced
    // on — for EVERY undressed quad, which is the window ground and all plain chrome.
    // GLOW is additive (`rgb + rgb * glow_t * u.vibe_glow`), so the whole surface rode
    // a time-varying lift no kit asked for: measured off a live capture, the molten
    // ground rendered #382E26 against its authored token #0A0705, +0x2E on every
    // channel, and matte nav cards washed to blue-grey. Authored colour could not
    // survive to glass on this branch, which made every palette fix look like it had
    // failed. `vibe_mask` is already unpacked from packed_flags above; a quad that
    // wants vibe says so through `SetMaterial`, and one that says nothing gets its
    // colour back untouched (`apply_vibe_matrix` early-outs at mask 0).
    if kind == KIND_DEFAULT {
        let vibed = apply_vibe_matrix(base_color, vibe_mask);
        return apply_resonance_glow(vibed, in.packed_flags);
    }

    var mat_color = base_color;
    if kind == KIND_GUNMETAL {
        let mat = material_palette[mat_idx];
        // Blend palette roughness with registry roughness (registry wins if populated).
        let effective_roughness = select(mat.roughness_pmy, entry.roughness_metallic >> 16u, entry.roughness_metallic != 0u);
        mat_color = apply_gunmetal(base_color, effective_roughness, uv, in.quad_size);
    } else if kind == KIND_GLASS {
        let mat = material_palette[mat_idx];
        mat_color = apply_glass(base_color, in.clip.xy, mat.roughness_pmy, uv);
    } else if kind == KIND_HOLOGRAM {
        let mat = material_palette[mat_idx];
        mat_color = apply_hologram(base_color, uv, mat.emission_pmy);
    } else if kind == KIND_TEXTURED {
        if (u.smithy_tex_enabled & (1u << layer)) != 0u {
            // textureSampleLevel (explicit LOD 0) — array is single-mip and this
            // dispatch is per-primitive (flat), so no derivative-uniformity hazard.
            let sample = textureSampleLevel(t_smithy, s_smithy, uv, i32(layer), 0.0);
            // MODULATE, never REPLACE (2026-07-30). This assigned `sample.rgb` flat and
            // threw the authored fill away, so a kit that said `color=palette.bg_near`
            // got whatever the smithy layer held — and an unloaded layer samples to a
            // flat grey, which is why four bronze nav cards measured #959DA4 against an
            // authored #1A0F09. A material is a SURFACE on a colour, not a colour.
            // UNITY GAIN (2026-07-31). The first cut multiplied by 2.0 to keep a
            // mid-grey texel neutral; that made a white texel a 2x overbright and
            // clipped any albedo over half — a (153,76,25) bronze read back
            // (255,152,50). A texel is a coefficient: 1.0 passes the authored fill
            // through untouched, and a dark albedo is SUPPOSED to darken it.
            mat_color = vec4<f32>(base_color.rgb * sample.rgb, base_color.a);
        } else {
            // Procedural fallback selected by layer (placeholder until a real
            // texture loads; stage-2 replaces these with a data-driven detail LUT).
            var proc = vec3<f32>(
                f32((entry.colour >> 24u) & 0xFFu) / 255.0,
                f32((entry.colour >> 16u) & 0xFFu) / 255.0,
                f32((entry.colour >>  8u) & 0xFFu) / 255.0,
            );
            if layer == 0u {
                proc = apply_parchment_proc(uv);
            } else if layer == 1u {
                proc = apply_bronze_proc(uv);
            } else if layer == 2u {
                proc = apply_wood_proc(uv);
            } else if layer == 3u {
                proc = apply_vellum_proc(uv);
            } else if layer == 4u {
                proc = apply_cobblestone_proc(uv);
            }
            // Same law as the texture branch: the procedural pattern MODULATES the
            // authored fill. `apply_bronze_proc` returns a hardcoded #A87044, so
            // assigning it flat made every bronze slot the same warm brown no matter
            // what its kit authored — the palette stopped being the source of truth
            // the moment a material was named.
            // Same unity-gain law as the texture branch above: the pattern is a
            // coefficient on the authored fill, never a 2x lift.
            mat_color = vec4<f32>(base_color.rgb * proc, base_color.a);
        }
    }
    // Unknown kind: mat_color stays base_color (safe fallback).

    // Film grain on dark surfaces (scotopic dither — prevents banding in dark UI)
    let luminance = dot(mat_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    if luminance < 0.15 && mat_color.a > 0.5 {
        let grain_seed = in.clip.xy + vec2<f32>(u.time * 60.0, u.time * 43.0);
        let film_grain = (hash_f(grain_seed) - 0.5) * 0.02;
        mat_color = vec4<f32>(mat_color.rgb + film_grain, mat_color.a);
    }

    // VibeMatrix post-processing, then the essence-weighted resonance halo (§6).
    let final_color = apply_vibe_matrix(mat_color, vibe_mask);

    return apply_resonance_glow(final_color, in.packed_flags);
}
