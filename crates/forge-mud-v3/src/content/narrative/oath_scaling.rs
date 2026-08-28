//! Oath Discipline → 7-stat hermetic profile bridge.
//!
//! Maps the 8 Oath Disciplines (virtue-based birth rite picks from ironroot.ron)
//! to hermetic stat profiles (vigor, shadow_weight, logic_depth, momentum, tarnish,
//! resonance, guilt). Clarity is always 0 at birth — it is earned in play.
//!
//! Design: Each discipline creates a biased stat seed by virtue signature.
//! The 7-stat model (via hermetics.rs + forge_arena sevenfold) replaces
//! the 8-stat CoreStat model for birth rite profile allocation.
//!
//! ## Clarity (the 8th register)
//!
//! Per hermetics.rs:100-101 and material.rs:157, Clarity is NOT dealt at birth.
//! It is earned in play via game mechanics (pending definition). The 7-stat profile
//! returned by `oath_to_hermetic_profile()` never includes Clarity; callers that need
//! the full 8-stat HermeticStats block must initialize clarity: 0 separately when
//! constructing a character.

/// The 8 Oath Disciplines from birth rite — virtue-centered identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OathDisciplineProfile {
    /// Edge: precision, kept sharp by discipline and restraint.
    Edge,
    /// Weight: bearing burden without complaint, steadfast endurance.
    Weight,
    /// Breath: composure amid chaos, stillness under pressure.
    Breath,
    /// Thread: restoration, mending what was broken.
    Thread,
    /// Ash: destruction cleaned, burning away the unnecessary.
    Ash,
    /// Root: remaining when all else moves, radical steadfastness.
    Root,
    /// Glass: clarity of sight, seeing through surface to truth.
    Glass,
    /// Salt: preservation against decay, holding the line against spoilage.
    Salt,
}

impl OathDisciplineProfile {
    /// Convert numeric index (0-7) to oath discipline profile (wraps with modulo).
    pub fn from_index(i: u8) -> Self {
        match i % 8 {
            0 => Self::Edge,
            1 => Self::Weight,
            2 => Self::Breath,
            3 => Self::Thread,
            4 => Self::Ash,
            5 => Self::Root,
            6 => Self::Glass,
            _ => Self::Salt,
        }
    }

    /// 7-stat profile: [vigor, shadow_weight, logic_depth, momentum, tarnish, resonance, guilt].
    /// Each discipline applies virtue-centered bias to these registers.
    /// Ranges: 0..255 (u8). Clarity is always 0 at birth (earned in play, never dealt).
    pub fn stat_profile(self) -> [u8; 7] {
        match self {
            OathDisciplineProfile::Edge => {
                // Precision, sharp mind, swift: +momentum, +logic_depth, -shadow_weight.
                // [vig, sha, log, mom, tar, res, gil]
                [50, 30, 80, 100, 20, 60, 40]
            }
            OathDisciplineProfile::Weight => {
                // Bearing burden, endurance, willful: +vigor, +shadow_weight, +guilt.
                [100, 90, 40, 40, 30, 50, 70]
            }
            OathDisciplineProfile::Breath => {
                // Composure, wisdom, attunement: +logic_depth, +resonance, -tarnish.
                [60, 70, 90, 50, 20, 100, 50]
            }
            OathDisciplineProfile::Thread => {
                // Restoration, mending, attunement: +resonance, +vigor, -tarnish.
                [80, 50, 60, 60, 20, 110, 40]
            }
            OathDisciplineProfile::Ash => {
                // Destruction, strength, sacrifice: +vigor, +tarnish, +logic_depth.
                [110, 40, 70, 50, 80, 40, 60]
            }
            OathDisciplineProfile::Root => {
                // Steadfast, grounded, unmoving: +shadow_weight, +vigor, +resonance.
                [100, 110, 50, 40, 40, 80, 60]
            }
            OathDisciplineProfile::Glass => {
                // Clarity of sight, sharp perception, swift communication: +logic_depth, +momentum, -tarnish.
                [50, 40, 110, 90, 15, 70, 45]
            }
            OathDisciplineProfile::Salt => {
                // Preservation, endurance, stability: +shadow_weight, +resonance, -tarnish.
                [70, 100, 50, 50, 20, 100, 50]
            }
        }
    }

