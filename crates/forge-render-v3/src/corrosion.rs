//! Corrosion visualization — clean steel → rust, driven by `corrosion_pct`.
//!
//! The shader is `shaders/core/corrosion.wgsl` (drained off the airgap quarry
//! 2026-07-28; it had a live producer in `forge_core::creature_engine` and no
//! home in this tree). This module is its live caller: the WGSL source and the
//! `repr(C)` uniform whose field layout IS the GPU contract.

/// The corrosion pass source, held here so the shader has exactly one consumer.
pub const CORROSION_WGSL: &str = include_str!("../shaders/core/corrosion.wgsl");

/// GPU uniform for `shaders/core/corrosion.wgsl`.
///
/// Field layout IS the wire contract — it mirrors the WGSL `Uniforms` block
/// member for member. `camera_pos` + `corrosion_pct` share one 16-byte slot
/// (WGSL packs a trailing scalar into the `vec3<f32>` pad), and `_pad` closes
/// the struct to the 16-byte alignment std140 requires.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CorrosionUniforms {
    /// Model-view-projection matrix.
    pub mvp: [[f32; 4]; 4],
    /// Model matrix for world-space transforms.
    pub model: [[f32; 4]; 4],
    /// Camera position in world space.
    pub camera_pos: [f32; 3],
    /// 0.0 = clean steel (grey, high metallic), 1.0 = fully rusted.
    pub corrosion_pct: f32,
    /// CUI risk band: 0=low, 1=med, 2=high, 3=critical. The shader tints at
    /// >= 1.5 and pulses a red border at >= 2.5.
    pub cui_risk: f32,
    /// Time value for animation/pulsing effects.
    pub time: f32,
    /// Padding for std140 alignment.
    pub _pad: [f32; 2],
}

impl CorrosionUniforms {
    /// Byte size the shader expects — asserted against the WGSL layout in tests.
    pub const SIZE: usize = 160;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_matches_the_wgsl_std140_layout() {
        assert_eq!(std::mem::size_of::<CorrosionUniforms>(), CorrosionUniforms::SIZE);
        assert_eq!(std::mem::align_of::<CorrosionUniforms>(), 4);
    }

    /// The source is REACHABLE, not just on disk: the const carries the real
    /// shader, entry points included.
    #[test]
    fn shader_source_carries_both_entry_points() {
        assert!(CORROSION_WGSL.contains("fn vs_main"));
        assert!(CORROSION_WGSL.contains("fn fs_main"));
        assert!(CORROSION_WGSL.contains("corrosion_pct"));
    }
}
