//! Smithy material parameters - tiered amplitude per SurfaceContext.
//!
//! Each PanelMaterial discriminant has a MaterialParams entry indexed by
//! `material as usize`. amplitude_for(SurfaceContext) dispatches the
//! correct Permyriad cap per context (chrome_over_canvas 2%, chrome_floating
//! 5%, audio_reactive_peak 12%, event_burst 8%, showcase 80%).
//!
//! Load-bearing invariant: `canvas_atmosphere_bleed = forbidden` - material
//! texture sampling MUST short-circuit when the pixel covers a canvas region.

use crate::draw::{DrawCmd, DrawList};

/// Material palette indices matching the WGSL material_palette array.
/// 0 = default (no material processing), 1+ = named materials.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelMaterial {
    /// No material processing.
    None = 0,
    /// Brushed dark metal.
    Gunmetal = 1,
    /// Translucent glass.
    Glass = 2,
    /// Scanline hologram sheen.
    Hologram = 3,
    /// Aged paper.
    Parchment = 4,
    /// Warm metal.
    Bronze = 5,
    /// Grained wood.
    Wood = 6,
    /// Fine aged paper.
    Vellum = 7,
    /// Rough stone.
    Cobblestone = 8,
}

/// Per-context texture amplitude caps + per-material visual params.
/// All amplitudes are Permyriad (0..10_000); 10_000 = 100%.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialParams {
    /// Amplitude cap when material sits over live canvas content (<= 200 = 2%).
    pub amplitude_over_canvas: u16,
    /// Amplitude cap for a floating (non-canvas) panel (<= 500 = 5%).
    pub amplitude_floating: u16,
    /// Amplitude cap during an audio-reactive peak (<= 1200 = 12%).
    pub amplitude_audio_peak: u16,
    /// Amplitude cap during an event burst (<= 800 = 8%).
    pub amplitude_event_burst: u16,
    /// Amplitude cap in showcase mode (<= 8000 = 80%).
    pub amplitude_showcase: u16,
    /// Surface roughness, Permyriad.
    pub roughness: u16,
    /// Packed RGBA edge-highlight colour.
    pub edge_highlight: u32,
    /// Grain frequency in cells per unit.
    pub grain_frequency: u16,
    /// Opacity, Permyriad.
    pub opacity: u16,
}

/// Rendering context a material's amplitude is being queried for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceContext {
    /// Chrome directly over live canvas content — tightest cap.
    ChromeOverCanvas,
    /// Chrome floating free of canvas content.
    ChromeFloating,
    /// Audio-reactive peak moment.
    AudioReactivePeak,
    /// Event burst (e.g. a hit or achievement).
    EventBurst,
    /// Showcase/demo mode — loosest cap.
    Showcase,
}

impl MaterialParams {
    /// Amplitude cap for the given surface context, Permyriad.
    pub const fn amplitude_for(&self, ctx: SurfaceContext) -> u16 {
        match ctx {
            SurfaceContext::ChromeOverCanvas => self.amplitude_over_canvas,
            SurfaceContext::ChromeFloating => self.amplitude_floating,
            SurfaceContext::AudioReactivePeak => self.amplitude_audio_peak,
            SurfaceContext::EventBurst => self.amplitude_event_burst,
            SurfaceContext::Showcase => self.amplitude_showcase,
        }
    }
}

impl PanelMaterial {
    /// All variants in palette order (same order as `MATERIAL_PARAMS`).
    pub const ALL: &'static [PanelMaterial] = &[
        Self::None, Self::Gunmetal, Self::Glass, Self::Hologram,
        Self::Parchment, Self::Bronze, Self::Wood, Self::Vellum, Self::Cobblestone,
    ];

    /// Human-readable display label for the picker swatch.
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Gunmetal => "Gunmetal",
            Self::Glass => "Glass",
            Self::Hologram => "Hologram",
            Self::Parchment => "Parchment",
            Self::Bronze => "Bronze",
            Self::Wood => "Wood",
            Self::Vellum => "Vellum",
            Self::Cobblestone => "Cobblestone",
        }
    }

    /// Raw palette index — same as the `u8` discriminant. Passed as `material_idx`
    /// in `DrawCmd::SetMaterial`.
    #[inline(always)]
    pub fn material_idx(self) -> u8 {
        self as u8
    }

    /// Push a `DrawCmd::SetMaterial` for this material into `draw`.
    /// `vibe_mask` uses the canvas renderer's vibe-bit constants.
    /// Call with `vibe_mask = 0` for a plain material switch with no reactive fx.
    pub fn emit_set_material(self, draw: &mut DrawList, vibe_mask: u8) {
        draw.push(DrawCmd::SetMaterial {
            material_idx: self as u8,
            vibe_mask,
            essence_id: 0,
        });
    }
}