    /// Human-readable name for this oath discipline.
    pub const fn name(self) -> &'static str {
        match self {
            OathDisciplineProfile::Edge => "Edge",
            OathDisciplineProfile::Weight => "Weight",
            OathDisciplineProfile::Breath => "Breath",
            OathDisciplineProfile::Thread => "Thread",
            OathDisciplineProfile::Ash => "Ash",
            OathDisciplineProfile::Root => "Root",
            OathDisciplineProfile::Glass => "Glass",
            OathDisciplineProfile::Salt => "Salt",
        }
    }

    /// Lore flavor text (from ironroot.ron birth rite prompt).
    pub const fn lore(self) -> &'static str {
        match self {
            OathDisciplineProfile::Edge => {
                "you kept something sharp when keeping it was the harder thing"
            }
            OathDisciplineProfile::Weight => {
                "you carried what was set down and did not say so"
            }
            OathDisciplineProfile::Breath => {
                "you stayed calm in a room that wanted otherwise"
            }
            OathDisciplineProfile::Thread => {
                "you mended; the mend outlived the maker"
            }
            OathDisciplineProfile::Ash => {
                "you burned it clean and kept the smoke to yourself"
            }
            OathDisciplineProfile::Root => {
                "you stayed. that was the whole of it"
            }
            OathDisciplineProfile::Glass => {
                "you saw through, and said only what was needed"
            }
            OathDisciplineProfile::Salt => {
                "you preserved what would otherwise have spoiled"
            }
        }
    }
}

