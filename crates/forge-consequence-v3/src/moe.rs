//! Ported from F:\NewRepo\crates\forge-consequence\src\moe.rs (2026-08-17 truth-hunt
//! lineage port, completing the 2026-08-13 wce-tags-port; moved out of forge-core-v3
//! into this sibling crate 2026-08-17 — see forge-consequence-v3/Cargo.toml).
//!
//! Two adaptations forced by the v3 primitive this ports onto (NOT forced by
//! Crate Zero — this crate is a sibling, not Crate Zero itself):
//!
//! - v2's `forge_hal::expert_pool::MoeRouter<TOTAL_CELLS, T>` pinned the query
//!   width at a free constant `MOE_QUERY_BYTES = 16`. v3's
//!   `forge_hal_clockspine::expert_pool::MoeRouter<TOTAL_CELLS, QUERY_BYTES, T>`
//!   (ported 2026-08-13, `bq_ep16` weld) generalized query width to a const
//!   generic — this file pins it back to 16 via the local `MOE_QUERY_BYTES`
//!   const and threads it through every generic instantiation. `hamming_16`
//!   became the width-generic `hamming::<16>`.
//! - v2's `binarize_query` used `bytemuck::bytes_of(q)`. v3's
//!   `InteractionQuery` (`forge-core-v3/src/consequence/query.rs`) has no
//!   `Pod`/`Zeroable` derive by design — Crate Zero dropped it (no `bytemuck`
//!   dependency there; see that file's own doc comment). `binarize_query`
//!   here packs the same 16-byte wire layout by hand, field by field, from
//!   the offset table `query.rs` documents on `InteractionQuery` — same
//!   bytes, no `Pod` requirement.
//!
//! MoE router — 49-cell `(source_family, target_family)` fallback (matches
//! Invention #169 HierarchicalMoE 7×7 topology).
//!
//! Used by the dispatcher when the precompiled curve table has no match for
//! a query's `(src_family, src_tag, tgt_family, tgt_tag)` tuple. Provides a
//! "learned default" so unknown combinations still produce a sensible
//! `Consequence`.
//!
//! Algorithm (mirrors `forge-ml::bq_router::BqRouter`):
//!
//! 1. Binarize the 16-byte query → 128 bits (one bit per source byte's MSB).
//! 2. For each active cell, XOR + POPCNT against the cell's centroid bits.
//! 3. Pick the minimum-hamming-distance cell.
//! 4. Return that cell's stored default `Consequence`.
//!
//! Training (`train_from_table`) seeds the centroids from the precompiled
//! `CurveTable`: for each curve, binarize a prototypical query (the curve's
//! own (src, tgt) tuple) and store as the centroid for that cell. After
//! training, untrained cells stay inactive (router returns `None`).

use forge_hal_clockspine::expert_pool::MoeRouter as HalMoeRouter;

use forge_core_v3::consequence::curves::CurveTable;
use forge_core_v3::consequence::query::{Consequence, InteractionQuery};

/// Query width in bytes — pins the v3 generic router back to v2's fixed width.
pub const MOE_QUERY_BYTES: usize = 16;

/// Number of source families.
pub const N_SRC_FAMILIES: usize = 7;
/// Number of target families.
pub const N_TGT_FAMILIES: usize = 7;
/// Total cells (matches Invention #169 HierarchicalMoE TOTAL_EXPERTS = 49).
pub const N_CELLS: usize = N_SRC_FAMILIES * N_TGT_FAMILIES;

/// One MoE cell — a binarized centroid + a stored default consequence.
///
/// Thin re-export over `forge_hal_clockspine::expert_pool::MoeCell<16,
/// Consequence>` so the public WCE API keeps its old shape (`bits`,
/// `default_consequence`, `active`). The underlying storage in the router IS
/// `forge_hal_clockspine::MoeCell<16, Consequence>`, which uses the field
/// name `payload` — see `cells()` for the field-name translation.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoeCell {
    /// Binarized query centroid (16 bytes = 128 bits, one bit per query MSB).
    pub bits: [u8; MOE_QUERY_BYTES],
    /// Default consequence emitted when this cell is the nearest match.
    pub default_consequence: Consequence,
    /// `false` until trained. Inactive cells are skipped during routing.
    pub active: bool,
}

/// The 49-cell MoE router.
///
/// Wraps the generic `forge_hal_clockspine::expert_pool::MoeRouter<49, 16,
/// Consequence>` primitive. Public API (`empty`, `from_table`,
/// `train_from_table`, `route`, `active_count`, `cells`) is preserved from
/// the pre-Phase-28 implementation so dispatcher + tests need no changes.
#[derive(Clone, Debug)]
pub struct MoeRouter {
    inner: HalMoeRouter<N_CELLS, MOE_QUERY_BYTES, Consequence>,
}

