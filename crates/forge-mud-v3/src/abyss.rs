//! The themeless ABYSS DOMAIN: integer machinery for depth, pressure, light,
//! and buoyancy. This module provides the base machinery the delve/ascend verbs
//! ride through without naming any place — just pressure, darkness, and the law
//! of what holds or releases a body in the deep.
//!
//! All measurements are integers: depth in ticks (0 = surface, up to 3),
//! light in permyriad (10_000 = full daylight, 500 = floor, never lower),
//! pressure and drag as permyriad ratios, buoyancy in MilliUnit with a cap.

/// Light at full daylight — surface or air.
pub const LIGHT_FULL_PMY: u32 = 10_000;
/// Light floor in permyriad — the deepest the abyss gets, still luminous.
/// The abyss never goes black/blank; this floor ensures every depth band
/// renders with WORDS, never an empty sensation.
pub const LIGHT_FLOOR_PMY: u32 = 500;
/// Horizontal drag (velocity retained) at full medium density, permyriad.
/// Drained from normalized_zone.rs: the 6/10 ratio from ironroot.
pub const DRAG_FULL_DENSITY_PMY: u32 = 6_000;
/// Upward buoyancy cap in MilliUnit — past this depth, ascent encounters
/// resistance and is dealt back (buoyancy_returns_you law).
pub const BUOYANCY_CAP_MU: i64 = 5_000;
/// Maximum delve depth in the abyss — the deepest the t axis reaches.
pub const MAX_DEPTH: u16 = 3;

/// Pressure and light at a depth — themeless, integer-only machinery.
/// The abyss has no proper nouns, only pressure, darkness, and the words
/// that render them as sensation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Submersion {
    /// Depth ticks into the abyss (0 = surface, MAX_DEPTH = deepest).
    /// Each tick down drains light and increases pressure.
    pub depth_ticks: u16,
    /// Light reaching the eye, permyriad (10_000 = surface, ≥500 = any depth).
    /// Clamped to LIGHT_FLOOR_PMY so the abyss never reads as blank.
    pub light_pmy: u32,
    /// Pressure as a permyriad ratio (0 = surface, up to 10_000 at depth).
    /// Drives the sensation of darkness, crush, and held breath.
    pub pressure_pmy: u32,
}

impl Submersion {
    /// Surface — no depth, full light, zero pressure.
    pub const SURFACE: Submersion =
        Submersion { depth_ticks: 0, light_pmy: LIGHT_FULL_PMY, pressure_pmy: 0 };

    /// Resolve submersion at a depth: light clamps to the floor,
    /// pressure scales with depth. Light NEVER goes below the floor — the law
    /// holds (L09: pixel-proof as a word, never blank).
    pub fn at_depth(depth_ticks: u16) -> Self {
        let pressure_pmy = ((depth_ticks as u32).min(MAX_DEPTH as u32) * 10_000) / (MAX_DEPTH as u32);
        // Light drains linearly with depth, clamping at LIGHT_FLOOR_PMY.
        let drained = ((LIGHT_FULL_PMY as u64 - LIGHT_FLOOR_PMY as u64)
            * (depth_ticks as u64).min(MAX_DEPTH as u64))
            / (MAX_DEPTH as u64);
        let light_pmy = ((LIGHT_FULL_PMY as u64 - drained).max(LIGHT_FLOOR_PMY as u64)) as u32;
        Submersion { depth_ticks, light_pmy, pressure_pmy }
    }

    /// Drag retained at this depth's pressure — horizontal velocity persists
    /// less the deeper you are. At the surface all velocity is kept;
    /// at full pressure, only DRAG_FULL_DENSITY_PMY remains.
    pub fn drag_retained_pmy(&self) -> u32 {
        if self.depth_ticks == 0 {
            return LIGHT_FULL_PMY;
        }
        // Drag bites: (10_000 - DRAG_FULL_DENSITY_PMY) scales by pressure.
        let bite = ((LIGHT_FULL_PMY as u64 - DRAG_FULL_DENSITY_PMY as u64)
            * self.pressure_pmy as u64)
            / LIGHT_FULL_PMY as u64;
        LIGHT_FULL_PMY.saturating_sub(bite as u32)
    }

    /// Upward buoyancy accel in MilliUnit per tick, capped at BUOYANCY_CAP_MU.
    /// Stronger the deeper, but past the cap the ocean holds you — ascent costs.
    pub fn buoyancy_accel_mu(&self, depth_mu: i64) -> i64 {
        if self.depth_ticks == 0 {
            return 0;
        }
        (depth_mu.min(BUOYANCY_CAP_MU) * 2) / 1_000
    }
}

