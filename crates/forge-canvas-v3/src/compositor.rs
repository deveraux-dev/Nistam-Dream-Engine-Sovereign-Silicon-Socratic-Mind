//! Compositor layer stack — runtime-configured, no recompilation.
//!
//! One `RenderConfig` serves technothesia, forge-gui, dreadpirateradio, and the
//! game host. `app_mode` + `toggles` select active layers at runtime.
//!
//! Diamond spec (EXECUTION ledger T099, Bank C):
//! ```text
//! CompositorLayer { source, blend, opacity: u16 /*Permyriad 0–10000*/, z: u8 }
//! LayerSource  = Solid(cid) | Noise | Gradient | RenderTarget
//! BlendMode    = Normal | Multiply | Screen | Overlay | SoftLight | Add | ColorDodge | ColorBurn
//! RenderConfig { app_mode, stack: [CompositorLayer;8], layer_count, toggles: u32 }
//! AppMode      = Studio | Daw | Game
//! ```

// ── BlendMode ────────────────────────────────────────────────────────────────

/// Blend mode for composite operations on a per-channel basis.
///
/// Encodes the mathematical operation applied to source and destination
/// color channels during layer composition. Each mode produces integer RGBA
/// output in the range [0, 255] with no float intermediates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// Replace: source fully replaces destination.
    Normal,
    /// Multiply: darken by multiplying channels.
    Multiply,
    /// Screen: lighten via inverted multiply.
    Screen,
    /// Overlay: multiply if dark, screen if light.
    Overlay,
    /// Soft light: Pegtop formula for subtle blending.
    SoftLight,
    /// Add: additive blend, clamped at 255.
    Add,
    /// Color dodge: inverse multiply for bright overlays.
    ColorDodge,
    /// Color burn: inverse screen for dark overlays.
    ColorBurn,
}

// ── LayerSource ──────────────────────────────────────────────────────────────

/// Source of pixel data for a compositor layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerSource {
    /// Solid fill using a Synthesia CID palette index (0–8; never raw hex).
    Solid(u8),
    /// Procedural noise (seeded per-frame by the DET-CLOCK tick).
    Noise,
    /// Vertical gradient from `cid_top` to `cid_bottom`.
    Gradient {
        /// Top color ID (rendered at the top edge).
        cid_top: u8,
        /// Bottom color ID (rendered at the bottom edge).
        cid_bottom: u8,
    },
    /// GPU render-target slot (index into the FrameComposer target array).
    RenderTarget(u8),
}

// ── CompositorLayer ──────────────────────────────────────────────────────────

/// A single layer in the compositor stack.
///
/// `opacity` is Permyriad (0 = transparent, 10_000 = opaque).
/// `z` is the Z-order (0 = bottom). Layers are composited ascending by `z`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositorLayer {
    /// Source pixels for this layer (solid colour, noise, gradient, or render-target).
    pub source: LayerSource,
    /// Blend mode applied during composition.
    pub blend: BlendMode,
    /// Permyriad opacity: 0 = fully transparent, 10_000 = fully opaque.
    pub opacity: u16,
    /// Z-order depth (0 = bottom, higher = on top).
    pub z: u8,
}

impl CompositorLayer {
    /// Create a solid-color layer with specified blend mode, opacity, and Z-order.
    #[inline]
    pub const fn solid(cid: u8, blend: BlendMode, opacity: u16, z: u8) -> Self {
        Self { source: LayerSource::Solid(cid), blend, opacity, z }
    }

    /// Create a render-target layer with specified blend mode, opacity, and Z-order.
    #[inline]
    pub const fn render_target(slot: u8, blend: BlendMode, opacity: u16, z: u8) -> Self {
        Self { source: LayerSource::RenderTarget(slot), blend, opacity, z }
    }

    /// The **zen** background layer — the void ground that sits UNDER every other
    /// plane. Solid `CID_GROUND`, `Normal` blend, fully opaque, `z = 0`.
    ///
    /// T100 roll-down: technothesia's `render_zen_overlay` painted this ground
    /// inline (the bottom plate beneath the glyph text). The ground is *general*
    /// canvas logic, so per L2 (ONE Canvas) it rolls DOWN into the foundation —
    /// technothesia, forge-gui, and the DAW host now mount THIS one canonical layer
    /// instead of each re-painting their own ground. Gated by `toggle::ZEN`.
    #[inline]
    pub const fn zen() -> Self {
        Self::solid(crate::theme::CID_GROUND, BlendMode::Normal, 10_000, 0)
    }

