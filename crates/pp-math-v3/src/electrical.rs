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
}
