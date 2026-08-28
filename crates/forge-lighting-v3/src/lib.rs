//! forge-lighting-v3 — the f32 light lane: rigs, Lambert diffuse, SH9 ambient,
//! and the trit-quantized direction lattice that indexes it.
//!
//! Drained from v2 `forge-lighting` and `public-tools/forge-ibl-bake`. The
//! integer kernel never sees these floats; that separation is the crate's
//! reason to exist (v2 `forge-lighting/src/lib.rs:77-78` states the same law).
//!
//! ONE HOME (L05): `sky_radiance` and `hemispheric_irradiance` are NOT
//! reimplemented here — they were already ported to
//! `forge-core-v3/src/zones/sky_irradiance.rs` and are re-exported below.
//! `sky_bake.rs`/`baked.rs` stay unported for the reason that file's own
//! header gives: they need `WorldContract`/`LutPack`/`presets_layout`, none of
//! which exist in v3.

pub mod lambert;
pub mod rig;
pub mod sh9;
pub mod trit_dir;

pub use lambert::{compute_illumination, compute_illumination_from_rig, tonemap};
pub use rig::{hieros_gamos_rig, is_rubedo_tier, rig_from_tod, LightRig};
pub use sh9::{bake_trit_ambient, eval_sh9, fibonacci_dir, project_sh9, sh9_basis, Sh9, SH9_COEFFS};
pub use trit_dir::{all_directions, TritDir, DIR_ORIGIN, DIR_STATES, TRITS_PER_DIR};

/// The analytic sky, re-exported from its one home in `forge-core-v3`.
pub use forge_core_v3::zones::sky_irradiance::{
    hemispheric_irradiance, sky_radiance, sky_visibility,
};

/// Project the analytic sky onto SH9 — the ambient term for a whole rig, taken
/// once instead of per fragment.
///
/// Binds this crate's [`project_sh9`] to the already-landed [`sky_radiance`],
/// which is what v2's `compute_sh9_from_sky` did against its own copy.
pub fn sh9_from_sky(
    samples: u32,
    sun_dir: [f32; 3],
    sun_color: [f32; 3],
    sun_energy: f32,
    turbidity: f32,
) -> Sh9 {
    let sun = (sun_dir[0], sun_dir[1], sun_dir[2]);
    let col = (sun_color[0], sun_color[1], sun_color[2]);
    project_sh9(samples, |d| {
        let (r, g, b) = sky_radiance((d[0], d[1], d[2]), sun, col, sun_energy, turbidity);
        [r, g, b]
    })
}

/// The rig's own sky ambient, baked to the 26-direction trit lattice — the
/// static lookup that replaces runtime ambient maths for a quantized normal.
pub fn trit_ambient_for_rig(
    rig: &LightRig,
    samples: u32,
    turbidity: f32,
) -> [[f32; 3]; DIR_STATES as usize] {
    let sh = sh9_from_sky(samples, rig.sun_direction, rig.sun_color, rig.sun_energy, turbidity);
    bake_trit_ambient(&sh)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(c: [f32; 3]) -> f32 {
        c[0] + c[1] + c[2]
    }

    #[test]
    fn the_re_exported_sky_is_the_one_in_forge_core() {
        // Not a copy: calling through this crate must equal calling the origin.
        let a = sky_radiance((0.0, 1.0, 0.0), (0.0, 1.0, 0.0), (1.0, 1.0, 1.0), 1.0, 2.0);
        let b = forge_core_v3::zones::sky_irradiance::sky_radiance(
            (0.0, 1.0, 0.0),
            (0.0, 1.0, 0.0),
            (1.0, 1.0, 1.0),
            1.0,
            2.0,
        );
        assert_eq!(a, b, "the re-export must not have become a second implementation");
    }

    #[test]
    fn a_noon_sky_projects_to_a_blue_dominant_ambient() {
        let sh = sh9_from_sky(256, [0.0, 1.0, 0.0], [1.0, 1.0, 1.0], 1.0, 2.0);
        let up = eval_sh9(&sh, [0.0, 1.0, 0.0]);
        assert!(up[2] > up[0], "daytime sky ambient is blue-dominant: {up:?}");
        assert!(lum(up) > 0.0);
    }

    #[test]
    fn a_storm_ambient_is_darker_than_a_clear_one() {
        let clear = sh9_from_sky(256, [0.0, 1.0, 0.0], [1.0; 3], 1.0, 2.0);
        let storm = sh9_from_sky(256, [0.0, 1.0, 0.0], [1.0; 3], 1.0, 8.5);
        let d = [0.0, 1.0, 0.0];
        assert!(
            lum(eval_sh9(&storm, d)) < lum(eval_sh9(&clear, d)),
            "turbidity must survive the projection"
        );
    }

    #[test]
    fn the_trit_table_is_brighter_looking_up_than_down_at_noon() {
        let rig = rig_from_tod(0.5, 6, 0.0);
        let table = trit_ambient_for_rig(&rig, 256, 2.0);
        let up = TritDir::quantize([0.0, 1.0, 0.0]).expect("up");
        let down = TritDir::quantize([0.0, -1.0, 0.0]).expect("down");
        assert!(
            lum(table[up.0 as usize]) > lum(table[down.0 as usize]),
            "sky ambient must favour the upward face: up {:?} down {:?}",
            table[up.0 as usize],
            table[down.0 as usize]
        );
    }

    #[test]
    fn a_quantized_normal_indexes_the_table_without_a_second_projection() {
        // The whole point: one bake, then every lit face is an array index.
        let rig = rig_from_tod(0.5, 6, 0.0);
        let table = trit_ambient_for_rig(&rig, 128, 2.0);
        let normal = [0.2, 0.95, -0.1];
        let d = TritDir::quantize(normal).expect("a real normal");
        assert_eq!(d.trits(), [0, 1, 0], "this normal is an up-face");
        assert_eq!(table[d.0 as usize], table[TritDir::from_trits([0, 1, 0]).0 as usize]);
    }

    #[test]
    fn a_lit_face_composes_rig_sun_with_baked_sky() {
        // Both halves of the crate meet here: Lambert from the rig, ambient
        // from the trit table, tone-mapped to display.
        let rig = rig_from_tod(0.5, 6, 0.0);
        let table = trit_ambient_for_rig(&rig, 128, 2.0);
        let n = [0.0, 1.0, 0.0];
        let diffuse = compute_illumination_from_rig(&rig, n);
        let amb = table[TritDir::quantize(n).expect("up").0 as usize];
        let lit = [
            diffuse * rig.sun_color[0] + amb[0],
            diffuse * rig.sun_color[1] + amb[1],
            diffuse * rig.sun_color[2] + amb[2],
        ];
        let px = tonemap(lit);
        assert!(px.iter().any(|c| *c > 0), "a noon up-face must not render black: {px:?}");
    }
}
