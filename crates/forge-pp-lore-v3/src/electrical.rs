//! Electrical system equations.
//!
//! All stateful RHS (no stateless equations in this category).
//! Phase 3 — these ship after the ODE integrator is built in pp-sim.

/// Arc Flash (#1) — Instantaneous arc power (integrand).
///
/// P_arc = V_arc · I_arc
///
/// The total energy E = ∫P_arc dt is computed by the integrator in pp-sim.
///
/// - `v_arc`: arc voltage (V)
/// - `i_arc`: arc current (A)
///
/// Returns: instantaneous power (W)
pub fn arc_flash_power(v_arc: f64, i_arc: f64) -> f64 {
    v_arc * i_arc
}

/// Generator Overspeed (#2) — Swing equation RHS.
/// Returns (dδ/dt, dω/dt) for the rotor.
///
/// (2H/ωs) · d²δ/dt² = Pm - Pe
/// → dδ/dt = ω - ωs
/// → dω/dt = (ωs / 2H) · (Pm - Pe)
///
/// - `h`: inertia constant (s)
/// - `omega_s`: synchronous speed (rad/s)
/// - `omega`: current rotor speed (rad/s)
/// - `pm`: mechanical power (W or per-unit)
/// - `pe`: electrical power (W or per-unit)
///
/// Returns: (d_delta_dt, d_omega_dt) in (rad/s, rad/s²)
pub fn swing_rhs(h: f64, omega_s: f64, omega: f64, pm: f64, pe: f64) -> (f64, f64) {
    let d_delta_dt = omega - omega_s;
    let d_omega_dt = if h > 0.0 {
        (omega_s / (2.0 * h)) * (pm - pe)
    } else {
        0.0
    };
    (d_delta_dt, d_omega_dt)
}

/// Black Start (#3) — RLC transient RHS.
/// Returns (di/dt, dq/dt) for the circuit.
///
/// L·di/dt + R·i + (1/C)·q = Vm·sin(ωt + θ)
/// → di/dt = [Vm·sin(ωt + θ) - R·i - q/C] / L
/// → dq/dt = i
///
/// - `l`: inductance (H)
/// - `r`: resistance (Ω)
/// - `c`: capacitance (F)
/// - `vm`: peak voltage (V)
/// - `omega`: angular frequency (rad/s)
/// - `theta`: phase angle (rad)
/// - `t`: current time (s)
/// - `i`: current (A)
/// - `q`: charge (C)
///
/// Returns: (di_dt, dq_dt)
pub fn rlc_rhs(
    l: f64,
    r: f64,
    c: f64,
    vm: f64,
    omega: f64,
    theta: f64,
    t: f64,
    i: f64,
    q: f64,
) -> (f64, f64) {
    if l <= 0.0 {
        return (0.0, i);
    }
    let driving = vm * (omega * t + theta).sin();
    let di_dt = (driving - r * i - q / c) / l;
    let dq_dt = i;
    (di_dt, dq_dt)
}

/// Transformer Energization (#4) — Faraday's law RHS.
/// Returns dΦ/dt (rate of change of magnetic flux).
///
/// v(t) = N · dΦ/dt → dΦ/dt = v(t) / N
///
/// - `v`: applied voltage (V)
/// - `n`: number of turns
///
/// Returns: dΦ/dt (Wb/s)
pub fn faraday_rhs(v: f64, n: f64) -> f64 {
    if n <= 0.0 {
        return 0.0;
    }
    v / n
}

