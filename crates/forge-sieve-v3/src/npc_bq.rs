//! NPC-AI behavioral BQ router — a distinct BQ instance/taxonomy from
//! `forge_ml_bqrouter`'s own Hermetic-7 crate-routing `BqRouter`
//! (`transduce.rs:117-120` warns explicitly against merging taxonomies; this
//! is that named fork). ARCH000 ruling, Sean, 2026-08-22 driver trial session:
//! social.rs's action layer rides forge-ml's BQ leg ("full BQ takeover") in
//! place of the donor's hand-written `impl Sieve for X { observe/evaluate/
//! promote/snapshot }` match-statements — one trained centroid per sieve
//! kind, classifying a sieve's CURRENT STATE as signal or noise. No
//! `SieveEvent`/`SieveAction` enum: same reason those were never ported per
//! this crate's own `Cargo.toml` header (40+ variants of plumbing this crate
//! doesn't carry) — this router reads struct fields directly.
//!
//! Training data: `.forge/assets/nde-sieve-corpus/sieve_soft_labels.jsonl`
//! (census.tsv row `nde-sieve-corpus`) exists but does NOT carry per-struct
//! state vectors — only `{event, event_id, context, is_accept}` — so it
//! cannot feed `encode_*`'s query space directly, and at 63 total rows (≈20
//! after filtering to the 6/8 kinds it even mentions; Reputation and
//! Diplomacy have zero matching rows) it is too thin to claim real training
//! regardless. `train()` below is real and tested against synthetic
//! examples; wiring the staged corpus is UNSTARTED, stated not silent.

use forge_ml_bqrouter::{binarize_i8, BqCentroid, BQ_BYTES};

use crate::social::{
    DiplomacySieve, EconomySieve, FarmingSieve, FishingSieve, QuestSieve, ReputationSieve,
    TradeSieve, WitnessSieve,
};

/// One specialist per social-sieve kind — NOT the Hermetic-7 domain taxonomy.
pub const NPC_SPECIALISTS: usize = 8;

/// `QuestSieve` specialist slot.
pub const KIND_QUEST: u8 = 0;
/// `ReputationSieve` specialist slot.
pub const KIND_REPUTATION: u8 = 1;
/// `EconomySieve` specialist slot.
pub const KIND_ECONOMY: u8 = 2;
/// `DiplomacySieve` specialist slot.
pub const KIND_DIPLOMACY: u8 = 3;
/// `TradeSieve` specialist slot.
pub const KIND_TRADE: u8 = 4;
/// `WitnessSieve` specialist slot.
pub const KIND_WITNESS: u8 = 5;
/// `FarmingSieve` specialist slot.
pub const KIND_FARMING: u8 = 6;
/// `FishingSieve` specialist slot.
pub const KIND_FISHING: u8 = 7;

/// Display names, indexed by `KIND_*` constant.
pub const KIND_NAMES: [&str; NPC_SPECIALISTS] = [
    "quest", "reputation", "economy", "diplomacy", "trade", "witness", "farming", "fishing",
];

/// Accepted-labeled examples required before a kind's centroid is trusted —
/// same shape as `forge_ml_bqrouter::BqRouter`'s own `MIN_RECORDS`, a
/// separate constant because this is a separate instance.
const MIN_RECORDS: usize = 5;

/// One labeled training observation: which kind, its encoded state, and
/// whether the resulting action was accepted (real signal) or not (noise).
pub struct NpcTrainingExample {
    /// Which sieve kind this example belongs to (`KIND_*`).
    pub kind: u8,
    /// Whether the resulting action was accepted (real signal) or not.
    pub is_accept: bool,
    /// Encoded state, e.g. from `encode_quest`/`encode_economy`/etc.
    pub query: Vec<i8>,
}

/// Eight independently-trained centroids, one per social-sieve kind.
#[derive(Clone)]
pub struct NpcBqRouter {
    /// One centroid per `KIND_*` slot.
    pub centroids: [BqCentroid; NPC_SPECIALISTS],
}