impl Default for MoeRouter {
    fn default() -> Self {
        Self::empty()
    }
}

impl MoeRouter {
    /// Construct with all cells inactive.
    pub fn empty() -> Self {
        Self { inner: HalMoeRouter::empty() }
    }

    /// Snapshot of the cell array in this router (in WCE field-naming).
    ///
    /// Allocates a fresh `[MoeCell; N_CELLS]` from the inner
    /// `forge_hal_clockspine::MoeCell<16, Consequence>` array — cheap (49
    /// cells × ~32 bytes). Use this only outside the hot path (tests,
    /// diagnostics).
    pub fn cells(&self) -> [MoeCell; N_CELLS] {
        let mut out = [MoeCell::default(); N_CELLS];
        for (i, cell) in self.inner.cells.iter().enumerate() {
            out[i] = MoeCell {
                bits: cell.bits,
                default_consequence: cell.payload,
                active: cell.active,
            };
        }
        out
    }

    /// Construct + train from the precompiled curve table. For each curve,
    /// binarize a prototypical query (from the curve's own (src, tgt) tuple)
    /// and store the curve's first-stage consequence as the cell default.
    pub fn from_table(table: &CurveTable) -> Self {
        let mut router = Self::empty();
        router.train_from_table(table);
        router
    }

    /// Train cells from the curve table. Idempotent — re-running with the
    /// same table produces the same centroids.
    pub fn train_from_table(&mut self, table: &CurveTable) {
        for id in 0..table.len() as u16 {
            let curve = match table.get(id) {
                Some(c) => c,
                None => continue,
            };
            let proto = InteractionQuery {
                source_tag: curve.src_tag,
                source_family: curve.src_family,
                target_tag: curve.tgt_tag,
                target_family: curve.tgt_family,
                intensity_pmy: 10_000,
                material_id: 0,
                resonance_pmy: curve.resonance_floor_pmy,
                target_state: 0,
                chain_depth: 0,
                context_celestial: 0,
                velocity_pmy: 0,
                faction: 0,
                relationship: 128,
            };
            let bits = binarize_query(&proto);
            let consequence = Consequence {
                kind: curve.consequence_on_stage[0] as u8,
                new_state: 1,
                catalytic_pmy: curve.catalytic_on_stage_pmy[0],
                debris_count: 0,
                sound_db: curve.sound_on_stage_db[0],
                plume_kind: curve.plume_on_stage[0],
                flags: 0,
            };
            let idx = cell_index(curve.src_family, curve.tgt_family);
            self.inner.train_cell(idx, bits, consequence);
        }
    }

    /// Route a query to the nearest active cell. Returns the cell's stored
    /// default `Consequence`, or `None` if no cell is active (router has
    /// not been trained).
    ///
    /// Takes `&self` to preserve the pre-Phase-28 signature. Internally
    /// delegates to `forge_hal_clockspine::MoeRouter::peek`, which does not
    /// increment the dispatched-count telemetry; dispatched-count tracking
    /// lives on the dispatcher, not the router.
    pub fn route(&self, q: &InteractionQuery) -> Option<Consequence> {
        let q_bits = binarize_query(q);
        self.inner.peek(&q_bits)
    }

    /// Number of active (trained) cells.
    pub fn active_count(&self) -> usize {
        self.inner.active_count()
    }
}

/// Compute the cell index for a `(src_family, tgt_family)` pair.
#[inline]
pub fn cell_index(src_family: u8, tgt_family: u8) -> usize {
    src_family as usize * N_TGT_FAMILIES + tgt_family as usize
}

