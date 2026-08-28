//! canvas_bridge — the Canvas-physics fold (Sean 2026-08-17 "fold to prim").
//!
//! v2's `forge-studio::forge_vision_lab::VixelLab::step` bridged a painted
//! pixel canvas into `forge_gpu::vixel_pass::step_field`'s CUDA-backed
//! automata. v3 needs no such bridge to be CUDA-backed: [`step_matter`] is
//! already CPU-integer-deterministic (`field::matter`, ported from the same
//! v2 lineage). The only missing piece was a way to drive it FROM a real
//! [`PixelCanvasState`] instead of a bespoke atom buffer — this module is
//! that piece, and nothing else.
//!
//! One tick: extract every painted (non-transparent) pixel into a scratch
//! [`FieldStack`] at the caller-chosen `material_id`, run one [`step_matter`]
//! tick, then clear the canvas and re-deposit the (possibly moved) cells —
//! colour preserved via [`FieldBuffer::rgba_at`]. `material_id` is an
//! explicit caller input, not derived from colour: v3 dropped v2's packed
//! `colour_id` (CLAUDE.md recon, 2026-08-17), so a painted RGBA pixel alone
//! carries no material — the Officina-fold (pigment→material mapping) is a
//! separate, later concern.

use crate::field::matter::step_matter;
use crate::field::FieldStack;
use forge_canvas_v3::pixel_canvas::PixelCanvasState;

/// Run one physics tick over `canvas`'s painted pixels at `material_id`.
///
/// Returns the tick's changed-cell count (the same counter [`step_matter`]
/// reports). A canvas with no painted pixels is a no-op (`0`).
pub fn step_pixel_canvas(canvas: &mut PixelCanvasState, material_id: u8, tick: u64) -> u32 {
    let (w, h) = (canvas.width, canvas.height);
    if w == 0 || h == 0 {
        return 0;
    }

    let mut stack = FieldStack::new(w, h);
    {
        let buf = &mut stack.layers[0].buffer;
        for y in 0..h {
            for x in 0..w {
                let rgba = canvas.get_pixel(x, y);
                if rgba[3] == 0 {
                    continue; // unpainted — no cell
                }
                let i = (y * w + x) as usize;
                buf.set_rgba(i, rgba);
                buf.material_id[i] = material_id;
            }
        }
    }

    let changed = step_matter(&mut stack, 0, tick);

    canvas.clear();
    let buf = &stack.layers[0].buffer;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if buf.coverage[i] == 0 {
                continue;
            }
            canvas.set_pixel(x, y, buf.rgba_at(i));
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::matter::MAT_SAND;

    fn occupied_cell(canvas: &PixelCanvasState) -> Option<(u32, u32, [u8; 4])> {
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let px = canvas.get_pixel(x, y);
                if px[3] != 0 {
                    return Some((x, y, px));
                }
            }
        }
        None
    }

    /// Mirrors v2's `a_painted_grain_falls_to_the_floor_with_its_colour`
    /// (forge_vision_lab.rs) on the real v3 canvas + field primitives.
    #[test]
    fn a_painted_grain_falls_to_the_floor_with_its_colour() {
        let mut canvas = PixelCanvasState::new(4, 4);
        let cid = [0xE6, 0x5A, 0x14, 255]; // forge-orange, opaque
        canvas.set_pixel(2, 0, cid); // top row
        assert_eq!(occupied_cell(&canvas), Some((2, 0, cid)));

        let mut ticks = 0u64;
        loop {
            let changed = step_pixel_canvas(&mut canvas, MAT_SAND, ticks);
            ticks += 1;
            if changed == 0 {
                break;
            }
            assert!(ticks < 100, "the grain must settle, not loop");
        }

        assert_eq!(
            occupied_cell(&canvas),
            Some((2, 3, cid)),
            "grain rests on the floor, colour preserved"
        );
    }

    #[test]
    fn a_grain_already_on_the_floor_is_at_rest() {
        let mut canvas = PixelCanvasState::new(4, 4);
        canvas.set_pixel(0, 3, [10, 20, 30, 255]); // bottom row
        let changed = step_pixel_canvas(&mut canvas, MAT_SAND, 0);
        assert_eq!(changed, 0, "a grain on the floor does not move");
    }

    #[test]
    fn an_empty_canvas_is_at_rest() {
        let mut canvas = PixelCanvasState::new(8, 8);
        let changed = step_pixel_canvas(&mut canvas, MAT_SAND, 0);
        assert_eq!(changed, 0, "no painted pixels -> nothing moves");
    }

    #[test]
    fn a_zero_sized_canvas_is_a_no_op() {
        let mut canvas = PixelCanvasState::new(0, 0);
        assert_eq!(step_pixel_canvas(&mut canvas, MAT_SAND, 0), 0);
    }
}
