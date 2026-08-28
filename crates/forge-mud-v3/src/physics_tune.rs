//! Physics tuning constants, hot-reloadable from RON configuration.
//!
//! This module mirrors the pattern used in `.forge/v3-directives.ron`
//! (V3 DIRECTIVES — tunable mechanisms). Default values are the current
//! hardcoded physics constants; target config file is `.forge/physics-tune.ron`.
//!
//! The caller is responsible for providing the path — this module never
//! embeds shell-specific paths, following the pattern in `world5d.rs` and
//! `bdo_controller.rs`.

use std::fs;
use std::path::Path;

/// Physics tuning constants for the simulator.
///
/// All values are tunable via a RON config file (default: `.forge/physics-tune.ron`).
/// If the file is missing or contains a parse error, [`load()`](Self::load) falls back
/// to [`Default`](Self::default) gracefully and logs the failure to stderr.
///
/// Follows the pattern established in `.forge/v3-directives.ron`.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PhysicsTune {
    /// Gravity acceleration, MilliUnit per tick squared.
    /// Default: 272 (from `world5d.rs::GRAVITY_MM_PER_TICK_SQ`).
    pub gravity_mm_per_tick_sq: i64,

    /// Upward terminal-velocity clamp, MilliUnit per tick.
    /// Default: 30_000 (from `world5d.rs::UPWARD_TERMINAL_VELOCITY_MU`).
    /// Set well above jump impulse so the clamp never caps a jump back down.
    pub upward_terminal_velocity_mu: i64,

    /// Horizontal impulse magnitude for a dash, MilliUnit per tick.
    /// Default: 1050 (from `world5d.rs::DASH_IMPULSE_MM_PER_TICK`).
    /// Roughly 3.5x normal walk speed at 120 Hz.
    pub dash_impulse_mm_per_tick: i64,

    /// Cooldown between successive dashes/evades, ticks.
    /// Default: 90 (from `world5d.rs::DASH_COOLDOWN_TICKS`).
    /// Roughly 0.75 seconds at 120 Hz.
    pub dash_cooldown_ticks: u32,

    /// Invulnerability window for an evade, ticks.
    /// Default: 30 (from `world5d.rs::EVADE_INVULN_TICKS`).
    /// Roughly 0.25 seconds at 120 Hz, a typical action-game i-frame duration.
    pub evade_invuln_ticks: u32,

    /// Upward impulse on a grounded jump, MilliUnit per tick.
    /// Default: 21_000 (from `shell/src/main.rs`, line ~814).
    /// 15x the original impulse, effective with asymmetric terminal-velocity clamp.
    pub jump_impulse_mu: i64,
}

impl Default for PhysicsTune {
    fn default() -> Self {
        Self {
            gravity_mm_per_tick_sq: 272,
            upward_terminal_velocity_mu: 30_000,
            dash_impulse_mm_per_tick: 1050,
            dash_cooldown_ticks: 90,
            evade_invuln_ticks: 30,
            jump_impulse_mu: 21_000,
        }
    }
}

impl PhysicsTune {
    /// Load physics tuning from a RON file.
    ///
    /// If the file exists and is valid RON, deserialize and return it.
    /// On any error (missing file, parse error, I/O error), fall back to
    /// [`Default`](Self::default) and log the error to stderr (never panic).
    ///
    /// This is a tuning convenience, not a hard dependency — the simulator
    /// always works, with or without the config file present.
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => {
                match ron::from_str::<PhysicsTune>(&content) {
                    Ok(tune) => tune,
                    Err(e) => {
                        eprintln!(
                            "physics_tune: failed to parse RON at {:?}: {}",
                            path, e
                        );
                        Self::default()
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "physics_tune: failed to read file at {:?}: {}",
                    path, e
                );
                Self::default()
            }
        }
    }

    /// Save physics tuning to a RON file.
    ///
    /// Serializes with pretty formatting (via `ron::ser::PrettyConfig::default()`).
    /// Creates parent directories if needed.
    /// Returns any I/O error; caller is responsible for handling.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let pretty = ron::ser::PrettyConfig::default();
        let content = ron::ser::to_string_pretty(self, pretty)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Test counter for unique temp filenames
    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_file() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("physics_tune_test_{}.ron", id))
    }

    #[test]
    fn default_has_exact_hardcoded_values() {
        let tune = PhysicsTune::default();
        assert_eq!(tune.gravity_mm_per_tick_sq, 272);
        assert_eq!(tune.upward_terminal_velocity_mu, 30_000);
        assert_eq!(tune.dash_impulse_mm_per_tick, 1050);
        assert_eq!(tune.dash_cooldown_ticks, 90);
        assert_eq!(tune.evade_invuln_ticks, 30);
        assert_eq!(tune.jump_impulse_mu, 21_000);
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_file();

        // Create and save a non-default config
        let original = PhysicsTune {
            gravity_mm_per_tick_sq: 300,
            upward_terminal_velocity_mu: 35_000,
            dash_impulse_mm_per_tick: 1200,
            dash_cooldown_ticks: 100,
            evade_invuln_ticks: 40,
            jump_impulse_mu: 25_000,
        };

        original.save(&path).expect("save failed");

        // Load it back
        let loaded = PhysicsTune::load(&path);

        // Verify exact match
        assert_eq!(loaded.gravity_mm_per_tick_sq, original.gravity_mm_per_tick_sq);
        assert_eq!(
            loaded.upward_terminal_velocity_mu,
            original.upward_terminal_velocity_mu
        );
        assert_eq!(
            loaded.dash_impulse_mm_per_tick,
            original.dash_impulse_mm_per_tick
        );
        assert_eq!(loaded.dash_cooldown_ticks, original.dash_cooldown_ticks);
        assert_eq!(loaded.evade_invuln_ticks, original.evade_invuln_ticks);
        assert_eq!(loaded.jump_impulse_mu, original.jump_impulse_mu);

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_nonexistent_file_falls_back_to_default() {
        let nonexistent = std::path::PathBuf::from("/this/path/does/not/exist/physics.ron");
        let tune = PhysicsTune::load(&nonexistent);

        // Should silently fall back to default
        assert_eq!(tune.gravity_mm_per_tick_sq, 272);
        assert_eq!(tune.jump_impulse_mu, 21_000);
    }

    #[test]
    fn load_malformed_ron_falls_back_to_default() {
        let path = temp_file();

        // Write invalid RON
        fs::write(&path, "this { is not: valid ron syntax").expect("write failed");

        let tune = PhysicsTune::load(&path);

        // Should silently fall back to default
        assert_eq!(tune.gravity_mm_per_tick_sq, 272);
        assert_eq!(tune.jump_impulse_mu, 21_000);

        // Clean up
        let _ = fs::remove_file(&path);
    }
}