/// Fault Ride-Through (#5) — Equal area criterion RHS.
/// Returns the accelerating power at a given rotor angle.
///
/// Pa = Pm - Pe(δ)
///
/// - `pm`: mechanical power (per-unit)
/// - `pe_max`: maximum electrical power (per-unit)
/// - `delta`: rotor angle (rad)
///
/// Returns: accelerating power (per-unit)
pub fn equal_area_rhs(pm: f64, pe_max: f64, delta: f64) -> f64 {
    pm - pe_max * delta.sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_flash_power() {
        assert_eq!(arc_flash_power(1000.0, 50.0), 50_000.0);
    }

    #[test]
    fn test_swing_rhs_balanced() {
        // At synchronous speed with balanced power, no acceleration
        let (dd, dw) = swing_rhs(3.0, 377.0, 377.0, 1.0, 1.0);
        assert_eq!(dd, 0.0);
        assert_eq!(dw, 0.0);
    }

    #[test]
    fn test_rlc_rhs_at_t_zero() {
        let (di, dq) = rlc_rhs(0.1, 10.0, 0.001, 120.0, 377.0, 0.0, 0.0, 0.0, 0.0);
        // At t=0, sin(0)=0, i=0, q=0: di/dt = 0/L = 0
        assert_eq!(di, 0.0);
        assert_eq!(dq, 0.0);
    }

    #[test]
    fn test_equal_area_positive_acceleration() {
        // Pm > Pe at small angle → positive acceleration
        let pa = equal_area_rhs(1.0, 1.5, 0.1);
        assert!(pa > 0.0);
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Celestial Bastion's ward-glyph arc-flash hazard (asset ref
    /// `assets/ironroot/Good/env-backdrop/E07_celestial_bastion.png`). The
    /// Bastion's storm-warding glyphs discharge at a named voltage/current
    /// pair; this test anchors that lore claim to a real published formula
    /// (P = V·I) rather than an invented number. [OBSERVED] fabric:
    /// `arc_flash_power`, already tested generically above.
    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Skyreach Pinnacle's lightning rod — the same summit the
    /// Audio-lane brick anchors at the literal altitude ceiling
    /// (`forge-soundwave-v3::ecology::skyreach_pinnacle_ecology_lore_tie_survives_wire_at_the_ceiling`)
    /// — channels a strike through a real induction coil, not an invented
    /// "it glows" flavour line. Anchors to the already-landed `faraday_rhs`
    /// (v = N·dΦ/dt) rather than a made-up flux number. [OBSERVED] fabric:
    /// `faraday_rhs`, already tested generically above.
    #[test]
    fn skyreach_pinnacle_lightning_rod_lore_tie() {
        // A real lightning-strike voltage across a 50-turn induction coil.
        let strike_voltage_v = 1.0e8; // ~100 MV, within the real range for a strong strike
        let coil_turns = 50.0;
        let dphi_dt = faraday_rhs(strike_voltage_v, coil_turns);
        assert!(dphi_dt > 0.0, "a real lightning strike must induce a nonzero flux rate");
        assert_eq!(dphi_dt, strike_voltage_v / coil_turns, "the coil's induced rate must follow the real Faraday formula, not an invented number");
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Broken Forge — the same forge the Physics-lane brick
    /// already anchors as dealing real Heat damage
    /// (`forge-physics-v3::types::tests::broken_forge_heat_damage_lore_tie`)
    /// — runs a bellows-driven generator that genuinely overspeeds when the
    /// forge's load drops, not an invented "it hums" flavour line. Anchors
    /// to the already-landed `swing_rhs` rather than a made-up acceleration
    /// number. [OBSERVED] fabric: `swing_rhs`, already tested generically
    /// above.
    #[test]
    fn broken_forge_bellows_generator_lore_tie() {
        // Mechanical power (the bellows crank) exceeds electrical load
        // (the forge's own draw drops when idle) — a real overspeed case.
        let inertia_h = 4.0;
        let sync_speed = 377.0;
        let current_speed = 377.0;
        let mechanical_power = 1.2;
        let electrical_load = 0.8;
        let (d_delta_dt, d_omega_dt) = swing_rhs(inertia_h, sync_speed, current_speed, mechanical_power, electrical_load);
        assert_eq!(d_delta_dt, 0.0, "at synchronous speed the angle rate must start at zero");
        assert!(d_omega_dt > 0.0, "the bellows generator must genuinely overspeed when the forge's load drops below the crank's mechanical power");
    }

    #[test]
    fn celestial_bastion_arc_flash_lore_tie() {
        // The Bastion's warding glyphs: a sustained 2kV discharge at 30A —
        // enough to be a real hazard (per catastrophic.rs's own DAMAGE_*
        // hazard-lore precedent), not an arbitrary flavour number.
        let ward_glyph_voltage_v = 2_000.0;
        let ward_glyph_current_a = 30.0;
        let power_w = arc_flash_power(ward_glyph_voltage_v, ward_glyph_current_a);
        assert_eq!(power_w, 60_000.0, "Celestial Bastion ward-glyph arc power failed the real P=V*I formula");
        assert!(power_w > 0.0, "a warding glyph with zero arc power protects nothing");
    }
}
