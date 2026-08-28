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

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Silent Warden's Gate — one of the six real, tested Bell
    /// Warden variants confirmed LIVE-WIRED into the actual game loop this
    /// session (`forge-mud-v3::game.rs:552-557`, selected by
    /// `select_warden_variant` on `commands_refused > 5`, its own lesson
    /// "Some systems cannot start unless you accuse or consent.") — holds
    /// perfectly still under a balanced force and only moves once that
    /// balance breaks, the physical shape of "cannot start unless." Anchors
    /// to the already-landed `fea_single_dof_accel` rather than an invented
    /// "the gate won't budge" flavour line. [OBSERVED] fabric:
    /// `fea_single_dof_accel`, already tested generically above
    /// (`test_fea_single_dof_static`).
    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the storm harrier's wing-flex — the same Skyreach Pinnacle
    /// creature Sieve's `creature_engine.rs` already anchors as agile
    /// (`skyreach_pinnacle_storm_harrier_creature_lore_tie_is_agile`) —
    /// flexes and damps rather than snapping rigid in a wind gust. Anchors
    /// to the already-landed `damped_harmonic_rhs` rather than an invented
    /// "wings flap" flavour line. [OBSERVED] fabric: `damped_harmonic_rhs`,
    /// already tested generically above.
    #[test]
    fn skyreach_pinnacle_storm_harrier_wing_flex_lore_tie() {
        // A light wing membrane, gust-displaced, with real feather damping.
        let wing_mass_kg = 0.05;
        let feather_damping = 0.3;
        let wing_stiffness = 2.0;
        let gust_displacement_m = 0.1;
        let gust_velocity_ms = 0.5;

        let (_, dv) = damped_harmonic_rhs(wing_mass_kg, feather_damping, wing_stiffness, 0.0, gust_displacement_m, gust_velocity_ms);
        assert!(dv < 0.0, "a gust-displaced wing must accelerate back toward its resting flex, not snap or fly loose");
    }

    #[test]
    fn silent_wardens_gate_lore_tie_holds_until_balance_breaks() {
        let mass = 500.0;
        let stiffness = 200.0;
        let displacement = 2.0;
        let balanced_force = stiffness * displacement; // exactly enough to hold position
        let held = fea_single_dof_accel(mass, 0.0, stiffness, balanced_force, displacement, 0.0);
        assert!(held.abs() < 1e-9, "the Silent Warden's Gate must hold perfectly still when force and stiffness balance exactly");

        let unbalanced_force = balanced_force * 1.5; // consent/accusation withdrawn — imbalance
        let moving = fea_single_dof_accel(mass, 0.0, stiffness, unbalanced_force, displacement, 0.0);
        assert!(moving.abs() > 0.0, "the gate must actually move once the balance genuinely breaks");
    }

    #[test]
    fn test_fea_single_dof_static() {
        // Static equilibrium: F = k*x, no velocity → zero acceleration
        let a = fea_single_dof_accel(10.0, 0.0, 100.0, 100.0, 1.0, 0.0);
        assert!(a.abs() < 0.01);
    }

    /// W04 Mythos-anchor (world-builder brick, Lorekeeper lane float per
    /// W11): the Hollowden Rope Bridge — the same Hollowden Pack territory
    /// (`forge-soundwave-v3::ecology`'s `hollowden_pack_ecology_lore_tie`
    /// brick, Audio lane) — sways under a traveler's footfall but its rope
    /// tension damps the sway instead of letting it run away. Anchors to the
    /// already-landed `damped_harmonic_rhs`: a displaced, undamped bridge
    /// must accelerate back toward center (restoring), and adding real
    /// damping — while the deck is still swinging outward — must resist that
    /// outward motion harder than stiffness alone, not invented numbers.
    /// [OBSERVED] fabric: `damped_harmonic_rhs`, already tested generically
    /// above.
    #[test]
    fn hollowden_rope_bridge_lore_tie_sways_but_recovers() {
        // A traveler's weight displaces the bridge deck 0.3m from rest;
        // rope tension supplies restoring stiffness, hemp fiber supplies damping.
        let deck_mass_kg = 40.0;
        let rope_damping = 8.0;
        let rope_stiffness = 60.0;
        let displacement_m = 0.3;
        let velocity_ms = 1.5; // swinging away from center

        let (_, dv_no_damping) = damped_harmonic_rhs(deck_mass_kg, 0.0, rope_stiffness, 0.0, displacement_m, velocity_ms);
        let (_, dv_with_damping) = damped_harmonic_rhs(deck_mass_kg, rope_damping, rope_stiffness, 0.0, displacement_m, velocity_ms);

        assert!(dv_no_damping < 0.0, "the rope bridge must accelerate back toward center, not fly apart");
        assert!(
            dv_with_damping < dv_no_damping,
            "real rope damping must resist the outward swing harder than stiffness alone, never let it run away: undamped={dv_no_damping}, damped={dv_with_damping}"
        );
    }
}