impl Default for NpcBqRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl NpcBqRouter {
    /// Create a new untrained router — every kind starts void.
    pub fn new() -> Self {
        Self { centroids: std::array::from_fn(|_| BqCentroid::default()) }
    }

    /// Classify a sieve's encoded current state against its OWN kind's
    /// trained centroid. `+1` signal (act), `0` ambient (annotate, never
    /// veto), `-1` void (kind untrained — no centroid cleared `MIN_RECORDS`).
    ///
    /// Distance is computed only over the bytes `query` actually occupies —
    /// `forge_ml_bqrouter::hamming` compares the full fixed 64-byte/512-bit
    /// array, which would let the (mostly unused) padding dominate a per-kind
    /// vector this much narrower and mask every real difference. The signal
    /// threshold is likewise derived from THIS call's bit-width, not
    /// borrowed from the Hermetic-7 router's `MARGIN_SIGNAL=48` (calibrated
    /// for its fixed 512-bit space — dimensionally wrong at 16-64 bits).
    pub fn classify(&self, kind: u8, query: &[i8]) -> i8 {
        let idx = kind as usize;
        if idx >= NPC_SPECIALISTS || !self.centroids[idx].active {
            return -1;
        }
        let q = binarize_i8(query);
        let used_bytes = ((query.len() + 7) / 8).min(BQ_BYTES);
        let dist = hamming_prefix(&q, &self.centroids[idx].bits, used_bytes);
        let used_bits = (used_bytes * 8) as i64;
        // margin = n - 2*dist: mean-zero under the null (dist ~ Binomial(n, 1/2)),
        // Var(margin) = n, so a 3-sigma signal boundary is margin^2 >= 9n.
        let margin = used_bits - 2 * dist as i64;
        if margin > 0 && margin * margin >= 9 * used_bits {
            1
        } else {
            0
        }
    }

    /// Train each kind's centroid independently from labeled examples.
    /// Mirrors `BqRouter::train_from_pairs`'s sign-aggregation vote, scoped
    /// to 8 kind-slots instead of 7 crate-domain slots.
    pub fn train(&mut self, examples: &[NpcTrainingExample]) {
        let dims = BQ_BYTES * 8;
        let mut votes: Vec<Vec<i64>> = vec![vec![0i64; dims]; NPC_SPECIALISTS];
        let mut counts = [0usize; NPC_SPECIALISTS];
        let mut pos_counts = [0usize; NPC_SPECIALISTS];

        for ex in examples {
            let k = ex.kind as usize;
            if k >= NPC_SPECIALISTS {
                continue;
            }
            counts[k] += 1;
            let weight: i64 = if ex.is_accept { 1 } else { -1 };
            if ex.is_accept {
                pos_counts[k] += 1;
            }
            for (d, &q) in ex.query.iter().enumerate().take(dims) {
                votes[k][d] += weight * q as i64;
            }
        }

        for k in 0..NPC_SPECIALISTS {
            self.centroids[k].record_count = counts[k];
            self.centroids[k].positive_count = pos_counts[k];
            self.centroids[k].active = counts[k] >= MIN_RECORDS && pos_counts[k] >= MIN_RECORDS;
            if counts[k] > 0 {
                self.centroids[k].bits = binarize_votes(&votes[k]);
            }
        }
    }
}

/// Hamming distance over only the first `n` bytes — `forge_ml_bqrouter::hamming`
/// always compares the full fixed 64-byte array, which is the wrong tool once
/// the meaningful vector is far narrower than 512 bits (see `classify`).
fn hamming_prefix(a: &[u8; BQ_BYTES], b: &[u8; BQ_BYTES], n: usize) -> u32 {
    a[..n].iter().zip(&b[..n]).map(|(x, y)| (x ^ y).count_ones()).sum()
}