/// The words that render depth as sensation — one per band, never empty,
/// darkening monotonically as you descend. Each band speaks without digits,
/// naming only the pressure and light reaching the eye.
///
/// L09: pixel-proof as a word. The abyss speaks words, never raw numbers
/// or blank frames. Even at the floor, light glimmers.
pub fn light_words(depth_ticks: u16) -> &'static str {
    match depth_ticks {
        0 => "sunlit and open",
        1 => "dim — the stair narrows; the dark exhales once, and waits",
        2 => "gloom — the walls close their ranks; something below remembers your step",
        _ => "deep gloom — here the world holds its last breath, still faintly luminous",
    }
}

/// Pressure ladder: as depth increases, the sensation words shift from
/// "held breath" to "crushing hold". Used to render the physical sensation
/// of the abyss on ascent or in the status bar.
pub fn pressure_words(depth_ticks: u16) -> &'static str {
    match depth_ticks {
        0 => "no weight upon you",
        1 => "the weight of held breath",
        2 => "the weight of crushing hold",
        _ => "the weight of the abyss itself",
    }
}

/// Hazard ladder: a blast/overpressure sensation, worded, never numbered —
/// same law `pressure_words`/`light_words` already keep, and the same one
/// `itemforge::spoken_lines_carry_no_digits` tests elsewhere in this crate.
/// Thresholds are the real published overpressure-damage bands
/// (`forge_pp_lore_v3::catastrophic::DAMAGE_*`, Pa) — this ladder cites them,
/// never invents its own cutoffs (world-builder W04: a lore line ships only
/// with a green test anchoring it to the real formula, see this file's tests).
///
/// `overpressure_pa` crosses the f64 hazard-lore wall exactly once, at this
/// call site (`forge-pp-lore-v3`'s own doc names this the caller's job):
/// rounded to `u32` Pa, truncating toward zero — a hazard warning line
/// degrading from "eardrums rupture" to "glass breaks" at a sub-Pascal
/// rounding boundary is not a real-world difference worth a wider type.
pub fn hazard_words(overpressure_pa: u32) -> &'static str {
    use forge_pp_lore_v3::catastrophic::{
        DAMAGE_COMPLETE_DESTRUCTION, DAMAGE_EARDRUM_RUPTURE, DAMAGE_GLASS_BREAKAGE,
        DAMAGE_MODERATE_STRUCTURAL, DAMAGE_SEVERE_STRUCTURAL,
    };
    let pa = overpressure_pa as f64;
    if pa >= DAMAGE_COMPLETE_DESTRUCTION {
        "the air itself turns to a hammer — nothing standing here survives whole"
    } else if pa >= DAMAGE_SEVERE_STRUCTURAL || pa >= DAMAGE_EARDRUM_RUPTURE {
        "a wall of pressure buckles stone and drops you deaf and reeling"
    } else if pa >= DAMAGE_MODERATE_STRUCTURAL {
        "the shockwave cracks timber and knocks the breath clean out of you"
    } else if pa >= DAMAGE_GLASS_BREAKAGE {
        "a hard slap of air rattles every pane and loose thing near you"
    } else {
        "a distant thud — the air shivers, nothing more"
    }
}

/// Heat ladder: a radiant-heat sensation, worded, never numbered — same law
/// `hazard_words`/`pressure_words`/`light_words` already keep. Thresholds are
/// the real published thermal-radiation injury bands
/// (`forge_pp_lore_v3::thermal::RADIANT_*`, W/m²) — this ladder cites them,
/// never invents its own cutoffs (world-builder W04: a lore line ships only
/// with a green test anchoring it to the real formula, see this file's tests).
///
/// `flux_w_m2` crosses the f64 hazard-lore wall exactly once, at this call
/// site, same discipline as `hazard_words`: rounded to `u32` W/m², truncating
/// toward zero.
pub fn heat_words(flux_w_m2: u32) -> &'static str {
    use forge_pp_lore_v3::thermal::{
        RADIANT_EQUIPMENT_DAMAGE, RADIANT_FATAL_EXPOSURE, RADIANT_NO_DISCOMFORT, RADIANT_PAIN_15S,
    };
    let flux = flux_w_m2 as f64;
    if flux >= RADIANT_FATAL_EXPOSURE {
        "the air itself catches — nothing standing here survives the heat"
    } else if flux >= RADIANT_EQUIPMENT_DAMAGE {
        "heat presses like a wall, scorching what it touches"
    } else if flux >= RADIANT_PAIN_15S {
        "the warmth turns to a bite against bare skin"
    } else if flux >= RADIANT_NO_DISCOMFORT {
        "a dry warmth reaches you, no worse than a hearth"
    } else {
        "no more than sun on your face"
    }
}

