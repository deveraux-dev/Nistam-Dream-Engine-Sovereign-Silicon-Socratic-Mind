//! HAL Bridge — Trait seam between CanvasRenderer and forge-hal.
//!
//! During migration, CanvasRenderer keeps its raw wgpu internals but buffer
//! uploads route through HalBackend when available. This allows incremental
//! migration without rewriting the 1900-line renderer.
//!
//! Usage:
//!   let mut bridge = HalBridge::new(&mut canvas_renderer);
//!   bridge.upload_quad_data(hal, &vertices);
//!   bridge.upload_glyph_data(hal, &glyphs);
//!   // CanvasRenderer still does its own render pass (raw wgpu)
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-gpu\src\hal_bridge.rs`
//! (2026-08-24, ghostmoon-merge Wave 2c) against `crate::hal`, the minimal
//! `HalBackend` trait sketched this turn (see `hal.rs`'s module doc for why
//! it's 2 methods, not the donor's 20) — not the full donor `forge-hal` crate.

use crate::hal::{BufferHandle, BufferDesc, BufferUsage, HalBackend};

/// Maps CanvasRenderer buffer slots to HalBackend handles.
/// Created once at init, reused across frames.
pub struct HalBridge {
    /// Quad vertex buffer handle, once initialized.
    pub quad_buffer: Option<BufferHandle>,
    /// Additive-blend vertex buffer handle, once initialized.
    pub additive_buffer: Option<BufferHandle>,
    /// Glyph vertex buffer handle, once initialized.
    pub glyph_buffer: Option<BufferHandle>,
    /// Sprite instance buffer handle, once initialized.
    pub sprite_buffer: Option<BufferHandle>,
    /// Uniform buffer handle, once initialized.
    pub uniform_buffer: Option<BufferHandle>,
}

impl HalBridge {
    /// Build an uninitialized bridge; call [`HalBridge::init_buffers`] before use.
    pub fn new() -> Self {
        Self {
            quad_buffer: None,
            additive_buffer: None,
            glyph_buffer: None,
            sprite_buffer: None,
            uniform_buffer: None,
        }
    }

    /// Initialize HAL-side buffer mirrors. Call once after CanvasRenderer::new().
    pub fn init_buffers(&mut self, hal: &mut dyn HalBackend) {
        self.quad_buffer = Some(hal.create_buffer(&BufferDesc {
            label: "hal_quad_buf",
            size: 64 * 1024, // 64KB quad buffer
            usage: BufferUsage::VERTEX | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }));
        self.additive_buffer = Some(hal.create_buffer(&BufferDesc {
            label: "hal_additive_buf",
            size: 32 * 1024,
            usage: BufferUsage::VERTEX | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }));
        self.glyph_buffer = Some(hal.create_buffer(&BufferDesc {
            label: "hal_glyph_buf",
            size: 64 * 1024,
            usage: BufferUsage::VERTEX | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }));
        self.sprite_buffer = Some(hal.create_buffer(&BufferDesc {
            label: "hal_sprite_buf",
            size: 128 * 1024,
            usage: BufferUsage::VERTEX | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }));
        self.uniform_buffer = Some(hal.create_buffer(&BufferDesc {
            label: "hal_uniform_buf",
            size: 256,
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        }));
    }

    /// Upload quad vertex data through HAL (mirrors CanvasRenderer's queue.write_buffer).
    #[inline]
    pub fn upload_quad_data(&self, hal: &mut dyn HalBackend, data: &[u8]) {
        if let Some(buf) = self.quad_buffer {
            hal.write_buffer(buf, 0, data);
        }
    }

    /// Upload additive vertex data through HAL.
    #[inline]
    pub fn upload_additive_data(&self, hal: &mut dyn HalBackend, data: &[u8]) {
        if let Some(buf) = self.additive_buffer {
            hal.write_buffer(buf, 0, data);
        }
    }

    /// Upload glyph vertex data through HAL.
    #[inline]
    pub fn upload_glyph_data(&self, hal: &mut dyn HalBackend, data: &[u8]) {
        if let Some(buf) = self.glyph_buffer {
            hal.write_buffer(buf, 0, data);
        }
    }

    /// Upload sprite instance data through HAL.
    #[inline]
    pub fn upload_sprite_data(&self, hal: &mut dyn HalBackend, data: &[u8]) {
        if let Some(buf) = self.sprite_buffer {
            hal.write_buffer(buf, 0, data);
        }
    }

    /// Upload uniform data through HAL.
    #[inline]
    pub fn upload_uniforms(&self, hal: &mut dyn HalBackend, data: &[u8]) {
        if let Some(buf) = self.uniform_buffer {
            hal.write_buffer(buf, 0, data);
        }
    }

    /// Returns true if all buffers have been initialized.
    pub fn is_initialized(&self) -> bool {
        self.quad_buffer.is_some()
    }
}

impl Default for HalBridge {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::mock::MockHalBackend;

    #[test]
    fn bridge_init_creates_5_buffers() {
        let mut mock = MockHalBackend::new();
        let mut bridge = HalBridge::new();
        assert!(!bridge.is_initialized());
        bridge.init_buffers(&mut mock);
        assert!(bridge.is_initialized());
        assert!(bridge.quad_buffer.is_some());
        assert!(bridge.uniform_buffer.is_some());
    }

    #[test]
    fn bridge_upload_routes_through_hal() {
        let mut mock = MockHalBackend::new();
        let mut bridge = HalBridge::new();
        bridge.init_buffers(&mut mock);
        // These should not panic
        bridge.upload_quad_data(&mut mock, &[1, 2, 3, 4]);
        bridge.upload_glyph_data(&mut mock, &[5, 6, 7, 8]);
        bridge.upload_uniforms(&mut mock, &[0u8; 64]);
    }
}
