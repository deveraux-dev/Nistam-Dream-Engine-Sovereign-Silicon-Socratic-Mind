#![deny(missing_docs, unsafe_code)]

//! Canvas shader — data contracts only. No device, no render pipeline.
//! Exposes the uber-shader `canvas_quad.wgsl` as const and mirrors its
//! bind-group structs in `#[repr(C)]` with compile-time size asserts.

pub use forge_look_v3::gpu_types::GpuMaterialEntry;

/// Uniforms buffer (@group(0) @binding(0)) — viewport + vibe parameters.
/// 48 bytes, std140-aligned (all scalars, no vec3 padding hazard).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CanvasUniforms {
    /// Viewport width in normalized device coordinates.
    pub vp_w: f32,
    /// Viewport height in normalized device coordinates.
    pub vp_h: f32,
    /// Elapsed time in seconds (driven by simulation tick).
    pub time: f32,
    /// Glow intensity (permyriad, 0–10000; audio-driven vibe signal).
    pub vibe_glow: f32,
    /// Screen-shake intensity (permyriad; distortion vibe signal).
    pub vibe_shake: f32,
    /// Chromatic aberration strength (permyriad; dispersion vibe signal).
    pub vibe_chromatic: f32,
    /// Pulse intensity (permyriad; particle-density vibe signal).
    pub vibe_pulse: f32,
    /// Alignment padding to 32 bytes.
    pub _pad: f32,
    /// Smithy texture layer enable bitset (1 << layer per material slot).
    pub smithy_tex_enabled: u32,
    /// Perspective tilt for KIND_GLASS quads (NDC-Y lean, applied in vertex).
    pub glass_tilt: f32,
    /// Glass opacity phase (permyriad 0–10000; sand→glass transition).
    pub glass_opacity_q: u32,
    /// Alignment padding to 48 bytes.
    pub _pad4: u32,
}

const _: () = assert!(core::mem::size_of::<CanvasUniforms>() == 48);

/// Material parameters (@group(1) @binding(0)) — packed colour and finish.
/// Mirrors the WGSL `struct MaterialParams` in the shader. 16 bytes, one per
/// material slot in the GPU palette array. Canvas shader reads all four u32 fields.
/// NOTE: named `CanvasMaterialParams` here (not `MaterialParams`) to avoid collision
/// with `forge-canvas-v3::material_params::MaterialParams` (UI panel material context).
/// Both are load-bearing but separate (L05 one-home law).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CanvasMaterialParams {
    /// sRGB colour packed as (R << 24 | G << 16 | B << 8 | A).
    pub albedo_packed: u32,
    /// Low u16: roughness in permyriad (0–10000). High u16: refraction in permyriad.
    pub roughness_pmy: u32,
    /// Low u16: emission in permyriad. High u16: reserved (zero).
    pub emission_pmy: u32,
    /// Alignment padding to 16 bytes.
    pub _pad: u32,
}

const _: () = assert!(core::mem::size_of::<CanvasMaterialParams>() == 16);

/// Material dispatch kind constants. Canvas reads `GpuRegistryEntry.kind` and
/// branches on this data field — never a hardcoded `mat_idx == N` pattern.
/// Append-only: adding a new kind is a new row in the registry, no shader edit.
pub mod kind {
    /// Solid colour, vibe-matrix modulated. Fallback for any unspecialized quad.
    pub const DEFAULT: u32 = 0;
    /// Brushed metal with specular + grain detail.
    pub const GUNMETAL: u32 = 1;
    /// Glass with Fresnel rim + scene-refraction + prism tint.
    pub const GLASS: u32 = 2;
    /// Hologram with scanline + chromatic + flicker.
    pub const HOLOGRAM: u32 = 3;
    /// Smithy textured, modulated by layer-selected atlas or procedural fallback.
    pub const TEXTURED: u32 = 4;
}

/// Bit flags in `QuadInstance.packed_flags`.
pub mod flags {
    /// Bit 23: drop-shadow halo (Gaussian SDF blur, stroke_thickness = radius).
    /// Instance inflated by apron; shader deflates half_size to recover true SDF.
    pub const SHADOW: u32 = 0x800000;
    /// Bit 24: additive glow halo (SDF-driven alpha, unmasked by quad coverage).
    /// Instance inflated by apron; shader deflates half_size to recover true SDF.
    pub const GLOW: u32 = 0x1000000;
    /// Essence resonance field: bits [16..22] = essence_id (one-based, 0 = inert).
    /// Modulates vibe_glow sensitivity via ESSENCE_LUMINANCE LUT indexing.
    pub const ESSENCE_MASK: u32 = 0x7F0000;
    /// Vibe-matrix mask: bits [8..15] = GLOW|SHAKE|CHROMATIC|PULSE per-quad enable.
    pub const VIBE_MASK: u32 = 0xFF00;
    /// Material kind: bits [0..7] = KIND_* or index into dod_registry.
    pub const MAT_KIND_MASK: u32 = 0xFF;
}

