//! Render resolution configuration.
//!
//! UI always renders at native window resolution (crisp text).
//! Game viewports can render at a lower internal resolution and upscale.

use forge_core_v3::fixed_point::Permyriad;

/// Texture atlas size for font/glyph rendering — auto-detected or user-configured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasSize {
    /// 1024×1024 (1 MB) — integrated GPU, 1080p.
    Standard,
    /// 2048×2048 (4 MB) — 1440p desktops.
    High,
    /// 4096×4096 (16 MB) — 4K, high-DPI, multi-font caching.
    Ultra,
}

impl AtlasSize {
    /// Pixel dimension (always square).
    pub fn pixels(&self) -> u32 {
        match self {
            Self::Standard => 1024,
            Self::High => 2048,
            Self::Ultra => 4096,
        }
    }

    /// Auto-detect from viewport resolution.
    /// Returns `Ultra` for 4K+, `High` for 1440p+, otherwise `Standard`.
    pub fn from_viewport(width: u32, height: u32) -> Self {
        let max_dim = width.max(height);
        if max_dim >= 3840 {
            Self::Ultra
        } else if max_dim >= 2560 {
            Self::High
        } else {
            Self::Standard
        }
    }
}

/// Game/viewport render resolution — decoupled from window size.
/// Allows internal rendering at a lower resolution than the display window,
/// then upscaling for performance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum RenderResolution {
    /// Render at window size (best quality).
    #[default]
    Native,
    /// Fixed internal resolution, upscaled to window.
    Fixed(u32, u32),
    /// Percentage of window size via permyriad (e.g., 7500 = 75%).
    Scaled(Permyriad),
}

impl RenderResolution {
    /// Resolve to actual pixel dimensions given window size.
    /// All arithmetic uses u64 intermediate to prevent u32 overflow during multiplication.
    pub fn resolve(&self, window_w: u32, window_h: u32) -> (u32, u32) {
        match self {
            Self::Native => (window_w, window_h),
            Self::Fixed(w, h) => (*w, *h),
            Self::Scaled(pct) => (
                ((window_w as u64) * (pct.0 as u64) / 10000) as u32,
                ((window_h as u64) * (pct.0 as u64) / 10000) as u32,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── L07-style determinism: idempotent resolution ─────────────────────────
    // Calling resolve() twice with the same window size must yield identical results.

    #[test]
    fn resolve_is_deterministic() {
        let r = RenderResolution::Scaled(Permyriad(7500));
        let (w1, h1) = r.resolve(1920, 1080);
        let (w2, h2) = r.resolve(1920, 1080);
        assert_eq!((w1, h1), (w2, h2), "resolve() must be deterministic");
    }

    #[test]
    fn atlas_auto_detect() {
        assert_eq!(AtlasSize::from_viewport(1920, 1080), AtlasSize::Standard);
        assert_eq!(AtlasSize::from_viewport(2560, 1440), AtlasSize::High);
        assert_eq!(AtlasSize::from_viewport(3840, 2160), AtlasSize::Ultra);
    }

    #[test]
    fn atlas_auto_detect_uses_max_dimension() {
        // Tall narrow display
        assert_eq!(AtlasSize::from_viewport(1080, 2560), AtlasSize::High);
        // Wide short display
        assert_eq!(AtlasSize::from_viewport(3840, 1080), AtlasSize::Ultra);
    }

    #[test]
    fn resolution_native() {
        let r = RenderResolution::Native;
        assert_eq!(r.resolve(1920, 1080), (1920, 1080));
    }

    #[test]
    fn resolution_fixed() {
        let r = RenderResolution::Fixed(1280, 720);
        assert_eq!(r.resolve(3840, 2160), (1280, 720));
        assert_eq!(r.resolve(1920, 1080), (1280, 720)); // ignores window size
    }

    #[test]
    fn resolution_scaled() {
        let r = RenderResolution::Scaled(Permyriad(7500)); // 75%
        assert_eq!(r.resolve(1920, 1080), (1440, 810));
    }

    #[test]
    fn resolution_scaled_at_unity() {
        let r = RenderResolution::Scaled(Permyriad(10000)); // 100%
        assert_eq!(r.resolve(1920, 1080), (1920, 1080));
    }

    #[test]
    fn resolution_scaled_at_zero() {
        let r = RenderResolution::Scaled(Permyriad(0));
        assert_eq!(r.resolve(1920, 1080), (0, 0));
    }

    // ── L18-style sabotage: flip the scale direction ─────────────────────────
    // If we flip the numerator/denominator in the Scaled case (multiply by 1/pct
    // instead of pct), scaled-up resolution would shrink instead. Verify that
    // Scaled(5000) at 1920x1080 gives 960x540 (half), not 3840x2160 (double).

    #[test]
    fn resolution_scaled_shrinks_below_unity() {
        let r = RenderResolution::Scaled(Permyriad(5000)); // 50%
        let (w, h) = r.resolve(1920, 1080);
        assert_eq!((w, h), (960, 540), "50% scaling must shrink, not enlarge");
        assert!(w < 1920 && h < 1080);
    }

    #[test]
    fn atlas_size_monotonic_with_viewport() {
        // Larger viewport → larger atlas.
        let sz_small = AtlasSize::from_viewport(1920, 1080);
        let sz_large = AtlasSize::from_viewport(4000, 2160);
        assert!(sz_large as u8 >= sz_small as u8, "atlas sizing must be monotonic");
    }
}
