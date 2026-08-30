//! look_composite — buffered look overlay with permyriad opacity + VibeMatrix.
//!
//! THE seam behind "overlay the layer with opacity over a buffer" (the dream-test
//! Magic Canvas: a translucent studio/look layer floating over the live render).
//! Integer permyriad in; float math is GPU-side ONLY — this fn is the CPU-parity
//! twin of the fragment, so `cargo test -p forge-look-v3` proves it WITHOUT a
//! screen, then the same code compiles to SPIR-V via rust-gpu.
//!
//! Pipeline (one fragment):
//!   foreground layer colour
//!     --[VibeMatrix modulate]-->        (reuse `vibe_post::vibe_post_process`)
//!     --[reactive opacity, bounded]-->  (authored `.vixi` edges, integer-clamped)
//!     --[alpha-over the background]-->   composited output.
//!
//! Reactive edges mirror the `.vixi` authoring grammar
//! (`vibematrix.src -> visual.tgt bounded=Z`): each edge maps ONE integer
//! VibeMatrix channel onto ONE visual target, clamped to its `bounded_q` ceiling.
//! The clamp is an integer `min` taken BEFORE the float boundary, so the authored
//! ceiling is exact — never a float-drifted approximation.

#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;

use crate::gpu_types::VibeUniforms;
use crate::vibe_post::vibe_post_process;

/// Maximum reactive edges per look_composite call. This is the ceiling that
/// forge-shaderbind expects for storage-buffer binding (DRIFT note: if this ever
/// changes, keep it in sync with forge-shaderbind::reactive::MAX_BINDS, or tests
/// will fail — see test_max_binds_matches_shaderbind_ceiling).
pub use forge_shaderbind::reactive::MAX_BINDS;

/// Permyriad (0..=10000) -> unit float at the single q_to_float boundary. The
/// clamp is integer (`u32::min`), so the value is exact up to the GPU.
#[inline]
pub fn permyriad_to_unit(q: u32) -> f32 {
    (q.min(10000) as f32) / 10000.0
}

/// Clamp a float to `[0, 1]` with branches — no `min`/`max` intrinsic, keeping the
/// GPU path `no_std`/SPIR-V clean (same discipline as the inline alpha clamp).
#[inline]
fn clamp_unit(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}

// ── VibeMatrix source channels (index a `VibeUniforms` field) ─────────────────
/// Combo heat source channel.
pub const SRC_COMBO_HEAT: u32 = 0;
/// Artifact glow source channel.
pub const SRC_ARTIFACT_GLOW: u32 = 1;
/// Chromatic aberration source channel.
pub const SRC_CHROMATIC: u32 = 2;
/// Distortion source channel.
pub const SRC_DISTORTION: u32 = 3;

// ── Visual targets a reactive edge can drive ──────────────────────────────────
/// Emissive visual target.
pub const TGT_EMISSIVE: u32 = 0;
/// Opacity visual target.
pub const TGT_OPACITY: u32 = 1;
/// Bloom visual target.
pub const TGT_BLOOM: u32 = 2;
/// Screen-space UV pinch toward center — the "void compression" black-hole warp
/// (Sean 2026-06-16: drive bass + bloom through a *geometric* warp, NOT an opacity
/// fade). Applied PRE-sample by [`warp_uv`]; the bound permyriad is the pinch ceiling.
pub const TGT_WARP: u32 = 3;

/// A bounded reactive edge — the GPU storage-buffer form of a `.vixi`
/// `vibematrix.src -> visual.tgt bounded=Z` line. This is the low-level
/// repr(C) layout (12 bytes, all scalar `u32`) used for GPU storage buffers.
/// The high-level parsed form (with enum sources/targets) lives in
/// forge-shaderbind::reactive::ReactiveBind.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GpuReactiveBind {
    /// VibeMatrix source channel (`SRC_*`).
    pub src: u32,
    /// Visual target channel (`TGT_*`).
    pub tgt: u32,
    /// Authored `bounded=` ceiling in permyriad — the max influence of this edge.
    pub bounded_q: u32,
}

const _: () = assert!(core::mem::size_of::<GpuReactiveBind>() == 12);

/// Read a VibeMatrix source channel as permyriad. Unknown channel -> 0 (silent
/// edge), never a panic on the GPU path.
#[inline]
fn vibe_src_q(vibe: &VibeUniforms, src: u32) -> u32 {
    match src {
        SRC_COMBO_HEAT => vibe.combo_heat,
        SRC_ARTIFACT_GLOW => vibe.artifact_glow,
        SRC_CHROMATIC => vibe.chromatic_aberration,
        SRC_DISTORTION => vibe.distortion_level,
        _ => 0,
    }
}

