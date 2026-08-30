//! Colour-quantize 3D-LUT coordinate mapping — shared host + `spirv` math.
//!
//! The GPU `lut_quantize_fs` entry point snaps each pixel to a palette colour by
//! sampling a `size`-edge 3D LUT cube with NEAREST filtering. `lut_coord` maps a
//! `[0,1]` channel value to the normalized texture coordinate that selects the
//! SAME cube cell the CPU builder (`forge_gpu::lut_quantize::QuantizeLut`) wrote —
//! the two MUST agree so the GPU output equals the CPU reference exactly. Mirror
//! discipline: keep in lockstep with `QuantizeLut::cell_for_channel`.

/// Normalized 3D-LUT coordinate for one channel. `c` in `[0,1]`; `size` = cube
/// edge. Picks the round-to-nearest cell of `c*(size-1)` and returns its texel
/// CENTER, so a NEAREST sample lands exactly on that cell — no interpolation
/// between palette colours (a hard quantize, not a blend).
#[inline]
pub fn lut_coord(c: f32, size: u32) -> f32 {
    if size < 2 {
        return 0.5;
    }
    let s = size as f32;
    let cell = (c * (s - 1.0) + 0.5) as u32 as f32; // round-to-nearest (c >= 0)
    (cell + 0.5) / s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recover the cube cell a normalized coordinate selects under NEAREST.
    fn cell_of(coord: f32, size: u32) -> u32 {
        (coord * size as f32) as u32
    }

    #[test]
    fn coord_selects_round_nearest_cell() {
        let size = 17u32;
        for v in 0u32..=255 {
            let c = v as f32 / 255.0;
            // Must equal the CPU integer round formula (QuantizeLut::cell_for_channel).
            let expected = ((v * (size - 1) + 127) / 255).min(size - 1);
            assert_eq!(cell_of(lut_coord(c, size), size), expected, "channel v={v}");
        }
    }

    #[test]
    fn endpoints_hit_first_and_last_cell() {
        let size = 33u32;
        assert_eq!(cell_of(lut_coord(0.0, size), size), 0);
        assert_eq!(cell_of(lut_coord(1.0, size), size), size - 1);
    }
}