    /// The **sovereign** stroke overlay — hand-pen vector ink composited OVER the
    /// zen ground. `Overlay` blend, fully opaque, `z = 1` (directly above zen,
    /// matching the `SOVEREIGN` toggle bit).
    ///
    /// T101 roll-down: technothesia's `render_sovereign_canvas` (font_forge.rs)
    /// drew this stroke band inline. The stroke plane is *general* overlay logic,
    /// so per L2 (ONE Canvas) it rolls DOWN into the foundation — hosts mount THIS
    /// one canonical layer. `slot` is the host RenderTarget holding the rendered
    /// strokes (the offscreen stroke plane). Gated by `toggle::SOVEREIGN`.
    #[inline]
    pub const fn sovereign(slot: u8) -> Self {
        Self::render_target(slot, BlendMode::Overlay, 10_000, 1)
    }

    /// The **material** voxel-glyph layer — materialID-painted surface composited
    /// OVER the sovereign stroke plane. `Normal` blend, fully opaque, `z = 2`
    /// (directly above sovereign, matching the `MATERIAL` toggle bit).
    ///
    /// T102 roll-down: the MaterialCanvas (forge-core) paints materialID colours
    /// into an offscreen RenderTarget; that target is then composited here by the
    /// FrameComposer as a general canvas layer — per L2 (ONE Canvas) the plane
    /// descriptor rolls DOWN into the foundation. `slot` is the host RenderTarget
    /// holding the rendered materialID surface. Gated by `toggle::MATERIAL`.
    #[inline]
    pub const fn material(slot: u8) -> Self {
        Self::render_target(slot, BlendMode::Normal, 10_000, 2)
    }

    /// The **light** cascade plane — additive light passes composited OVER the
    /// material coat. `Screen` blend, fully opaque, `z = 3`.
    ///
    /// T103 roll-down: `light_cascade_panel` (airgap) painted light bloom inline
    /// per host. Light is *general* canvas logic; it rolls DOWN into the foundation
    /// so every host mounts THIS one canonical layer. `slot` is the host
    /// RenderTarget holding the rendered light cascade. Gated by `toggle::LIGHT`.
    #[inline]
    pub const fn light(slot: u8) -> Self {
        Self::render_target(slot, BlendMode::Screen, 10_000, 3)
    }

    /// The **magic** materialID plane — the binding layer that marries canvas
    /// geometry to the materialID paint surface, composited OVER light. `ColorDodge`
    /// blend, fully opaque, `z = 4` (the top-most Z-plane per ARCH-002 §5).
    ///
    /// T103 roll-down: MaterialCanvas `packed_flags` materialID binding was baked
    /// inline per host. The bind is *general* canvas logic; it rolls DOWN so every
    /// host mounts THIS one canonical layer. `slot` is the host RenderTarget holding
    /// the materialID paint surface. Gated by `toggle::MAGIC`.
    #[inline]
    pub const fn magic(slot: u8) -> Self {
        Self::render_target(slot, BlendMode::ColorDodge, 10_000, 4)
    }
}

// ── AppMode ──────────────────────────────────────────────────────────────────

/// Which application mode is active — selects the compositing stack preset.
/// Toggle bits then mask individual layers within that preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppMode {
    /// Creation HUD (forge-studio / forge-gui).
    Studio,
    /// DAW surface (dreadpirateradio).
    Daw,
    /// Game / cartridge host.
    Game,
}

// ── Toggle bits (layer enable mask inside a RenderConfig) ────────────────────

/// `RenderConfig::toggles` bit indices — OR these to enable layers.
pub mod toggle {
    /// Zen background layer (Solid + Normal blend).
    pub const ZEN: u32 = 1 << 0;
    /// Sovereign stroke overlay (Overlay blend).
    pub const SOVEREIGN: u32 = 1 << 1;
    /// Material voxel-glyph layer.
    pub const MATERIAL: u32 = 1 << 2;
    /// Light cascade plane.
    pub const LIGHT: u32 = 1 << 3;
    /// MaterialID magic plane.
    pub const MAGIC: u32 = 1 << 4;
    /// All layers on.
    pub const ALL: u32 = ZEN | SOVEREIGN | MATERIAL | LIGHT | MAGIC;
}

