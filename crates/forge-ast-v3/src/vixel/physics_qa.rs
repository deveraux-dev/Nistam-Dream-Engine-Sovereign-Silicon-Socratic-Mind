//! # physics_qa.rs — Compile-Time Physics QA Gate
//!
//! Validates a `VixelAst` against physical plausibility constraints before
//! the AST enters the AOT compilation pipeline. Invalid physics are rejected
//! before the cryptographic seal is generated.
//!
//! **Physics checks (AC-4.1 through AC-4.4):**
//! - AC-4.1: Materials with `mass_pmy > 0` named "air" are rejected
//! - AC-4.2: Automata referencing material pairs with mass delta > 5000 pmy are rejected
//! - AC-4.3: Center-of-mass discontinuities (mass delta > 5000 between referenced materials) are flagged
//! - AC-4.4: `flammability_pmy > 0` for `metallic_pmy > 8000` is rejected (metals don't burn)
//!
//! **Implementation:** Heuristic checks at the AST level. No runtime simulation.
//! Regex-free byte-level scanning for material name references in WGSL source.

use super::{MaterialDef, PhysicsViolation, VixelAst};

/// Maximum allowed mass delta (permyriad) between materials referenced
/// in a single automata rule tick.
const MAX_MASS_DELTA_PMY: u16 = 5000;

/// Metallic threshold (permyriad) above which flammability must be zero.
const METALLIC_BURN_THRESHOLD: u16 = 8000;

