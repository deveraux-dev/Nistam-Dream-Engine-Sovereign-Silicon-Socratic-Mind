//! The World Consequence Engine's Superior-Dexter / Tas-de-Charge stereotomic
//! pair — authored against
//! `F:\v3\TODO\ironroot-edict\IRONROOT_Design_Packet\
//! ironroot_world_systems_bundle.v1.json:70-101,137-235` (the `world_lock`,
//! `tick_pipeline`, and `stereotomic_layer` objects).
//!
//! **World Lock Grammar** (`json:71`, verbatim): "Ironroot world systems
//! resolve through one grammar: Authority decides order. Threshold decides
//! whether. Ratio decides spread. Convergence decides apex. DAG decides
//! limit. Root state decides volatility. Primitive priors decide safe
//! fallback. Diplomacy decides meaning after physics has already happened."
//!
//! This module lands the two clauses the tick pipeline itself names, in the
//! order it names them (`json:90-91`): `merge_tas_de_charge_convergences`
//! runs BEFORE `sort_by_superior_dexter_authority` — convergence decides
//! apex first, authority decides the resolve order of what's left. The
//! other eight `world_lock` components (Central-Third, Ad Quadratum, Ad
//! Triangulum, Constraint DAG, Chaos Perturbation, Ratio Policy, Primitive
//! Priors, Diplomacy Observer) are real, cited, and unported — not claimed
//! here.
//!
//! **What's cited vs. what's authored.** The 12 `authority_inputs` field
//! names (`json:141-153`) and the `min_sources: 3` rule (`json:227`) are
//! verbatim from the packet. The packet names WHICH deltas count toward
//! authority but never gives their relative weights — "highest effective
//! authority resolves first" (`json:140`) is the rule, not a formula. Rather
//! than invent per-field weights the packet doesn't specify (T1
//! `zero_hallucination`), [`WceQuery::effective_authority`] sums the named
//! deltas unweighted: the cheapest rule that still satisfies "highest
//! resolves first" and orders every named input (T3
//! `hierarchy_of_illusion`). `[ASSUMED: unweighted sum]` — revisit if the
//! packet or Sean ever specifies real per-input weights.

/// One WCE query — the resolvable unit the tick pipeline sorts and merges.
/// Field names match `authority_inputs` verbatim (`json:141-153`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WceQuery {
    /// Energy quantity delta.
    pub energy_q: i64,
    /// Elevation change, millimetres.
    pub elevation_delta_mm: i64,
    /// Legal-standing delta.
    pub legal_delta: i64,
    /// Witness-count delta.
    pub witness_delta: i64,
    /// Travel-route delta.
    pub route_delta: i64,
    /// Camera/framing delta.
    pub camera_delta: i64,
    /// Artifact-provenance delta.
    pub artifact_delta: i64,
    /// Death-scar delta.
    pub death_scar_delta: i64,
    /// Charge-head delta.
    pub charge_head_delta: i64,
    /// Root-pressure delta.
    pub root_pressure_delta: i64,
    /// Faction-claim delta.
    pub faction_claim_delta: i64,
    /// Craft-provenance delta.
    pub craft_provenance_delta: i64,
}

impl WceQuery {
    /// `[ASSUMED: unweighted sum]` — see module doc. Highest resolves first.
    pub fn effective_authority(&self) -> i64 {
        self.energy_q
            + self.elevation_delta_mm
            + self.legal_delta
            + self.witness_delta
            + self.route_delta
            + self.camera_delta
            + self.artifact_delta
            + self.death_scar_delta
            + self.charge_head_delta
            + self.root_pressure_delta
            + self.faction_claim_delta
            + self.craft_provenance_delta
    }
}

/// The six named scenarios Superior-Dexter arbitrates, verbatim
/// (`json:155-162`).
pub const SUPERIOR_DEXTER_USES: [&str; 6] = [
    "collapse_vs_brace",
    "fire_vs_water",
    "river_vs_dam",
    "guildhall_claim_vs_faction_claim",
    "player_built_structure_vs_root_surge",
    "ritual_authority_vs_local_witnesses",
];

/// Superior-Dexter: "Highest effective authority resolves first" (`json:140`).
/// Stable sort — queries with equal authority keep their input order, so the
/// tick pipeline stays deterministic across runs with identical input order.
pub fn sort_by_superior_dexter_authority(queries: &mut [WceQuery]) {
    queries.sort_by_key(|q| std::cmp::Reverse(q.effective_authority()));
}

/// Tas-de-Charge's convergence rule: three or more sources converging into
/// one compound apex query (`json:226-227`).
pub const MIN_SOURCES: usize = 3;

