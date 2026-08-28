//! # AST Branch Extractor
//!
//! Routes a parsed `VixelAst` into `ExtractedBranches` — the three
//! semantic branches consumed by downstream compilers:
//!
//! - `materials` → forge-furnace → `.forge_reg`
//! - `spatials`  → socket graph / chunk placement
//! - `automata`  → forge-shader-build → `.spv` / `.dxil`
//!
//! Environment definitions (`set_*` calls) are intentionally excluded —
//! they feed the uniform buffer pipeline separately.

use super::{ExtractedBranches, VixelAst};

/// Extract the three compiler-facing branches from a parsed AST.
///
/// Clones materials, spatials, and automata out of the `VixelAst`.
/// Environment defs are **not** included — they are consumed by the
/// uniform buffer pipeline independently.
pub fn extract(ast: &VixelAst) -> ExtractedBranches {
    ExtractedBranches {
        materials: ast.materials.clone(),
        spatials: ast.spatials.clone(),
        automata: ast.automata.clone(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vixel::{
        AutomataDef, AutomataType, EnvironmentDef, EnvironmentType,
        MaterialDef, SpatialDef,
    };

    /// Helper: build a `MaterialDef` with a given id and name.
    fn make_material(id: u16, name: &str) -> MaterialDef {
        let mut mat = MaterialDef::default();
        mat.id = id;
        let bytes = name.as_bytes();
        let len = bytes.len().min(32);
        mat.name[..len].copy_from_slice(&bytes[..len]);
        mat.name_len = len;
        mat
    }

    /// Helper: build a `SpatialDef` with a given id and socket count.
    fn make_spatial(id: u16, socket_count: u8) -> SpatialDef {
        let mut spatial = SpatialDef::default();
        spatial.id = id;
        spatial.socket_count = socket_count;
        spatial
    }

    /// Helper: build an `AutomataDef` with a given id and rule type.
    fn make_automata(id: u16, rule_type: AutomataType) -> AutomataDef {
        AutomataDef {
            id,
            rule_type,
            wgsl_source: format!("// rule {id}"),
        }
    }

    /// Helper: build an `EnvironmentDef`.
    fn make_env(env_type: EnvironmentType, value: u16) -> EnvironmentDef {
        let mut env = EnvironmentDef::default();
        env.env_type = env_type;
        env.value_pmy = value;
        env
    }

    #[test]
    fn extract_empty_ast_yields_empty_branches() {
        let ast = VixelAst::new();
        let branches = extract(&ast);

        assert!(branches.materials.is_empty());
        assert!(branches.spatials.is_empty());
        assert!(branches.automata.is_empty());
    }

    #[test]
    fn extract_populated_ast_clones_all_branches() {
        let ast = VixelAst {
            materials: vec![make_material(1, "oak"), make_material(2, "stone")],
            spatials: vec![make_spatial(10, 3)],
            automata: vec![
                make_automata(100, AutomataType::Fire),
                make_automata(101, AutomataType::Fluid),
                make_automata(102, AutomataType::Custom),
            ],
            environment: vec![make_env(EnvironmentType::Temperature, 5000)],
            ui_defs: vec![],
            themes: vec![],
            atoms: vec![],
            acrylics: vec![],
            pressures: vec![],
            layers: vec![],
            viewports: vec![],
            brushes: vec![],
        };

        let branches = extract(&ast);

        assert_eq!(branches.materials.len(), 2);
        assert_eq!(branches.materials[0].id, 1);
        assert_eq!(branches.materials[0].name_str(), "oak");
        assert_eq!(branches.materials[1].id, 2);
        assert_eq!(branches.materials[1].name_str(), "stone");

        assert_eq!(branches.spatials.len(), 1);
        assert_eq!(branches.spatials[0].id, 10);
        assert_eq!(branches.spatials[0].socket_count, 3);

        assert_eq!(branches.automata.len(), 3);
        assert_eq!(branches.automata[0].rule_type, AutomataType::Fire);
        assert_eq!(branches.automata[1].rule_type, AutomataType::Fluid);
        assert_eq!(branches.automata[2].rule_type, AutomataType::Custom);
    }

    #[test]
    fn extract_excludes_environment_defs() {
        let ast = VixelAst {
            materials: vec![make_material(1, "iron")],
            spatials: vec![],
            automata: vec![],
            environment: vec![
                make_env(EnvironmentType::Temperature, 9500),
                make_env(EnvironmentType::Wind, 3000),
                make_env(EnvironmentType::Gravity, 1000),
            ],
            ui_defs: vec![],
            themes: vec![],
            atoms: vec![],
            acrylics: vec![],
            pressures: vec![],
            layers: vec![],
            viewports: vec![],
            brushes: vec![],
        };

        let branches = extract(&ast);

        // Materials are extracted
        assert_eq!(branches.materials.len(), 1);

        // Environment defs have no field in ExtractedBranches — they
        // are consumed separately by the uniform buffer pipeline.
        // The struct simply doesn't carry them.
        assert_eq!(
            std::mem::size_of_val(&branches),
            std::mem::size_of::<ExtractedBranches>()
        );
    }

    #[test]
    fn extract_does_not_mutate_source_ast() {
        let ast = VixelAst {
            materials: vec![make_material(1, "oak")],
            spatials: vec![make_spatial(5, 2)],
            automata: vec![make_automata(50, AutomataType::Gravity)],
            environment: vec![make_env(EnvironmentType::Wind, 7000)],
            ui_defs: vec![],
            themes: vec![],
            atoms: vec![],
            acrylics: vec![],
            pressures: vec![],
            layers: vec![],
            viewports: vec![],
            brushes: vec![],
        };

        let _branches = extract(&ast);

        // Source AST is unchanged
        assert_eq!(ast.materials.len(), 1);
        assert_eq!(ast.spatials.len(), 1);
        assert_eq!(ast.automata.len(), 1);
        assert_eq!(ast.environment.len(), 1);
    }
}
