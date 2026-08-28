//! field/ — the SUPERMAX-ATOM field simulation. Ported from v2
//! `F:\NewRepo\crates\sf-wasm\src\{matter,reactions,electricity,heat}.rs`
//! (2026-08-17, sf-wasm port Wave 1; `sky_lightning.rs` + `meteors.rs` follow
//! in Wave 2 once `lightning.rs` has its one v3 home — today it lives inside
//! forge-audio-v3, too heavy a dep for this crate's Crate-Zero footprint).
//!
//! Donor logic verbatim. The two v2 store adaptations live here, nowhere else:
//!   * [`FieldBuffer`] replaces v2 `forge_core::vibe_buffer::VibeBuffer` —
//!     the same seven sim lanes; the editor-only `perceptual`/`colour_id`
//!     lanes (OKLCH + 31-bit ColourID) collapse into one plain `rgba` lane,
//!     because the field organ displays palette/phase tints, never authored
//!     exact colour.
//!   * [`FieldStack`] replaces v2 `forge_core::layer_stack::LayerStack` — the
//!     sim touches `layers[active].buffer` + dims; name/opacity/visible/
//!     alpha-lock landed 2026-08-17 (Officina fold-to-prim, Step 2). Tonal
//!     `BlendMode` stays unported, same as v2's own incremental history.
//! Integer-deterministic throughout; floats never cross.

pub mod canvas_bridge;
pub mod electricity;
pub mod heat;
pub mod matter;
pub mod reactions;

use serde::{Deserialize, Serialize};

pub use forge_correspondence_v3::correspondence::palette_rgb;

/// The field's SoA cell store — one entry per cell, row-major
/// (`i = y * width + x`). Lanes are separate `Vec`s so each pass borrows only
/// what it reads. Allocated once at organ birth; mutated in place.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldBuffer {
    /// Grid width in cells.
    pub width: u32,
    /// Grid height in cells.
    pub height: u32,
    /// 6-bit key into `forge_correspondence_v3::material_registry::MATERIALS`.
    pub material_id: Vec<u8>,
    /// 6-bit key into the essence palette (carried whole through swaps).
    pub essence_id: Vec<u8>,
    /// Anti-alias fill / presence (0 = void cell).
    pub coverage: Vec<u8>,
    /// Sub-pixel slope `[x, z]` (signed).
    pub normal: Vec<[i8; 2]>,
    /// Glow lane (additive highlight).
    pub bloom: Vec<u8>,
    /// Light / fuel / charge / heat life, per the cell's phase.
    pub light: Vec<u8>,
    /// Phase lane: 0 rest · 1 fire · 2 smoke · 3 charged · 4 hot.
    pub phase: Vec<u8>,
    /// Authored display colour (the collapsed `perceptual`+`colour_id` lanes).
    pub rgba: Vec<[u8; 4]>,
}

impl FieldBuffer {
    /// A zeroed buffer of `width * height` cells.
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width as usize) * (height as usize);
        Self {
            width,
            height,
            material_id: vec![0; n],
            essence_id: vec![0; n],
            coverage: vec![0; n],
            normal: vec![[0, 0]; n],
            bloom: vec![0; n],
            light: vec![0; n],
            phase: vec![0; n],
            rgba: vec![[0, 0, 0, 0]; n],
        }
    }

    /// Number of cells (`width * height`).
    pub fn len(&self) -> usize {
        self.material_id.len()
    }

    /// True when the buffer holds zero cells.
    pub fn is_empty(&self) -> bool {
        self.material_id.is_empty()
    }

    /// Hard-write one RGBA sample at cell `i` (v2 `VibeBuffer::set_rgba`
    /// semantics on the collapsed lane): colour lands verbatim, coverage is
    /// overwritten with the alpha; `a == 0` erases the cell's colour.
    pub fn set_rgba(&mut self, i: usize, rgba: [u8; 4]) {
        if rgba[3] == 0 {
            self.rgba[i] = [0, 0, 0, 0];
            self.coverage[i] = 0;
            return;
        }
        self.rgba[i] = rgba;
        self.coverage[i] = rgba[3];
    }

    /// Read cell `i` as RGBA — colour from the collapsed lane, alpha = coverage.
    pub fn rgba_at(&self, i: usize) -> [u8; 4] {
        let [r, g, b, _] = self.rgba[i];
        [r, g, b, self.coverage[i]]
    }
}

