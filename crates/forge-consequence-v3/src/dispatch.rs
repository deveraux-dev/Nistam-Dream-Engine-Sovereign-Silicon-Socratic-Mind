//! Ported from F:\NewRepo\crates\forge-consequence\src\dispatch.rs (2026-08-17
//! truth-hunt lineage port — the named blocker is now resolved: forge-physics-v3
//! gained a scoped `types.rs` port covering exactly {MaterialId, MilliUnit,
//! PhysicsEffect, PlumeSource, SoundSource} this file needs, see that
//! module's doc comment for scope).
//!
//! Dispatcher — drives the per-tick WCE loop.
//!
//! Each tick:
//!
//! ```text
//! 1. release deferred PendingInteractions (each carries its own cell_id + position)
//! 2. for each new PendingInteraction:
//!      - budget.admit
//!      - if admitted: look up curve from (src, tgt) tuple
//!      - if curve found: step the cell's persisted (counter, state)
//!      - if threshold crossed: emit Consequence + map to PhysicsEffect at cell position
//! 3. return DispatchReport
//! ```
//!
//! `Dispatcher` owns per-cell state in a `BTreeMap<cell_id, CellState>` for
//! determinism (iteration order is deterministic, unlike HashMap). The map
//! survives across ticks so deferred queries reattach to the right cell on
//! release.
//!
//! Consequence → `PhysicsEffect` mapping happens in `consequence_to_effect`,
//! producing existing `forge_physics_v3::types::PhysicsEffect` variants.

use std::collections::BTreeMap;

use forge_physics_v3::types::{MaterialId, PhysicsEffect, PlumeSource, SoundSource};
use forge_physics_v3::types::MilliUnit as PhysMilliUnit;

use forge_core_v3::consequence::budget::{BudgetOutcome, InteractionBudget};
use forge_core_v3::consequence::curves::{CurveTable, NO_CURVE};
use forge_core_v3::consequence::query::{Consequence, ConsequenceKind, PendingInteraction};
use forge_core_v3::fixed_point::MilliUnit;

use crate::moe::MoeRouter;

/// `PendingInteraction::position` (this crate's `PhysicsEffect` variants)
/// use two nominally distinct-but-same-shape `MilliUnit` types:
/// `forge-core-v3::fixed_point::MilliUnit` (what `query.rs`'s
/// `PendingInteraction` was ported against, see that module's doc comment)
/// and `forge-physics-v3::types::MilliUnit` (re-exported from `pp-math-v3`,
/// what the v2 donor's `PhysicsEffect` enum used and what `types.rs` here
/// ported verbatim). Both are plain `i64` newtypes — this is the one
/// conversion point between them.
#[inline]
fn to_phys_pos(p: [MilliUnit; 3]) -> [PhysMilliUnit; 3] {
    [PhysMilliUnit(p[0].0), PhysMilliUnit(p[1].0), PhysMilliUnit(p[2].0)]
}

/// Per-cell mutable state. Dispatcher owns; survives across ticks.
#[derive(Clone, Copy, Debug, Default)]
pub struct CellState {
    /// Accumulated counter for the active curve.
    pub counter: u32,
    /// Current degradation/growth stage (0..=255).
    pub state: u8,
}

/// Outcome of a `Dispatcher::tick` call.
#[derive(Debug, Default)]
pub struct DispatchReport {
    /// Consequences produced this tick. One per `PendingInteraction` that
    /// crossed a threshold.
    pub consequences: Vec<Consequence>,
    /// `PhysicsEffect`s produced from `consequences`, ready for handler
    /// fan-out (fire.rs / structural.rs / sound_consumer.rs / VRAMBridge).
    pub effects: Vec<PhysicsEffect>,
    /// Queries the budget admitted (counted against per-tick cap).
    pub admitted: u32,
    /// Queries parked in the defer queue this tick.
    pub deferred: u32,
    /// Queries dropped because both per-tick cap AND defer queue were full.
    pub dropped: u32,
    /// Queries with no matching curve (skipped — accounted separately so
    /// "no curve" doesn't masquerade as "consequence fired").
    pub skipped_no_curve: u32,
}