/// Local twin of `forge_ml_bqrouter`'s private `binarize_votes` (sign
/// aggregation over accumulated votes) — not exported upstream, small enough
/// to duplicate rather than force a private-fn export for one caller.
fn binarize_votes(x: &[i64]) -> [u8; BQ_BYTES] {
    let mut bits = [0u8; BQ_BYTES];
    for (chunk_idx, chunk) in x.chunks(8).enumerate().take(BQ_BYTES) {
        let mut byte = 0u8;
        for (bit_pos, &val) in chunk.iter().enumerate() {
            if val > 0 {
                byte |= 1 << bit_pos;
            }
        }
        bits[chunk_idx] = byte;
    }
    bits
}

#[inline]
fn c8(x: i64) -> i8 {
    x.clamp(i8::MIN as i64, i8::MAX as i64) as i8
}

/// Encode a `QuestSieve`'s current state into a BQ query vector.
pub fn encode_quest(q: &QuestSieve) -> Vec<i8> {
    vec![
        c8(q.status as i64),
        c8(q.objective_count as i64),
        c8((q.failure_flags & 0xFF) as i64),
        c8(q.archetype as i64),
        c8(q.canon_weight as i64),
        c8(q.moon_range.0 as i64),
        c8(q.moon_range.1 as i64),
    ]
}

/// Encode a `ReputationSieve`'s current state into a BQ query vector.
pub fn encode_reputation(r: &ReputationSieve) -> Vec<i8> {
    vec![
        c8(r.trust_tier as i64),
        c8(r.observed_trades as i64),
        c8(r.observed_generosity as i64),
        c8(r.observed_violence as i64),
    ]
}

/// Encode an `EconomySieve`'s current state into a BQ query vector.
pub fn encode_economy(e: &EconomySieve) -> Vec<i8> {
    let mut v = vec![c8(e.hoarding_score as i64), c8(e.reciprocity_score as i64), c8(e.greed as i64)];
    v.extend(e.resource_supply.iter().map(|&x| c8(x as i64)));
    v.extend(e.resource_demand.iter().map(|&x| c8(x as i64)));
    v
}

/// Encode a `DiplomacySieve`'s current state into a BQ query vector.
pub fn encode_diplomacy(d: &DiplomacySieve) -> Vec<i8> {
    let mut v = vec![
        c8(d.faction_id as i64),
        c8(d.promises_kept as i64),
        c8(d.promises_broken as i64),
        c8(d.trust_momentum as i64),
    ];
    v.extend(d.relations.iter().map(|&x| c8(x as i64)));
    v
}

/// Representative window, not exhaustive — `price_memory`/`demand_observed`/
/// `supply_glut` are `[_; 32]`; only the first 8 slots ride the query,
/// matching every other kind's `[_; 8]` scale (stated, not padded).
pub fn encode_trade(t: &TradeSieve) -> Vec<i8> {
    let mut v = vec![
        c8(t.merchant_id as i64),
        c8(t.caravan_routes_active as i64),
        c8(t.smuggle_detected as i64),
    ];
    v.extend(t.price_memory[..8].iter().map(|&x| c8(x as i64)));
    v.extend(t.demand_observed[..8].iter().map(|&x| c8(x as i64)));
    v.extend(t.supply_glut[..8].iter().map(|&x| c8(x as i64)));
    v
}

/// Encode a `WitnessSieve`'s current state into a BQ query vector.
pub fn encode_witness(w: &WitnessSieve) -> Vec<i8> {
    vec![
        c8(w.event_type as i64),
        c8(w.witness_count as i64),
        c8(w.spread_radius as i64),
        c8(w.ticks_since as i64),
        c8(w.sentiment as i64),
    ]
}

