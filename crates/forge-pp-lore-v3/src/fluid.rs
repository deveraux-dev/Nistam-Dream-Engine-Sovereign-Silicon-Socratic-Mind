//! Fluid mechanics equations.
//!
//! Stateless: joukowsky, korteweg_wave_speed, pipe_period, line_pack_pressure,
//!            cavity_collapse_pressure, wave_reflection, ergun, darcy_weisbach,
//!            two_film_transfer, manning
//! Stateful RHS: multiphase_ns_rhs (Slug Flow #12)

/// Water Hammer (#11) — Joukowsky equation.
/// Instantaneous pressure rise from sudden valve closure.
///
/// ΔP = ρ · a · Δv
///
/// - `rho`: fluid density (kg/m³)
/// - `wave_speed`: pressure wave speed in fluid (m/s)
/// - `delta_v`: change in flow velocity (m/s)
///
/// Returns: pressure rise in Pascals (Pa)
pub fn joukowsky(rho: f64, wave_speed: f64, delta_v: f64) -> f64 {
    rho * wave_speed * delta_v
}

/// Korteweg wave speed — acoustic velocity in an elastic fluid-filled pipe.
/// The critical variable in water hammer. Accounts for pipe wall elasticity
/// that slows the pressure wave below the fluid's pure acoustic velocity.
///
/// a = sqrt(K/ρ) / sqrt(1 + (K·D)/(E·e) · ψ)
///
/// - `bulk_modulus`: fluid bulk modulus K (Pa). Water=2.2e9, crude=1.5e9, LPG=0.8e9
/// - `fluid_density`: fluid density ρ (kg/m³)
/// - `pipe_diameter`: internal pipe diameter D (m)
/// - `wall_thickness`: pipe wall thickness e (m)
/// - `youngs_modulus`: pipe material Young's modulus E (Pa). Steel=200e9, HDPE=1e9
/// - `restraint_psi`: pipe restraint correction factor ψ
///   Anchored: 1 - ν² (0.91 for steel)
///   Expansion joints: 1 - ν/2
///   Free end: 1.25 - ν
///
/// Returns: pressure wave speed (m/s)
pub fn korteweg_wave_speed(
    bulk_modulus: f64,
    fluid_density: f64,
    pipe_diameter: f64,
    wall_thickness: f64,
    youngs_modulus: f64,
    restraint_psi: f64,
) -> f64 {
    if fluid_density <= 0.0 || wall_thickness <= 0.0 || youngs_modulus <= 0.0 {
        return 0.0;
    }
    let c_fluid = (bulk_modulus / fluid_density).sqrt();
    let elasticity_ratio = (bulk_modulus * pipe_diameter) / (youngs_modulus * wall_thickness) * restraint_psi;
    c_fluid / (1.0 + elasticity_ratio).sqrt()
}

/// Pipe restraint factor ψ for anchored pipe (most common industrial case).
/// ψ = 1 - ν²
///
/// - `poisson_ratio`: Poisson's ratio ν (steel ≈ 0.3, HDPE ≈ 0.45)
///
/// Returns: dimensionless restraint factor
pub fn restraint_psi_anchored(poisson_ratio: f64) -> f64 {
    1.0 - poisson_ratio * poisson_ratio
}

/// Pipe period — the fundamental communication time.
/// Determines whether a valve closure is "rapid" (tc < 2L/a) or "gradual" (tc > 2L/a).
///
/// T = 2L/a
///
/// - `pipe_length`: total pipe length L (m)
/// - `wave_speed`: Korteweg wave speed a (m/s)
///
/// Returns: pipe period (seconds)
pub fn pipe_period(pipe_length: f64, wave_speed: f64) -> f64 {
    if wave_speed <= 0.0 { return 0.0; }
    2.0 * pipe_length / wave_speed
}

/// Is the valve closure "rapid" (produces full Joukowsky surge)?
///
/// - `closure_time`: valve closure time (s)
/// - `pipe_length`: pipe length (m)
/// - `wave_speed`: wave speed (m/s)
///
/// Returns: true if closure is rapid (tc < 2L/a)
pub fn is_rapid_closure(closure_time: f64, pipe_length: f64, wave_speed: f64) -> bool {
    closure_time < pipe_period(pipe_length, wave_speed)
}