impl DispatchReport {
    /// Reset all fields for reuse across ticks: counters zeroed, `Vec`s emptied
    /// but their heap capacity retained (so the reused report stays alloc-free
    /// on the hot path). Used by [`Dispatcher::tick_into`].
    pub fn clear(&mut self) {
        self.consequences.clear();
        self.effects.clear();
        self.admitted = 0;
        self.deferred = 0;
        self.dropped = 0;
        self.skipped_no_curve = 0;
    }
}

/// The WCE dispatcher.
///
/// `Clone` is load-bearing for the U4 world-scrub snapshot ring (the app clones
/// the whole `WorldSim`, and the dispatcher's `cells` accumulator must ride
/// along — rebuilding it fresh would drop per-cell state and break the rewind).
#[derive(Clone)]
pub struct Dispatcher {
    table: CurveTable,
    budget: InteractionBudget,
    cells: BTreeMap<u32, CellState>,
    /// Optional MoE fallback. When present, queries that miss the curve
    /// table are routed to the nearest learned cell. `None` = no fallback,
    /// unknown queries land in `skipped_no_curve`.
    moe: Option<MoeRouter>,
}

impl Dispatcher {
    /// Construct with the FULL curve table (3 core + 9 bridges), default
    /// budget (200/tick, 512 defer), and no MoE fallback.
    pub fn new_full() -> Self {
        Self {
            table: CurveTable::full(),
            budget: InteractionBudget::default_budget(),
            cells: BTreeMap::new(),
            moe: None,
        }
    }

    /// Construct with the FULL curve table + MoE fallback trained from that
    /// same table. Use this for production paths where unknown (src, tgt)
    /// pairs should still produce sensible consequences.
    pub fn new_full_with_moe() -> Self {
        let table = CurveTable::full();
        let moe = Some(MoeRouter::from_table(&table));
        Self {
            table,
            budget: InteractionBudget::default_budget(),
            cells: BTreeMap::new(),
            moe,
        }
    }

    /// Construct with the MVP-3 curve table only. Use for tests that need
    /// minimal coverage.
    pub fn mvp() -> Self {
        Self {
            table: CurveTable::mvp(),
            budget: InteractionBudget::default_budget(),
            cells: BTreeMap::new(),
            moe: None,
        }
    }

    /// Construct with explicit table + budget. No MoE.
    pub fn with_parts(table: CurveTable, budget: InteractionBudget) -> Self {
        Self { table, budget, cells: BTreeMap::new(), moe: None }
    }

    /// Attach a MoE router as fallback for unknown queries. Returns the
    /// dispatcher for chaining.
    pub fn with_moe(mut self, moe: MoeRouter) -> Self {
        self.moe = Some(moe);
        self
    }

    /// Borrow the curve table.
    #[inline]
    pub fn table(&self) -> &CurveTable { &self.table }

    /// Borrow the budget.
    #[inline]
    pub fn budget(&self) -> &InteractionBudget { &self.budget }

    /// Borrow the optional MoE router.
    #[inline]
    pub fn moe(&self) -> Option<&MoeRouter> { self.moe.as_ref() }

    /// Number of distinct cells the dispatcher has tracked.
    #[inline]
    pub fn cell_count(&self) -> usize { self.cells.len() }

    /// Peek at a cell's persisted state.
    pub fn cell(&self, id: u32) -> Option<&CellState> { self.cells.get(&id) }

    /// Drop a cell from the persistent map (caller signals the cell is no
    /// longer active — e.g. plant died, voxel destroyed).
    pub fn forget_cell(&mut self, id: u32) -> Option<CellState> { self.cells.remove(&id) }

    /// Reset all cell state. Useful for tests + game restart.
    pub fn clear_cells(&mut self) { self.cells.clear(); }

    /// Run one tick. Caller provides a batch of `PendingInteraction`s; the
    /// dispatcher handles routing, budget gating, threshold crossing, and
    /// consequence emission.
    ///
    /// Convenience wrapper that allocates a fresh `DispatchReport` (two `Vec`s)
    /// per call. The per-frame WCE loop should prefer
    /// [`tick_into`](Self::tick_into) with a reused report to stay
    /// allocation-free on the hot path.
    pub fn tick(&mut self, interactions: &[PendingInteraction]) -> DispatchReport {
        let mut report = DispatchReport::default();
        self.tick_into(interactions, &mut report);
        report
    }