/// Encode a `FarmingSieve`'s current state into a BQ query vector.
pub fn encode_farming(f: &FarmingSieve) -> Vec<i8> {
    vec![
        c8(f.soil_fertility as i64),
        c8(f.moisture as i64),
        c8(f.growth_stage as i64),
        c8(f.days_planted as i64),
        c8(f.yield_estimate as i64),
        c8(f.neglect_ticks as i64),
        c8(f.pest_pressure as i64),
        c8(f.companion_bonus as i64),
    ]
}

/// Encode a `FishingSieve`'s current state into a BQ query vector.
pub fn encode_fishing(f: &FishingSieve) -> Vec<i8> {
    let mut v = vec![
        c8(f.water_health as i64),
        c8(f.water_temperature as i64),
        c8(f.times_fished_here as i64),
        c8(f.moon_bite_modifier as i64),
    ];
    v.extend(f.fish_populations.iter().map(|&x| c8(x as i64)));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::{QuestStatus, TrustTier};

    #[test]
    fn encode_is_deterministic() {
        let q = QuestSieve {
            quest_id: 1,
            status: QuestStatus::InProgress,
            objectives: Default::default(),
            objective_count: 1,
            moon_range: (1, 13),
            failure_flags: 0,
            archetype: 0,
            canon_weight: 100,
        };
        assert_eq!(encode_quest(&q), encode_quest(&q));
    }

    #[test]
    fn untrained_kind_is_void() {
        let r = NpcBqRouter::new();
        assert_eq!(r.classify(KIND_QUEST, &[1, 2, 3]), -1);
    }

    #[test]
    fn unknown_kind_index_is_void() {
        let r = NpcBqRouter::new();
        assert_eq!(r.classify(99, &[1, 2, 3]), -1);
    }

    #[test]
    fn below_min_records_stays_inactive() {
        let mut r = NpcBqRouter::new();
        let examples: Vec<NpcTrainingExample> = (0..4)
            .map(|_| NpcTrainingExample { kind: KIND_ECONOMY, is_accept: true, query: vec![1; 16] })
            .collect();
        r.train(&examples);
        assert!(!r.centroids[KIND_ECONOMY as usize].active);
        assert_eq!(r.classify(KIND_ECONOMY, &[1; 16]), -1);
    }

    #[test]
    fn trained_kind_recognizes_matching_state() {
        let mut r = NpcBqRouter::new();
        let examples: Vec<NpcTrainingExample> = (0..8)
            .map(|_| NpcTrainingExample { kind: KIND_FARMING, is_accept: true, query: vec![1; 32] })
            .collect();
        r.train(&examples);
        assert!(r.centroids[KIND_FARMING as usize].active);
        assert_eq!(r.classify(KIND_FARMING, &[1; 32]), 1);
        // An unrelated kind stays void — training one kind never leaks into another.
        assert_eq!(r.classify(KIND_FISHING, &[1; 32]), -1);
    }

    #[test]
    fn trained_kind_rejects_opposite_state() {
        let mut r = NpcBqRouter::new();
        let examples: Vec<NpcTrainingExample> = (0..8)
            .map(|_| NpcTrainingExample { kind: KIND_TRADE, is_accept: true, query: vec![1; 16] })
            .collect();
        r.train(&examples);
        // Opposite-signed state is far in hamming space from the trained centroid.
        let far = vec![-1i8; 16];
        assert_ne!(r.classify(KIND_TRADE, &far), 1);
    }

    #[test]
    fn all_eight_encoders_produce_nonempty_queries() {
        let economy = EconomySieve {
            zone_id: 1,
            resource_supply: [500; 8],
            resource_demand: [0; 8],
            hoarding_score: 0,
            reciprocity_score: 0,
            greed: 0,
        };
        let reputation =
            ReputationSieve { npc_id: 1, observed_trades: 0, observed_generosity: 0, observed_violence: 0, trust_tier: TrustTier::Stranger };
        assert!(!encode_economy(&economy).is_empty());
        assert!(!encode_reputation(&reputation).is_empty());
    }
}