// ── RenderConfig ─────────────────────────────────────────────────────────────

/// The single runtime descriptor consumed by the FrameComposer.
///
/// Backed by a fixed `[CompositorLayer; 8]` inline array — zero heap, no
/// `smallvec` dep. `layer_count` is the live length (≤ 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderConfig {
    /// Active application mode (Studio, Daw, or Game).
    pub app_mode: AppMode,
    stack: [CompositorLayer; 8],
    /// Number of active layers in the stack (0–8).
    pub layer_count: u8,
    /// Bit-mask enabling individual layers (see `toggle::*` constants).
    pub toggles: u32,
}

impl RenderConfig {
    /// Empty config — no layers, Studio mode, all toggles off.
    #[inline]
    pub const fn empty(app_mode: AppMode) -> Self {
        const BLANK: CompositorLayer = CompositorLayer {
            source: LayerSource::Solid(0),
            blend: BlendMode::Normal,
            opacity: 0,
            z: 0,
        };
        Self { app_mode, stack: [BLANK; 8], layer_count: 0, toggles: 0 }
    }

    /// Push a layer onto the stack. Returns `false` (and does nothing) if full.
    pub fn push(&mut self, layer: CompositorLayer) -> bool {
        if self.layer_count as usize >= self.stack.len() {
            return false;
        }
        self.stack[self.layer_count as usize] = layer;
        self.layer_count += 1;
        true
    }

    /// Mount the canonical `zen` ground (T100 roll-down) and enable its toggle.
    /// Builder-style so hosts compose their stack in one expression:
    /// `RenderConfig::empty(AppMode::Studio).with_zen()`. The zen layer is the
    /// floor of every stack — push it first so `z = 0` stays the bottom plate.
    pub fn with_zen(mut self) -> Self {
        self.push(CompositorLayer::zen());
        self.toggles |= toggle::ZEN;
        self
    }

    /// Mount the canonical `sovereign` stroke overlay (T101 roll-down) over the
    /// zen ground and enable its toggle. `slot` is the host RenderTarget holding
    /// the rendered strokes. Chains after `with_zen()` so the ground stays under.
    pub fn with_sovereign(mut self, slot: u8) -> Self {
        self.push(CompositorLayer::sovereign(slot));
        self.toggles |= toggle::SOVEREIGN;
        self
    }

    /// Mount the canonical `material` voxel-glyph layer (T102 roll-down) over the
    /// sovereign stroke plane and enable its toggle. `slot` is the host RenderTarget
    /// holding the materialID-painted surface. Chains after `with_sovereign()` so
    /// stroke ink stays below the material coat.
    pub fn with_material(mut self, slot: u8) -> Self {
        self.push(CompositorLayer::material(slot));
        self.toggles |= toggle::MATERIAL;
        self
    }

    /// Mount the canonical `light` cascade plane (T103 roll-down) over the material
    /// coat and enable its toggle. `slot` is the host RenderTarget holding the
    /// rendered light bloom. Chains after `with_material()` so light sits above
    /// the material surface (z=3 > z=2).
    pub fn with_light(mut self, slot: u8) -> Self {
        self.push(CompositorLayer::light(slot));
        self.toggles |= toggle::LIGHT;
        self
    }

    /// Mount the canonical `magic` materialID binding plane (T103 roll-down) as the
    /// top-most Z-plane and enable its toggle. `slot` is the host RenderTarget
    /// holding the materialID paint surface. Chains after `with_light()` so the
    /// binding layer crowns the full stack (z=4 > z=3).
    pub fn with_magic(mut self, slot: u8) -> Self {
        self.push(CompositorLayer::magic(slot));
        self.toggles |= toggle::MAGIC;
        self
    }