/// Build a 7-stat hermetic profile for an oath discipline.
/// Returns: [vigor, shadow_weight, logic_depth, momentum, tarnish, resonance, guilt].
/// Clarity is always 0 at birth (earned in play, never dealt or rolled).
pub fn oath_to_hermetic_profile(discipline: OathDisciplineProfile) -> [u8; 7] {
    discipline.stat_profile()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_profile_prioritizes_momentum_logic() {
        let profile = oath_to_hermetic_profile(OathDisciplineProfile::Edge);
        // Edge: [vig, sha, log, mom, tar, res, gil]
        // Profile: [50, 30, 80, 100, 20, 60, 40]
        let (_vig, _sha, log, mom, _tar, _res, _gil) = (
            profile[0], profile[1], profile[2], profile[3], profile[4], profile[5], profile[6],
        );
        assert!(mom > log, "Edge should prioritize momentum (precision/speed)");
        assert!(mom >= 100, "Edge momentum should be strong (>=100)");
        assert!(log >= 80, "Edge logic should be elevated (>=80)");
    }

    #[test]
    fn weight_profile_prioritizes_vigor_shadow() {
        let profile = oath_to_hermetic_profile(OathDisciplineProfile::Weight);
        // Weight: [vig, sha, log, mom, tar, res, gil]
        // Profile: [100, 90, 40, 40, 30, 50, 70]
        let (vig, sha, _log, _mom, _tar, _res, _gil) = (
            profile[0], profile[1], profile[2], profile[3], profile[4], profile[5], profile[6],
        );
        assert!(vig >= 90, "Weight vigor should be high (>=90)");
        assert!(sha >= 90, "Weight shadow_weight should be high (>=90)");
    }

    #[test]
    fn all_disciplines_have_7stat_profiles() {
        let all = [
            OathDisciplineProfile::Edge,
            OathDisciplineProfile::Weight,
            OathDisciplineProfile::Breath,
            OathDisciplineProfile::Thread,
            OathDisciplineProfile::Ash,
            OathDisciplineProfile::Root,
            OathDisciplineProfile::Glass,
            OathDisciplineProfile::Salt,
        ];
        for d in all {
            let profile = oath_to_hermetic_profile(d);
            assert_eq!(profile.len(), 7, "Profile should have 7 stats");
            // The dealt registers carry a bias, never a hole: Clarity is the one
            // register birth leaves at zero, and it is not in this array.
            for (i, stat) in profile.iter().enumerate() {
                assert!(*stat > 0, "{} register {i} was dealt zero", d.name());
            }
        }
    }

    #[test]
    fn clarity_is_never_dealt_at_birth() {
        // Clarity is earned in play, never rolled/dealt. This is enforced
        // by returning only 7 stats; the 8th (clarity) is always 0 at birth.
        // Test verifies we never allocate clarity in the profile.
        let profile = oath_to_hermetic_profile(OathDisciplineProfile::Edge);
        assert_eq!(profile.len(), 7, "Profile must not include clarity");
    }

    #[test]
    fn each_discipline_has_unique_name() {
        let all = [
            OathDisciplineProfile::Edge,
            OathDisciplineProfile::Weight,
            OathDisciplineProfile::Breath,
            OathDisciplineProfile::Thread,
            OathDisciplineProfile::Ash,
            OathDisciplineProfile::Root,
            OathDisciplineProfile::Glass,
            OathDisciplineProfile::Salt,
        ];
        let names: Vec<_> = all.iter().map(|d| d.name()).collect();
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                assert_ne!(names[i], names[j], "Disciplines should have unique names");
            }
        }
    }

    #[test]
    fn each_discipline_has_lore_text() {
        let all = [
            OathDisciplineProfile::Edge,
            OathDisciplineProfile::Weight,
            OathDisciplineProfile::Breath,
            OathDisciplineProfile::Thread,
            OathDisciplineProfile::Ash,
            OathDisciplineProfile::Root,
            OathDisciplineProfile::Glass,
            OathDisciplineProfile::Salt,
        ];
        for d in all {
            let lore = d.lore();
            assert!(!lore.is_empty(), "Discipline {} should have lore text", d.name());
        }
    }

    #[test]
    fn from_index_wraps_eight_disciplines() {
        let d0 = OathDisciplineProfile::from_index(0);
        assert_eq!(d0, OathDisciplineProfile::Edge);

        let d7 = OathDisciplineProfile::from_index(7);
        assert_eq!(d7, OathDisciplineProfile::Salt);

        let d8 = OathDisciplineProfile::from_index(8);
        assert_eq!(d8, OathDisciplineProfile::Edge, "Index 8 should wrap to Edge");

        let d15 = OathDisciplineProfile::from_index(15);
        assert_eq!(d15, OathDisciplineProfile::Salt, "Index 15 should wrap to Salt");
    }

    #[test]
    fn stat_profiles_use_7stat_model() {
        // Verify structure: [vigor, shadow_weight, logic_depth, momentum, tarnish, resonance, guilt]
        let edge = oath_to_hermetic_profile(OathDisciplineProfile::Edge);
        let (vig, sha, log, mom, tar, res, gil) = (
            edge[0], edge[1], edge[2], edge[3], edge[4], edge[5], edge[6],
        );
        // Positional contract. This array's order is NOT HermeticStats' field
        // order (which runs vigor, momentum, logic_depth, shadow_weight, ...),
        // so anyone building that struct from this array must name the fields
        // rather than splat positionally.
        assert_eq!([vig, sha, log, mom, tar, res, gil], [50, 30, 80, 100, 20, 60, 40]);
        assert_eq!(sha, edge[1], "index 1 is shadow_weight here, momentum in HermeticStats");
        assert_eq!(mom, edge[3], "index 3 is momentum here, shadow_weight in HermeticStats");
    }
}