/// Line pack pressure — the additional pressure from friction recovery.
/// In long pipelines, the frictional pressure drop during steady flow is "recovered"
/// and added to the surge when the valve closes.
///
/// ΔP_linepack = f · L · ρ · v² / (2 · D)
///
/// - `friction_factor`: Darcy friction factor (dimensionless)
/// - `pipe_length`: L (m)
/// - `fluid_density`: ρ (kg/m³)
/// - `velocity`: steady-state flow velocity (m/s)
/// - `pipe_diameter`: D (m)
///
/// Returns: line pack pressure addition (Pa)
pub fn line_pack_pressure(
    friction_factor: f64,
    pipe_length: f64,
    fluid_density: f64,
    velocity: f64,
    pipe_diameter: f64,
) -> f64 {
    if pipe_diameter <= 0.0 { return 0.0; }
    friction_factor * pipe_length * fluid_density * velocity * velocity / (2.0 * pipe_diameter)
}

/// Cavity collapse pressure — peak pressure from vapor cavity collapse.
/// When column separation occurs and the liquid columns rejoin,
/// the collapse pressure can reach 2x or more of the initial Joukowsky surge.
///
/// Uses the DGCM (Discrete Gas Cavity Model) simplified estimate:
/// The collapse pressure depends on the relative velocity of the
/// rejoining liquid columns, which can exceed the original flow velocity.
///
/// - `fluid_density`: ρ (kg/m³)
/// - `wave_speed`: a (m/s)
/// - `rejoining_velocity`: velocity at which liquid columns collide (m/s)
///   Typically 1.5-2.5× the original flow velocity due to gravity and reflections
///
/// Returns: cavity collapse overpressure (Pa)
pub fn cavity_collapse_pressure(
    fluid_density: f64,
    wave_speed: f64,
    rejoining_velocity: f64,
) -> f64 {
    // Same Joukowsky equation but with the rejoining velocity
    // which is typically higher than the original flow velocity
    fluid_density * wave_speed * rejoining_velocity.abs()
}

/// Check if column separation will occur.
/// Column separation happens when the transient downsurge drops
/// the local pressure below the fluid's vapor pressure.
///
/// - `static_pressure`: steady-state pressure at the location (Pa)
/// - `joukowsky_surge`: the Joukowsky pressure rise (Pa, positive)
/// - `vapor_pressure`: fluid vapor pressure at operating temperature (Pa)
///
/// Returns: true if column separation is predicted
pub fn will_cavitate(static_pressure: f64, joukowsky_surge: f64, vapor_pressure: f64) -> bool {
    (static_pressure - joukowsky_surge) < vapor_pressure
}

/// Wave reflection factor at a pipe junction.
/// When a pressure wave hits a junction of pipes, it partially reflects and
/// partially transmits. Dead ends double the pressure (r=1).
///
/// s = 2·(A₀/a₀) / Σ(Aᵢ/aᵢ)  (transmission)
/// r = s - 1                      (reflection)
///
/// - `incoming_area`: cross-sectional area of incoming pipe (m²)
/// - `incoming_wave_speed`: wave speed in incoming pipe (m/s)
/// - `outgoing_areas`: areas of all outgoing pipes (m²)
/// - `outgoing_wave_speeds`: wave speeds in all outgoing pipes (m/s)
///
/// Returns: (transmission_factor, reflection_factor)
pub fn wave_reflection(
    incoming_area: f64,
    incoming_wave_speed: f64,
    outgoing_areas: &[f64],
    outgoing_wave_speeds: &[f64],
) -> (f64, f64) {
    if incoming_wave_speed <= 0.0 {
        return (0.0, 1.0);
    }
    let incoming_impedance = incoming_area / incoming_wave_speed;

    let mut sum_outgoing_impedance = 0.0;
    for (area, speed) in outgoing_areas.iter().zip(outgoing_wave_speeds.iter()) {
        if *speed > 0.0 {
            sum_outgoing_impedance += area / speed;
        }
    }

    // Dead end: no outgoing pipes → full reflection (doubles pressure)
    if sum_outgoing_impedance <= 0.0 {
        return (0.0, 1.0);
    }

    let total_impedance = incoming_impedance + sum_outgoing_impedance;
    let transmission = 2.0 * incoming_impedance / total_impedance;
    let reflection = transmission - 1.0;
    (transmission, reflection)
}