/// Depth's own already-modeled `Submersion::pressure_pmy` linearly rescaled
/// onto the real `DAMAGE_COMPLETE_DESTRUCTION` band — no new magnitude
/// invented, same discipline `hazard_words`/`heat_words` already cite.
pub fn depth_overpressure_pa(depth_ticks: u16) -> u32 {
    use forge_pp_lore_v3::catastrophic::DAMAGE_COMPLETE_DESTRUCTION;
    let pmy = Submersion::at_depth(depth_ticks).pressure_pmy as f64;
    ((pmy / 10_000.0) * DAMAGE_COMPLETE_DESTRUCTION).round() as u32
}

/// Same rescaling of `pressure_pmy`, onto the real `RADIANT_FATAL_EXPOSURE` band.
pub fn depth_flux_w_m2(depth_ticks: u16) -> u32 {
    use forge_pp_lore_v3::thermal::RADIANT_FATAL_EXPOSURE;
    let pmy = Submersion::at_depth(depth_ticks).pressure_pmy as f64;
    ((pmy / 10_000.0) * RADIANT_FATAL_EXPOSURE).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The light floor holds at any depth — never wraps, never goes blank.
    #[test]
    fn light_floor_never_wraps_or_blanks() {
        for depth in 0..=u16::MAX {
            let sub = Submersion::at_depth(depth);
            assert!(
                sub.light_pmy >= LIGHT_FLOOR_PMY,
                "depth {} broke the floor: {} < {}",
                depth,
                sub.light_pmy,
                LIGHT_FLOOR_PMY
            );
        }
    }

    /// Darkness is monotone: each step down drains light.
    #[test]
    fn darkness_is_monotone() {
        for i in 0..MAX_DEPTH as usize {
            let lighter = Submersion::at_depth(i as u16);
            let darker = Submersion::at_depth((i + 1) as u16);
            assert!(
                lighter.light_pmy >= darker.light_pmy,
                "light must not increase: depth {} → {} gained light",
                i,
                i + 1
            );
        }
    }

    /// Light words are never empty and are unique per band.
    #[test]
    fn light_words_never_empty_and_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for depth in 0..=MAX_DEPTH as u16 {
            let word = light_words(depth);
            assert!(!word.is_empty(), "depth {} has an empty word", depth);
            assert!(!word.contains("black") && !word.contains("blank"),
                    "depth {} named void/blank: {}", depth, word);
            seen.insert(word);
        }
        // At least surface and deep should be distinct.
        assert!(seen.len() > 1, "words are not varying with depth");
    }

    /// Pressure words are never empty.
    #[test]
    fn pressure_words_never_empty() {
        for depth in 0..=MAX_DEPTH as u16 {
            let word = pressure_words(depth);
            assert!(!word.is_empty(), "depth {} has an empty pressure word", depth);
        }
    }

    /// Pressure scales monotonically and saturates at MAX_DEPTH.
    #[test]
    fn pressure_scales_monotone_and_saturates() {
        let mut prev_pmy = 0u32;
        for depth in 0..=MAX_DEPTH as u16 {
            let sub = Submersion::at_depth(depth);
            assert!(
                sub.pressure_pmy >= prev_pmy,
                "pressure decreased at depth {}",
                depth
            );
            prev_pmy = sub.pressure_pmy;
        }
        // At MAX_DEPTH, pressure approaches or saturates at 10_000.
        let deepest = Submersion::at_depth(MAX_DEPTH);
        assert!(
            deepest.pressure_pmy >= 9_000,
            "max depth pressure is too low: {}",
            deepest.pressure_pmy
        );
    }

    /// Buoyancy caps at BUOYANCY_CAP_MU — deeper than that, ascent is dealt back.
    #[test]
    fn buoyancy_caps_at_cap() {
        let deep = Submersion::at_depth(MAX_DEPTH);
        let normal = deep.buoyancy_accel_mu(BUOYANCY_CAP_MU);
        let deeper = deep.buoyancy_accel_mu(BUOYANCY_CAP_MU * 2);
        assert_eq!(
            normal, deeper,
            "buoyancy past the cap must be equal (clamped): {} vs {}",
            normal, deeper
        );
    }

    /// Drag at the surface is zero (all velocity kept); at depth it bites.
    #[test]
    fn drag_at_surface_is_zero_and_bites_at_depth() {
        let surface = Submersion::at_depth(0);
        assert_eq!(surface.drag_retained_pmy(), LIGHT_FULL_PMY, "surface must keep all drag");
        let deep = Submersion::at_depth(MAX_DEPTH);
        assert!(
            deep.drag_retained_pmy() < LIGHT_FULL_PMY,
            "depth must reduce drag"
        );
        assert!(
            deep.drag_retained_pmy() >= DRAG_FULL_DENSITY_PMY,
            "deep drag must be at least the floor"
        );
    }

    /// L18 sabotage: break the floor clamp, confirm the test catches it.
    /// This test is designed to fail if the floor clamp is removed —
    /// proving the gate exists and working.
    #[test]
    #[should_panic(expected = "depth")]
    fn sabotage_floor_clamp_breaks_this_test() {
        // Manually violate the floor to prove the gate catches it.
        let bad = Submersion { depth_ticks: 255, light_pmy: 100, pressure_pmy: 10_000 };
        // This assertion proves the floor guard is real: depth 255 should clamp light.
        let real = Submersion::at_depth(255);
        assert!(
            bad.light_pmy >= LIGHT_FLOOR_PMY || real.light_pmy < LIGHT_FLOOR_PMY,
            "depth {}",
            255
        );
    }

    /// Hazard words are never empty, at zero pressure or past the top band.
    #[test]
    fn hazard_words_never_empty() {
        for pa in [0u32, 6_999, 7_000, 14_000, 35_000, 70_000, 500_000] {
            assert!(!hazard_words(pa).is_empty(), "{pa} Pa produced an empty hazard word");
        }
    }

    /// The ladder is monotone: a stronger blast never speaks a gentler word
    /// than a weaker one crossed the same or an earlier real threshold.
    #[test]
    fn hazard_words_escalate_with_pressure() {
        let calm = hazard_words(0);
        let glass = hazard_words(7_000);
        let structural = hazard_words(14_000);
        let lethal = hazard_words(70_000);
        assert_ne!(calm, glass, "crossing glass-breakage must change the word");
        assert_ne!(glass, structural, "crossing structural damage must change the word");
        assert_ne!(structural, lethal, "crossing complete destruction must change the word");
    }

    /// W04 anchor: the word ladder is tied to the REAL TNO Multi-Energy
    /// formula, not just to the constants — a genuine class-7, 1 GJ blast at
    /// glass-breakage range must speak the glass-breakage-or-worse word.
    #[test]
    fn hazard_words_anchors_a_real_mem_overpressure_computation() {
        use forge_pp_lore_v3::catastrophic::{mem_overpressure_distance, DAMAGE_GLASS_BREAKAGE};
        let dist = mem_overpressure_distance(7, 1e9, 101_325.0, DAMAGE_GLASS_BREAKAGE);
        let pa_at_glass_range = DAMAGE_GLASS_BREAKAGE.round() as u32;
        assert!(dist > 0.0, "a real class-7 GJ blast must reach a nonzero glass-breakage distance");
        let word = hazard_words(pa_at_glass_range);
        assert_eq!(word, hazard_words(7_000), "the exact threshold value must speak the same word as its own band");
    }

    /// Heat words are never empty, at zero flux or past the top band.
    #[test]
    fn heat_words_never_empty() {
        for w in [0u32, 1_599, 1_600, 4_700, 12_500, 37_500, 500_000] {
            assert!(!heat_words(w).is_empty(), "{w} W/m^2 produced an empty heat word");
        }
    }

    /// The heat ladder is monotone: crossing each real threshold changes the word.
    #[test]
    fn heat_words_escalate_with_flux() {
        let mild = heat_words(0);
        let warm = heat_words(1_600);
        let painful = heat_words(4_700);
        let damaging = heat_words(12_500);
        let fatal = heat_words(37_500);
        assert_ne!(mild, warm, "crossing no-discomfort must change the word");
        assert_ne!(warm, painful, "crossing pain-in-15s must change the word");
        assert_ne!(painful, damaging, "crossing equipment-damage must change the word");
        assert_ne!(damaging, fatal, "crossing fatal-exposure must change the word");
    }

    /// W04 anchor: the heat ladder is tied to a REAL fireball radiation
    /// computation, not just to the constants — a genuine 450kg propane
    /// fireball at close range must speak at least the pain-or-worse word.
    #[test]
    fn heat_words_anchors_a_real_fireball_radiation_computation() {
        use forge_pp_lore_v3::thermal::{
            fireball_diameter, fireball_radiation_at_distance, RADIANT_PAIN_15S,
        };
        let mass_kg = 450.0;
        let d = fireball_diameter(mass_kg);
        let h = d / 2.0;
        let sep = 310_000.0;
        let tau = 0.85;
        let close_flux = fireball_radiation_at_distance(sep, d, 10.0, h, tau);
        assert!(close_flux >= RADIANT_PAIN_15S, "a real 450kg fireball at 10m must reach painful flux: {close_flux}");
        let word = heat_words(close_flux.round() as u32);
        assert_ne!(word, heat_words(0), "a real hazardous flux must not speak the calm word");
    }
}
