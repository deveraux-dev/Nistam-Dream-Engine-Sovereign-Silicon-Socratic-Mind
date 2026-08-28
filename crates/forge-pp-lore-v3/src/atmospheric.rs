//! Atmospheric dispersion equations.
//!
//! Stateless: gaussian_plume
//! Stateful RHS: shallow_water_rhs (LNG Dispersion #14)

/// Toxic Plume (#19) — Gaussian plume model.
/// Steady-state concentration at point (x, y, z) downwind of a continuous release.
///
/// C(x,y,z) = Q/(2πuσyσz) · exp(-y²/2σy²) · [exp(-(z-H)²/2σz²) + exp(-(z+H)²/2σz²)]
///
/// - `q`: emission rate (kg/s or g/s — units carry through)
/// - `u`: wind speed at release height (m/s)
/// - `sigma_y`: horizontal dispersion coefficient (m)
/// - `sigma_z`: vertical dispersion coefficient (m)
/// - `x`: downwind distance (m) — not used directly but sigma_y/sigma_z are functions of x
/// - `y`: crosswind distance from plume centerline (m)
/// - `z`: receptor height above ground (m)
/// - `h`: effective release height (m)
///
/// Returns: concentration at (x,y,z) in same mass units as q per m³
pub fn gaussian_plume(q: f64, u: f64, sigma_y: f64, sigma_z: f64, y: f64, z: f64, h: f64) -> f64 {
    use std::f64::consts::PI;

    if u <= 0.0 || sigma_y <= 0.0 || sigma_z <= 0.0 {
        return 0.0;
    }

    let lateral = (-y * y / (2.0 * sigma_y * sigma_y)).exp();
    let vertical_direct = (-(z - h) * (z - h) / (2.0 * sigma_z * sigma_z)).exp();
    let vertical_reflected = (-(z + h) * (z + h) / (2.0 * sigma_z * sigma_z)).exp();

    q / (2.0 * PI * u * sigma_y * sigma_z) * lateral * (vertical_direct + vertical_reflected)
}

/// Pasquill-Gifford sigma_y approximation.
/// Horizontal dispersion coefficient as function of downwind distance and stability class.
///
/// - `x`: downwind distance (m)
/// - `stability`: Pasquill stability class (1=A very unstable, 6=F very stable)
///
/// Returns: sigma_y in meters
pub fn pasquill_sigma_y(x: f64, stability: i32) -> f64 {
    // Simplified power-law approximation
    // sigma_y = a * x^b (Briggs urban formulation)
    let (a, b) = match stability {
        1 => (0.22, 0.894),  // A
        2 => (0.16, 0.894),  // B
        3 => (0.11, 0.894),  // C
        4 => (0.08, 0.894),  // D
        5 => (0.06, 0.894),  // E
        _ => (0.04, 0.894),  // F
    };
    a * x.powf(b)
}

/// Pasquill-Gifford sigma_z approximation.
/// Vertical dispersion coefficient as function of downwind distance and stability class.
///
/// - `x`: downwind distance (m)
/// - `stability`: Pasquill stability class (1=A, 6=F)
///
/// Returns: sigma_z in meters
pub fn pasquill_sigma_z(x: f64, stability: i32) -> f64 {
    let (a, b) = match stability {
        1 => (0.20, 0.894),  // A
        2 => (0.12, 0.894),  // B
        3 => (0.08, 0.894),  // C
        4 => (0.06, 0.894),  // D
        5 => (0.03, 0.894),  // E
        _ => (0.016, 0.894), // F
    };
    a * x.powf(b)
}

// ── Stateful RHS ────────────────────────────────────────────────────────────

