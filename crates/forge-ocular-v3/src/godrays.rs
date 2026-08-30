//! ENB Crepuscular God-Rays & Dithered Star Bloom pass uniforms and shader contract.
//!
//! Bridges transpiled ENB screen-space radial raymarching with 13Forge's
//! zero-float integer / dithered pixel aesthetic and 5D celestial star dome.

use bytemuck::{Pod, Zeroable};
use forge_core_v3::atom::Pexil;

/// Embedded WGSL shader for ENB radial god-rays and dithered star bloom.
pub const ENB_GODRAYS_WGSL: &str = include_str!("../shaders/enb_godrays.wgsl");

/// Embedded WGSL compute shader for 5D pentaract raymarching (Phase 3.1).
pub const PENTARACT_MARCH_5D_WGSL: &str = include_str!("../shaders/pentaract_march_5d.wgsl");

/// Uniform buffer payload matching `GodRayUniforms` in `enb_godrays.wgsl`.
/// 48 bytes, aligned to 16 bytes for standard WebGPU / WGSL uniform buffers.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct GodRayUniforms {
    /// Screen-space light position [0.0..1.0, 0.0..1.0] (8 bytes).
    pub light_pos_ss: [f32; 2],
    /// Ray step spacing multiplier (4 bytes).
    pub density: f32,
    /// Sample weight per step (4 bytes).
    pub weight: f32,
    /// Exponential attenuation factor (4 bytes).
    pub decay: f32,
    /// Light shaft exposure multiplier (4 bytes).
    pub exposure: f32,
    /// Ray march sample count (e.g. 32 or 64) (4 bytes).
    pub num_samples: u32,
    /// Atmospheric glaze intensity [0.0..1.0] mapped from permyriad (4 bytes).
    pub glaze_intensity: f32,
    /// Atmospheric haze / bio-film tint [RGBA] (16 bytes).
    pub haze_color: [f32; 4],
}

impl Default for GodRayUniforms {
    fn default() -> Self {
        Self {
            light_pos_ss: [0.5, 0.5],
            density: 1.0,
            weight: 0.05,
            decay: 0.96,
            exposure: 1.2,
            num_samples: 32,
            glaze_intensity: 1.0,
            haze_color: [1.0, 0.95, 0.85, 1.0], // Warm celestial moonlight default
        }
    }
}

impl GodRayUniforms {
    /// Construct uniforms anchored to an active moon or star cluster screen position.
    pub fn from_celestial_light(
        screen_x: f32,
        screen_y: f32,
        glaze_intensity_pmy: u32,
        haze_rgba: [f32; 4],
    ) -> Self {
        Self {
            light_pos_ss: [screen_x.clamp(0.0, 1.0), screen_y.clamp(0.0, 1.0)],
            density: 0.92,
            weight: 0.06,
            decay: 0.95,
            exposure: 1.15,
            num_samples: 32,
            glaze_intensity: (glaze_intensity_pmy.min(10_000) as f32) / 10_000.0,
            haze_color: haze_rgba,
        }
    }