/// Canvas shader — the über-raster. Procedural + textured materials, vibe-matrix
/// animation, SDF quads with soft corners + drop shadows + glows, glass refractions,
/// essence-weighted resonance halos. 633 lines; read sections annotated (§1–§6).
///
/// Entry points: vs_main (vertex), fs_main (fragment).
/// Bind groups:
/// - @group(0) @binding(0) Uniforms
/// - @group(1) @binding(0) MaterialParams array
/// - @group(2) @binding(0) screen_texture (readback for glass refraction)
/// - @group(3) @binding(0) GpuRegistryEntry array (material dispatch)
/// - @group(3) @binding(1) Smithy material atlas (texture_2d_array)
/// - @group(3) @binding(2) Smithy sampler
/// - @group(3) @binding(3) essence_luminance array (Permyriad LUT per essence)
pub const CANVAS_QUAD_WGSL: &str = include_str!("../shaders/canvas_quad.wgsl");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_uniforms_size() {
        assert_eq!(
            core::mem::size_of::<CanvasUniforms>(),
            48,
            "CanvasUniforms must be 48 bytes (std140 safe)"
        );
    }

    #[test]
    fn test_canvas_material_params_size() {
        assert_eq!(
            core::mem::size_of::<CanvasMaterialParams>(),
            16,
            "CanvasMaterialParams must be 16 bytes"
        );
    }

    #[test]
    fn test_gpu_registry_entry_size() {
        assert_eq!(
            core::mem::size_of::<GpuMaterialEntry>(),
            64,
            "GpuMaterialEntry must be 64 bytes (64 bytes, one cache line)"
        );
    }

    #[test]
    fn test_shader_nonempty() {
        assert!(!CANVAS_QUAD_WGSL.is_empty(), "Shader must be embedded");
        assert!(
            CANVAS_QUAD_WGSL.len() > 1000,
            "Shader too short; verify include_str! path"
        );
    }

    #[test]
    fn test_shader_entry_points() {
        assert!(
            CANVAS_QUAD_WGSL.contains("fn vs_main"),
            "Shader must declare vs_main"
        );
        assert!(
            CANVAS_QUAD_WGSL.contains("fn fs_main"),
            "Shader must declare fs_main"
        );
    }

    #[test]
    fn test_shader_bind_groups() {
        assert!(
            CANVAS_QUAD_WGSL.contains("@group(0) @binding(0)"),
            "Shader must declare @group(0) @binding(0) for Uniforms"
        );
        assert!(
            CANVAS_QUAD_WGSL.contains("@group(1) @binding(0)"),
            "Shader must declare @group(1) @binding(0) for MaterialParams"
        );
        assert!(
            CANVAS_QUAD_WGSL.contains("@group(3) @binding(0)"),
            "Shader must declare @group(3) @binding(0) for GpuRegistryEntry"
        );
    }

    #[test]
    fn test_kind_constants() {
        assert_eq!(kind::DEFAULT, 0);
        assert_eq!(kind::GUNMETAL, 1);
        assert_eq!(kind::GLASS, 2);
        assert_eq!(kind::HOLOGRAM, 3);
        assert_eq!(kind::TEXTURED, 4);
    }

    #[test]
    fn test_flag_constants() {
        assert_eq!(flags::SHADOW, 0x800000);
        assert_eq!(flags::GLOW, 0x1000000);
        assert_eq!(flags::ESSENCE_MASK, 0x7F0000);
        assert_eq!(flags::VIBE_MASK, 0xFF00);
        assert_eq!(flags::MAT_KIND_MASK, 0xFF);
    }

    #[test]
    fn test_shader_kind_declarations() {
        assert!(CANVAS_QUAD_WGSL.contains("const KIND_DEFAULT"));
        assert!(CANVAS_QUAD_WGSL.contains("const KIND_GUNMETAL"));
        assert!(CANVAS_QUAD_WGSL.contains("const KIND_GLASS"));
        assert!(CANVAS_QUAD_WGSL.contains("const KIND_HOLOGRAM"));
        assert!(CANVAS_QUAD_WGSL.contains("const KIND_TEXTURED"));
    }
}