/// Indexed by PanelMaterial discriminant. Entry 0 = None (neutral),
/// 1..3 = legacy sci-fi (preserved baseline), 4..7 = Smithy register, 8 = Cobblestone.
pub const MATERIAL_PARAMS: [MaterialParams; 9] = [
    /* 0 None */        MaterialParams { amplitude_over_canvas: 0,   amplitude_floating: 0,   amplitude_audio_peak: 0,    amplitude_event_burst: 0,   amplitude_showcase: 0,    roughness: 0,    edge_highlight: 0x00000000, grain_frequency: 0,  opacity: 10_000 },
    /* 1 Gunmetal */    MaterialParams { amplitude_over_canvas: 100, amplitude_floating: 300, amplitude_audio_peak: 800,  amplitude_event_burst: 400, amplitude_showcase: 4000, roughness: 6000, edge_highlight: 0x88AABBFFu32, grain_frequency: 8,  opacity: 10_000 },
    /* 2 Glass */       MaterialParams { amplitude_over_canvas: 0,   amplitude_floating: 200, amplitude_audio_peak: 600,  amplitude_event_burst: 300, amplitude_showcase: 3000, roughness: 1500, edge_highlight: 0xCCDDEEFFu32, grain_frequency: 0,  opacity: 7000 },
    /* 3 Hologram */    MaterialParams { amplitude_over_canvas: 0,   amplitude_floating: 400, amplitude_audio_peak: 1200, amplitude_event_burst: 800, amplitude_showcase: 6000, roughness: 0,    edge_highlight: 0x44FFAAFFu32, grain_frequency: 16, opacity: 6000 },
    /* 4 Parchment */   MaterialParams { amplitude_over_canvas: 150, amplitude_floating: 400, amplitude_audio_peak: 800,  amplitude_event_burst: 500, amplitude_showcase: 6000, roughness: 8500, edge_highlight: 0x8C7662FFu32, grain_frequency: 24, opacity: 10_000 },
    /* 5 Bronze */      MaterialParams { amplitude_over_canvas: 200, amplitude_floating: 500, amplitude_audio_peak: 1200, amplitude_event_burst: 800, amplitude_showcase: 7000, roughness: 4500, edge_highlight: 0xE46C34FFu32, grain_frequency: 12, opacity: 10_000 },
    /* 6 Wood */        MaterialParams { amplitude_over_canvas: 180, amplitude_floating: 480, amplitude_audio_peak: 900,  amplitude_event_burst: 600, amplitude_showcase: 6500, roughness: 7000, edge_highlight: 0x5C3A22FFu32, grain_frequency: 6,  opacity: 10_000 },
    /* 7 Vellum */      MaterialParams { amplitude_over_canvas: 100, amplitude_floating: 350, amplitude_audio_peak: 700,  amplitude_event_burst: 400, amplitude_showcase: 5000, roughness: 9000, edge_highlight: 0xF0E8D8FFu32, grain_frequency: 18, opacity: 7000 },
    /* 8 Cobblestone */ MaterialParams { amplitude_over_canvas: 200, amplitude_floating: 500, amplitude_audio_peak: 800,  amplitude_event_burst: 600, amplitude_showcase: 6500, roughness: 9500, edge_highlight: 0x4A3D33FFu32, grain_frequency: 16, opacity: 10_000 },
];

/// Look up the params row for a material.
pub const fn params_for(material: PanelMaterial) -> MaterialParams {
    MATERIAL_PARAMS[material as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smithy_texture_amplitude_respects_surface_context() {
        use SurfaceContext::*;
        for material in [PanelMaterial::Parchment, PanelMaterial::Bronze, PanelMaterial::Wood, PanelMaterial::Vellum] {
            let p = params_for(material);
            assert!(p.amplitude_for(ChromeOverCanvas) <= 200, "{material:?} over-canvas > 2%");
            assert!(p.amplitude_for(ChromeFloating) <= 500, "{material:?} floating > 5%");
            assert!(p.amplitude_for(AudioReactivePeak) <= 1200, "{material:?} audio-peak > 12%");
            assert!(p.amplitude_for(EventBurst) <= 800, "{material:?} event-burst > 8%");
            assert!(p.amplitude_for(Showcase) <= 8000, "{material:?} showcase > 80%");
        }
    }

    #[test]
    fn panel_material_count_is_9() {
        assert_eq!(MATERIAL_PARAMS.len(), 9);
        assert_eq!(PanelMaterial::Cobblestone as u8, 8);
    }

    /// L18 sabotage: the 2% over-canvas rail is load-bearing (canvas_atmosphere_bleed
    /// = forbidden). Confirm every textured material respects it; a flipped cap
    /// (e.g. amplitude_over_canvas: 2000) would fail this assert loudly.
    #[test]
    fn canvas_atmosphere_bleed_is_forbidden_regardless_of_tier() {
        for material in [PanelMaterial::Parchment, PanelMaterial::Bronze, PanelMaterial::Wood, PanelMaterial::Vellum] {
            let p = params_for(material);
            assert!(
                p.amplitude_for(SurfaceContext::ChromeOverCanvas) <= 200,
                "{material:?} over_canvas amp > 200 (2% load-bearing rail)"
            );
        }
    }
}