    /// PaTeX A1..A5 glaze binding: the five balanced trits of `pexil.lattice`
    /// (A1 Ground -> weight, A2 Light -> exposure, A3 Depth -> density,
    /// A4 Drift -> decay, A5 Witness -> sample count 16/32/64) select integer
    /// permyriad parameters first (L08); `pexil.payload[0..2]` is glaze
    /// intensity permyriad, LE. A sentinel lattice byte is out-of-band control,
    /// not a coordinate: it yields the default glaze at the given anchor.
    pub fn from_pexil_glaze(pexil: Pexil, light_pos_ss: [f32; 2], haze_rgba: [f32; 4]) -> Self {
        let anchor = [light_pos_ss[0].clamp(0.0, 1.0), light_pos_ss[1].clamp(0.0, 1.0)];
        let Some([ground, light, depth, drift, witness]) = pexil.lattice.trits() else {
            return Self { light_pos_ss: anchor, haze_color: haze_rgba, ..Self::default() };
        };
        let weight_pmy: i32 = 500 + ground as i32 * 100;
        let exposure_pmy: i32 = 11_500 + light as i32 * 1_500;
        let density_pmy: i32 = 9_200 + depth as i32 * 800;
        let decay_pmy: i32 = 9_500 + drift as i32 * 100;
        let glaze_pmy = u16::from_le_bytes([pexil.payload[0], pexil.payload[1]]).min(10_000);
        Self {
            light_pos_ss: anchor,
            density: density_pmy as f32 / 10_000.0,
            weight: weight_pmy as f32 / 10_000.0,
            decay: decay_pmy as f32 / 10_000.0,
            exposure: exposure_pmy as f32 / 10_000.0,
            num_samples: 16u32 << (witness + 1) as u32,
            glaze_intensity: glaze_pmy as f32 / 10_000.0,
            haze_color: haze_rgba,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn godray_uniforms_layout_and_size() {
        assert_eq!(std::mem::size_of::<GodRayUniforms>(), 48);
        assert_eq!(std::mem::align_of::<GodRayUniforms>(), 16);
    }

    #[test]
    fn godray_shader_embedded_valid() {
        assert!(ENB_GODRAYS_WGSL.contains("struct GodRayUniforms"));
        assert!(ENB_GODRAYS_WGSL.contains("fn fs_main"));
        assert!(ENB_GODRAYS_WGSL.contains("bayer8_threshold"));
    }

    #[test]
    fn pexil_glaze_binding_maps_all_five_axes() {
        use forge_core_v3::atom::{CellOrdinal, TritCell5D, ValidityMask};

        let origin = Pexil {
            lattice: TritCell5D::ORIGIN,
            validity: ValidityMask::ALL_KNOWN,
            ordinal: CellOrdinal(0),
            payload: 10_000u32.to_le_bytes(),
        };
        let u = GodRayUniforms::from_pexil_glaze(origin, [0.5, 0.5], [1.0; 4]);
        assert!((u.weight - 0.05).abs() < 1e-6);
        assert!((u.exposure - 1.15).abs() < 1e-6);
        assert!((u.density - 0.92).abs() < 1e-6);
        assert!((u.decay - 0.95).abs() < 1e-6);
        assert_eq!(u.num_samples, 32);
        assert!((u.glaze_intensity - 1.0).abs() < 1e-6);

        let hot = Pexil { lattice: TritCell5D::from_trits([1, 1, 1, 1, 1]), ..origin };
        let u = GodRayUniforms::from_pexil_glaze(hot, [0.5, 0.5], [1.0; 4]);
        assert!((u.weight - 0.06).abs() < 1e-6);
        assert!((u.exposure - 1.30).abs() < 1e-6);
        assert!((u.density - 1.00).abs() < 1e-6);
        assert!((u.decay - 0.96).abs() < 1e-6);
        assert_eq!(u.num_samples, 64);

        let cold = Pexil { lattice: TritCell5D::from_trits([-1, -1, -1, -1, -1]), ..origin };
        let u = GodRayUniforms::from_pexil_glaze(cold, [0.5, 0.5], [1.0; 4]);
        assert!((u.weight - 0.04).abs() < 1e-6);
        assert!((u.exposure - 1.00).abs() < 1e-6);
        assert!((u.density - 0.84).abs() < 1e-6);
        assert!((u.decay - 0.94).abs() < 1e-6);
        assert_eq!(u.num_samples, 16);

        let sentinel = Pexil { lattice: TritCell5D(250), ..origin };
        let u = GodRayUniforms::from_pexil_glaze(sentinel, [2.0, -1.0], [0.5; 4]);
        assert_eq!(u.num_samples, GodRayUniforms::default().num_samples);
        assert_eq!(u.light_pos_ss, [1.0, 0.0]);
        assert_eq!(u.haze_color, [0.5; 4]);
    }

    #[test]
    fn default_uniforms_valid() {
        let u = GodRayUniforms::default();
        assert_eq!(u.num_samples, 32);
        assert!((u.glaze_intensity - 1.0).abs() < 1e-5);
    }

    #[test]
    fn pentaract_march_5d_shader_embedded_valid() {
        // Phase 3.1: Verify shader embeds and contains critical structure markers.
        assert!(PENTARACT_MARCH_5D_WGSL.contains("struct M5Params"));
        assert!(PENTARACT_MARCH_5D_WGSL.contains("fn check_absence_5d"));
        assert!(PENTARACT_MARCH_5D_WGSL.contains("fn quantize_trit"));
        assert!(PENTARACT_MARCH_5D_WGSL.contains("@compute"));
        assert!(PENTARACT_MARCH_5D_WGSL.contains("transmittance < 0.001"));
        assert!(PENTARACT_MARCH_5D_WGSL.contains("uv_step = (ray_dir_4d.xy / d_h) * delta_h"));
    }

    #[test]
    fn pentaract_march_5d_wgsl_to_spirv_compilation() {
        use naga::front::wgsl;
        use naga::back::spv as naga_spv;

        // Parse WGSL to intermediate representation
        let module = wgsl::parse_str(PENTARACT_MARCH_5D_WGSL)
            .expect("WGSL parse failed: check shader syntax");

        // Validate against Vulkan/SPIR-V target
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        let info = validator.validate(&module)
            .expect("Naga validation failed: check shader semantics");

        // Emit SPIR-V bytecode (Vulkan target)
        let options = naga_spv::Options::default();
        let spirv_binary = naga_spv::write_vec(&module, &info, &options, None)
            .expect("SPIR-V emission failed: check shader structure");

        // Validate emission succeeded
        assert!(!spirv_binary.is_empty(), "Emitted SPIR-V bytecode is empty");
        assert!(spirv_binary.len() > 4, "SPIR-V bytecode too small (header missing)");

        // Verify magic number (0x07230203 = SPIR-V magic, first word as u32)
        assert_eq!(spirv_binary[0], 0x07230203u32, "SPIR-V magic number mismatch");
    }
}