/// One field layer — the sim half of v2's `PaintLayer` (name, opacity,
/// visibility, alpha-lock; tonal `BlendMode` stays unported, same as v2's own
/// incremental history — every layer composites `Normal` today).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldLayer {
    /// Layer label (layer-panel display only).
    pub name: String,
    /// The layer's cell store.
    pub buffer: FieldBuffer,
    /// Opacity in permyriad (0 = transparent, 10000 = opaque).
    pub opacity_pmy: u16,
    /// Whether this layer contributes to `flatten_to_rgba`.
    pub visible: bool,
    /// When true, paint only lands on cells that already have coverage > 0.
    pub alpha_lock: bool,
}

impl FieldLayer {
    /// A transparent, fully-opaque, visible layer of `width * height` cells.
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            name: name.into(),
            buffer: FieldBuffer::new(width, height),
            opacity_pmy: 10_000,
            visible: true,
            alpha_lock: false,
        }
    }
}

/// The layer stack the sim + authoring contract share (v2 `LayerStack`
/// shape): `width`/`height`/`layers[active].buffer`, one base layer at birth,
/// ordered bottom-up (index 0 = bottom, last = top).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldStack {
    /// Grid width in cells (shared by every layer).
    pub width: u32,
    /// Grid height in cells (shared by every layer).
    pub height: u32,
    /// The layers, bottom-up; born with one base layer.
    pub layers: Vec<FieldLayer>,
    /// Index of the layer sim/paint ops act on (always a valid index).
    pub active: usize,
}

impl FieldStack {
    /// A stack with one opaque base layer ("Layer 1"), active.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            layers: vec![FieldLayer::new("Layer 1", width, height)],
            active: 0,
        }
    }

    /// Select the layer sim/paint ops act on; out-of-range is a no-op.
    pub fn set_active(&mut self, idx: usize) {
        if idx < self.layers.len() {
            self.active = idx;
        }
    }

    /// Push a new transparent layer on TOP and make it active. Returns its index.
    pub fn add_layer(&mut self, name: impl Into<String>) -> usize {
        self.layers.push(FieldLayer::new(name, self.width, self.height));
        self.active = self.layers.len() - 1;
        self.active
    }

    /// Delete layer `idx`. The stack always keeps at least one layer (deleting
    /// the last is a no-op). `active` is clamped to stay valid.
    pub fn delete_layer(&mut self, idx: usize) {
        if self.layers.len() <= 1 || idx >= self.layers.len() {
            return;
        }
        self.layers.remove(idx);
        if self.active >= self.layers.len() {
            self.active = self.layers.len() - 1;
        }
    }

    /// Swap layer `idx` with the one above it (toward the top). No-op at the top.
    pub fn move_layer_up(&mut self, idx: usize) {
        if idx + 1 < self.layers.len() {
            self.layers.swap(idx, idx + 1);
            if self.active == idx {
                self.active = idx + 1;
            } else if self.active == idx + 1 {
                self.active = idx;
            }
        }
    }

    /// Swap layer `idx` with the one below it (toward the bottom). No-op at the bottom.
    pub fn move_layer_down(&mut self, idx: usize) {
        if idx > 0 && idx < self.layers.len() {
            self.move_layer_up(idx - 1);
        }
    }

    /// Toggle layer `idx` visibility; out-of-range is a no-op.
    pub fn toggle_visible(&mut self, idx: usize) {
        if let Some(l) = self.layers.get_mut(idx) {
            l.visible = !l.visible;
        }
    }

    /// The active layer's buffer (paint/sim ops target this).
    pub fn active_buffer(&self) -> &FieldBuffer {
        &self.layers[self.active].buffer
    }

    /// Mutable active-layer buffer (paint/sim ops write here).
    pub fn active_buffer_mut(&mut self) -> &mut FieldBuffer {
        &mut self.layers[self.active].buffer
    }

    /// Stamp one RGBA sample onto the active layer — hard write. Honours the
    /// active layer's `alpha_lock` (paint only lands where coverage already
    /// exists), mirroring v2's `LayerStack::paint_rgba`.
    pub fn paint_rgba(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = (y * self.width + x) as usize;
        let layer = &mut self.layers[self.active];
        if layer.alpha_lock && layer.buffer.coverage[i] == 0 {
            return;
        }
        layer.buffer.set_rgba(i, rgba);
    }

    /// Composite every visible layer (bottom-up; topmost painted layer's
    /// colour wins each cell, effective coverage = `coverage * opacity_pmy /
    /// 10000`) into one packed RGBA8 buffer (`width * height * 4` bytes).
    /// v3 mirrors v2's `LayerStack::flatten_to_rgba` on the collapsed `rgba`
    /// lane directly (no palette re-derivation needed — the colour rides the
    /// cell already).
    pub fn flatten_to_rgba(&self) -> Vec<u8> {
        let n = (self.width * self.height) as usize;
        let mut out = vec![0u8; n * 4];
        for i in 0..n {
            let (mut rgb, mut cov) = ([0u8; 3], 0u8);
            for layer in self.layers.iter().filter(|l| l.visible && l.opacity_pmy > 0) {
                if i >= layer.buffer.len() {
                    continue;
                }
                let c = (layer.buffer.coverage[i] as u32 * layer.opacity_pmy as u32 / 10_000) as u8;
                if c == 0 {
                    continue;
                }
                let [r, g, b, _] = layer.buffer.rgba[i];
                rgb = [r, g, b];
                cov = c;
            }
            out[i * 4] = rgb[0];
            out[i * 4 + 1] = rgb[1];
            out[i * 4 + 2] = rgb[2];
            out[i * 4 + 3] = cov;
        }
        out
    }
}

