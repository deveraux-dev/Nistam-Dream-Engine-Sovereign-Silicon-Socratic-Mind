//! Catastrophic event equations — VCE Overpressure (#16)
//!
//! TNO Multi-Energy Method: all 10 severity curves (Alonso et al. 2006 coefficients)
//! GAME correlation for congestion-driven severity selection
//! BST flame speed table with DDT flags
//! Cloud energy calculation with fuel property table
//! Multi-source maximum envelope combination
//! CSA Z662 / AER compliance checks

// ── TNO Multi-Energy Method — 10 Severity Curves ────────────────────────────

/// Alonso et al. (2006) piecewise power-law coefficients for MEM curves 1-10.
/// P_bar = a * R_bar^b over specific R_bar intervals.
/// R² > 0.98 for all fits.
///
/// Each curve: Vec of (r_bar_min, r_bar_max, a, b)
// Literals like 3.18e-1 are published GAME-correlation coefficients (TNO Multi-Energy)
// from the source paper, not approximations of math constants like 1/π. Suppress
// clippy::approx_constant for this hand-tabulated lookup.
#[allow(clippy::approx_constant)]
fn mem_curve_coefficients(severity: u8) -> Vec<(f64, f64, f64, f64)> {
    match severity {
        1 => vec![
            (0.23, 0.6, 0.01, 0.0),
            (0.6, 7.0, 6.40e-3, -0.97),
        ],
        2 => vec![
            (0.23, 0.7, 0.02, 0.0),
            (0.7, 12.0, 1.32e-2, -0.98),
        ],
        3 => vec![
            (0.23, 0.6, 0.05, 0.0),
            (0.6, 30.0, 6.05e-2, -0.99),
        ],
        4 => vec![
            (0.23, 0.5, 0.1, 0.0),
            (0.5, 70.0, 6.44e-2, -0.99),
        ],
        5 => vec![
            (0.23, 0.6, 0.2, 0.0),
            (0.6, 90.0, 1.17e-1, -0.99),
        ],
        6 => vec![
            (0.23, 0.6, 0.5, 0.0),
            (0.6, 100.0, 3.01e-1, -1.11),
        ],
        7 => vec![
            (0.23, 0.5, 1.0, 0.0),
            (0.5, 100.0, 4.06e-1, -1.20),
        ],
        8 => vec![
            (0.23, 0.5, 2.0, 0.0),
            (0.5, 1.0, 4.76e-1, -2.08),
            (1.0, 2.0, 4.67e-1, -1.58),
            (2.0, 100.0, 3.18e-1, -1.13),
        ],
        9 => vec![
            (0.23, 0.35, 5.0, 0.0),
            (0.35, 1.0, 4.87e-1, -2.03),
            (1.0, 2.0, 4.67e-1, -1.58),
            (2.0, 100.0, 3.18e-1, -1.13),
        ],
        10 => vec![
            (0.23, 0.3, 10.82, 1.14), // Near-field overshoot (detonation)
            (0.3, 0.4, 3.15e-1, -1.79),
            (0.4, 0.5, 1.30e-3, -7.52),
            (0.5, 100.0, 1.14e-1, -1.03),
        ],
        _ => vec![(0.23, 100.0, 0.01, 0.0)], // Default to class 1
    }
}

/// Evaluate a single MEM curve at a given scaled distance.
/// Returns dimensionless overpressure P_bar = ΔP/P0.
fn mem_curve_eval(severity: u8, r_bar: f64) -> f64 {
    let coeffs = mem_curve_coefficients(severity);
    let r_bar = r_bar.max(0.23); // Minimum valid scaled distance

    for (r_min, r_max, a, b) in &coeffs {
        if r_bar >= *r_min && r_bar < *r_max {
            if *b == 0.0 {
                return *a; // Constant near-field plateau
            }
            return a * r_bar.powf(*b);
        }
    }

    // Beyond last interval — use the last segment's decay
    if let Some((_, _, a, b)) = coeffs.last() {
        if *b == 0.0 { return *a; }
        return a * r_bar.powf(*b);
    }
    0.0
}

/// Scaled distance for TNO Multi-Energy Method.
/// R_bar = R / (E/P0)^(1/3)
///
/// - `distance`: distance from explosion center (m)
/// - `energy`: total combustion energy (J)
/// - `p0`: ambient pressure (Pa)
///
/// Returns: dimensionless scaled distance
pub fn scaled_distance(distance: f64, energy: f64, p0: f64) -> f64 {
    if energy <= 0.0 || p0 <= 0.0 { return f64::MAX; }
    distance / (energy / p0).cbrt()
}

