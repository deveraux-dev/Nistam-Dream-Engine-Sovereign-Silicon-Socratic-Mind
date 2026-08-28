//! # ast_optimizer.rs — Dead Node Pruning
//!
//! Removes unreferenced materials, disconnected sockets, and unreachable
//! automata rules from a `VixelAst`. Returns a `PruneReport` counting
//! all removals.
//!
//! **Pruning rules (AC-2.1 through AC-2.5):**
//! - Unreferenced materials: not used by any spatial or automata
//! - Disconnected sockets: no matching partner `(-x, -y, -z)` in another spatial
//! - Unreachable automata: empty `wgsl_source` (no trigger condition)
//! - Idempotent: `optimize(optimize(ast))` == `optimize(ast)`
//!
//! **Constraints:**
//! - Mutates the AST in place (`&mut VixelAst`)
//! - No runtime dependencies — build-time only

use std::collections::HashSet;

use super::{PruneReport, VixelAst};

/// Prune dead nodes from the AST. Returns a report of what was removed.
///
/// This function is idempotent — calling it twice produces the same result
/// as calling it once (AC-2.4).
pub fn optimize(ast: &mut VixelAst) -> PruneReport {
    let mut report = PruneReport::default();

    report.automata_pruned = prune_unreachable_automata(ast);
    report.materials_pruned = prune_unreferenced_materials(ast);
    report.sockets_pruned = prune_disconnected_sockets(ast);

    report
}

/// AC-2.3: Remove automata rules with empty `wgsl_source` (no trigger).
///
/// An automata rule is unreachable if its wgsl_source, after trimming
/// whitespace, is empty.
fn prune_unreachable_automata(ast: &mut VixelAst) -> usize {
    let before = ast.automata.len();
    ast.automata.retain(|a| !a.wgsl_source.trim().is_empty());
    before - ast.automata.len()
}

/// AC-2.1: Remove materials not referenced by any spatial or automata.
///
/// A material is "referenced" if its `name_str()` appears in any
/// `AutomataDef.wgsl_source`. `SpatialDef` has no string fields in the
/// current data model, so spatial references are a no-op until the struct
/// gains a material name field.
fn prune_unreferenced_materials(ast: &mut VixelAst) -> usize {
    if ast.materials.is_empty() {
        return 0;
    }

    // Collect all text that could reference a material name.
    // Currently only automata wgsl_source contains string content.
    let mut reference_corpus = String::new();
    for automata in &ast.automata {
        reference_corpus.push(' ');
        reference_corpus.push_str(&automata.wgsl_source);
    }

    let before = ast.materials.len();
    ast.materials.retain(|mat| {
        let name = mat.name_str();
        // Empty-named materials are always pruned (can't be referenced).
        if name.is_empty() {
            return false;
        }
        reference_corpus.contains(name)
    });
    before - ast.materials.len()
}

