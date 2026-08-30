//! GPU material layout — the shader-side mirror of forge_core::forge_reg::GpuMaterialEntry.
//!
//! `forge-core` is a `std` crate (memmap2 / regex / bytemuck), so a `no_std`
//! SPIR-V shader cannot import it. Instead both sides agree on ONE `#[repr(C)]`
//! byte layout: 16 × `u32` = 64 bytes (one cache line). The host-side size
//! assertion below is the contract — if forge-core's struct ever drifts off
//! 64 bytes, `cargo test -p forge-shaders` fails HERE on the host, not silently
//! on the GPU. Field order/offsets are copied verbatim from `forge_reg.rs`;
//! keep them in lockstep. (Interim until forge-core gets a no_std feature gate.)
//!
//! VixelAtom lives in forge-core-v3::vixel_automata per L05 one-home law.

pub use forge_core_v3::vixel_automata::VixelAtom;

/// 64-byte GPU-packed material entry. Plain POD (all `u32`) so it is valid on
/// both the host and the `spirv` target with no `bytemuck`/`std` dep in the shader.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GpuMaterialEntry {
    /// Canvas-compatible colour.
    pub colour: u32,
    /// Roughness and metallic packed.
    pub roughness_metallic: u32,
    /// Material kind.
    pub kind: u32,
    /// Material parameters.
    pub params: u32,
    /// PBR texture index.
    pub texture_index: u32,
    /// Normal map index.
    pub normal_index: u32,
    /// Ambient occlusion index.
    pub ao_index: u32,
    /// Material flags.
    pub flags: u32,
    /// Emission value.
    pub emission: u32,
    /// Detail LUT index.
    pub detail_lut: u32,
    /// Ring frequency and attack.
    pub ring_freq_attack: u32,
    /// Hardness and mass.
    pub hardness_mass: u32,
    /// Flammability and flags.
    pub flammability_flags: u32,
    /// Destruction (Charpy) value.
    pub destruction_charpy: u32,
    /// Reserved field 0.
    pub _reserved0: u32,
    /// Reserved field 1.
    pub _reserved1: u32,
}

const _: () = assert!(core::mem::size_of::<GpuMaterialEntry>() == 64);

/// Camera + light uniforms — shader-side mirror of `forge_gpu::pbr_pass::PbrUniforms`
/// (128 bytes, std140). Packed as `Vec4`s so the uniform layout is unambiguous on
/// the `spirv` target (no vec3 alignment padding to get wrong). Byte layout matches
/// the host: `mvp`(64) + camera(16) + light_dir(16) + light_color|ambient(16) +
/// viewport(16). The fragment reads camera/light; `mvp` is the vertex stage's.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PbrUniforms {
    /// Model-view-projection matrix (4x4).
    pub mvp: [glam::Vec4; 4],
    /// xyz = camera world position; w = pad.
    pub camera_pos: glam::Vec4,
    /// xyz = direction the light travels; w = pad.
    pub light_dir: glam::Vec4,
    /// rgb = light colour; w = ambient term.
    pub light_color_ambient: glam::Vec4,
    /// xy = viewport size; zw = pad.
    pub viewport: glam::Vec4,
}

const _: () = assert!(core::mem::size_of::<PbrUniforms>() == 128);

/// VibeMatrix uniform buffer — integer signals from the 120Hz audio/sim kernel.
/// Float conversion happens ONLY inside the shader (`/ 10000.0`).
/// 32 bytes = 8 × u32. std430/std140 safe (all scalar u32).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VibeUniforms {
    /// Audio combo heat (permyriad 0–10000). Drives bloom + vignette intensity.
    pub combo_heat: u32,
    /// Resonance frequency (integer Hz). Drives Perlin UV displacement period.
    pub resonance_hz: u32,
    /// Rain/particle intensity (permyriad). Drives particle emission density.
    pub rain_intensity: u32,
    /// Chromatic aberration strength (permyriad). Drives RGB channel UV offset.
    pub chromatic_aberration: u32,
    /// Artifact glow (permyriad). Drives additive bloom weight.
    pub artifact_glow: u32,
    /// Particle density (permyriad). Drives spawn rate of GPU particles.
    pub particle_density: u32,
    /// Distortion level (permyriad). Drives UV noise displacement magnitude.
    pub distortion_level: u32,
    /// Padding to 32 bytes (8 × u32).
    pub _pad: u32,
}

const _: () = assert!(core::mem::size_of::<VibeUniforms>() == 32);

/// Viewport uniforms for vixel projection (integer → NDC at the boundary).
/// 32 bytes.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VixelViewport {
    /// Viewport width in pixels.
    pub width: u32,
    /// Viewport height in pixels.
    pub height: u32,
    /// Camera offset X (MilliUnit i32, for pan/scroll).
    pub cam_x: i32,
    /// Camera offset Y (MilliUnit i32).
    pub cam_y: i32,
    /// Zoom level (permyriad; 10000 = 1.0×).
    pub zoom: u32,
    /// Current simulation tick (for automata phase).
    pub tick: u32,
    /// Reserved.
    pub _pad0: u32,
    /// Reserved.
    pub _pad1: u32,
}

const _: () = assert!(core::mem::size_of::<VixelViewport>() == 32);