/// Total peak pressure from water hammer with all contributing factors.
/// Combines Joukowsky surge + line pack + potential cavity collapse.
///
/// - `static_pressure`: operating pressure (Pa)
/// - `joukowsky_surge`: ΔP from joukowsky() (Pa)
/// - `line_pack`: ΔP from line_pack_pressure() (Pa)
/// - `vapor_pressure`: fluid vapor pressure (Pa)
/// - `wave_speed`: Korteweg wave speed (m/s)
/// - `fluid_density`: ρ (kg/m³)
/// - `flow_velocity`: original steady velocity (m/s)
///
/// Returns: (peak_pressure_pa, cavitation_occurred, failure_mechanism)
/// failure_mechanism: 0=none, 1=joukowsky_only, 2=with_linepack, 3=cavity_collapse
pub fn total_water_hammer_pressure(
    static_pressure: f64,
    joukowsky_surge: f64,
    line_pack: f64,
    vapor_pressure: f64,
    wave_speed: f64,
    fluid_density: f64,
    flow_velocity: f64,
) -> (f64, bool, u8) {
    let p_joukowsky = static_pressure + joukowsky_surge + line_pack;

    let cavitates = will_cavitate(static_pressure, joukowsky_surge, vapor_pressure);

    if cavitates {
        // Cavity collapse produces higher pressure
        // Rejoining velocity typically 2x original for gravity-assisted collapse
        let rejoin_v = flow_velocity * 2.0;
        let p_collapse = cavity_collapse_pressure(fluid_density, wave_speed, rejoin_v);
        let p_total = static_pressure + p_collapse + line_pack;
        (p_total, true, 3)
    } else if line_pack > 0.1 * joukowsky_surge {
        (p_joukowsky, false, 2)
    } else {
        (static_pressure + joukowsky_surge, false, 1)
    }
}

/// CSA Z662 compliance check — transient pressure must not exceed 110% of MOP.
///
/// - `peak_pressure`: total peak transient pressure (Pa)
/// - `mop`: Maximum Operating Pressure (Pa)
///
/// Returns: (compliant, exceedance_ratio) where ratio = peak/mop
pub fn csa_z662_compliance(peak_pressure: f64, mop: f64) -> (bool, f64) {
    if mop <= 0.0 { return (false, 0.0); }
    let ratio = peak_pressure / mop;
    (ratio <= 1.1, ratio)
}

/// FCCU Reversal (#6) — Ergun equation.
/// Pressure drop per unit length through a packed bed.
///
/// ΔP/L = 150μ(1-ε)² / (Φs²Dp²ε³) · v₀ + 1.75ρ(1-ε) / (ΦsDpε³) · v₀²
///
/// - `mu`: fluid viscosity (Pa·s)
/// - `epsilon`: void fraction (dimensionless, 0-1)
/// - `phi_s`: sphericity of particles (dimensionless, 0-1)
/// - `dp`: particle diameter (m)
/// - `v0`: superficial velocity (m/s)
/// - `rho`: fluid density (kg/m³)
///
/// Returns: pressure drop per unit length (Pa/m)
pub fn ergun(mu: f64, epsilon: f64, phi_s: f64, dp: f64, v0: f64, rho: f64) -> f64 {
    let e3 = epsilon * epsilon * epsilon;
    let one_minus_e = 1.0 - epsilon;
    let phi_dp = phi_s * dp;

    let viscous = 150.0 * mu * one_minus_e * one_minus_e / (phi_dp * phi_dp * e3) * v0;
    let inertial = 1.75 * rho * one_minus_e / (phi_dp * e3) * v0 * v0;

    viscous + inertial
}

/// Firewater Hydraulics (#18) — Darcy-Weisbach equation.
/// Friction head loss in a pipe.
///
/// hf = fD · (L/D) · v² / (2g)
///
/// - `f_d`: Darcy friction factor (dimensionless)
/// - `length`: pipe length (m)
/// - `diameter`: pipe inner diameter (m)
/// - `velocity`: flow velocity (m/s)
/// - `g`: gravitational acceleration (m/s², default 9.81)
///
/// Returns: friction head loss (m)
pub fn darcy_weisbach(f_d: f64, length: f64, diameter: f64, velocity: f64, g: f64) -> f64 {
    f_d * (length / diameter) * velocity * velocity / (2.0 * g)
}

