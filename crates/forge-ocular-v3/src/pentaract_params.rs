//! Raymarching parameters for the pentaract kernel.

/// Pentaract raymarching parameters (mirrors WGSL M5Params).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct M5Params {
    /// 256-bit O(1) absence mask (243 cells), packed as 8 u32s.
    pub absence_mask: [u32; 8],
    /// Sun direction in 5D (only first 3 used for lighting).
    pub sun_dir_5d: [f32; 4],
    /// Scale dimension for s (scale) axis.
    pub scale_dim_s: f32,
    /// T axis zero point.
    pub t_zero: f32,
    /// S axis zero point.
    pub s_zero: f32,
    /// Step size along the ray.
    pub step_size: f32,
}

impl M5Params {
    /// Create a new M5Params with default values.
    pub fn new() -> Self {
        Self {
            absence_mask: [0; 8],
            sun_dir_5d: [0.707, 0.0, 0.707, 0.0],
            scale_dim_s: 10.0,
            t_zero: -5.0,
            s_zero: 0.0,
            step_size: 0.1,
        }
    }
}

impl Default for M5Params {
    fn default() -> Self {
        Self::new()
    }
}