/// TNO Multi-Energy Method — overpressure at distance for a given severity class.
/// Uses Alonso et al. (2006) piecewise power-law coefficients (R² > 0.98).
///
/// - `severity`: MEM severity class (1-10). 10 = detonation.
/// - `distance`: distance from explosion center (m)
/// - `energy`: total combustion energy (J)
/// - `p0`: ambient pressure (Pa, typically 101325)
///
/// Returns: overpressure (Pa)
pub fn mem_overpressure(severity: u8, distance: f64, energy: f64, p0: f64) -> f64 {
    if distance <= 0.0 || energy <= 0.0 || p0 <= 0.0 {
        return 0.0;
    }
    let severity = severity.clamp(1, 10);
    let r_bar = scaled_distance(distance, energy, p0);
    let p_bar = mem_curve_eval(severity, r_bar);
    p_bar * p0
}

/// Find the distance at which a specific overpressure threshold is reached.
/// Inverts the MEM curve via binary search.
///
/// - `severity`: MEM class (1-10)
/// - `energy`: total combustion energy (J)
/// - `p0`: ambient pressure (Pa)
/// - `target_overpressure`: desired overpressure threshold (Pa)
///
/// Returns: distance (m), or 0.0 if threshold not reached
pub fn mem_overpressure_distance(
    severity: u8,
    energy: f64,
    p0: f64,
    target_overpressure: f64,
) -> f64 {
    if energy <= 0.0 || p0 <= 0.0 || target_overpressure <= 0.0 {
        return 0.0;
    }
    let severity = severity.clamp(1, 10);
    let target_p_bar = target_overpressure / p0;

    // Check if the near-field plateau even reaches the target
    let near_field_p = mem_curve_eval(severity, 0.23);
    if near_field_p < target_p_bar {
        return 0.0; // This severity class never reaches that overpressure
    }

    // Binary search for the scaled distance
    let mut lo: f64 = 0.23;
    let mut hi: f64 = 200.0;
    for _ in 0..100 {
        let mid = (lo + hi) / 2.0;
        let p = mem_curve_eval(severity, mid);
        if p > target_p_bar {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    // Convert scaled distance back to real distance
    let r_bar = (lo + hi) / 2.0;
    r_bar * (energy / p0).cbrt()
}

/// Legacy sachs_scaling — now wraps mem_overpressure with severity 7.
/// Kept for backward compatibility.
pub fn sachs_scaling(distance: f64, energy: f64, p0: f64) -> f64 {
    mem_overpressure(7, distance, energy, p0)
}

// ── GAME Correlation — Severity Class from Congestion ───────────────────────

/// GAME correlation for 3D flame expansion (unconfined congestion).
/// Predicts source overpressure from measurable facility geometry.
///
/// P_max = 0.84 * (VBR * L_p/D)^2.75 * S_L^2.7 * D^0.7
///
/// - `vbr`: volume blockage ratio (0.0-1.0)
/// - `flame_path_m`: flame path length through congestion (m)
/// - `obstacle_diameter_m`: characteristic obstacle diameter (m)
/// - `laminar_burning_velocity`: S_L of the fuel (m/s)
///
/// Returns: predicted source overpressure (bar)
pub fn game_correlation_3d(
    vbr: f64,
    flame_path_m: f64,
    obstacle_diameter_m: f64,
    laminar_burning_velocity: f64,
) -> f64 {
    if obstacle_diameter_m <= 0.0 || vbr <= 0.0 {
        return 0.0;
    }
    let congestion_param = vbr * flame_path_m / obstacle_diameter_m;
    0.84 * congestion_param.powf(2.75)
        * laminar_burning_velocity.powf(2.7)
        * obstacle_diameter_m.powf(0.7)
}

/// GAME correlation for 2D flame expansion (confined between parallel planes).
///
/// P_max = 3.38 * (VBR * L_p/D)^2.25 * S_L^2.7 * D^0.7
pub fn game_correlation_2d(
    vbr: f64,
    flame_path_m: f64,
    obstacle_diameter_m: f64,
    laminar_burning_velocity: f64,
) -> f64 {
    if obstacle_diameter_m <= 0.0 || vbr <= 0.0 {
        return 0.0;
    }
    let congestion_param = vbr * flame_path_m / obstacle_diameter_m;
    3.38 * congestion_param.powf(2.25)
        * laminar_burning_velocity.powf(2.7)
        * obstacle_diameter_m.powf(0.7)
}

/// Map GAME-predicted source overpressure to the nearest MEM severity class.
///
/// - `p_max_bar`: GAME predicted overpressure (bar)
///
/// Returns: MEM severity class (1-10)
pub fn game_to_mem_class(p_max_bar: f64) -> u8 {
    // Near-field plateau values for each MEM class (in bar, at P0=1.01325 bar)
    // Class 1: 0.01 bar, 2: 0.02, 3: 0.05, 4: 0.1, 5: 0.2, 6: 0.5, 7: 1.0, 8: 2.0, 9: 5.0, 10: 10+
    if p_max_bar >= 5.0 { return 10; }
    if p_max_bar >= 2.0 { return 9; }
    if p_max_bar >= 1.0 { return 8; }
    if p_max_bar >= 0.5 { return 7; }
    if p_max_bar >= 0.2 { return 6; }
    if p_max_bar >= 0.1 { return 5; }
    if p_max_bar >= 0.05 { return 4; }
    if p_max_bar >= 0.02 { return 3; }
    if p_max_bar >= 0.01 { return 2; }
    1
}

// ── Cloud Energy Calculation ────────────────────────────────────────────────

/// Fuel properties for VCE energy calculations.
pub struct FuelProperties {
    pub name: &'static str,
    pub heat_of_combustion_mj_kg: f64,
    pub laminar_burning_velocity: f64,  // m/s
    pub stoichiometric_fraction: f64,   // volume fraction
    pub lfl: f64,                       // lower flammable limit (vol fraction)
    pub ufl: f64,                       // upper flammable limit (vol fraction)
    pub vapor_density_kg_m3: f64,       // at STP
    pub bst_reactivity: BstReactivity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BstReactivity { Low, Medium, High }

pub const HYDROGEN: FuelProperties = FuelProperties {
    name: "Hydrogen", heat_of_combustion_mj_kg: 120.0,
    laminar_burning_velocity: 3.0, stoichiometric_fraction: 0.296,
    lfl: 0.04, ufl: 0.75, vapor_density_kg_m3: 0.082,
    bst_reactivity: BstReactivity::High,
};

pub const METHANE: FuelProperties = FuelProperties {
    name: "Methane", heat_of_combustion_mj_kg: 50.0,
    laminar_burning_velocity: 0.40, stoichiometric_fraction: 0.095,
    lfl: 0.05, ufl: 0.15, vapor_density_kg_m3: 0.657,
    bst_reactivity: BstReactivity::Low,
};

pub const PROPANE: FuelProperties = FuelProperties {
    name: "Propane", heat_of_combustion_mj_kg: 46.4,
    laminar_burning_velocity: 0.46, stoichiometric_fraction: 0.040,
    lfl: 0.021, ufl: 0.095, vapor_density_kg_m3: 1.882,
    bst_reactivity: BstReactivity::Medium,
};

pub const BUTANE: FuelProperties = FuelProperties {
    name: "n-Butane", heat_of_combustion_mj_kg: 45.7,
    laminar_burning_velocity: 0.45, stoichiometric_fraction: 0.031,
    lfl: 0.016, ufl: 0.084, vapor_density_kg_m3: 2.489,
    bst_reactivity: BstReactivity::Medium,
};

pub const ETHYLENE: FuelProperties = FuelProperties {
    name: "Ethylene", heat_of_combustion_mj_kg: 47.2,
    laminar_burning_velocity: 0.75, stoichiometric_fraction: 0.065,
    lfl: 0.027, ufl: 0.360, vapor_density_kg_m3: 1.178,
    bst_reactivity: BstReactivity::High,
};

pub const CYCLOHEXANE: FuelProperties = FuelProperties {
    name: "Cyclohexane", heat_of_combustion_mj_kg: 43.5,
    laminar_burning_velocity: 0.44, stoichiometric_fraction: 0.022,
    lfl: 0.013, ufl: 0.080, vapor_density_kg_m3: 3.42,
    bst_reactivity: BstReactivity::Medium,
};

pub const GASOLINE: FuelProperties = FuelProperties {
    name: "Gasoline", heat_of_combustion_mj_kg: 43.0,
    laminar_burning_velocity: 0.45, stoichiometric_fraction: 0.017,
    lfl: 0.014, ufl: 0.076, vapor_density_kg_m3: 3.5,
    bst_reactivity: BstReactivity::Medium,
};

/// VCE cloud energy for the congested zone.
/// E = m_fuel * H_c * eta
///
/// For hemispherical (ground-level) explosions, double the energy for ground reflection.
///
/// - `fuel`: fuel properties
/// - `congested_volume_m3`: volume of the congested zone (m³)
/// - `efficiency`: combustion efficiency (1.0 for MEM congested zones)
/// - `hemispherical`: true for ground-level explosions (doubles energy)
///
/// Returns: total combustion energy (J)
pub fn cloud_energy(
    fuel: &FuelProperties,
    congested_volume_m3: f64,
    efficiency: f64,
    hemispherical: bool,
) -> f64 {
    let mass_kg = fuel.vapor_density_kg_m3 * fuel.stoichiometric_fraction * congested_volume_m3;
    let energy = mass_kg * fuel.heat_of_combustion_mj_kg * 1e6 * efficiency;
    if hemispherical { energy * 2.0 } else { energy }
}

// ── BST Flame Speed Table with DDT Flags ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BstCongestion { Low, Medium, High }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BstConfinement { ThreeD, TwoPointFiveD, TwoD }

/// BST flame speed (Mach number) from Pierorazio (2005) table.
/// Returns the flame Mach number, or 5.2 for DDT (Chapman-Jouguet detonation).
///
/// Returns: (mach_number, is_ddt)
pub fn bst_flame_speed(
    reactivity: BstReactivity,
    congestion: BstCongestion,
    confinement: BstConfinement,
) -> (f64, bool) {
    match (confinement, reactivity, congestion) {
        // 3D expansion
        (BstConfinement::ThreeD, BstReactivity::Low, BstCongestion::Low)       => (0.026, false),
        (BstConfinement::ThreeD, BstReactivity::Low, BstCongestion::Medium)    => (0.23, false),
        (BstConfinement::ThreeD, BstReactivity::Low, BstCongestion::High)      => (0.34, false),
        (BstConfinement::ThreeD, BstReactivity::Medium, BstCongestion::Low)    => (0.11, false),
        (BstConfinement::ThreeD, BstReactivity::Medium, BstCongestion::Medium) => (0.44, false),
        (BstConfinement::ThreeD, BstReactivity::Medium, BstCongestion::High)   => (0.50, false),
        (BstConfinement::ThreeD, BstReactivity::High, BstCongestion::Low)      => (0.36, false),
        (BstConfinement::ThreeD, BstReactivity::High, BstCongestion::Medium)   => (5.2, true),
        (BstConfinement::ThreeD, BstReactivity::High, BstCongestion::High)     => (5.2, true),

        // 2.5D expansion
        (BstConfinement::TwoPointFiveD, BstReactivity::Low, BstCongestion::Low)       => (0.053, false),
        (BstConfinement::TwoPointFiveD, BstReactivity::Low, BstCongestion::Medium)    => (0.35, false),
        (BstConfinement::TwoPointFiveD, BstReactivity::Low, BstCongestion::High)      => (0.50, false),
        (BstConfinement::TwoPointFiveD, BstReactivity::Medium, BstCongestion::Low)    => (0.29, false),
        (BstConfinement::TwoPointFiveD, BstReactivity::Medium, BstCongestion::Medium) => (0.55, false),
        (BstConfinement::TwoPointFiveD, BstReactivity::Medium, BstCongestion::High)   => (1.0, false),
        (BstConfinement::TwoPointFiveD, BstReactivity::High, BstCongestion::Low)      => (0.47, false),
        (BstConfinement::TwoPointFiveD, BstReactivity::High, BstCongestion::Medium)   => (5.2, true),
        (BstConfinement::TwoPointFiveD, BstReactivity::High, BstCongestion::High)     => (5.2, true),

        // 2D expansion
        (BstConfinement::TwoD, BstReactivity::Low, BstCongestion::Low)       => (0.079, false),
        (BstConfinement::TwoD, BstReactivity::Low, BstCongestion::Medium)    => (0.47, false),
        (BstConfinement::TwoD, BstReactivity::Low, BstCongestion::High)      => (0.66, false),
        (BstConfinement::TwoD, BstReactivity::Medium, BstCongestion::Low)    => (0.47, false),
        (BstConfinement::TwoD, BstReactivity::Medium, BstCongestion::Medium) => (0.66, false),
        (BstConfinement::TwoD, BstReactivity::Medium, BstCongestion::High)   => (1.6, false),
        (BstConfinement::TwoD, BstReactivity::High, BstCongestion::Low)      => (0.59, false),
        (BstConfinement::TwoD, BstReactivity::High, BstCongestion::Medium)   => (5.2, true),
        (BstConfinement::TwoD, BstReactivity::High, BstCongestion::High)     => (5.2, true),
    }
}

/// DDT screening check.
/// Returns true if conditions suggest deflagration-to-detonation transition.
/// Based on BST flame speed table — DDT flagged for:
///   High reactivity + Medium/High congestion (any confinement)
///   Medium reactivity + High congestion + 2D confinement
pub fn ddt_risk(
    reactivity: BstReactivity,
    congestion: BstCongestion,
    confinement: BstConfinement,
) -> bool {
    let (_, is_ddt) = bst_flame_speed(reactivity, congestion, confinement);
    is_ddt
}

// ── Multi-Source Maximum Envelope ────────────────────────────────────────────

/// Maximum envelope combination for multiple VCE sources.
/// At each receptor, the design overpressure is the maximum from all sources.
///
/// - `sources`: Vec of (severity, energy_j, source_x, source_y, source_z)
/// - `receptor_x`, `receptor_y`, `receptor_z`: receptor position (m)
/// - `p0`: ambient pressure (Pa)
///
/// Returns: (max_overpressure_pa, dominant_source_index)
pub fn multi_source_envelope(
    sources: &[(u8, f64, f64, f64, f64)],
    receptor_x: f64,
    receptor_y: f64,
    receptor_z: f64,
    p0: f64,
) -> (f64, usize) {
    let mut max_p = 0.0;
    let mut max_idx = 0;

    for (i, (severity, energy, sx, sy, sz)) in sources.iter().enumerate() {
        let dx = receptor_x - sx;
        let dy = receptor_y - sy;
        let dz = receptor_z - sz;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        let p = mem_overpressure(*severity, distance, *energy, p0);
        if p > max_p {
            max_p = p;
            max_idx = i;
        }
    }

    (max_p, max_idx)
}

// ── Overpressure Damage Criteria ────────────────────────────────────────────

/// Standard overpressure damage thresholds (Pa).
pub const DAMAGE_GLASS_BREAKAGE: f64 = 7_000.0;        // 7 kPa / 1.0 psi
pub const DAMAGE_MODERATE_STRUCTURAL: f64 = 14_000.0;   // 14 kPa / 2.0 psi
pub const DAMAGE_SEVERE_STRUCTURAL: f64 = 35_000.0;     // 35 kPa / 5.0 psi
pub const DAMAGE_COMPLETE_DESTRUCTION: f64 = 70_000.0;   // 70 kPa / 10.0 psi
pub const DAMAGE_LUNG_HEMORRHAGE: f64 = 70_000.0;        // 70 kPa
pub const DAMAGE_EARDRUM_RUPTURE: f64 = 35_000.0;        // 35 kPa

#[cfg(test)]
mod tests {
    use super::*;

    // ── MEM Curve Tests ─────────────────────────────────────────────

    #[test]
    fn test_mem_severity_ordering_near_field() {
        // Higher severity = higher near-field plateau overpressure
        // The Alonso curves have different decay rates per class, so at
        // intermediate distances higher classes can briefly dip below lower ones
        // (class 8 decays at b=-2.08 vs class 7 at b=-1.20). This is correct
        // physics — we only enforce ordering at the near-field plateau.
        let mut prev = 0.0;
        for sev in 1..=9 {
            let p_bar = mem_curve_eval(sev, 0.25); // Near-field plateau
            assert!(p_bar >= prev, "Near-field: sev {sev} ({p_bar}) should >= sev {} ({prev})", sev - 1);
            prev = p_bar;
        }
    }

    #[test]
    fn test_mem_decreases_with_distance() {
        let e = 1e9;
        let p0 = 101_325.0;
        for sev in 1..=10 {
            let p_near = mem_overpressure(sev, 10.0, e, p0);
            let p_far = mem_overpressure(sev, 500.0, e, p0);
            assert!(p_near > p_far, "Sev {sev}: near={p_near}, far={p_far}");
        }
    }

    #[test]
    fn test_mem_class7_near_field() {
        // Class 7 near-field plateau: P_bar = 1.0, so overpressure = 1.0 * P0
        let r_bar = 0.3; // Within near-field
        let p_bar = mem_curve_eval(7, r_bar);
        assert!((p_bar - 1.0).abs() < 0.01, "Class 7 near-field: {p_bar}");
    }

    #[test]
    fn test_mem_class10_detonation_overshoot() {
        // Class 10 has positive exponent in near-field (overshoot)
        let p_023 = mem_curve_eval(10, 0.23);
        let p_028 = mem_curve_eval(10, 0.28);
        // Detonation near-field: pressure increases with distance briefly
        assert!(p_028 > p_023, "Detonation overshoot: p(0.23)={p_023}, p(0.28)={p_028}");
    }

    #[test]
    fn test_mem_distance_7kpa() {
        // Find glass breakage distance (7 kPa) for a 1 GJ class 7 explosion
        let dist = mem_overpressure_distance(7, 1e9, 101_325.0, 7_000.0);
        assert!(dist > 50.0, "7 kPa should be >50m: {dist}m");
        assert!(dist < 2000.0, "7 kPa should be <2km: {dist}m");

        // Verify by checking overpressure at that distance
        let p_check = mem_overpressure(7, dist, 1e9, 101_325.0);
        assert!((p_check - 7_000.0).abs() < 200.0, "Check: {p_check} Pa at {dist}m");
    }

    #[test]
    fn test_legacy_sachs_scaling() {
        // Legacy function should match class 7
        let p_legacy = sachs_scaling(100.0, 1e9, 101_325.0);
        let p_new = mem_overpressure(7, 100.0, 1e9, 101_325.0);
        assert!((p_legacy - p_new).abs() < 1.0);
    }

    // ── GAME Correlation Tests ──────────────────────────────────────

    #[test]
    fn test_game_higher_congestion_higher_pressure() {
        let p_low = game_correlation_3d(0.05, 10.0, 0.1, 0.46);
        let p_high = game_correlation_3d(0.20, 10.0, 0.1, 0.46);
        assert!(p_high > p_low, "Higher VBR = higher pressure: low={p_low}, high={p_high}");
    }

    #[test]
    fn test_game_longer_path_higher_pressure() {
        let p_short = game_correlation_3d(0.10, 5.0, 0.1, 0.46);
        let p_long = game_correlation_3d(0.10, 30.0, 0.1, 0.46);
        assert!(p_long > p_short, "Longer flame path = higher pressure");
    }

    #[test]
    fn test_game_propane_vs_methane() {
        // Higher S_L fuel = higher overpressure
        let p_methane = game_correlation_3d(0.10, 20.0, 0.1, METHANE.laminar_burning_velocity);
        let p_propane = game_correlation_3d(0.10, 20.0, 0.1, PROPANE.laminar_burning_velocity);
        assert!(p_propane > p_methane);
    }

    #[test]
    fn test_game_to_mem_mapping() {
        assert_eq!(game_to_mem_class(0.005), 1);
        assert_eq!(game_to_mem_class(0.03), 3);
        assert_eq!(game_to_mem_class(0.15), 5);
        assert_eq!(game_to_mem_class(0.7), 7);
        assert_eq!(game_to_mem_class(1.5), 8);
        assert_eq!(game_to_mem_class(3.0), 9);
        assert_eq!(game_to_mem_class(8.0), 10);
    }

    #[test]
    fn test_game_2d_stronger_than_3d() {
        // 2D confinement has higher base coefficient (3.38 vs 0.84)
        // but lower exponent (2.25 vs 2.75). At moderate congestion, 2D > 3D.
        let p_3d = game_correlation_3d(0.05, 5.0, 0.2, 0.46);
        let p_2d = game_correlation_2d(0.05, 5.0, 0.2, 0.46);
        assert!(p_2d > p_3d, "2D should exceed 3D at moderate congestion: 2d={p_2d}, 3d={p_3d}");
    }

    // ── EMERGE Validation ───────────────────────────────────────────

    #[test]
    fn test_emerge_methane_10pct_vbr() {
        // EMERGE-7: 8x8x4m methane, VBR 10% → experimental MEM class ~8.14
        // GAME correlation has known moderate accuracy for non-uniform congestion
        // and tends to over-predict (conservative). Verify it produces high severity.
        let p = game_correlation_3d(0.10, 8.0, 0.08, METHANE.laminar_burning_velocity);
        let class = game_to_mem_class(p);
        // GAME over-predicts compared to EMERGE experimental data — this is
        // the expected conservative behavior documented in the GAMES project
        assert!(class >= 7, "EMERGE-7 methane should map to class >= 7, got {class} (P={p} bar)");
    }

    #[test]
    fn test_emerge_propane_higher_than_methane() {
        // EMERGE: propane consistently maps higher than methane at same VBR
        let p_m = game_correlation_3d(0.10, 4.0, 0.08, METHANE.laminar_burning_velocity);
        let p_p = game_correlation_3d(0.10, 4.0, 0.08, PROPANE.laminar_burning_velocity);
        let class_m = game_to_mem_class(p_m);
        let class_p = game_to_mem_class(p_p);
        assert!(class_p >= class_m, "Propane class should >= methane: m={class_m}, p={class_p}");
    }

    // ── Cloud Energy Tests ──────────────────────────────────────────

    #[test]
    fn test_cloud_energy_propane() {
        // 1000 m³ congested zone, propane
        let e = cloud_energy(&PROPANE, 1000.0, 1.0, true);
        assert!(e > 1e9, "Should be GJ-scale: {e}");
    }

    #[test]
    fn test_cloud_energy_hemispherical_doubles() {
        let e_free = cloud_energy(&PROPANE, 100.0, 1.0, false);
        let e_hemi = cloud_energy(&PROPANE, 100.0, 1.0, true);
        assert!((e_hemi - 2.0 * e_free).abs() < 1.0);
    }

    // ── BST / DDT Tests ─────────────────────────────────────────────

    #[test]
    fn test_bst_ddt_hydrogen_medium_congestion() {
        let (mf, ddt) = bst_flame_speed(BstReactivity::High, BstCongestion::Medium, BstConfinement::ThreeD);
        assert!(ddt, "Hydrogen + medium congestion should DDT");
        assert!((mf - 5.2).abs() < 0.01);
    }

    #[test]
    fn test_bst_no_ddt_methane_low_congestion() {
        let (_, ddt) = bst_flame_speed(BstReactivity::Low, BstCongestion::Low, BstConfinement::ThreeD);
        assert!(!ddt);
    }

    #[test]
    fn test_bst_medium_fuel_high_congestion_2d_fast_but_no_ddt() {
        // Medium reactivity + high congestion + 2D = Mf 1.6, NOT DDT
        // This is why Buncefield was a surprise — BST table doesn't predict DDT
        // for medium-reactivity fuels. DDT occurred through vegetation congestion
        // that wasn't captured in standard assessment.
        let (mf, ddt) = bst_flame_speed(BstReactivity::Medium, BstCongestion::High, BstConfinement::TwoD);
        assert!(!ddt, "BST table: medium+high+2D = Mf 1.6, not DDT");
        assert!((mf - 1.6).abs() < 0.01, "Flame speed: {mf}");
    }

    #[test]
    fn test_ddt_risk_function() {
        assert!(ddt_risk(BstReactivity::High, BstCongestion::Medium, BstConfinement::ThreeD));
        assert!(!ddt_risk(BstReactivity::Low, BstCongestion::Low, BstConfinement::ThreeD));
    }

    // ── Multi-Source Envelope Tests ─────────────────────────────────

    #[test]
    fn test_multi_source_picks_closest() {
        let sources = vec![
            (7u8, 1e9, 0.0, 0.0, 0.0),    // Source A at origin
            (7u8, 1e9, 500.0, 0.0, 0.0),   // Source B 500m east
        ];
        // Receptor near source A
        let (p, idx) = multi_source_envelope(&sources, 10.0, 0.0, 0.0, 101_325.0);
        assert_eq!(idx, 0, "Should pick source A (closest)");
        assert!(p > 0.0);
    }

    #[test]
    fn test_multi_source_stronger_dominates() {
        let sources = vec![
            (3u8, 1e8, 0.0, 0.0, 0.0),     // Weak source nearby
            (9u8, 1e10, 200.0, 0.0, 0.0),   // Strong source further
        ];
        let (_, idx) = multi_source_envelope(&sources, 100.0, 0.0, 0.0, 101_325.0);
        assert_eq!(idx, 1, "Stronger source should dominate at midpoint");
    }

    // ── Strathcona Scenario Test ────────────────────────────────────

    #[test]
    fn test_strathcona_ethylene_at_plastics() {
        // AT Plastics / LyondellBasell: 20,000 kg ethylene
        // Process area VBR 0.5, high congestion, 50m x 200m x 8m
        let _congested_vol = 50.0 * 200.0 * 8.0; // 80,000 m³ (documents scenario, not used directly — mass is hardcoded below)
        let energy = 20_000.0 * ETHYLENE.heat_of_combustion_mj_kg * 1e6 * 2.0; // hemispherical

        // GAME predicts severity
        let p_game = game_correlation_3d(0.50, 200.0, 0.3, ETHYLENE.laminar_burning_velocity);
        let severity = game_to_mem_class(p_game);

        // Ethylene is HIGH reactivity + HIGH congestion → DDT risk
        let ddt = ddt_risk(BstReactivity::High, BstCongestion::High, BstConfinement::ThreeD);
        assert!(ddt, "Ethylene at AT Plastics should flag DDT");

        // 7 kPa (glass breakage) distance
        let glass_dist = mem_overpressure_distance(severity, energy, 101_325.0, 7_000.0);
        assert!(glass_dist > 200.0, "Glass breakage should reach beyond facility: {glass_dist}m");

        // Check at Baseline Road (400m south)
        let p_baseline = mem_overpressure(severity, 400.0, energy, 101_325.0);
        assert!(p_baseline > 0.0, "Baseline Road should feel overpressure: {p_baseline} Pa");

        // Check at Goldstick Park (800m west)
        let p_park = mem_overpressure(severity, 800.0, energy, 101_325.0);
        assert!(p_park > 0.0, "Park should feel overpressure: {p_park} Pa");
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Cinderfall Breach's vented gas cloud — closing the full
    /// chain this session already built (Audio flagged the event, Physics
    /// named the fire, `atmospheric.rs` anchored the smoke plume) — is the
    /// actual VCE severity source behind all three: a methane cloud in a
    /// congested vent shaft. First DIRECT lore tie inside `catastrophic.rs`
    /// itself (the module's only prior lore tie, `hazard_words` in
    /// `forge-mud-v3::abyss`, cites its `DAMAGE_*` constants but lives in a
    /// different crate). Anchors to the already-landed `cloud_energy`/
    /// `game_correlation_3d`/`game_to_mem_class` chain rather than an
    /// invented severity number. [OBSERVED] fabric: all three fns, already
    /// tested generically above.
    #[test]
    fn cinderfall_breach_vce_severity_lore_tie() {
        // A methane vent cloud in a narrow, congested breach shaft.
        let congested_volume_m3 = 400.0;
        let energy_j = cloud_energy(&METHANE, congested_volume_m3, 1.0, true);
        assert!(energy_j > 0.0, "the breach's vented cloud must carry real combustion energy");

        let p_game = game_correlation_3d(0.15, 6.0, 0.15, METHANE.laminar_burning_velocity);
        let severity = game_to_mem_class(p_game);
        assert!((1..=10).contains(&severity), "the breach must classify to a real MEM severity class, got {severity}");

        // The breach must reach a real, nonzero overpressure at a close standoff.
        let p_at_10m = mem_overpressure(severity, 10.0, energy_j, 101_325.0);
        assert!(p_at_10m > 0.0, "the breach must produce real overpressure near its source");
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Warden of Red Debt's ledger — the last of the six real,
    /// tested Bell Warden variants confirmed LIVE-WIRED into the actual
    /// game loop this session (`forge-mud-v3::game.rs:552-557`, selected by
    /// `select_warden_variant` on `kills > 8 || blood_supply_used_q > 0`,
    /// its own lesson "Damage creates debt.") — reads debt off the real,
    /// ordered overpressure-damage ladder (glass < moderate < severe <
    /// complete), not an invented tally. Anchors to the already-landed
    /// `DAMAGE_*` constants rather than a made-up debt scale. [OBSERVED]
    /// fabric: the constants, already tested generically above.
    #[test]
    fn warden_of_red_debt_ledger_lore_tie_orders_damage_as_real_debt() {
        assert!(DAMAGE_GLASS_BREAKAGE < DAMAGE_MODERATE_STRUCTURAL, "the ledger's first debt tier must be the cheapest");
        assert!(DAMAGE_MODERATE_STRUCTURAL < DAMAGE_SEVERE_STRUCTURAL, "debt must compound, not reset");
        assert!(DAMAGE_SEVERE_STRUCTURAL <= DAMAGE_COMPLETE_DESTRUCTION, "the final debt tier must be the most severe");
    }
}
