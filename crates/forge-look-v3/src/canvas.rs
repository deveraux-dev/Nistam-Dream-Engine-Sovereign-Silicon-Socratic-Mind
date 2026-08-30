//! Vixel flat-splat projection — integer position → NDC screen quad.
//!
//! CPU-testable math shared between host parity tests and the `spirv` entry points
//! in `entry_points.rs`. The core function `project_vixel` converts a VixelAtom's
//! integer position to normalized device coordinates, and `gaussian_falloff`
//! computes the opacity weight for the flat splat.

use crate::gpu_types::{VixelAtom, VixelViewport};

/// Quad corner offsets (CCW triangle strip: TL, BL, TR, BR).
pub const QUAD_OFFSETS: [[f32; 2]; 4] = [
    [-0.5, -0.5],
    [-0.5,  0.5],
    [ 0.5, -0.5],
    [ 0.5,  0.5],
];

/// Project a VixelAtom to NDC, emitting one corner of the screen-aligned quad.
/// `corner_idx` selects which of the 4 quad vertices (0..3).
///
/// Integer→float boundary happens HERE (the ONE conversion, never inverted).
#[inline]
pub fn project_vixel(atom: &VixelAtom, vp: &VixelViewport, corner_idx: u32) -> (f32, f32, f32, f32) {
    // Integer position → pixel center (MilliUnit / 1000.0)
    let px = atom.pos_x as f32 / 1000.0;
    let py = atom.pos_y as f32 / 1000.0;

    // Camera offset (MilliUnit)
    let cam_x = vp.cam_x as f32 / 1000.0;
    let cam_y = vp.cam_y as f32 / 1000.0;

    // Zoom (permyriad → multiplier)
    let zoom = vp.zoom as f32 / 10000.0;

    // Splat radius in pixels (permyriad of a pixel → actual pixels)
    let radius = atom.size as f32 / 10000.0;

    // Screen-space position with camera and zoom
    let sx = (px - cam_x) * zoom;
    let sy = (py - cam_y) * zoom;

    // Quad corner offset scaled by splat radius
    let ci = (corner_idx & 3) as usize;
    let ox = QUAD_OFFSETS[ci][0] * radius * zoom;
    let oy = QUAD_OFFSETS[ci][1] * radius * zoom;

    // NDC: pixel coords → [-1, 1]
    let w = vp.width as f32;
    let h = vp.height as f32;
    let ndc_x = ((sx + ox) / w) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((sy + oy) / h) * 2.0; // Y-flip for screen coords

    // UV for this corner (0→1 across the quad, used for gaussian falloff)
    let uv_x = QUAD_OFFSETS[ci][0] + 0.5;
    let uv_y = QUAD_OFFSETS[ci][1] + 0.5;

    (ndc_x, ndc_y, uv_x, uv_y)
}

/// Flat gaussian splat falloff. Input: UV (0,0)→(1,1), center=(0.5,0.5).
/// Returns opacity multiplier [0.0, 1.0]. sigma=0.33 gives soft edge.
#[inline]
pub fn gaussian_falloff(uv_x: f32, uv_y: f32) -> f32 {
    let dx = uv_x - 0.5;
    let dy = uv_y - 0.5;
    let d2 = dx * dx + dy * dy;
    // sigma² = 0.11 (sigma ≈ 0.33); exp(-d²/(2σ²)) = exp(-d²/0.22)
    // Use fast approximation: 1.0 - smoothstep for no_std compat
    let t = (d2 * 9.0).min(1.0); // 9.0 ≈ 1/0.11, maps radius 0.33 → 1.0
    let smooth = t * t * (3.0 - 2.0 * t);
    1.0 - smooth
}

/// Unpack RGBA u32 (0xRRGGBBAA) → (r, g, b) as f32 [0,1].
#[inline]
pub fn unpack_rgb(packed: u32) -> (f32, f32, f32) {
    let r = ((packed >> 24) & 0xFF) as f32 / 255.0;
    let g = ((packed >> 16) & 0xFF) as f32 / 255.0;
    let b = ((packed >> 8) & 0xFF) as f32 / 255.0;
    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_viewport() -> VixelViewport {
        VixelViewport {
            width: 1920, height: 1080,
            cam_x: 0, cam_y: 0,
            zoom: 10000, // 1.0×
            tick: 0, _pad0: 0, _pad1: 0,
        }
    }

    #[test]
    fn center_atom_projects_to_origin_area() {
        let atom = VixelAtom {
            pos_x: 960_000, pos_y: 540_000, pos_z: 0, // center of 1920×1080 (MilliUnit)
            material: 0, opacity: 10000, size: 10000, flags: 1,
        };
        let (ndc_x, ndc_y, _, _) = project_vixel(&atom, &test_viewport(), 0);
        // Center-ish (offset by quad corner, but close to 0)
        assert!(ndc_x.abs() < 0.01, "ndc_x={}", ndc_x);
        assert!(ndc_y.abs() < 0.01, "ndc_y={}", ndc_y);
    }

    #[test]
    fn gaussian_center_is_one() {
        let v = gaussian_falloff(0.5, 0.5);
        assert!((v - 1.0).abs() < 0.001, "center={}", v);
    }

    #[test]
    fn gaussian_edge_is_low() {
        let v = gaussian_falloff(0.0, 0.0); // corner
        assert!(v < 0.15, "corner={}", v);
    }

    #[test]
    fn unpack_rgb_white() {
        let (r, g, b) = unpack_rgb(0xFFFFFF00);
        assert!((r - 1.0).abs() < 0.004);
        assert!((g - 1.0).abs() < 0.004);
        assert!((b - 1.0).abs() < 0.004);
    }
}