    /// Alloc-free variant of [`tick`](Self::tick): writes this tick's outcome
    /// into the caller-owned `report` instead of allocating a fresh one. The
    /// report is cleared first (counters zeroed, `Vec`s emptied but capacity
    /// retained), so a single `DispatchReport` reused across frames performs
    /// zero heap allocations once its buffers are warm. Mirrors the
    /// `SieveRunner::tick_into` caller-buffer contract.
    pub fn tick_into(&mut self, interactions: &[PendingInteraction], report: &mut DispatchReport) {
        report.clear();

        // Step 1: release deferred from previous tick first — they share the
        // new tick's cap. Each released item carries its own cell_id + position.
        let released = self.budget.next_tick();
        report.admitted += released.len() as u32;
        for p in &released {
            apply_pending(p, &self.table, self.moe.as_ref(), &mut self.cells, report);
        }

        // Step 2: admit new interactions.
        for p in interactions {
            match self.budget.admit(*p) {
                BudgetOutcome::Admitted => {
                    report.admitted += 1;
                    apply_pending(p, &self.table, self.moe.as_ref(), &mut self.cells, report);
                }
                BudgetOutcome::Deferred => {
                    report.deferred += 1;
                }
                BudgetOutcome::Dropped => {
                    report.dropped += 1;
                }
                BudgetOutcome::EvictedWeakest => {
                    // Defer count unchanged; one weakest was dropped, one
                    // strong took its slot.
                    report.deferred += 1;
                    report.dropped += 1;
                }
            }
        }
    }
}

/// Look up the curve for `p`'s query, step the cell's persisted counter,
/// and append any produced consequence + effect to `report`. Falls back to
/// the MoE router if the curve table has no match and a router is attached.
fn apply_pending(
    p: &PendingInteraction,
    table: &CurveTable,
    moe: Option<&MoeRouter>,
    cells: &mut BTreeMap<u32, CellState>,
    report: &mut DispatchReport,
) {
    let q = &p.query;
    let id = table.lookup(q.source_family, q.source_tag, q.target_family, q.target_tag);
    if id != NO_CURVE {
        if let Some(curve) = table.get(id) {
            let cell = cells.entry(p.cell_id).or_default();
            if let Some(c) = curve.step(
                &mut cell.counter,
                &mut cell.state,
                q.intensity_pmy,
                q.resonance_pmy,
            ) {
                let mat: MaterialId = q.material_id;
                let effect = consequence_to_effect(&c, mat, p.position);
                report.consequences.push(c);
                if let Some(e) = effect { report.effects.push(e); }
            }
            return;
        }
    }

    // Curve table miss. Fall back to MoE if attached.
    if let Some(router) = moe {
        if let Some(c) = router.route(q) {
            let mat: MaterialId = q.material_id;
            let effect = consequence_to_effect(&c, mat, p.position);
            report.consequences.push(c);
            if let Some(e) = effect { report.effects.push(e); }
            return;
        }
    }

    report.skipped_no_curve += 1;
}