#[cfg(test)]
mod layer_stack_tests {
    use super::*;

    #[test]
    fn add_layer_pushes_on_top_and_activates_it() {
        let mut stack = FieldStack::new(2, 2);
        let idx = stack.add_layer("Layer 2");
        assert_eq!(idx, 1);
        assert_eq!(stack.active, 1);
        assert_eq!(stack.layers.len(), 2);
    }

    #[test]
    fn delete_layer_never_empties_the_stack() {
        let mut stack = FieldStack::new(2, 2);
        stack.delete_layer(0);
        assert_eq!(stack.layers.len(), 1, "deleting the last layer is a no-op");
    }

    #[test]
    fn delete_layer_clamps_active_index() {
        let mut stack = FieldStack::new(2, 2);
        stack.add_layer("Layer 2");
        stack.set_active(1);
        stack.delete_layer(1);
        assert_eq!(stack.active, 0);
        assert_eq!(stack.layers.len(), 1);
    }

    #[test]
    fn move_layer_up_and_down_swap_order_and_track_active() {
        let mut stack = FieldStack::new(2, 2);
        stack.add_layer("Layer 2"); // active = 1
        stack.move_layer_down(1);
        assert_eq!(stack.active, 0, "active follows the layer it was tracking");
        assert_eq!(stack.layers[0].name, "Layer 2");
        stack.move_layer_up(0);
        assert_eq!(stack.active, 1);
        assert_eq!(stack.layers[1].name, "Layer 2");
    }

    #[test]
    fn toggle_visible_flips_the_flag_and_hides_from_flatten() {
        let mut stack = FieldStack::new(2, 2);
        stack.paint_rgba(0, 0, [200, 0, 0, 255]);
        assert_eq!(stack.flatten_to_rgba()[3], 255, "painted cell visible pre-toggle");
        stack.toggle_visible(0);
        assert!(!stack.layers[0].visible);
        assert_eq!(stack.flatten_to_rgba()[3], 0, "hidden layer contributes nothing");
    }

    #[test]
    fn alpha_lock_blocks_paint_on_uncovered_cells() {
        let mut stack = FieldStack::new(2, 2);
        stack.layers[0].alpha_lock = true;
        stack.paint_rgba(0, 0, [1, 2, 3, 255]);
        assert_eq!(
            stack.active_buffer().rgba_at(0),
            [0, 0, 0, 0],
            "alpha-locked layer refuses paint on an empty cell"
        );
    }

    #[test]
    fn alpha_lock_allows_paint_over_existing_coverage() {
        let mut stack = FieldStack::new(2, 2);
        stack.paint_rgba(0, 0, [1, 2, 3, 255]); // seed coverage, lock still off
        stack.layers[0].alpha_lock = true;
        stack.paint_rgba(0, 0, [9, 9, 9, 255]);
        assert_eq!(stack.active_buffer().rgba_at(0), [9, 9, 9, 255]);
    }

    #[test]
    fn flatten_to_rgba_lets_the_topmost_visible_painted_layer_win() {
        let mut stack = FieldStack::new(2, 2);
        stack.paint_rgba(0, 0, [10, 20, 30, 255]); // Layer 1, bottom
        stack.add_layer("Layer 2");
        stack.paint_rgba(0, 0, [200, 100, 50, 255]); // Layer 2, top, active
        let flat = stack.flatten_to_rgba();
        assert_eq!(&flat[0..4], &[200, 100, 50, 255], "top layer wins the cell");
    }

    #[test]
    fn flatten_to_rgba_applies_effective_coverage_from_opacity() {
        let mut stack = FieldStack::new(1, 1);
        stack.paint_rgba(0, 0, [255, 0, 0, 255]);
        stack.layers[0].opacity_pmy = 5_000; // half opacity
        let flat = stack.flatten_to_rgba();
        assert_eq!(flat[3], 127, "255 * 5000 / 10000 = 127 (integer truncation)");
    }
}