/// Validate a `VixelAst` against physical plausibility constraints.
///
/// Returns `Ok(())` if all checks pass, or `Err(PhysicsViolation)` on the
/// first violation found.
///
/// # Checks
/// 1. Per-material: air-mass contradiction (AC-4.1)
/// 2. Per-material: structural plausibility — hardness <= mass * 2
/// 3. Per-material: burning metal rejection (AC-4.4)
/// 4. Per-automata: mass delta between referenced materials (AC-4.2, AC-4.3)
///
/// # Guarantees
/// - Never panics
/// - Deterministic: same input always produces the same result
pub fn physics_qa_gate(ast: &VixelAst) -> Result<(), PhysicsViolation> {
    // --- Per-material checks ---
    for mat in &ast.materials {
        // AC-4.1: Materials named "air" (case-insensitive) must have mass_pmy == 0
        if mat.mass_pmy > 0 && is_air_material(mat) {
            return Err(PhysicsViolation {
                material_id: mat.id,
                rule: "air_mass".into(),
                message: format!(
                    "Material '{}' is named air but has mass_pmy={} (AC-4.1: air must have zero mass)",
                    mat.name_str(),
                    mat.mass_pmy
                ),
            });
        }

        // Structural plausibility: hardness_pmy <= mass_pmy * 2
        // A material can't be harder than twice its mass allows.
        let mass_limit = mat.mass_pmy.saturating_mul(2);
        if mat.hardness_pmy > mass_limit && mat.mass_pmy > 0 {
            return Err(PhysicsViolation {
                material_id: mat.id,
                rule: "structural_plausibility".into(),
                message: format!(
                    "Material '{}' has hardness_pmy={} exceeding 2x mass_pmy={} (limit: {})",
                    mat.name_str(),
                    mat.hardness_pmy,
                    mat.mass_pmy,
                    mass_limit
                ),
            });
        }

        // AC-4.4: Metals don't burn — flammability must be 0 when metallic > 8000
        if mat.metallic_pmy > METALLIC_BURN_THRESHOLD && mat.flammability_pmy > 0 {
            return Err(PhysicsViolation {
                material_id: mat.id,
                rule: "burning_metal".into(),
                message: format!(
                    "Material '{}' has flammability_pmy={} with metallic_pmy={} \
                     (AC-4.4: metals with metallic > {} cannot burn)",
                    mat.name_str(),
                    mat.flammability_pmy,
                    mat.metallic_pmy,
                    METALLIC_BURN_THRESHOLD
                ),
            });
        }
    }

    // --- Per-automata checks (AC-4.2, AC-4.3) ---
    // Heuristic: scan WGSL source for material name references, then check
    // if any pair of referenced materials has a mass delta > MAX_MASS_DELTA_PMY.
    for rule in &ast.automata {
        let referenced = find_referenced_materials(&rule.wgsl_source, &ast.materials);

        // Check all pairs of referenced materials for mass delta violations
        for i in 0..referenced.len() {
            for j in (i + 1)..referenced.len() {
                let mass_a = referenced[i].mass_pmy;
                let mass_b = referenced[j].mass_pmy;
                let delta = mass_a.abs_diff(mass_b);

                if delta > MAX_MASS_DELTA_PMY {
                    // AC-4.2: Mass delta > 5000 in one tick
                    return Err(PhysicsViolation {
                        material_id: referenced[i].id,
                        rule: "mass_delta".into(),
                        message: format!(
                            "Automata rule {} references materials '{}' (mass={}) and '{}' (mass={}) \
                             with delta={} > {} (AC-4.2: mass delta too large for single tick)",
                            rule.id,
                            referenced[i].name_str(),
                            mass_a,
                            referenced[j].name_str(),
                            mass_b,
                            delta,
                            MAX_MASS_DELTA_PMY
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Check if a material name contains "air" (case-insensitive).
fn is_air_material(mat: &MaterialDef) -> bool {
    let name = mat.name_str().to_ascii_lowercase();
    name.contains("air")
}

/// Find materials referenced by name in an automata rule's WGSL source.
///
/// Scans the WGSL source for occurrences of each material's name (case-insensitive).
/// Returns references to all matched `MaterialDef`s.
fn find_referenced_materials<'a>(
    wgsl_source: &str,
    materials: &'a [MaterialDef],
) -> Vec<&'a MaterialDef> {
    let src_lower = wgsl_source.to_ascii_lowercase();
    let mut referenced = Vec::new();

    for mat in materials {
        let name = mat.name_str();
        if name.is_empty() {
            continue;
        }
        let name_lower = name.to_ascii_lowercase();
        if src_lower.contains(&name_lower) {
            referenced.push(mat);
        }
    }

    referenced
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vixel::{AutomataDef, AutomataType, MaterialDef, VixelAst};

    /// Helper: build a `MaterialDef` with physical properties.
    fn make_material(id: u16, name: &str, mass: u16, hardness: u16, metallic: u16, flammability: u16) -> MaterialDef {
        let mut mat = MaterialDef::default();
        mat.id = id;
        let bytes = name.as_bytes();
        let len = bytes.len().min(32);
        mat.name[..len].copy_from_slice(&bytes[..len]);
        mat.name_len = len;
        mat.mass_pmy = mass;
        mat.hardness_pmy = hardness;
        mat.metallic_pmy = metallic;
        mat.flammability_pmy = flammability;
        mat
    }

    /// Helper: build an `AutomataDef` with given wgsl_source.
    fn make_automata(id: u16, wgsl: &str) -> AutomataDef {
        AutomataDef {
            id,
            rule_type: AutomataType::Custom,
            wgsl_source: wgsl.to_string(),
        }
    }

    // -- Empty AST passes ----------------------------------------------------

    #[test]
    fn empty_ast_passes() {
        let ast = VixelAst::new();
        assert!(physics_qa_gate(&ast).is_ok());
    }

    // -- Clean materials pass ------------------------------------------------

    #[test]
    fn clean_materials_pass() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "oak", 4200, 3500, 500, 7800));
        ast.materials.push(make_material(1, "stone", 6000, 8000, 200, 0));
        assert!(physics_qa_gate(&ast).is_ok());
    }

    // -- AC-4.1: Air mass contradiction --------------------------------------

    #[test]
    fn reject_air_with_mass() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "air", 100, 0, 0, 0));

        let err = physics_qa_gate(&ast).unwrap_err();
        assert_eq!(err.rule, "air_mass");
        assert!(err.message.contains("AC-4.1"));
    }

    #[test]
    fn allow_air_with_zero_mass() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "air", 0, 0, 0, 0));
        assert!(physics_qa_gate(&ast).is_ok());
    }

    #[test]
    fn reject_air_case_insensitive() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "AIR", 50, 0, 0, 0));

        let err = physics_qa_gate(&ast).unwrap_err();
        assert_eq!(err.rule, "air_mass");
    }

    #[test]
    fn reject_air_substring() {
        // "thin_air" contains "air" — should be caught
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "thin_air", 10, 0, 0, 0));

        let err = physics_qa_gate(&ast).unwrap_err();
        assert_eq!(err.rule, "air_mass");
    }

    // -- AC-4.4: Burning metal -----------------------------------------------

    #[test]
    fn reject_burning_metal() {
        let mut ast = VixelAst::new();
        // metallic=9000 > 8000, flammability=500 > 0 → reject
        ast.materials.push(make_material(0, "steel", 7000, 5000, 9000, 500));

        let err = physics_qa_gate(&ast).unwrap_err();
        assert_eq!(err.rule, "burning_metal");
        assert!(err.message.contains("AC-4.4"));
    }

    #[test]
    fn allow_non_flammable_metal() {
        let mut ast = VixelAst::new();
        // metallic=9000 > 8000, flammability=0 → OK
        ast.materials.push(make_material(0, "steel", 7000, 5000, 9000, 0));
        assert!(physics_qa_gate(&ast).is_ok());
    }

    #[test]
    fn allow_flammable_non_metal() {
        let mut ast = VixelAst::new();
        // metallic=2000 < 8000, flammability=8000 → OK (wood can burn)
        ast.materials.push(make_material(0, "oak", 4200, 3500, 2000, 8000));
        assert!(physics_qa_gate(&ast).is_ok());
    }

    #[test]
    fn allow_metallic_at_threshold() {
        let mut ast = VixelAst::new();
        // metallic=8000 exactly at threshold, flammability=100 → OK (threshold is >8000, not >=)
        ast.materials.push(make_material(0, "alloy", 5000, 3000, 8000, 100));
        assert!(physics_qa_gate(&ast).is_ok());
    }

    // -- Structural plausibility ---------------------------------------------

    #[test]
    fn reject_implausible_hardness() {
        let mut ast = VixelAst::new();
        // mass=1000, hardness=3000 > 2*1000=2000 → reject
        ast.materials.push(make_material(0, "foam", 1000, 3000, 0, 0));

        let err = physics_qa_gate(&ast).unwrap_err();
        assert_eq!(err.rule, "structural_plausibility");
    }

    #[test]
    fn allow_hardness_at_limit() {
        let mut ast = VixelAst::new();
        // mass=3000, hardness=6000 == 2*3000 → OK (at limit, not over)
        ast.materials.push(make_material(0, "rock", 3000, 6000, 0, 0));
        assert!(physics_qa_gate(&ast).is_ok());
    }

    // -- AC-4.2 / AC-4.3: Mass delta in automata ----------------------------

    #[test]
    fn reject_mass_delta_in_automata() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "feather", 100, 50, 0, 0));
        ast.materials.push(make_material(1, "lead", 9000, 7000, 5000, 0));
        // Automata references both materials → delta = 8900 > 5000
        ast.automata.push(make_automata(0, "when feather near lead then swap"));

        let err = physics_qa_gate(&ast).unwrap_err();
        assert_eq!(err.rule, "mass_delta");
        assert!(err.message.contains("AC-4.2"));
    }

    #[test]
    fn allow_valid_mass_delta_in_automata() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "oak", 4200, 3500, 500, 7800));
        ast.materials.push(make_material(1, "pine", 3800, 3000, 400, 8000));
        // delta = 400 < 5000 → OK
        ast.automata.push(make_automata(0, "when oak near pine then spread_fire"));

        assert!(physics_qa_gate(&ast).is_ok());
    }

    #[test]
    fn allow_automata_referencing_single_material() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "stone", 6000, 8000, 200, 0));
        ast.materials.push(make_material(1, "iron", 7500, 5000, 9500, 0));
        // Only references "stone" — no pair to compare
        ast.automata.push(make_automata(0, "when stone below then fall"));

        assert!(physics_qa_gate(&ast).is_ok());
    }

    #[test]
    fn allow_automata_with_no_material_refs() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "stone", 6000, 8000, 200, 0));
        // WGSL doesn't reference any material by name
        ast.automata.push(make_automata(0, "when neighbor_count > 3 then activate"));

        assert!(physics_qa_gate(&ast).is_ok());
    }
}