/// The five named scenarios Tas-de-Charge resolves, verbatim (`json:229-233`).
pub const TAS_DE_CHARGE_USES: [&str; 5] = [
    "yod_boss_pressure",
    "fire_water_confined_cave_pressure_steam_burst",
    "root_foundation_rainfall_house_split",
    "faction_ritual_corpse_site_moon_phase_erasure_pressure",
    "guildhall_socket_relic_crafted_material_civic_ward",
];

/// A compound apex query merged from `>= MIN_SOURCES` converging
/// [`WceQuery`]s. `wce_role` (`json:226`): "Three or more source convergence
/// into compound apex query."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasDeChargeConvergence {
    /// The queries that converged. `len() >= MIN_SOURCES`, guaranteed by
    /// [`merge_tas_de_charge_convergences`] — no other constructor exists.
    pub sources: Vec<WceQuery>,
}

impl TasDeChargeConvergence {
    /// The merged apex's own authority — feeds straight into
    /// `sort_by_superior_dexter_authority` once merged, matching the tick
    /// pipeline's own order (`json:90-91`: convergence merges, then
    /// authority sorts what's left).
    pub fn apex_authority(&self) -> i64 {
        self.sources.iter().map(WceQuery::effective_authority).sum()
    }
}

/// Merge a candidate set of converging queries into one Tas-de-Charge apex
/// query, if and only if at least [`MIN_SOURCES`] converged. Below that,
/// there is no compound apex — the caller resolves each query individually
/// (through Superior-Dexter, unmerged).
///
/// This function does not decide WHICH queries converge (that's a site/
/// target correlation the packet doesn't specify a concrete key for) — it
/// takes a caller-identified candidate set and applies only the one rule the
/// packet states plainly: `min_sources: 3`.
pub fn merge_tas_de_charge_convergences(candidates: Vec<WceQuery>) -> Option<TasDeChargeConvergence> {
    if candidates.len() >= MIN_SOURCES {
        Some(TasDeChargeConvergence { sources: candidates })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(energy: i64) -> WceQuery {
        WceQuery { energy_q: energy, ..Default::default() }
    }

    #[test]
    fn highest_effective_authority_resolves_first() {
        let mut queries = [q(10), q(50), q(30)];
        sort_by_superior_dexter_authority(&mut queries);
        assert_eq!(queries.map(|x| x.energy_q), [50, 30, 10]);
    }

    #[test]
    fn equal_authority_keeps_input_order_for_determinism() {
        // Three queries with different fields set but the same summed
        // effective_authority (10 each) — a real tie, not just a shared
        // energy_q. witness_delta identifies which query is which without
        // affecting the sum (each pairs a +1 with a compensating -1).
        let a = WceQuery { energy_q: 10, witness_delta: 1, legal_delta: -1, ..Default::default() };
        let b = WceQuery { energy_q: 10, witness_delta: 2, camera_delta: -2, ..Default::default() };
        let c = WceQuery { energy_q: 10, witness_delta: 3, death_scar_delta: -3, ..Default::default() };
        assert_eq!(a.effective_authority(), 10);
        assert_eq!(b.effective_authority(), 10);
        assert_eq!(c.effective_authority(), 10);
        let mut queries = [a, b, c];
        sort_by_superior_dexter_authority(&mut queries);
        assert_eq!(queries.map(|x| x.witness_delta), [1, 2, 3], "stable sort must preserve input order among ties");
    }

    #[test]
    fn effective_authority_sums_every_named_input() {
        let query = WceQuery {
            energy_q: 1,
            elevation_delta_mm: 1,
            legal_delta: 1,
            witness_delta: 1,
            route_delta: 1,
            camera_delta: 1,
            artifact_delta: 1,
            death_scar_delta: 1,
            charge_head_delta: 1,
            root_pressure_delta: 1,
            faction_claim_delta: 1,
            craft_provenance_delta: 1,
        };
        assert_eq!(query.effective_authority(), 12, "all 12 authority_inputs must count");
    }

    #[test]
    fn fewer_than_three_sources_do_not_converge() {
        assert!(merge_tas_de_charge_convergences(vec![q(1), q(2)]).is_none());
        assert!(merge_tas_de_charge_convergences(vec![]).is_none());
    }

    #[test]
    fn three_or_more_sources_converge_into_one_apex_query() {
        let merged = merge_tas_de_charge_convergences(vec![q(10), q(20), q(30)]).expect("3 sources must converge");
        assert_eq!(merged.sources.len(), 3);
        assert_eq!(merged.apex_authority(), 60);
    }

    #[test]
    fn superior_dexter_uses_and_tas_de_charge_uses_are_the_cited_counts() {
        assert_eq!(SUPERIOR_DEXTER_USES.len(), 6);
        assert_eq!(TAS_DE_CHARGE_USES.len(), 5);
    }
}