/// Binarize a 16-byte query by packing `InteractionQuery`'s fields by hand
/// into its documented wire layout (see `forge-core-v3/src/consequence/
/// query.rs`'s offset table on `InteractionQuery`). No `bytemuck` — v3's
/// `InteractionQuery` carries no `Pod`/`Zeroable` derive by design.
fn binarize_query(q: &InteractionQuery) -> [u8; MOE_QUERY_BYTES] {
    let mut out = [0u8; MOE_QUERY_BYTES];
    out[0] = q.source_tag;
    out[1] = q.source_family;
    out[2] = q.target_tag;
    out[3] = q.target_family;
    out[4..6].copy_from_slice(&q.intensity_pmy.to_le_bytes());
    out[6..8].copy_from_slice(&q.material_id.to_le_bytes());
    out[8..10].copy_from_slice(&q.resonance_pmy.to_le_bytes());
    out[10] = q.target_state;
    out[11] = q.chain_depth;
    out[12] = q.context_celestial;
    out[13] = q.velocity_pmy;
    out[14] = q.faction;
    out[15] = q.relationship;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::consequence::query::ConsequenceKind;
    use forge_core_v3::consequence::tags::*;
    use forge_hal_clockspine::expert_pool::hamming;

    #[test]
    fn empty_router_returns_none() {
        let r = MoeRouter::empty();
        let q = InteractionQuery::default();
        assert!(r.route(&q).is_none());
    }

    #[test]
    fn trained_router_has_active_cells() {
        let table = CurveTable::full();
        let r = MoeRouter::from_table(&table);
        // 12 curves → at least 12 active cells (some collide on
        // (src_family, tgt_family); count should be ≤ 12 and ≥ ~8).
        let active = r.active_count();
        assert!(active >= 6, "expected ≥6 active cells, got {active}");
        assert!(active <= N_CELLS, "active cells should not exceed total");
    }

    #[test]
    fn router_picks_matching_cell_for_known_query() {
        let table = CurveTable::full();
        let r = MoeRouter::from_table(&table);
        // Query that exactly matches the water-on-stone curve's prototype.
        let q = InteractionQuery {
            source_tag: SRC_WATER_FLOW,
            source_family: SRC_FAMILY_FLUID,
            target_tag: TGT_STONE,
            target_family: TGT_FAMILY_TERRAIN,
            intensity_pmy: 10_000,
            ..InteractionQuery::default()
        };
        // First-stage consequence for water-on-stone is None (state 0→1 has
        // None at index 0). MoE should still route deterministically.
        let consequence = r.route(&q);
        assert!(consequence.is_some(), "trained router should route a known query");
    }

    #[test]
    fn router_picks_fire_consequence_for_fire_on_wood_query() {
        let table = CurveTable::full();
        let r = MoeRouter::from_table(&table);
        let q = InteractionQuery {
            source_tag: SRC_FIRE,
            source_family: SRC_FAMILY_FIRE,
            target_tag: TGT_WOOD,
            target_family: TGT_FAMILY_TERRAIN,
            intensity_pmy: 10_000,
            ..InteractionQuery::default()
        };
        let c = r.route(&q).expect("router should match (fire, terrain) cell");
        // fire-on-wood first stage is Ignite.
        assert_eq!(c.kind(), ConsequenceKind::Ignite);
    }

    #[test]
    fn router_picks_lightning_consequence_for_lightning_on_entity() {
        let table = CurveTable::full();
        let r = MoeRouter::from_table(&table);
        let q = InteractionQuery {
            source_tag: SRC_LIGHTNING,
            source_family: SRC_FAMILY_GRAVITY,
            target_tag: TGT_ENTITY_ALIVE,
            target_family: TGT_FAMILY_ENTITY,
            intensity_pmy: 10_000,
            ..InteractionQuery::default()
        };
        let c = r.route(&q).expect("router should match (gravity, entity) cell");
        assert_eq!(c.kind(), ConsequenceKind::Damage);
    }

    #[test]
    fn cell_index_packs_seven_by_seven() {
        // Spot-check the row-major layout.
        assert_eq!(cell_index(0, 0), 0);
        assert_eq!(cell_index(0, 6), 6);
        assert_eq!(cell_index(1, 0), 7);
        assert_eq!(cell_index(6, 6), 48);
    }

    #[test]
    fn hamming_self_distance_zero() {
        let a = [0x42u8; 16];
        // Delegated to forge-hal-clockspine — verifies the integer XOR + popcount.
        assert_eq!(hamming(&a, &a), 0);
    }

    #[test]
    fn hamming_inverted_distance_full() {
        let a = [0x00u8; 16];
        let b = [0xFFu8; 16];
        assert_eq!(hamming(&a, &b), 128); // 16 bytes × 8 bits
    }

    #[test]
    fn router_is_deterministic_across_runs() {
        let table = CurveTable::full();
        let r1 = MoeRouter::from_table(&table);
        let r2 = MoeRouter::from_table(&table);
        let q = InteractionQuery {
            source_tag: SRC_FIRE, source_family: SRC_FAMILY_FIRE,
            target_tag: TGT_WOOD, target_family: TGT_FAMILY_TERRAIN,
            intensity_pmy: 10_000, ..InteractionQuery::default()
        };
        assert_eq!(r1.route(&q), r2.route(&q));
    }
}