/// Map a WCE `Consequence` to a `forge_physics_v3::types::PhysicsEffect` for
/// handler fan-out.
pub fn consequence_to_effect(
    c: &Consequence,
    material_id: MaterialId,
    position: [MilliUnit; 3],
) -> Option<PhysicsEffect> {
    let position = to_phys_pos(position);
    match c.kind() {
        ConsequenceKind::None => None,
        ConsequenceKind::Shatter => Some(PhysicsEffect::Shatter {
            entity_id: 0,
            material: material_id,
            fragments: c.debris_count as u32,
        }),
        ConsequenceKind::Ignite => Some(PhysicsEffect::FireIgnite {
            position,
            material_id,
        }),
        ConsequenceKind::Extinguish => Some(PhysicsEffect::FireExtinguish { fire_id: 0 }),
        ConsequenceKind::VoxelBreak => Some(PhysicsEffect::VoxelBreak {
            position,
            material_id,
        }),
        ConsequenceKind::VoxelPlace => Some(PhysicsEffect::VoxelPlace {
            position,
            material_id,
        }),
        ConsequenceKind::Sound => Some(PhysicsEffect::SoundEvent {
            position,
            intensity_db: PhysMilliUnit((c.sound_db as i64) * 500),
            source: SoundSource::Impact,
        }),
        ConsequenceKind::Plume => Some(PhysicsEffect::PlumeSpawn {
            position,
            source: match c.plume_kind {
                1 => PlumeSource::Fire,
                // 2 = steam (placeholder: PlumeSource has no Steam variant; map to Fire).
                2 => PlumeSource::Fire,
                3 => PlumeSource::Dust,
                4 => PlumeSource::Spores,
                _ => PlumeSource::Supernatural,
            },
        }),
        ConsequenceKind::Damage => Some(PhysicsEffect::Damage {
            target_id: 0,
            amount: PhysMilliUnit(c.catalytic_pmy as i64 * 100), // Permyriad → milli-joules
            source: forge_physics_v3::types::DamageSource::Impact,
        }),
        ConsequenceKind::Visibility => Some(PhysicsEffect::VisibilityChange {
            zone_id: 0,
            // catalytic_pmy is "how much transmissivity reduction" — clamp
            // to Permyriad valid range.
            transmissivity: forge_physics_v3::types::Permyriad(
                10_000i32.saturating_sub(c.catalytic_pmy as i32).max(0)
            ),
        }),
        // sem-δ will wire these to PhysicsEffect variants; stubs return None until then.
        ConsequenceKind::Reveal
        | ConsequenceKind::MemoryStabilize
        | ConsequenceKind::WitnessTrace => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::consequence::query::InteractionQuery;
    use forge_core_v3::consequence::tags::*;

    /// Builds a `PendingInteraction` position (`forge-core-v3::fixed_point::MilliUnit`).
    fn pos(x: i64) -> [MilliUnit; 3] { [MilliUnit(x), MilliUnit(0), MilliUnit(0)] }
    /// Builds a `PhysicsEffect`-comparison position (`forge-physics-v3::types::MilliUnit`).
    fn phys_pos(x: i64) -> [PhysMilliUnit; 3] { [PhysMilliUnit(x), PhysMilliUnit(0), PhysMilliUnit(0)] }

    fn water_on_stone(cell_id: u32, intensity: u16) -> PendingInteraction {
        PendingInteraction::new(
            cell_id,
            pos(cell_id as i64),
            InteractionQuery {
                source_tag: SRC_WATER_FLOW,
                source_family: SRC_FAMILY_FLUID,
                target_tag: TGT_STONE,
                target_family: TGT_FAMILY_TERRAIN,
                intensity_pmy: intensity,
                material_id: 1,
                ..InteractionQuery::default()
            },
        )
    }

    fn sound_on_stone(cell_id: u32, intensity: u16, resonance: u16) -> PendingInteraction {
        PendingInteraction::new(
            cell_id,
            pos(cell_id as i64),
            InteractionQuery {
                source_tag: SRC_SOUND,
                source_family: SRC_FAMILY_SOUND,
                target_tag: TGT_STONE,
                target_family: TGT_FAMILY_TERRAIN,
                intensity_pmy: intensity,
                resonance_pmy: resonance,
                material_id: 1,
                ..InteractionQuery::default()
            },
        )
    }

    fn lightning_on_wood(cell_id: u32) -> PendingInteraction {
        PendingInteraction::new(
            cell_id,
            pos(cell_id as i64),
            InteractionQuery {
                source_tag: SRC_LIGHTNING,
                source_family: SRC_FAMILY_GRAVITY,
                target_tag: TGT_WOOD,
                target_family: TGT_FAMILY_TERRAIN,
                intensity_pmy: 10_000,
                material_id: 2,
                ..InteractionQuery::default()
            },
        )
    }

    fn fire_on_wood(cell_id: u32) -> PendingInteraction {
        PendingInteraction::new(
            cell_id,
            pos(cell_id as i64),
            InteractionQuery {
                source_tag: SRC_FIRE,
                source_family: SRC_FAMILY_FIRE,
                target_tag: TGT_WOOD,
                target_family: TGT_FAMILY_TERRAIN,
                intensity_pmy: 10_000,
                material_id: 2,
                ..InteractionQuery::default()
            },
        )
    }

    #[test]
    fn dispatcher_admits_under_cap() {
        let mut d = Dispatcher::new_full();
        let batch: Vec<_> = (0..50).map(|i| water_on_stone(i, 10_000)).collect();
        let r = d.tick(&batch);
        assert_eq!(r.admitted, 50);
        assert_eq!(r.deferred, 0);
        assert_eq!(r.dropped, 0);
        assert_eq!(r.skipped_no_curve, 0);
    }

    #[test]
    fn dispatcher_skips_unknown_curve() {
        let mut d = Dispatcher::new_full();
        let unknown = PendingInteraction::new(
            0, pos(0),
            InteractionQuery {
                source_tag: 99, source_family: SRC_FAMILY_ENTITY,
                target_tag: 99, target_family: TGT_FAMILY_ATMOSPHERE,
                intensity_pmy: 10_000, ..InteractionQuery::default()
            },
        );
        let r = d.tick(&[unknown]);
        assert_eq!(r.skipped_no_curve, 1);
        assert!(r.consequences.is_empty());
        assert!(r.effects.is_empty());
    }

    #[test]
    fn dispatcher_fires_shatter_after_threshold() {
        let mut d = Dispatcher::new_full();
        let mut shatter_seen = false;
        for _ in 0..30_000 {
            let r = d.tick(&[sound_on_stone(7, 10_000, 10_000)]);
            if r.consequences.iter().any(|c| c.kind() == ConsequenceKind::Shatter) {
                shatter_seen = true;
                assert!(r.effects.iter().any(|e| matches!(e, PhysicsEffect::Shatter { .. })));
                break;
            }
        }
        assert!(shatter_seen);
    }

    #[test]
    fn dispatcher_emits_ignite_for_fire_on_wood() {
        let mut d = Dispatcher::new_full();
        let mut ignite_seen = false;
        for _ in 0..50 {
            let r = d.tick(&[fire_on_wood(1)]);
            if r.consequences.iter().any(|c| c.kind() == ConsequenceKind::Ignite) {
                ignite_seen = true;
                assert!(r.effects.iter().any(|e| matches!(e, PhysicsEffect::FireIgnite { .. })));
                break;
            }
        }
        assert!(ignite_seen);
    }

    #[test]
    fn dispatcher_lightning_fires_instantly() {
        let mut d = Dispatcher::new_full();
        let r = d.tick(&[lightning_on_wood(42)]);
        assert!(r.consequences.iter().any(|c| c.kind() == ConsequenceKind::Ignite),
            "lightning should fire on the FIRST tick");
        assert!(r.effects.iter().any(|e| matches!(e, PhysicsEffect::FireIgnite { .. })));
    }

    #[test]
    fn dispatcher_persists_cell_state_across_ticks() {
        let mut d = Dispatcher::new_full();
        // 10 ticks @ fire-on-wood; cell state should accumulate.
        for _ in 0..10 {
            let _ = d.tick(&[fire_on_wood(99)]);
        }
        let cell = d.cell(99).expect("cell 99 should be tracked");
        assert!(cell.counter > 0 || cell.state > 0);
    }

    #[test]
    fn dispatcher_per_cell_position_threads_through_to_effects() {
        // Ignite at cell 5 → emit FireIgnite at position (5, 0, 0).
        let mut d = Dispatcher::new_full();
        let r = d.tick(&[lightning_on_wood(5)]);
        let found = r.effects.iter().find_map(|e| match e {
            PhysicsEffect::FireIgnite { position, .. } => Some(*position),
            _ => None,
        });
        assert_eq!(found, Some(phys_pos(5)));
    }

    #[test]
    fn dispatcher_releases_deferred_with_preserved_cell_binding() {
        // Build a dispatcher with cap=2 so admits >2 → defer.
        let mut d = Dispatcher::with_parts(CurveTable::full(), InteractionBudget::new(2, 4));
        // Tick 1: submit 5 lightning interactions. 2 admit (instant ignite),
        // 3 defer.
        let batch: Vec<_> = (1..=5).map(lightning_on_wood).collect();
        let r1 = d.tick(&batch);
        assert_eq!(r1.admitted, 2);
        assert_eq!(r1.deferred, 3);
        assert_eq!(r1.consequences.iter().filter(|c| c.kind() == ConsequenceKind::Ignite).count(), 2);
        // Tick 2: nothing new. Budget releases 2 of the 3 deferred. Each
        // released item must reattach to its original cell_id, emitting
        // FireIgnite at the cell's position.
        let r2 = d.tick(&[]);
        assert_eq!(r2.admitted, 2);
        let positions: Vec<_> = r2.effects.iter().filter_map(|e| match e {
            PhysicsEffect::FireIgnite { position, .. } => Some(*position),
            _ => None,
        }).collect();
        // The released items should be cells 3, 4 (FIFO from the defer queue
        // after cells 1, 2 admitted in tick 1).
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&phys_pos(3)));
        assert!(positions.contains(&phys_pos(4)));
    }

    #[test]
    fn stress_8k_water_cells_stays_under_budget() {
        let mut d = Dispatcher::new_full();
        let batch: Vec<_> = (0..8_000u32)
            .map(|i| water_on_stone(i, ((i % 10_000) + 1) as u16))
            .collect();
        let r = d.tick(&batch);
        let cap = d.budget().cap_per_tick() as u32;
        let defer_cap = d.budget().defer_cap() as u32;
        assert!(r.admitted <= cap);
        assert!(d.budget().deferred_count() as u32 <= defer_cap);
        let accounted = r.admitted + r.deferred + r.dropped + r.skipped_no_curve;
        assert!(accounted >= 8_000);
    }

    #[test]
    fn determinism_same_seed_same_consequence_stream() {
        fn run() -> Vec<u8> {
            let mut d = Dispatcher::new_full();
            let mut kinds = Vec::new();
            for _ in 0..20_000 {
                let r = d.tick(&[sound_on_stone(0, 10_000, 10_000)]);
                for c in r.consequences { kinds.push(c.kind); }
            }
            kinds
        }
        assert_eq!(run(), run());
    }

    #[test]
    fn forget_cell_drops_persisted_state() {
        let mut d = Dispatcher::new_full();
        let _ = d.tick(&[fire_on_wood(11)]);
        assert!(d.cell(11).is_some());
        let dropped = d.forget_cell(11);
        assert!(dropped.is_some());
        assert!(d.cell(11).is_none());
    }

    // ── MoE fallback tests ─────────────────────────────────────────────────

    #[test]
    fn moe_fallback_routes_known_family_pair_without_specific_curve() {
        // Dispatcher with MoE: unknown specific tag in a known family pair
        // (FIRE × TERRAIN) still emits a sensible consequence via the MoE
        // fallback (nearest cell = fire-on-wood's centroid).
        let mut d = Dispatcher::new_full_with_moe();
        let q = PendingInteraction::new(
            0, pos(0),
            InteractionQuery {
                source_tag: SRC_FIRE, source_family: SRC_FAMILY_FIRE,
                target_tag: TGT_DIRT, // Not wired in any curve.
                target_family: TGT_FAMILY_TERRAIN,
                intensity_pmy: 10_000, material_id: 5,
                ..InteractionQuery::default()
            },
        );
        let r = d.tick(&[q]);
        assert_eq!(r.skipped_no_curve, 0,
            "MoE should route the unknown query, not skip it");
        assert!(!r.consequences.is_empty(),
            "MoE fallback should produce at least one consequence");
    }

    #[test]
    fn moe_disabled_skips_unknown_queries() {
        // Dispatcher without MoE: unknown query lands in skipped_no_curve.
        let mut d = Dispatcher::new_full();
        let q = PendingInteraction::new(
            0, pos(0),
            InteractionQuery {
                source_tag: SRC_FIRE, source_family: SRC_FAMILY_FIRE,
                target_tag: TGT_DIRT, // Not wired in any curve.
                target_family: TGT_FAMILY_TERRAIN,
                intensity_pmy: 10_000, material_id: 5,
                ..InteractionQuery::default()
            },
        );
        let r = d.tick(&[q]);
        assert_eq!(r.skipped_no_curve, 1, "no MoE → unknown is skipped");
        assert!(r.consequences.is_empty());
    }

    #[test]
    fn moe_does_not_interfere_with_known_curves() {
        // Curve table should still take precedence over MoE for known
        // (src, tgt) tuples.
        let mut d = Dispatcher::new_full_with_moe();
        let r = d.tick(&[lightning_on_wood(99)]);
        let ignites = r.consequences.iter().filter(|c| c.kind() == ConsequenceKind::Ignite).count();
        assert_eq!(ignites, 1, "exactly one Ignite from the precise curve, not MoE-doubled");
    }
}