/// One edge's contribution as a unit float: `min(src, bounded_q)` clamped in
/// INTEGER permyriad (exact ceiling), then crossed to float once.
#[inline]
pub fn apply_bind(vibe: &VibeUniforms, bind: &GpuReactiveBind) -> f32 {
    permyriad_to_unit(vibe_src_q(vibe, bind.src).min(bind.bounded_q))
}

/// Sum every edge driving `tgt`, clamped to 1.0. Index loop (no iterator) keeps
/// the GPU path `no_std`/SPIR-V clean.
#[inline]
pub fn target_drive(vibe: &VibeUniforms, binds: &[GpuReactiveBind], tgt: u32) -> f32 {
    let mut acc = 0.0f32;
    let mut i = 0usize;
    while i < binds.len() {
        if binds[i].tgt == tgt {
            acc += apply_bind(vibe, &binds[i]);
        }
        i += 1;
    }
    if acc > 1.0 {
        1.0
    } else {
        acc
    }
}

/// Void-compression UV pinch — warp a fragment's sample coordinate TOWARD screen
/// center proportional to `warp` (the bounded `TGT_WARP` drive, 0..=1). This is the
/// "black hole" kick-drum signature ported from the mirror's `void_compression`,
/// re-homed onto the integer reactive bus: the author sets the pinch ceiling via the
/// edge's `bounded=` permyriad, so the strength lives in `.vixi`, never a Rust const.
///
/// `factor = warp * (1 - dist_from_center)` (strongest at center, zero at the corner),
/// clamped to `[0,1]` so the sample never crosses past center. Applied PRE-sample by
/// the fragment (`look_composite_fs`); parity-tested here on the CPU twin. Returns the
/// warped `(uv_x, uv_y)`. `warp == 0` is the identity — a quiet passage is undistorted.
#[inline]
pub fn warp_uv(uv_x: f32, uv_y: f32, warp: f32) -> (f32, f32) {
    let dx = uv_x - 0.5;
    let dy = uv_y - 0.5;
    let dist = (dx * dx + dy * dy).sqrt();
    // Pull toward center; clamp the pull so it never overshoots past (0.5, 0.5).
    let mut factor = warp * (1.0 - dist);
    if factor < 0.0 {
        factor = 0.0;
    }
    if factor > 1.0 {
        factor = 1.0;
    }
    (uv_x - dx * factor, uv_y - dy * factor)
}