/// LNG Dispersion (#14) — Shallow water heavy gas RHS.
/// Returns time derivatives of height and velocity.
///
/// ∂h/∂t = -∇·(hu) + Ė
/// ∂(hu)/∂t = -∇·(huu) - g'h∇h + friction
///
/// - `h`: gas cloud height (m)
/// - `hu`: momentum (m²/s)
/// - `g_prime`: reduced gravity g·(ρ_gas - ρ_air)/ρ_air (m/s²)
/// - `h_gradient`: spatial gradient of h (m/m)
/// - `source_rate`: volumetric source term Ė (m/s)
/// - `friction`: ground friction term (m/s²)
///
/// Returns: (dh_dt, d_hu_dt)
pub fn shallow_water_rhs(
    h: f64,
    hu: f64,
    g_prime: f64,
    h_gradient: f64,
    source_rate: f64,
    friction: f64,
) -> (f64, f64) {
    let u = if h > 1e-6 { hu / h } else { 0.0 };
    let dh_dt = -hu * h_gradient + source_rate;
    let d_hu_dt = -hu * u * h_gradient - g_prime * h * h_gradient + friction;
    (dh_dt, d_hu_dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_plume_centerline() {
        // On centerline (y=0), ground level (z=0), H=50m
        let c = gaussian_plume(10.0, 5.0, 50.0, 25.0, 0.0, 0.0, 50.0);
        assert!(c > 0.0);
        assert!(c < 1.0); // Sanity: concentration should be small at distance
    }

    #[test]
    fn test_gaussian_plume_zero_wind() {
        let c = gaussian_plume(10.0, 0.0, 50.0, 25.0, 0.0, 0.0, 50.0);
        assert_eq!(c, 0.0); // No wind = no transport = no concentration
    }

    #[test]
    fn test_gaussian_plume_symmetry() {
        // Crosswind symmetry: C(y) == C(-y)
        let c_pos = gaussian_plume(10.0, 5.0, 50.0, 25.0, 30.0, 0.0, 50.0);
        let c_neg = gaussian_plume(10.0, 5.0, 50.0, 25.0, -30.0, 0.0, 50.0);
        assert!((c_pos - c_neg).abs() < 1e-15);
    }

    #[test]
    fn test_sigma_increases_with_distance() {
        for stability in 1..=6 {
            let s1 = pasquill_sigma_y(100.0, stability);
            let s2 = pasquill_sigma_y(1000.0, stability);
            assert!(s2 > s1);
        }
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Cinderfall Breach's smoke plume — closing a chain this
    /// session already built: the Audio-lane brick flagged an active
    /// habitat-discontinuity event there
    /// (`forge-soundwave-v3::ecology::cinderfall_breach_ecology_lore_tie_carries_an_active_event`),
    /// the Physics-lane brick named its cause as a fire ignition
    /// (`forge-physics-v3::types::tests::cinderfall_breach_fire_ignition_lore_tie_is_a_milestone`),
    /// and this one anchors the smoke it produces downwind. Anchors to the
    /// already-landed `gaussian_plume`/`pasquill_sigma_y` rather than an
    /// invented "smoke drifts" flavour line. [OBSERVED] fabric:
    /// `gaussian_plume`, already tested generically above.
    #[test]
    fn cinderfall_breach_smoke_plume_lore_tie() {
        // A moderate fire source, light wind, near-ground receptor on the
        // plume centerline vs. 100m crosswind.
        let source_rate = 5.0;
        let wind_speed = 3.0;
        let downwind_distance = 200.0;
        let stability_class = 4; // D — neutral, common daytime condition
        let sigma_y = pasquill_sigma_y(downwind_distance, stability_class);
        let sigma_z = pasquill_sigma_z(downwind_distance, stability_class);

        let centerline = gaussian_plume(source_rate, wind_speed, sigma_y, sigma_z, 0.0, 0.0, 5.0);
        let crosswind = gaussian_plume(source_rate, wind_speed, sigma_y, sigma_z, 100.0, 0.0, 5.0);

        assert!(centerline > 0.0, "the breach's smoke must reach a real receptor downwind");
        assert!(crosswind < centerline, "smoke 100m off the plume centerline must be thinner than dead ahead");
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Precipice of Null's heavy-gas seep — the same sheer-edge
    /// zone the Audio and Physics bricks already anchor
    /// (`forge-soundwave-v3::ecology::precipice_of_null_ecology_lore_tie_survives_wire_at_the_sheer_edge`,
    /// `forge-physics-v3::types::tests::precipice_of_null_rockslide_lore_tie`)
    /// — pools a real heavy gas seep at its base rather than dispersing
    /// upward, the way LNG-style heavy gas actually behaves near ground
    /// level. Anchors to the already-landed `shallow_water_rhs` rather than
    /// an invented "gas pools here" flavour line. [OBSERVED] fabric:
    /// `shallow_water_rhs`, real per this file's own module doc, not yet
    /// directly tested in this file's own test suite.
    #[test]
    fn precipice_of_null_heavy_gas_seep_lore_tie() {
        // A heavy gas cloud spreading under its own weight: positive source
        // rate, real reduced-gravity term, a real height gradient.
        let cloud_height_m = 0.5;
        let momentum = 0.0; // still pooling, not yet flowing
        let g_prime = 4.0; // real reduced gravity for a dense gas vs air
        let h_gradient = 0.1; // spreading outward from the seep point
        let source_rate = 0.05; // continuous seep
        let friction = -0.02; // ground drag resists the spread

        let (dh_dt, d_hu_dt) = shallow_water_rhs(cloud_height_m, momentum, g_prime, h_gradient, source_rate, friction);
        assert!(dh_dt > 0.0, "a fed heavy-gas seep must accumulate height at its source, not vanish");
        assert!(d_hu_dt < 0.0, "the reduced-gravity spreading term must dominate and drive the cloud outward from the seep");
    }
}