/// AC-2.2: Remove disconnected sockets from spatials.
///
/// A socket `(x, y, z)` in a `SpatialDef` is "connected" if any *other*
/// `SpatialDef` has a socket at `(-x, -y, -z)`. Sockets with no partner
/// are zeroed and `socket_count` is decremented.
fn prune_disconnected_sockets(ast: &mut VixelAst) -> usize {
    if ast.spatials.len() < 2 {
        // With 0 or 1 spatial, no cross-spatial partners can exist.
        // Prune all sockets from the single spatial (if any).
        let mut pruned = 0;
        for spatial in &mut ast.spatials {
            pruned += spatial.socket_count as usize;
            spatial.socket_count = 0;
            spatial.sockets = [(0, 0, 0); 6];
        }
        return pruned;
    }

    // Build a set of all (spatial_index, socket_coord) pairs.
    // A socket is connected if another spatial has the negated coordinate.
    //
    // For each spatial, collect all non-zero sockets from *other* spatials
    // as potential partners.

    // First pass: collect all sockets grouped by spatial index.
    let all_sockets: Vec<Vec<(i8, i8, i8)>> = ast
        .spatials
        .iter()
        .map(|s| {
            (0..s.socket_count as usize)
                .map(|i| s.sockets[i])
                .collect()
        })
        .collect();

    // Build a set of negated sockets from each spatial, keyed by spatial index.
    // For socket (x,y,z) in spatial A, it's connected if any spatial B (B != A)
    // has (-x,-y,-z).
    let mut total_pruned = 0;

    for (idx, spatial) in ast.spatials.iter_mut().enumerate() {
        // Collect all sockets from OTHER spatials (negated) into a lookup set.
        let mut partner_set: HashSet<(i8, i8, i8)> = HashSet::new();
        for (other_idx, other_sockets) in all_sockets.iter().enumerate() {
            if other_idx == idx {
                continue;
            }
            for &(x, y, z) in other_sockets {
                // The partner of (x,y,z) is (-x,-y,-z)
                partner_set.insert((-x, -y, -z));
            }
        }

        // Check each socket in this spatial for a partner.
        let mut kept = 0u8;
        let mut new_sockets = [(0i8, 0i8, 0i8); 6];

        for i in 0..spatial.socket_count as usize {
            let sock = spatial.sockets[i];
            if partner_set.contains(&sock) {
                new_sockets[kept as usize] = sock;
                kept += 1;
            } else {
                total_pruned += 1;
            }
        }

        spatial.sockets = new_sockets;
        spatial.socket_count = kept;
    }

    total_pruned
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vixel::{AutomataDef, AutomataType, MaterialDef, SpatialDef};

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

    /// Helper: build an `AutomataDef` with given wgsl_source.
    fn make_automata(id: u16, wgsl: &str) -> AutomataDef {
        AutomataDef {
            id,
            rule_type: AutomataType::Fire,
            wgsl_source: wgsl.to_string(),
        }
    }

    /// Helper: build a `SpatialDef` with specific sockets.
    fn make_spatial(id: u16, sockets: &[(i8, i8, i8)]) -> SpatialDef {
        let mut def = SpatialDef::default();
        def.id = id;
        for (i, &s) in sockets.iter().enumerate().take(6) {
            def.sockets[i] = s;
        }
        def.socket_count = sockets.len().min(6) as u8;
        def
    }

    // -- AC-2.1: Unreferenced materials --------------------------------------

    #[test]
    fn prune_unreferenced_material() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "oak"));
        ast.materials.push(make_material(1, "stone"));
        // Only "oak" is referenced in automata
        ast.automata
            .push(make_automata(0, "when: oak.mass > 100 then: destroy"));

        let report = optimize(&mut ast);

        assert_eq!(report.materials_pruned, 1);
        assert_eq!(ast.materials.len(), 1);
        assert_eq!(ast.materials[0].name_str(), "oak");
    }

    #[test]
    fn keep_referenced_material() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "iron"));
        ast.automata
            .push(make_automata(0, "when: iron.hardness > 5000 then: spark"));

        let report = optimize(&mut ast);

        assert_eq!(report.materials_pruned, 0);
        assert_eq!(ast.materials.len(), 1);
        assert_eq!(ast.materials[0].name_str(), "iron");
    }

    // -- AC-2.2: Disconnected sockets ----------------------------------------

    #[test]
    fn prune_disconnected_socket() {
        let mut ast = VixelAst::new();
        // Spatial A has socket (1, 0, 0)
        // Spatial B has socket (-1, 0, 0) — partner of A's socket
        // Spatial A also has socket (0, 1, 0) — no partner anywhere
        ast.spatials
            .push(make_spatial(0, &[(1, 0, 0), (0, 1, 0)]));
        ast.spatials.push(make_spatial(1, &[(-1, 0, 0)]));

        let report = optimize(&mut ast);

        // (0, 1, 0) has no partner → pruned
        assert_eq!(report.sockets_pruned, 1);
        // Spatial A should have 1 socket remaining
        assert_eq!(ast.spatials[0].socket_count, 1);
        assert_eq!(ast.spatials[0].sockets[0], (1, 0, 0));
        // Spatial B keeps its socket (partner exists in A)
        assert_eq!(ast.spatials[1].socket_count, 1);
    }

    // -- AC-2.3: Unreachable automata ----------------------------------------

    #[test]
    fn prune_unreachable_automata_empty_wgsl() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(0, "when: fire then: burn"));
        ast.automata.push(make_automata(1, ""));
        ast.automata.push(make_automata(2, "   \n\t  "));

        let report = optimize(&mut ast);

        assert_eq!(report.automata_pruned, 2);
        assert_eq!(ast.automata.len(), 1);
        assert_eq!(ast.automata[0].id, 0);
    }

    // -- AC-2.4: Idempotency -------------------------------------------------

    #[test]
    fn idempotency_optimize_twice_same_result() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "oak"));
        ast.materials.push(make_material(1, "unused"));
        ast.automata
            .push(make_automata(0, "when: oak.mass > 0 then: grow"));
        ast.automata.push(make_automata(1, ""));
        ast.spatials
            .push(make_spatial(0, &[(1, 0, 0), (0, 1, 0)]));
        ast.spatials.push(make_spatial(1, &[(-1, 0, 0)]));

        // First pass
        let report1 = optimize(&mut ast);
        let ast_after_first = ast.clone();

        // Second pass — should produce zero changes
        let report2 = optimize(&mut ast);

        assert_eq!(ast, ast_after_first, "AST changed on second optimize");
        assert_eq!(report2.materials_pruned, 0);
        assert_eq!(report2.sockets_pruned, 0);
        assert_eq!(report2.automata_pruned, 0);

        // First pass should have pruned something
        assert!(
            report1.materials_pruned > 0
                || report1.sockets_pruned > 0
                || report1.automata_pruned > 0
        );
    }

    // -- AC-2.5: Accurate PruneReport ----------------------------------------

    #[test]
    fn prune_report_counts_are_accurate() {
        let mut ast = VixelAst::new();
        // 3 materials: "fire" referenced, "water" not, "air" not
        ast.materials.push(make_material(0, "fire"));
        ast.materials.push(make_material(1, "water"));
        ast.materials.push(make_material(2, "air"));
        // 2 automata: one valid referencing "fire", one empty
        ast.automata
            .push(make_automata(0, "when: fire.temp > 9000 then: spread"));
        ast.automata.push(make_automata(1, ""));

        // 2 spatials with sockets
        ast.spatials
            .push(make_spatial(0, &[(1, 0, 0), (0, 0, 1)]));
        ast.spatials.push(make_spatial(1, &[(-1, 0, 0)]));

        let report = optimize(&mut ast);

        // Automata: 1 empty rule pruned
        assert_eq!(report.automata_pruned, 1);
        // Materials: "water" and "air" unreferenced (only "fire" in remaining automata)
        assert_eq!(report.materials_pruned, 2);
        // Sockets: (0,0,1) has no partner → pruned
        assert_eq!(report.sockets_pruned, 1);
    }

    // -- Empty AST -----------------------------------------------------------

    #[test]
    fn empty_ast_produces_zero_count_report() {
        let mut ast = VixelAst::new();
        let report = optimize(&mut ast);

        assert_eq!(report.materials_pruned, 0);
        assert_eq!(report.sockets_pruned, 0);
        assert_eq!(report.automata_pruned, 0);
    }
}