/// Composite a foreground look layer OVER a background with a global permyriad
/// opacity and live VibeMatrix modulation — the buffered Magic-Canvas overlay.
///
/// * `fg_rgb` — the layer's colour (a sample from the offscreen buffer).
/// * `bg_rgb` — the colour already on the surface (e.g. the prairie sky).
/// * `uv_*`   — fragment UV (0,0)->(1,1) for the chromatic/glow field.
/// * `layer_opacity_q` — global layer opacity in permyriad (0 = layer invisible,
///   10000 = fully opaque). The "with opacity" half of the original ask.
/// * `vibe`  — live integer VibeMatrix signals (audio-reactive).
/// * `binds` — authored reactive edges (`.vixi`-sourced).
///
/// Returns the composited `(r, g, b)`. Every input crossed the CPU/GPU boundary
/// as an integer (permyriad / `VibeUniforms`); float lives only inside here.
#[inline]
pub fn look_composite(
    fg_rgb: [f32; 3],
    bg_rgb: [f32; 3],
    uv_x: f32,
    uv_y: f32,
    layer_opacity_q: u32,
    // sand→glass phase in permyriad (0 = sand/opaque, 10_000 = glass/fully transparent);
    // attenuates composite alpha after the reactive-edge sum.
    material_phase_q: u32,
    vibe: &VibeUniforms,
    binds: &[GpuReactiveBind],
) -> [f32; 3] {
    // 1 ─ VibeMatrix-modulate the foreground (the canonical post fn).
    let (mr, mg, mb, _glow) =
        vibe_post_process(fg_rgb[0], fg_rgb[1], fg_rgb[2], uv_x, uv_y, vibe);

    // 2 ─ Reactive opacity: authored base + any bounded edges driving TGT_OPACITY,
    //     clamped to [0, 1]. Base is integer permyriad; float only at the seam.
    let mut alpha = permyriad_to_unit(layer_opacity_q) + target_drive(vibe, binds, TGT_OPACITY);
    if alpha > 1.0 {
        alpha = 1.0;
    }
    if alpha < 0.0 {
        alpha = 0.0;
    }

    // 3 ─ Sand→glass phase interpolation (glass-shader-recipe.md).
    //     phase=0 (sand): reactive alpha holds.
    //     phase=1 (glass): alpha = 0.12 + fres*0.55 + lum*0.97 — Fresnel rim + the
    //     lum-keeps-text-solid trick (bright fg pixels push glass alpha to ~1.0, dark
    //     stays translucent). fres ≈ UV dist-to-centre normalised to [0,1].
    let phase = permyriad_to_unit(material_phase_q);
    if phase > 0.0 {
        let lum = clamp_unit(0.2126 * mr + 0.7152 * mg + 0.0722 * mb);
        let dx = uv_x - 0.5;
        let dy = uv_y - 0.5;
        let dist = (dx * dx + dy * dy).sqrt();
        let fres = clamp_unit(dist * 1.414); // max diagonal ≈ 0.707 → × 1.414 → [0,1]
        let glass_alpha = clamp_unit(0.12 + fres * 0.55 + lum * 0.97);
        alpha = clamp_unit(alpha + (glass_alpha - alpha) * phase);
    }

    // 4 ─ Alpha-over composite: fg*a + bg*(1-a). The buffered overlay.
    let inv = 1.0 - alpha;
    let r = mr * alpha + bg_rgb[0] * inv;
    let g = mg * alpha + bg_rgb[1] * inv;
    let b = mb * alpha + bg_rgb[2] * inv;

    // 5 ─ Reactive bloom: edges driving TGT_BLOOM lift output brightness (a glow),
    //     clamped to 1.0. Zero drive is the identity. Bass + bloom ride this + the
    //     PRE-sample TGT_WARP pinch (`warp_uv`, applied in the fragment) — NOT opacity
    //     (Sean 2026-06-16: geometric warp + glow, not an alpha fade).
    let lift = 1.0 + target_drive(vibe, binds, TGT_BLOOM);
    [clamp_unit(r * lift), clamp_unit(g * lift), clamp_unit(b * lift)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_vibe() -> VibeUniforms {
        VibeUniforms {
            combo_heat: 0,
            resonance_hz: 0,
            rain_intensity: 0,
            chromatic_aberration: 0,
            artifact_glow: 0,
            particle_density: 0,
            distortion_level: 0,
            _pad: 0,
        }
    }

    #[test]
    fn max_binds_matches_shaderbind_ceiling() {
        // DRIFT: forge-shaderbind::MAX_CHANNELS = 64 (line ~574 in lib.rs).
        // If this ever changes, both sides must update in lockstep or we get
        // silent buffer overruns. This test fails if they diverge.
        assert_eq!(MAX_BINDS, 64, "MAX_BINDS must match forge-shaderbind::MAX_CHANNELS");
    }

    #[test]
    fn permyriad_boundary_is_exact() {
        assert!((permyriad_to_unit(0) - 0.0).abs() < 1e-6);
        assert!((permyriad_to_unit(10000) - 1.0).abs() < 1e-6);
        assert!((permyriad_to_unit(5000) - 0.5).abs() < 1e-6);
        // Over-range clamps (never exceeds 1.0).
        assert!((permyriad_to_unit(15000) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn zero_opacity_shows_only_background() {
        let bg = [0.2, 0.3, 0.4];
        let out = look_composite([1.0, 1.0, 1.0], bg, 0.5, 0.5, 0, 0, &zero_vibe(), &[]);
        for c in 0..3 {
            assert!((out[c] - bg[c]).abs() < 1e-4, "channel {c}: {} != {}", out[c], bg[c]);
        }
    }

    #[test]
    fn full_opacity_zero_vibe_shows_only_foreground() {
        let fg = [0.7, 0.5, 0.2];
        let out = look_composite(fg, [0.0, 0.0, 0.0], 0.5, 0.5, 10000, 0, &zero_vibe(), &[]);
        for c in 0..3 {
            assert!((out[c] - fg[c]).abs() < 1e-4, "channel {c}: {} != {}", out[c], fg[c]);
        }
    }

    #[test]
    fn half_opacity_lerps_fg_and_bg() {
        // fg=white, bg=black, opacity 0.5, no vibe -> 0.5 grey.
        let out = look_composite([1.0, 1.0, 1.0], [0.0, 0.0, 0.0], 0.5, 0.5, 5000, 0, &zero_vibe(), &[]);
        for c in 0..3 {
            assert!((out[c] - 0.5).abs() < 1e-4, "channel {c} = {}", out[c]);
        }
    }

    #[test]
    fn bounded_edge_caps_at_authored_ceiling() {
        let mut v = zero_vibe();
        v.artifact_glow = 10000; // source pinned high
        // Ceiling 2000 permyriad -> exactly 0.20, never more.
        let capped = GpuReactiveBind { src: SRC_ARTIFACT_GLOW, tgt: TGT_OPACITY, bounded_q: 2000 };
        assert!((apply_bind(&v, &capped) - 0.20).abs() < 1e-6);
        // Ceiling at full range -> the source passes through (1.0).
        let open = GpuReactiveBind { src: SRC_ARTIFACT_GLOW, tgt: TGT_OPACITY, bounded_q: 10000 };
        assert!((apply_bind(&v, &open) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn target_drive_sums_then_clamps() {
        let mut v = zero_vibe();
        v.artifact_glow = 10000;
        v.combo_heat = 10000;
        // Two edges to the same target, each 0.7 -> sum 1.4 -> clamped to 1.0.
        let binds = [
            GpuReactiveBind { src: SRC_ARTIFACT_GLOW, tgt: TGT_OPACITY, bounded_q: 7000 },
            GpuReactiveBind { src: SRC_COMBO_HEAT, tgt: TGT_OPACITY, bounded_q: 7000 },
        ];
        assert!((target_drive(&v, &binds, TGT_OPACITY) - 1.0).abs() < 1e-6);
        // An unrelated target sees nothing.
        assert!(target_drive(&v, &binds, TGT_BLOOM).abs() < 1e-6);
    }

    #[test]
    fn warp_zero_is_identity() {
        // A quiet passage (warp drive 0) must leave every UV untouched.
        for &(x, y) in &[(0.0, 0.0), (0.25, 0.75), (0.5, 0.5), (1.0, 1.0)] {
            let (wx, wy) = warp_uv(x, y, 0.0);
            assert!((wx - x).abs() < 1e-6 && (wy - y).abs() < 1e-6, "({x},{y}) moved at warp=0");
        }
    }

    #[test]
    fn warp_center_is_fixed_point() {
        // The pinch is toward (0.5,0.5); the center can never move, any warp.
        let (wx, wy) = warp_uv(0.5, 0.5, 1.0);
        assert!((wx - 0.5).abs() < 1e-6 && (wy - 0.5).abs() < 1e-6);
    }

    #[test]
    fn warp_pulls_toward_center() {
        // An off-center fragment moves CLOSER to center under positive warp.
        let (x, y) = (0.9_f32, 0.1_f32);
        let d0 = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
        let (wx, wy) = warp_uv(x, y, 0.5);
        let d1 = ((wx - 0.5).powi(2) + (wy - 0.5).powi(2)).sqrt();
        assert!(d1 < d0, "warp must reduce distance to center: {d1} !< {d0}");
    }

    #[test]
    fn warp_never_overshoots_past_center() {
        // Even an over-range warp clamps the pull factor to 1.0 → lands AT center,
        // never on the far side (the same sign on each axis as the center offset).
        let (x, y) = (0.8_f32, 0.2_f32);
        let (wx, wy) = warp_uv(x, y, 10.0);
        // dx>0 so wx stays >= 0.5; dy<0 so wy stays <= 0.5 (no flip past center).
        assert!(wx >= 0.5 - 1e-6 && wx <= x + 1e-6, "wx {wx} flipped past center");
        assert!(wy <= 0.5 + 1e-6 && wy >= y - 1e-6, "wy {wy} flipped past center");
    }

    #[test]
    fn reactive_bloom_lifts_output_brightness() {
        // A bounded edge driving TGT_BLOOM brightens the composited output (glow),
        // and an edge to bloom does NOT touch opacity. fg=mid-grey, full opacity.
        let mut v = zero_vibe();
        v.combo_heat = 10000;
        let dark = look_composite([0.4, 0.4, 0.4], [0.0, 0.0, 0.0], 0.5, 0.5, 10000, 0, &v, &[]);
        let binds = [GpuReactiveBind { src: SRC_COMBO_HEAT, tgt: TGT_BLOOM, bounded_q: 5000 }];
        let bright = look_composite([0.4, 0.4, 0.4], [0.0, 0.0, 0.0], 0.5, 0.5, 10000, 0, &v, &binds);
        for c in 0..3 {
            assert!(bright[c] > dark[c], "bloom must brighten channel {c}: {} !> {}", bright[c], dark[c]);
            assert!(bright[c] <= 1.0 + 1e-6, "bloom output must clamp to 1.0, got {}", bright[c]);
        }
    }

    #[test]
    fn reactive_opacity_lifts_an_invisible_layer() {
        // opacity_q = 0, but a bounded edge drives TGT_OPACITY to 0.5 -> the
        // music makes a hidden layer fade IN. fg=white over bg=black -> ~0.5.
        let mut v = zero_vibe();
        v.combo_heat = 10000;
        let binds = [GpuReactiveBind { src: SRC_COMBO_HEAT, tgt: TGT_OPACITY, bounded_q: 5000 }];
        let out = look_composite([1.0, 1.0, 1.0], [0.0, 0.0, 0.0], 0.5, 0.5, 0, 0, &v, &binds);
        // Some chromatic/glow modulation is possible, but alpha is ~0.5 and bg is
        // black, so each channel sits near 0.5 (>0 proves the edge lifted it).
        for c in 0..3 {
            assert!(out[c] > 0.4 && out[c] < 0.8, "channel {c} = {}", out[c]);
        }
    }
}
