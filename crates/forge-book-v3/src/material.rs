//! Material — PBR parameters as integers: albedo rgba8, metallic + roughness in
//! permyriad (harvested from forge-materials).

use serde::{Deserialize, Serialize};

/// A physically-based material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Material {
    /// RGBA color value (sRGB bytes: red, green, blue, alpha).
    pub albedo: [u8; 4],
    /// Metallic parameter in permyriad (0–10,000), clamped on construction.
    pub metallic_pmy: u32,
    /// Roughness parameter in permyriad (0–10,000), clamped on construction.
    pub roughness_pmy: u32,
}

impl Material {
    /// Construct a material with given albedo and parameters, clamping permyriad values to 0–10,000.
    pub fn new(albedo: [u8; 4], metallic_pmy: u32, roughness_pmy: u32) -> Self {
        Self { albedo, metallic_pmy: metallic_pmy.min(10_000), roughness_pmy: roughness_pmy.min(10_000) }
    }
    /// A matte dielectric (non-metal, rough).
    pub fn matte(albedo: [u8; 4]) -> Self {
        Self::new(albedo, 0, 8000)
    }
    /// A polished metal.
    pub fn metal(albedo: [u8; 4]) -> Self {
        Self::new(albedo, 10_000, 1500)
    }
    /// Metallic workflow threshold.
    pub fn is_metal(&self) -> bool {
        self.metallic_pmy > 5000
    }
    /// Return the albedo color as a hex string in `#RRGGBB` format.
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.albedo[0], self.albedo[1], self.albedo[2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_classify() {
        assert!(Material::metal([200, 200, 200, 255]).is_metal());
        assert!(!Material::matte([120, 90, 60, 255]).is_metal());
    }

    #[test]
    fn params_clamp_and_hex() {
        let m = Material::new([0x1e, 0x1a, 0x12, 0xff], 99_999, 99_999);
        assert_eq!(m.metallic_pmy, 10_000);
        assert_eq!(m.roughness_pmy, 10_000);
        assert_eq!(m.hex(), "#1e1a12");
    }
}