/// H2S Excursion (#10) — Two-film mass transfer.
/// Mass transfer flux across a gas-liquid interface.
///
/// NA = KL · (CA_star - CA)
///
/// - `kl`: liquid-side mass transfer coefficient (m/s)
/// - `ca_star`: equilibrium concentration at interface (mol/m³)
/// - `ca`: bulk liquid concentration (mol/m³)
///
/// Returns: mass transfer flux (mol/m²·s)
pub fn two_film_transfer(kl: f64, ca_star: f64, ca: f64) -> f64 {
    kl * (ca_star - ca)
}

/// Manning equation — open channel flow velocity.
/// Not in the original 20, added for pp-sim hydrology.
///
/// v = (1/n) · R^(2/3) · S^(1/2)
///
/// - `n`: Manning roughness coefficient (dimensionless)
/// - `hydraulic_radius`: cross-sectional area / wetted perimeter (m)
/// - `slope`: channel bed slope (m/m)
///
/// Returns: flow velocity (m/s)
pub fn manning(n: f64, hydraulic_radius: f64, slope: f64) -> f64 {
    (1.0 / n) * hydraulic_radius.powf(2.0 / 3.0) * slope.sqrt()
}

/// Slug Flow (#12) — Multiphase Navier-Stokes right-hand side.
/// Returns the time derivative of the state vector.
///
/// ∂(αkρkvk)/∂t = -αk∇P + ∇·τk + αkρkg - ∇·(αkρkvkvk)
///
/// This is a placeholder signature. The full implementation requires
/// spatial discretization handled by pp-sim's PDE integrator.
///
/// - `alpha`: phase volume fraction
/// - `rho`: phase density (kg/m³)
/// - `v`: phase velocity (m/s)
/// - `pressure_gradient`: ∇P (Pa/m)
/// - `shear_stress_div`: ∇·τ (Pa/m)
/// - `g`: gravitational acceleration (m/s²)
///
/// Returns: time derivative of (αρv) — momentum source term (kg/m²·s²)
pub fn multiphase_ns_rhs(
    alpha: f64,
    rho: f64,
    _v: f64,
    pressure_gradient: f64,
    shear_stress_div: f64,
    g: f64,
) -> f64 {
    -alpha * pressure_gradient + shear_stress_div + alpha * rho * g
}

// ── Surface tension ──────────────────────────────────────────────────────────

/// Young-Laplace pressure — the pressure jump across a curved liquid interface
/// (why small droplets and bubbles hold higher internal pressure).
///
/// ΔP = 2γ / r
///
/// - `surface_tension_n_m`: γ (N/m). Water ≈ 0.0728 at 20 °C.
/// - `radius_m`: interface radius of curvature (m)
///
/// Returns: pressure difference across the interface (Pa)
pub fn young_laplace_pressure(surface_tension_n_m: f64, radius_m: f64) -> f64 {
    if radius_m <= 0.0 {
        return 0.0;
    }
    2.0 * surface_tension_n_m / radius_m
}

/// Capillary rise — Jurin's law. How far liquid climbs a narrow tube against
/// gravity, pulled up by surface tension.
///
/// h = 2γ·cosθ / (ρ·g·r)
///
/// - `surface_tension_n_m`: γ (N/m)
/// - `contact_angle_rad`: wetting contact angle θ (rad; water/glass ≈ 0)
/// - `density`: liquid density ρ (kg/m³)
/// - `radius_m`: tube radius r (m)
/// - `g`: gravitational acceleration (m/s²)
///
/// Returns: rise height (m; negative = depression for θ > 90°)
pub fn capillary_rise(surface_tension_n_m: f64, contact_angle_rad: f64, density: f64, radius_m: f64, g: f64) -> f64 {
    if density <= 0.0 || radius_m <= 0.0 || g <= 0.0 {
        return 0.0;
    }
    2.0 * surface_tension_n_m * contact_angle_rad.cos() / (density * g * radius_m)
}

/// Weber number — the ratio of disruptive inertia to cohesive surface tension.
/// We ≫ 1 means flow inertia tears the surface apart (atomization, splashing).
///
/// We = ρ·v²·L / γ
pub fn weber_number(density: f64, velocity: f64, length: f64, surface_tension_n_m: f64) -> f64 {
    if surface_tension_n_m <= 0.0 {
        return f64::MAX;
    }
    density * velocity * velocity * length / surface_tension_n_m
}