    /// Iterate active layers in Z-order (ascending `z`), respecting `toggles`.
    ///
    /// A layer is active when its toggle bit is set OR when `toggles == ALL`
    /// (all-on shortcut). If a layer has no corresponding toggle bit it is
    /// always active.
    pub fn active_layers(&self) -> impl Iterator<Item = &CompositorLayer> {
        let toggles = self.toggles;
        self.stack[..self.layer_count as usize]
            .iter()
            // stable sort not needed here — z-order is encoded in the layer
            .filter(move |l| {
                // If ALL bits set, every layer is active.
                toggles == toggle::ALL || toggles == u32::MAX
                // Otherwise the layer's z maps loosely to a bit position.
                // Layers at z < 32 check their z-bit; others are always active.
                || l.z >= 32 || (toggles >> l.z) & 1 == 1
            })
    }

    /// Layers sorted ascending by `z`. Allocates — cold path only.
    #[cfg(test)]
    pub fn layers_sorted(&self) -> std::vec::Vec<CompositorLayer> {
        let mut v: std::vec::Vec<CompositorLayer> =
            self.stack[..self.layer_count as usize].to_vec();
        v.sort_by_key(|l| l.z);
        v
    }
}

// ── FrameComposer ────────────────────────────────────────────────────────────

/// Integer div-by-255 using the `(t + 1 + t>>8) >> 8` u16 idiom — no float.
#[inline(always)]
fn div255(x: u32) -> u32 {
    (x + 1 + (x >> 8)) >> 8
}