/// Colour-quantize LUT params. `size` = the 3D LUT cube edge the `lut_quantize_fs`
/// shader samples; `< 2` = passthrough (Full true colour — the depth toggle off).
/// 16 bytes, all SCALAR u32 (std140/relaxed-uniform safe — an array pad would
/// violate the 16-byte member-alignment rule; scalar pads match VibeUniforms).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct LutParams {
    /// 3D LUT cube edge length; `< 2` disables quantization (true-colour passthrough).
    pub size: u32,
    /// Padding to 16 bytes (scalars, not an array — std140 member alignment).
    pub _pad0: u32,
    /// Padding to 16 bytes.
    pub _pad1: u32,
    /// Padding to 16 bytes.
    pub _pad2: u32,
}

const _: () = assert!(core::mem::size_of::<LutParams>() == 16);

/// Push constants for the VibeBuffer AO baking compute pass.
///
/// 16 bytes (scalar u32/f32 pads — same discipline as `LutParams`; an array pad
/// would violate the 16-byte std140 member-alignment rule on some drivers).
///
/// `sample_count` is clamped inside the shader to `MAX_AO_SAMPLES` (32); the
/// host can send fewer to trade quality for dispatch time.  `max_distance` is in
/// voxel units — 8.0 is a sensible default for a 32³ chunk (covers half the
/// diagonal), 32.0 covers the whole chunk.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct AoBakeConstants {
    /// Number of hemisphere samples per voxel (clamped to MAX_AO_SAMPLES).
    pub sample_count: u32,
    /// Ray max distance in voxel units before the DDA escapes the chunk.
    pub max_distance: f32,
    /// Padding.
    pub _pad0: f32,
    /// Padding.
    pub _pad1: f32,
}

const _: () = assert!(core::mem::size_of::<AoBakeConstants>() == 16);

/// Per-vertex output from the vixel vertex shader. Screen-space quad corner.
/// Used as the interface between vs and fs (locations 0–2).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SplatVertex {
    /// Normalized quad UV (0,0)→(1,1) for gaussian falloff calculation.
    pub uv_x: f32,
    /// Normalized quad UV (0,0)→(1,1) for gaussian falloff calculation.
    pub uv_y: f32,
    /// Flat material colour (unpacked from GpuMaterialEntry.colour at vs time).
    pub color_r: f32,
    /// Flat material colour (unpacked from GpuMaterialEntry.colour at vs time).
    pub color_g: f32,
    /// Flat material colour (unpacked from GpuMaterialEntry.colour at vs time).
    pub color_b: f32,
    /// Opacity (from VixelAtom.opacity / 10000.0).
    pub alpha: f32,
}

const _: () = assert!(core::mem::size_of::<SplatVertex>() == 24);

/// Push-constant for `canvas_projection_vertex` — the StructuralBox F5 swap. The
/// canvas IS a 3D box; this flips the *projection*, never the data.
/// `projection_mode`: `0` = 2D Front plane (force `z = 0`, "a 2D sprite is z=0"),
/// `1` = 3D perspective playtest (keep the vertex z). The combined view·projection
/// is stored as `[Vec4;4]` COLUMNS (unambiguous spirv layout — the same discipline
/// as `PbrUniforms.mvp`; the shader rebuilds the `Mat4` via `from_cols`). 80 bytes:
/// 64 (matrix) + 16 (mode + 3 scalar pads, std430 push-constant safe).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ProjectionConstants {
    /// Combined view·projection, column-major (`[col0, col1, col2, col3]`).
    pub view_proj: [glam::Vec4; 4],
    /// `0` = 2D flat (z→0, the pixel canvas), `1` = 3D perspective playtest.
    pub projection_mode: u32,
    /// Padding.
    pub _pad0: u32,
    /// Padding.
    pub _pad1: u32,
    /// Padding.
    pub _pad2: u32,
}

const _: () = assert!(core::mem::size_of::<ProjectionConstants>() == 80);

/// Push-constant for `canvas_post_process_fragment` — fused LUT-quantize + bloom.
/// `bloom_threshold` is a `[0,1]` Rec.709-luma gate; `bloom_intensity` scales the
/// additive glow. Both are derived HOST-side from the permyriad integer scale
/// (deterministic — locking emission to whole-number signals, no per-frame float
/// drift). 16 bytes (2 × f32 + 2 scalar pads, push-constant safe).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PostProcessConstants {
    /// Bloom intensity multiplier.
    pub bloom_intensity: f32,
    /// Bloom threshold (Rec.709 luminance).
    pub bloom_threshold: f32,
    /// Padding.
    pub _pad0: u32,
    /// Padding.
    pub _pad1: u32,
}

const _: () = assert!(core::mem::size_of::<PostProcessConstants>() == 16);

/// Push-constant for `look_composite_fs` — global layer opacity + sand→glass phase.
/// 16 bytes (4 × u32, std430 push-constant safe).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct LookCompositeConstants {
    /// Global layer opacity in permyriad (0 = invisible, 10_000 = fully opaque).
    pub layer_opacity_q: u32,
    /// Sand→glass phase in permyriad (0 = sand/opaque, 10_000 = glass/fully transparent).
    /// Attenuates composite alpha after the reactive-edge sum — models the material
    /// phase transition: sand holds opacity, glass surrenders it.
    pub material_phase_q: u32,
    /// Padding.
    pub _pad1: u32,
    /// Padding.
    pub _pad2: u32,
}

const _: () = assert!(core::mem::size_of::<LookCompositeConstants>() == 16);