/// Bond (Eötvös) number — gravity vs surface tension. Bo ≪ 1 = tension-dominated
/// (beading droplets); Bo ≫ 1 = gravity-dominated (puddling, flattening).
///
/// Bo = ρ·g·L² / γ
pub fn bond_number(density: f64, g: f64, length: f64, surface_tension_n_m: f64) -> f64 {
    if surface_tension_n_m <= 0.0 {
        return f64::MAX;
    }
    density * g * length * length / surface_tension_n_m
}

#[cfg(test)]
mod surface_tension_tests {
    use super::*;

    #[test]
    fn young_laplace_rises_as_the_droplet_shrinks() {
        let big = young_laplace_pressure(0.0728, 0.01);
        let small = young_laplace_pressure(0.0728, 0.0001);
        assert!(small > big, "smaller droplet = higher internal pressure");
        assert!((big - 2.0 * 0.0728 / 0.01).abs() < 1e-9, "ΔP = 2γ/r");
        assert_eq!(young_laplace_pressure(0.0728, 0.0), 0.0, "zero radius guarded");
    }

    #[test]
    fn water_climbs_a_thin_capillary() {
        // Water in a 0.1 mm-radius glass tube (θ≈0) rises ~0.15 m.
        let h = capillary_rise(0.0728, 0.0, 1000.0, 0.0001, 9.81);
        assert!(h > 0.1 && h < 0.2, "capillary rise ≈ 0.15 m, got {h}");
    }