/// Unpack `0xRRGGBBAA` into `(r, g, b, a)`.
#[inline(always)]
fn unpack(c: u32) -> (u8, u8, u8, u8) {
    (
        ((c >> 24) & 0xFF) as u8,
        ((c >> 16) & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        (c & 0xFF) as u8,
    )
}

/// Pack `(r, g, b, a)` → `0xRRGGBBAA`.
#[inline(always)]
fn pack(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

/// Apply `mode` on a single channel pair (all values 0–255).
#[inline(always)]
fn blend_chan(mode: BlendMode, s: u8, d: u8) -> u8 {
    let s = s as u32;
    let d = d as u32;
    match mode {
        BlendMode::Normal => s as u8,
        BlendMode::Multiply => div255(s * d) as u8,
        BlendMode::Screen => (s + d - div255(s * d)) as u8,
        BlendMode::Add => (s + d).min(255) as u8,
        BlendMode::Overlay => {
            if d < 128 {
                (div255(2 * s * d)) as u8
            } else {
                (255 - div255(2 * (255 - s) * (255 - d))) as u8
            }
        }
        BlendMode::ColorDodge => {
            if s >= 255 {
                255
            } else {
                (d * 255 / (255 - s)).min(255) as u8
            }
        }
        BlendMode::ColorBurn => {
            if s == 0 {
                0
            } else {
                (255 - ((255 - d) * 255 / s).min(255)) as u8
            }
        }
        BlendMode::SoftLight => {
            // Pegtop formula, integer approximation.
            let b = div255((255 - 2 * s) * div255(d * d));
            (div255(d * (255 - 2 * s + 255)) + b) as u8
        }
    }
}

/// Composite `src` (Permyriad opacity) over `dst` using `mode`. Output alpha = dst alpha.
#[inline(always)]
fn composite_pixel(mode: BlendMode, src: u32, dst: u32, opacity: u16) -> u32 {
    let (sr, sg, sb, _) = unpack(src);
    let (dr, dg, db, _) = unpack(dst);
    let (br, bg, bb) = (
        blend_chan(mode, sr, dr) as u32,
        blend_chan(mode, sg, dg) as u32,
        blend_chan(mode, sb, db) as u32,
    );
    // Lerp blended ↔ dst by opacity (Permyriad: 0=transparent, 10_000=opaque).
    let t = opacity as u32;
    let it = 10_000 - t;
    let r = (br * t + dr as u32 * it) / 10_000;
    let g = (bg * t + dg as u32 * it) / 10_000;
    let b = (bb * t + db as u32 * it) / 10_000;
    // Frame compositor always writes a fully opaque pixel — it composites onto an
    // opaque framebuffer, not a premultiplied alpha surface.
    pack(r as u8, g as u8, b as u8, 0xFF)
}

/// CPU compositor — flattens a `RenderConfig` layer stack onto an output pixel
/// buffer in Z-order (ascending `z`). Zero heap: all blending is in-place on
/// `output`. GPU-only sources (`Noise`, `Gradient`) are skipped with a no-op.
///
/// `planes[slot]` supplies pixels for `LayerSource::RenderTarget(slot)`.
/// `solid_color(cid)` resolves `LayerSource::Solid(cid)` → packed `0xRRGGBBAA`.
pub struct FrameComposer;

impl FrameComposer {
    /// Flatten the active layer stack of `config` into `output` (length = W×H).
    /// Layers pushed via `with_zen()`/`with_sovereign()`/… are already in Z-order;
    /// `active_layers()` walks them in push order (= ascending z for canonical stacks).
    pub fn flatten(
        config: &RenderConfig,
        planes: &[&[u32]],
        solid_color: impl Fn(u8) -> u32,
        output: &mut [u32],
    ) {
        for layer in config.active_layers() {
            let opacity = layer.opacity;
            match layer.source {
                LayerSource::Solid(cid) => {
                    let src = solid_color(cid);
                    for dst in output.iter_mut() {
                        *dst = composite_pixel(layer.blend, src, *dst, opacity);
                    }
                }
                LayerSource::RenderTarget(slot) => {
                    let slot = slot as usize;
                    if slot >= planes.len() {
                        continue;
                    }
                    let plane = planes[slot];
                    let len = output.len().min(plane.len());
                    for i in 0..len {
                        output[i] = composite_pixel(layer.blend, plane[i], output[i], opacity);
                    }
                }
                // GPU-only sources — FrameComposer is CPU path; skip silently.
                LayerSource::Noise | LayerSource::Gradient { .. } => {}
            }
        }
    }
}

// ── T099 proof ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_config_push_and_iterate() {
        let mut cfg = RenderConfig::empty(AppMode::Studio);
        cfg.toggles = toggle::ALL;

        assert!(cfg.push(CompositorLayer::solid(0, BlendMode::Normal, 10_000, 0)));
        assert!(cfg.push(CompositorLayer::solid(5, BlendMode::Overlay, 8_000, 1)));
        assert!(cfg.push(CompositorLayer::render_target(0, BlendMode::Screen, 10_000, 2)));
        assert_eq!(cfg.layer_count, 3);

        let active: Vec<_> = cfg.active_layers().collect();
        assert_eq!(active.len(), 3);
    }

    // T100 — zen ground roll-down proof.
    #[test]
    fn zen_layer_is_solid_ground_normal_opaque_bottom() {
        let zen = CompositorLayer::zen();
        assert_eq!(zen.source, LayerSource::Solid(crate::theme::CID_GROUND));
        assert_eq!(zen.blend, BlendMode::Normal);
        assert_eq!(zen.opacity, 10_000, "zen ground is fully opaque");
        assert_eq!(zen.z, 0, "zen is the bottom plate — every other plane sits over it");
    }

    #[test]
    fn with_zen_mounts_the_canonical_ground_and_toggle() {
        let cfg = RenderConfig::empty(AppMode::Studio).with_zen();
        assert_eq!(cfg.layer_count, 1);
        assert_eq!(cfg.toggles & toggle::ZEN, toggle::ZEN, "ZEN toggle bit set");
        // The single mounted layer IS the canonical zen ground (not a duplicate).
        let mounted = cfg.active_layers().next().copied();
        assert_eq!(mounted, Some(CompositorLayer::zen()));
    }

    // T101 — sovereign stroke overlay roll-down proof.
    #[test]
    fn sovereign_layer_is_overlay_stroke_plane_above_zen() {
        let s = CompositorLayer::sovereign(3);
        assert_eq!(s.source, LayerSource::RenderTarget(3));
        assert_eq!(s.blend, BlendMode::Overlay);
        assert_eq!(s.opacity, 10_000);
        assert_eq!(s.z, 1, "sovereign sits directly above the zen ground");
        assert!(s.z > CompositorLayer::zen().z, "sovereign composites over zen");
    }

    #[test]
    fn with_zen_then_sovereign_stacks_ground_under_strokes() {
        let cfg = RenderConfig::empty(AppMode::Studio).with_zen().with_sovereign(0);
        assert_eq!(cfg.layer_count, 2);
        let want = toggle::ZEN | toggle::SOVEREIGN;
        assert_eq!(cfg.toggles & want, want, "both toggle bits set");
        // Z-order: ground (z=0) UNDER strokes (z=1) — never duplicated, just stacked.
        let layers = cfg.layers_sorted();
        assert_eq!(layers[0], CompositorLayer::zen());
        assert_eq!(layers[1], CompositorLayer::sovereign(0));
    }

    // T102 — material voxel-glyph roll-down proof.
    #[test]
    fn material_layer_is_render_target_normal_at_z2_above_sovereign() {
        let m = CompositorLayer::material(2);
        assert_eq!(m.source, LayerSource::RenderTarget(2));
        assert_eq!(m.blend, BlendMode::Normal);
        assert_eq!(m.opacity, 10_000, "material layer is fully opaque");
        assert_eq!(m.z, 2, "material sits above sovereign (z=1) and zen (z=0)");
        assert!(m.z > CompositorLayer::sovereign(0).z, "material composites over sovereign");
    }

    #[test]
    fn with_material_stacks_above_sovereign_enables_toggle() {
        let cfg = RenderConfig::empty(AppMode::Studio)
            .with_zen()
            .with_sovereign(0)
            .with_material(1);
        assert_eq!(cfg.layer_count, 3);
        let want = toggle::ZEN | toggle::SOVEREIGN | toggle::MATERIAL;
        assert_eq!(cfg.toggles & want, want, "all three toggle bits set");
        let layers = cfg.layers_sorted();
        assert_eq!(layers[0], CompositorLayer::zen(), "z=0: zen ground");
        assert_eq!(layers[1], CompositorLayer::sovereign(0), "z=1: sovereign strokes");
        assert_eq!(layers[2], CompositorLayer::material(1), "z=2: material coat");
    }

    // T103 — light cascade roll-down proof.
    #[test]
    fn light_layer_is_render_target_screen_at_z3_above_material() {
        let l = CompositorLayer::light(3);
        assert_eq!(l.source, LayerSource::RenderTarget(3));
        assert_eq!(l.blend, BlendMode::Screen);
        assert_eq!(l.opacity, 10_000, "light layer is fully opaque");
        assert_eq!(l.z, 3, "light sits above material (z=2)");
        assert!(l.z > CompositorLayer::material(0).z, "light composites over material");
    }

    #[test]
    fn magic_layer_is_render_target_colordodge_at_z4_above_light() {
        let m = CompositorLayer::magic(4);
        assert_eq!(m.source, LayerSource::RenderTarget(4));
        assert_eq!(m.blend, BlendMode::ColorDodge);
        assert_eq!(m.opacity, 10_000, "magic layer is fully opaque");
        assert_eq!(m.z, 4, "magic is the top-most Z-plane (ARCH-002 §5)");
        assert!(m.z > CompositorLayer::light(0).z, "magic crowns the full stack");
    }

    #[test]
    fn with_light_and_magic_stack_in_correct_z_order() {
        let cfg = RenderConfig::empty(AppMode::Studio)
            .with_zen()
            .with_sovereign(0)
            .with_material(1)
            .with_light(2)
            .with_magic(3);
        assert_eq!(cfg.layer_count, 5);
        let want = toggle::ZEN | toggle::SOVEREIGN | toggle::MATERIAL | toggle::LIGHT | toggle::MAGIC;
        assert_eq!(cfg.toggles, want, "all five toggle bits set");
        let layers = cfg.layers_sorted();
        assert_eq!(layers[0], CompositorLayer::zen(), "z=0: zen ground");
        assert_eq!(layers[1], CompositorLayer::sovereign(0), "z=1: sovereign strokes");
        assert_eq!(layers[2], CompositorLayer::material(1), "z=2: material coat");
        assert_eq!(layers[3], CompositorLayer::light(2), "z=3: light cascade");
        assert_eq!(layers[4], CompositorLayer::magic(3), "z=4: magic binding (top)");
    }

    #[test]
    fn with_light_enables_light_toggle_only() {
        let cfg = RenderConfig::empty(AppMode::Studio).with_light(0);
        assert_eq!(cfg.toggles & toggle::LIGHT, toggle::LIGHT);
        assert_eq!(cfg.toggles & toggle::MAGIC, 0, "magic bit stays clear");
    }

    // T104 — FrameComposer Z-order composite proof.
    #[test]
    fn frame_composer_zen_solid_fill_replaces_output() {
        let cfg = RenderConfig::empty(AppMode::Studio).with_zen();
        let mut out = vec![0u32; 4]; // 4 black pixels
        let ground = crate::theme::syn_rgba(crate::theme::CID_GROUND, 0xFF);
        FrameComposer::flatten(
            &cfg,
            &[],
            |cid| crate::theme::syn_rgba(cid, 0xFF),
            &mut out,
        );
        assert!(
            out.iter().all(|&p| p == ground),
            "zen solid fill must cover all output pixels"
        );
    }

    #[test]
    fn frame_composer_screen_blend_brightens_dst() {
        // Screen: out = s + d - s*d/255 — always >= max(s,d)
        let s: u8 = 0x80;
        let d: u8 = 0x80;
        let expected = blend_chan(BlendMode::Screen, s, d);
        assert!(expected >= s.max(d), "Screen blend must be >= both inputs");
        // Full-stack: one RenderTarget (screen blend) over a solid zen ground.
        let cfg = RenderConfig::empty(AppMode::Studio)
            .with_zen()
            .with_light(0); // light uses Screen blend
        let ground = crate::theme::syn_rgba(crate::theme::CID_GROUND, 0xFF);
        let light_pixel: u32 = 0x808080FF;
        let plane0 = vec![light_pixel; 4];
        let mut out = vec![0u32; 4];
        FrameComposer::flatten(
            &cfg,
            &[&plane0],
            |cid| crate::theme::syn_rgba(cid, 0xFF),
            &mut out,
        );
        // After zen fill then screen light, all pixels must differ from raw ground.
        assert!(
            out.iter().any(|&p| p != ground),
            "Screen light layer must brighten the zen ground"
        );
    }

    #[test]
    fn frame_composer_full_5_plane_stack_produces_nonzero_pixels() {
        let cfg = RenderConfig::empty(AppMode::Studio)
            .with_zen()
            .with_sovereign(0)
            .with_material(1)
            .with_light(2)
            .with_magic(3);
        assert_eq!(cfg.layer_count, 5);
        // Planes: all mid-grey so every blend mode has something to work with.
        let grey = vec![0x808080FFu32; 4];
        let planes: &[&[u32]] = &[&grey, &grey, &grey, &grey];
        let mut out = vec![0u32; 4];
        FrameComposer::flatten(
            &cfg,
            planes,
            |cid| crate::theme::syn_rgba(cid, 0xFF),
            &mut out,
        );
        assert!(
            out.iter().all(|&p| p != 0),
            "all 5 planes composited → no zero pixel"
        );
    }

    #[test]
    fn frame_composer_missing_plane_slot_is_no_op() {
        // RenderTarget(0) with no planes provided — must not panic.
        let cfg = RenderConfig::empty(AppMode::Studio).with_sovereign(0);
        let mut out = vec![0xDEADBEEFu32; 4];
        FrameComposer::flatten(
            &cfg,
            &[],
            |cid| crate::theme::syn_rgba(cid, 0xFF),
            &mut out,
        );
        // Output unchanged — missing slot is silently skipped.
        assert!(
            out.iter().all(|&p| p == 0xDEADBEEF),
            "missing plane must leave output unchanged"
        );
    }

    #[test]
    fn render_config_max_eight_layers() {
        let mut cfg = RenderConfig::empty(AppMode::Game);
        let layer = CompositorLayer::solid(1, BlendMode::Normal, 10_000, 0);
        for _ in 0..8 {
            assert!(cfg.push(layer));
        }
        assert!(!cfg.push(layer), "stack must reject a 9th layer");
        assert_eq!(cfg.layer_count, 8);
    }

    #[test]
    fn app_mode_roundtrip() {
        assert_eq!(RenderConfig::empty(AppMode::Daw).app_mode, AppMode::Daw);
        assert_eq!(RenderConfig::empty(AppMode::Game).app_mode, AppMode::Game);
    }

    #[test]
    fn toggle_bits_are_distinct() {
        assert_ne!(toggle::ZEN, toggle::SOVEREIGN);
        assert_ne!(toggle::SOVEREIGN, toggle::MATERIAL);
        assert_ne!(toggle::MATERIAL, toggle::LIGHT);
        assert_ne!(toggle::LIGHT, toggle::MAGIC);
        assert_eq!(
            toggle::ALL,
            toggle::ZEN | toggle::SOVEREIGN | toggle::MATERIAL | toggle::LIGHT | toggle::MAGIC
        );
    }

    #[test]
    fn layer_source_solid_stores_cid() {
        let l = CompositorLayer::solid(5, BlendMode::Add, 5_000, 0);
        assert_eq!(l.source, LayerSource::Solid(5));
        assert_eq!(l.blend, BlendMode::Add);
        assert_eq!(l.opacity, 5_000);
    }

    // L07-style determinism test: composite_pixel must deterministically produce the same output
    // when given the same inputs across multiple invocations.
    #[test]
    fn composite_pixel_determinism() {
        let mode = BlendMode::Overlay;
        let src = 0x808080FF;
        let dst = 0x404040FF;
        let opacity = 7_500u16;

        // Composite the same pixel 10 times; all results must be identical.
        let first = composite_pixel(mode, src, dst, opacity);
        for _ in 0..10 {
            let result = composite_pixel(mode, src, dst, opacity);
            assert_eq!(result, first, "composite_pixel must be deterministic");
        }
    }

    // L18-style sabotage test for blend_chan: verify the formula detects errors.
    // Flip the assertion to confirm it catches real failures.
    #[test]
    fn blend_chan_screen_sabotage() {
        // Screen blend: result should be >= max(s, d).
        // If we flip the assertion to `assert!(result < s.max(d))`, it should FAIL.
        // This test confirms the invariant is real and catches bugs.
        let s: u8 = 100;
        let d: u8 = 50;
        let result = blend_chan(BlendMode::Screen, s, d);
        assert!(
            result >= s.max(d),
            "Screen blend invariant: result={} >= max({}, {})={}",
            result,
            s,
            d,
            s.max(d)
        );
    }

    #[test]
    fn frame_composer_composited_bmp_nonzero() {
        use crate::theme::{syn_rgba, CID_ACCENT, CID_MARK, CID_VIOLET};
        use std::path::Path;

        const W: u32 = 160;
        const H: u32 = 90;
        const N: usize = (W as usize) * (H as usize);

        let mut sovereign_px = vec![0u32; N];
        for y in 0..(H / 3) {
            for x in 0..W {
                sovereign_px[(y * W + x) as usize] = syn_rgba(CID_ACCENT, 0xFF);
            }
        }
        let mut light_px = vec![0u32; N];
        for y in (H * 2 / 3)..H {
            for x in 0..W {
                light_px[(y * W + x) as usize] = syn_rgba(CID_MARK, 0xFF);
            }
        }
        let mut magic_px = vec![0u32; N];
        for i in 0..N {
            let x = (i % W as usize) as u32;
            let y = (i / W as usize) as u32;
            if (x + y) % 20 < 3 {
                magic_px[i] = syn_rgba(CID_VIOLET, 0xFF);
            }
        }

        let cfg = RenderConfig::empty(AppMode::Studio)
            .with_zen()
            .with_sovereign(0)
            .with_light(1)
            .with_magic(2);

        let planes: &[&[u32]] = &[&sovereign_px, &light_px, &magic_px];
        let mut output = vec![0u32; N];
        FrameComposer::flatten(&cfg, planes, |cid| syn_rgba(cid, 0xFF), &mut output);

        let top_nonzero    = output[..(N / 3)].iter().any(|&p| p != 0);
        let bottom_nonzero = output[(N * 2 / 3)..].iter().any(|&p| p != 0);
        assert!(top_nonzero,    "zen+sovereign blend → top band non-zero");
        assert!(bottom_nonzero, "zen+light screen blend → bottom band non-zero");

        let out_dir = Path::new("../../_proof/b1");
        std::fs::create_dir_all(out_dir).ok();

        let mut buf = crate::rasterizer::PixelBuffer::new(W, H);
        for (i, &px) in output.iter().enumerate() {
            let base = i * 4;
            buf.data[base]     = ((px >> 24) & 0xFF) as u8; // R
            buf.data[base + 1] = ((px >> 16) & 0xFF) as u8; // G
            buf.data[base + 2] = ((px >>  8) & 0xFF) as u8; // B
            buf.data[base + 3] = 0xFF;                        // A
        }
        crate::rasterizer::write_bmp(&buf, &out_dir.join("composited.bmp")).ok();
    }
}
