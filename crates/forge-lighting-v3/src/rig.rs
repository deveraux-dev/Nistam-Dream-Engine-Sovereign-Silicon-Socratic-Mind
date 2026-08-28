//! `LightRig` and its constructors. Ported from v2
//! `forge-lighting/src/lib.rs:16-146`; `WorldContract` has no v3 home, so the
//! three fields the donor actually read are passed directly.

/// A directional light rig. All f32 — this crate is the float lane; the
/// integer kernel never sees these.
#[derive(Debug, Clone, PartialEq)]
pub struct LightRig {
    /// Rig identity.
    pub name: &'static str,
    /// Direction the sun points, world space.
    pub sun_direction: [f32; 3],
    /// Sun colour, linear.
    pub sun_color: [f32; 3],
    /// Sun intensity. Above 1.0 is HDR and will bloom.
    pub sun_energy: f32,
    /// Ambient fill colour, linear.
    pub ambient_color: [f32; 3],
    /// Ambient fill intensity.
    pub ambient_energy: f32,
    /// Fog colour, linear.
    pub fog_color: [f32; 3],
    /// Fog density.
    pub fog_density: f32,
    /// Opposing directional light. `None` is a single-light rig.
    pub secondary_direction: Option<[f32; 3]>,
    /// Secondary colour.
    pub secondary_color: Option<[f32; 3]>,
    /// Secondary intensity.
    pub secondary_energy: Option<f32>,
    /// Subsurface scattering colour where the two lights meet.
    pub sss_color: Option<[f32; 3]>,
    /// Subsurface scattering intensity.
    pub sss_intensity: Option<f32>,
}

impl LightRig {
    /// True when a second directional light is armed.
    #[inline]
    pub const fn is_dual(&self) -> bool {
        self.secondary_direction.is_some()
    }

    /// True when any channel exceeds display range and will drive bloom.
    #[inline]
    pub fn is_hdr(&self) -> bool {
        self.sun_energy > 1.0
            || self.secondary_energy.is_some_and(|e| e > 1.0)
            || self.sss_intensity.is_some_and(|i| i > 1.0)
    }
}

/// Time-of-day rig. `tod` is `0.0` midnight to `0.5` noon; `season` is
/// `0..12`; `threat` is `0.0..1.0` and darkens and fogs the rig as it rises.
pub fn rig_from_tod(tod: f32, season: u8, threat: f32) -> LightRig {
    let tod = tod.clamp(0.0, 1.0);
    let threat = threat.clamp(0.0, 1.0);
    let sun_angle = (tod * core::f32::consts::PI * 2.0 - core::f32::consts::FRAC_PI_2).sin();
    let sun_energy = sun_angle.max(0.0);

    let warmth = 1.0 - (tod - 0.5).abs() * 2.0;
    let season_warmth =
        ((season as f32 / 12.0) * core::f32::consts::TAU).sin() * 0.5 + 0.5;

    LightRig {
        name: "auto_tod",
        sun_direction: [0.3, sun_angle.max(0.1), 0.5],
        sun_color: [1.0, 0.9 + warmth * 0.1, 0.8 + warmth * 0.2],
        sun_energy: sun_energy * (1.0 - threat * 0.8),
        ambient_color: [
            0.3 + season_warmth * 0.2,
            0.35 + season_warmth * 0.1,
            0.5 - season_warmth * 0.15,
        ],
        ambient_energy: 0.3 * (1.0 - threat * 0.5),
        fog_color: [0.5 - threat * 0.4, 0.5 - threat * 0.4, 0.55 - threat * 0.4],
        fog_density: 0.01 + threat * 0.1,
        secondary_direction: None,
        secondary_color: None,
        secondary_energy: None,
        sss_color: None,
        sss_intensity: None,
    }
}