    #[test]
    fn weber_and_bond_scale_the_right_way() {
        let calm = weber_number(1000.0, 0.1, 0.01, 0.0728);
        let violent = weber_number(1000.0, 10.0, 0.01, 0.0728);
        assert!(violent > calm && calm > 0.0, "faster flow = higher Weber (atomization)");
        let beads = bond_number(1000.0, 9.81, 0.0005, 0.0728); // tiny drop
        let puddles = bond_number(1000.0, 9.81, 0.05, 0.0728); // big pool
        assert!(beads < 1.0 && puddles > 1.0, "small = tension-dominated, large = gravity-dominated");
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Void Marshes' mist — the same zone the Audio-lane brick
    /// anchors (`forge-soundwave-v3::ecology::void_marshes_ecology_lore_tie`)
    /// and the Lorekeeper-lane dew-point brick already explains as
    /// perpetually foggy (`psychrometric::void_marshes_perpetual_fog_lore_tie`)
    /// — condenses into fine, high-pressure mist droplets rather than heavy
    /// dew drops. Anchors to the already-landed `young_laplace_pressure`
    /// rather than an invented "misty" flavour line. [OBSERVED] fabric:
    /// `young_laplace_pressure`, already tested generically above.
    #[test]
    fn void_marshes_mist_droplet_lore_tie() {
        // A fine airborne mist droplet (10 micron radius) vs. a heavy dew
        // drop (2 mm radius), same water surface tension.
        let mist_radius_m = 0.00001;
        let dew_radius_m = 0.002;
        let mist_pressure = young_laplace_pressure(0.0728, mist_radius_m);
        let dew_pressure = young_laplace_pressure(0.0728, dew_radius_m);
        assert!(mist_pressure > dew_pressure, "the marsh's fine mist must carry higher internal pressure than a heavy dew drop");
        assert!(mist_pressure > 0.0, "the mist droplet must carry a real nonzero pressure");
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Broken Forge's quench trough — closing another leg of the
    /// Broken Forge chain — genuinely risks cavitation when a red-hot ingot
    /// plunges in, not an invented "it hisses" flavour line. Anchors to the
    /// already-landed `will_cavitate` rather than a made-up cavitation call.
    /// [OBSERVED] fabric: `will_cavitate`, real per this file, not yet
    /// directly tested in this file's own suite.
    #[test]
    fn broken_forge_quench_trough_cavitation_lore_tie() {
        let static_pressure_pa = 101_325.0; // atmospheric, open trough
        let vapor_pressure_pa = 2_339.0; // water vapor pressure near quench temperature
        let mild_surge_pa = 5_000.0; // a small ingot, gentle plunge — no cavitation
        let violent_surge_pa = 100_000.0; // a large red-hot ingot, sudden vapor collapse

        assert!(!will_cavitate(static_pressure_pa, mild_surge_pa, vapor_pressure_pa), "a gentle quench must not cavitate");
        assert!(will_cavitate(static_pressure_pa, violent_surge_pa, vapor_pressure_pa), "a violent quench plunge must genuinely risk cavitation");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_korteweg_water_steel_sch40() {
        // Test Case 1: 12" Sch 40 carbon steel, water at 20°C
        // Expected: ~1303 m/s (from research derivation)
        let a = korteweg_wave_speed(
            2.19e9,   // K: water bulk modulus
            998.0,    // ρ: water density
            0.3048,   // D: 12" pipe ID
            0.0104,   // e: Sch 40 wall thickness
            200e9,    // E: steel Young's modulus
            0.91,     // ψ: anchored (1 - 0.3²)
        );
        assert!((a - 1303.0).abs() < 20.0, "Wave speed: {a} m/s");
    }

    #[test]
    fn test_korteweg_hdpe_much_slower() {
        // HDPE pipe: wave speed should be much lower (~380 m/s)
        let a_steel = korteweg_wave_speed(2.2e9, 998.0, 0.3, 0.01, 200e9, 0.91);
        let a_hdpe = korteweg_wave_speed(2.2e9, 998.0, 0.3, 0.027, 1.0e9, 0.80);
        assert!(a_hdpe < a_steel * 0.5, "HDPE should be much slower: steel={a_steel}, hdpe={a_hdpe}");
    }

    #[test]
    fn test_pipe_period_1km() {
        // 1000m pipe, wave speed 1303 m/s → period = 1.53s
        let t = pipe_period(1000.0, 1303.0);
        assert!((t - 1.535).abs() < 0.01, "Period: {t}");
    }

    #[test]
    fn test_rapid_closure() {
        // 30s closure on 50km pipe with a=1150 → period = 86.9s
        // 30 < 86.9 → RAPID (full Joukowsky)
        assert!(is_rapid_closure(30.0, 50_000.0, 1150.0));
        // 100s closure → GRADUAL
        assert!(!is_rapid_closure(100.0, 50_000.0, 1150.0));
    }

    #[test]
    fn test_case1_joukowsky_water() {
        // Test Case 1: 1000m, 12" Sch40, water, 3 m/s, instantaneous
        let a = korteweg_wave_speed(2.19e9, 998.0, 0.3048, 0.0104, 200e9, 0.91);
        let dp = joukowsky(998.0, a, 3.0);
        // Expected: ~3.90 MPa
        assert!((dp - 3.90e6).abs() < 0.1e6, "Joukowsky surge: {dp} Pa");
    }

    #[test]
    fn test_case2_crude_oil_winter() {
        // Test Case 2: 50km, 24" X65, crude at -20°C, 2 m/s, 30s closure
        // Wave speed ~1150 m/s, pipe period 86.9s, closure 30s < 86.9s → RAPID
        let a = korteweg_wave_speed(1.6e9, 870.0, 0.590, 0.010, 205e9, 0.91);
        assert!(is_rapid_closure(30.0, 50_000.0, a));
        let dp = joukowsky(870.0, a, 2.0);
        // Expected: ~2.0 MPa
        assert!((dp - 2.0e6).abs() < 0.2e6, "Crude oil surge: {dp} Pa");

        // CSA Z662: peak must not exceed 110% of MOP
        let static_p = 8.5e6;
        let line_pack = line_pack_pressure(0.02, 50_000.0, 870.0, 2.0, 0.590);
        let (compliant, ratio) = csa_z662_compliance(static_p + dp + line_pack, 9.93e6);
        assert!(!compliant, "Should FAIL CSA Z662. Ratio: {ratio}");
    }

    #[test]
    fn test_case3_lpg_cavitation() {
        // Test Case 3: 5km, 8" Sch80, LPG at 20°C, 2.5 m/s, pump trip
        let a = korteweg_wave_speed(0.8e9, 500.0, 0.1937, 0.0127, 200e9, 0.91);
        let dp = joukowsky(500.0, a, 2.5);

        // Check cavitation: static 1.5 MPa, downsurge dp, vapor pressure 0.85 MPa
        let cavitates = will_cavitate(1.5e6, dp, 0.85e6);
        assert!(cavitates, "LPG should cavitate. ΔP={dp}, residual={}", 1.5e6 - dp);

        // Cavity collapse at 2x velocity
        let p_collapse = cavity_collapse_pressure(500.0, a, 5.0);
        assert!(p_collapse > dp * 1.5, "Collapse should exceed initial surge: collapse={p_collapse}, joukowsky={dp}");
    }

    #[test]
    fn test_wave_reflection_dead_end() {
        // Dead end: full reflection, doubles pressure
        let (s, r) = wave_reflection(0.1, 1300.0, &[], &[]);
        assert_eq!(s, 0.0);
        assert_eq!(r, 1.0);
    }

    #[test]
    fn test_wave_reflection_equal_branch() {
        // Pipe splits into two identical pipes
        let a = 1300.0;
        let area = 0.1;
        let (s, r) = wave_reflection(area, a, &[area, area], &[a, a]);
        // Transmission = 2/3, reflection = -1/3
        assert!((s - 0.6667).abs() < 0.01, "Transmission: {s}");
        assert!((r - (-0.3333)).abs() < 0.01, "Reflection: {r}");
    }

    #[test]
    fn test_total_pressure_with_linepack() {
        // Long pipeline where line pack matters
        let static_p = 5.0e6;
        let dp_j = 2.0e6;
        let lp = 0.5e6;
        let (peak, cav, mech) = total_water_hammer_pressure(
            static_p, dp_j, lp, 0.0, 1200.0, 998.0, 2.0,
        );
        assert!(!cav);
        assert_eq!(mech, 2); // with linepack
        assert!((peak - 7.5e6).abs() < 0.01e6);
    }

    #[test]
    fn test_winter_increases_wave_speed() {
        // Same pipe, cold fluid has higher bulk modulus → higher wave speed
        let a_summer = korteweg_wave_speed(1.5e9, 850.0, 0.590, 0.010, 200e9, 0.91);
        let a_winter = korteweg_wave_speed(1.6e9, 870.0, 0.590, 0.010, 205e9, 0.91);
        assert!(a_winter > a_summer, "Winter wave speed should be higher");
    }

    // ── Original Tests ──────────────────────────────────────────────

    #[test]
    fn test_joukowsky_water() {
        // Water at 998 kg/m³, wave speed 1482 m/s, velocity change 2.5 m/s
        let dp = joukowsky(998.0, 1482.0, 2.5);
        assert!((dp - 3_697_590.0).abs() < 1.0);
    }

    #[test]
    fn test_joukowsky_zero_velocity() {
        assert_eq!(joukowsky(998.0, 1482.0, 0.0), 0.0);
    }

    #[test]
    fn test_darcy_weisbach_basic() {
        // fD=0.02, L=100m, D=0.1m, v=2.0 m/s, g=9.81
        let hf = darcy_weisbach(0.02, 100.0, 0.1, 2.0, 9.81);
        assert!((hf - 4.077).abs() < 0.01);
    }

    #[test]
    fn test_manning_basic() {
        // n=0.03, R=0.5m, S=0.001
        let v = manning(0.03, 0.5, 0.001);
        assert!((v - 0.665).abs() < 0.01);
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Thornhaven Market's aqueduct — the same Thornhaven Market
    /// zone the Audio-lane brick already anchors
    /// (`forge-soundwave-v3::ecology::thornhaven_market_ecology_lore_tie`) —
    /// surges when its stone sluice-gate slams shut. Anchors to the
    /// already-landed `joukowsky` water-hammer formula rather than an
    /// invented surge number. [OBSERVED] fabric: `joukowsky`, already tested
    /// generically above.
    #[test]
    fn thornhaven_aqueduct_sluice_gate_lore_tie() {
        // A stone aqueduct: water density, a real Korteweg-scale wave speed
        // for a rigid stone channel, and a hard 4 m/s flow arrest.
        let aqueduct_water_density = 998.0;
        let stone_channel_wave_speed = 1400.0;
        let flow_arrest_ms = 4.0;
        let surge_pa = joukowsky(aqueduct_water_density, stone_channel_wave_speed, flow_arrest_ms);
        assert!(surge_pa > 0.0, "a slammed sluice gate must produce a real pressure surge");
        assert_eq!(surge_pa, 998.0 * 1400.0 * 4.0, "the aqueduct surge must follow the real Joukowsky formula, not an invented number");
    }
}
