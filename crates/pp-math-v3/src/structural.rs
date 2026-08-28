//! Structural mechanics equations.
//!
//! Stateful RHS: damped_harmonic_rhs (#15), fea_dynamic_rhs (#20)
//! Phase 3 — these ship after the ODE integrator and matrix solvers are built.

/// Railcar Impact (#15) — Damped harmonic oscillator RHS.
/// Returns (dx/dt, dv/dt) for a mass-spring-damper system.
///
/// mẍ + cẋ + kx = F_impact
/// → dx/dt = v
/// → dv/dt = (F - c·v - k·x) / m
///
/// - `m`: mass (kg)
/// - `c`: damping coefficient (N·s/m)
/// - `k`: spring stiffness (N/m)
/// - `f`: applied force (N)
/// - `x`: displacement (m)
/// - `v`: velocity (m/s)
///
/// Returns: (dx_dt, dv_dt)
pub fn damped_harmonic_rhs(m: f64, c: f64, k: f64, f: f64, x: f64, v: f64) -> (f64, f64) {
    if m <= 0.0 {
        return (0.0, 0.0);
    }
    let dx_dt = v;
    let dv_dt = (f - c * v - k * x) / m;
    (dx_dt, dv_dt)
}

/// Seismic Piping (#20) — FEA dynamic matrix equation RHS.
/// Returns acceleration vector {ẍ} for a multi-DOF system.
///
/// \[M\]{ẍ} + \[C\]{ẋ} + \[K\]{x} = {F(t)}
/// → {ẍ} = \[M\]⁻¹ · ({F} - \[C\]{ẋ} - \[K\]{x})
///
/// For the full matrix version, pp-sim's integrator handles the nalgebra
/// matrix operations. This function computes the RHS for a single DOF
/// as a simplified entry point.
///
/// - `m`: mass at this DOF (kg)
/// - `c`: damping at this DOF (N·s/m)
/// - `k`: stiffness at this DOF (N/m)
/// - `f`: applied force at this DOF (N)
/// - `x`: displacement at this DOF (m)
/// - `v`: velocity at this DOF (m/s)
///
/// Returns: acceleration (m/s²)
pub fn fea_single_dof_accel(m: f64, c: f64, k: f64, f: f64, x: f64, v: f64) -> f64 {
    if m <= 0.0 {
        return 0.0;
    }
    (f - c * v - k * x) / m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damped_harmonic_free_vibration() {
        // No force, displaced from equilibrium: should accelerate back
        let (dx, dv) = damped_harmonic_rhs(10.0, 0.0, 100.0, 0.0, 1.0, 0.0);
        assert_eq!(dx, 0.0);        // not moving yet
        assert!((dv - (-10.0)).abs() < 0.01); // k*x/m = 100*1/10 = 10, restoring
    }

    #[test]
    fn test_damped_harmonic_with_damping() {
        // Moving with damping: should decelerate
        let (_, dv) = damped_harmonic_rhs(10.0, 5.0, 100.0, 0.0, 0.0, 2.0);
        // dv = (0 - 5*2 - 100*0) / 10 = -1.0
        assert!((dv - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn test_fea_single_dof_static() {
        // Static equilibrium: F = k*x, no velocity → zero acceleration
        let a = fea_single_dof_accel(10.0, 0.0, 100.0, 100.0, 1.0, 0.0);
        assert!(a.abs() < 0.01);
    }
}