/// Red Solar: masculine sulfur, warm from upper-left.
const SOLAR_DIRECTION: [f32; 3] = [0.6, 0.7, -0.4];
/// Deep crimson.
const SOLAR_COLOR: [f32; 3] = [1.0, 0.15, 0.05];
/// HDR intensity.
const SOLAR_ENERGY: f32 = 3.5;
/// White Mercury: feminine lunar, cool from upper-right.
const MERCURY_DIRECTION: [f32; 3] = [-0.6, 0.7, -0.4];
/// Near-white with a blue cast.
const MERCURY_COLOR: [f32; 3] = [0.95, 0.95, 1.0];
/// HDR intensity.
const MERCURY_ENERGY: f32 = 2.8;
/// Ruby-violet plasma at the collision point.
const RUBEDO_SSS_COLOR: [f32; 3] = [0.85, 0.1, 0.6];
/// Above the bloom threshold by design.
const RUBEDO_SSS_INTENSITY: f32 = 4.0;

/// The Hieros Gamos dual-light rig for Rubedo transmutation: two opposing
/// directionals whose intersection drives subsurface scatter. Ambient is
/// suppressed so the two lights carry the frame.
pub fn hieros_gamos_rig(threat: f32) -> LightRig {
    let threat = threat.clamp(0.0, 1.0);
    LightRig {
        name: "hieros_gamos_rubedo",
        sun_direction: SOLAR_DIRECTION,
        sun_color: SOLAR_COLOR,
        sun_energy: SOLAR_ENERGY,
        ambient_color: [0.02, 0.01, 0.03],
        ambient_energy: 0.05 * (1.0 - threat * 0.5),
        fog_color: [0.15, 0.02, 0.2],
        fog_density: 0.03 + threat * 0.05,
        secondary_direction: Some(MERCURY_DIRECTION),
        secondary_color: Some(MERCURY_COLOR),
        secondary_energy: Some(MERCURY_ENERGY),
        sss_color: Some(RUBEDO_SSS_COLOR),
        sss_intensity: Some(RUBEDO_SSS_INTENSITY),
    }
}

/// Whether a material qualifies for Hieros Gamos: Void-dominated and ringing
/// at 800 Hz.
#[inline]
pub fn is_rubedo_tier(void_ratio: f32, resonance_hz: u32) -> bool {
    void_ratio > 0.5 && resonance_hz == 800
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noon_has_high_sun_and_midnight_has_none() {
        assert!(rig_from_tod(0.5, 0, 0.0).sun_energy > 0.5);
        assert!(rig_from_tod(0.0, 0, 0.0).sun_energy < 0.1);
    }

    #[test]
    fn threat_darkens_and_fogs() {
        let calm = rig_from_tod(0.5, 0, 0.0);
        let dread = rig_from_tod(0.5, 0, 1.0);
        assert!(dread.sun_energy < calm.sun_energy, "threat must darken the sun");
        assert!(dread.fog_density > calm.fog_density, "threat must thicken the fog");
        assert!(dread.ambient_energy < calm.ambient_energy);
    }

    #[test]
    fn threat_outside_range_is_clamped_not_wrapped() {
        let over = rig_from_tod(0.5, 0, 9.0);
        let at_one = rig_from_tod(0.5, 0, 1.0);
        assert_eq!(over.fog_density, at_one.fog_density);
        assert!(over.sun_energy >= 0.0, "an over-range threat must not invert the sun");
    }

    #[test]
    fn hieros_gamos_is_dual_and_hdr() {
        let rig = hieros_gamos_rig(0.0);
        assert!(rig.is_dual());
        assert!(rig.is_hdr(), "both lights and the SSS exceed display range by design");
        assert!(rig.sun_color[0] > 0.9 && rig.sun_color[1] < 0.2, "primary is red solar");
        let sec = rig.secondary_color.expect("mercury");
        assert!(sec.iter().all(|c| *c > 0.9), "secondary is white mercury");
        assert!(rig.ambient_energy < rig_from_tod(0.5, 0, 0.0).ambient_energy);
    }

    #[test]
    fn the_day_rig_is_single_and_not_hdr() {
        let rig = rig_from_tod(0.5, 6, 0.0);
        assert!(!rig.is_dual());
        assert!(!rig.is_hdr(), "a plain daylight rig must not trip bloom");
    }

    #[test]
    fn rubedo_gate_needs_both_conditions() {
        assert!(is_rubedo_tier(0.6, 800));
        assert!(!is_rubedo_tier(0.4, 800), "not void-dominated");
        assert!(!is_rubedo_tier(0.6, 400), "wrong resonance");
    }
}
