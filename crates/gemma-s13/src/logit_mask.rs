// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Courtroom-Admissible RAG-DAG Logit Masking Engine.
//!
//! Compiles an acyclic reference graph (RAG-DAG) containing strictly witnessed, courtroom-admissible
//! Plains Cree forms. Intercepts Gemma logit decoding distributions and dynamically masks
//! unauthorized / un-witnessed logit transition paths to absolute zero probability (`i32::MIN`).

#![deny(unsafe_code)]

/// Masked logit representing absolute zero probability ($-\infty$ in fixed-point space).
pub const LOGIT_MASKED_ZERO_PROB: i32 = i32::MIN;

/// Maximum number of nodes in static RAG-DAG.
pub const MAX_DAG_NODES: usize = 32;

/// Maximum outward edges per DAG node.
pub const MAX_EDGES_PER_NODE: usize = 8;

/// Acyclic Witnessed Node in the Cree Reference Graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WitnessedNode {
    /// Token identifier.
    pub token_id: u32,
    /// Courtroom citation / provenance hash.
    pub provenance_tag: u64,
    /// Number of valid outward edges.
    pub edge_count: usize,
    /// Allowed next token identifiers.
    pub next_token_ids: [u32; MAX_EDGES_PER_NODE],
}

impl WitnessedNode {
    /// Create a new witnessed DAG node.
    pub const fn new(token_id: u32, provenance_tag: u64, allowed_next: &[u32]) -> Self {
        let mut next_token_ids = [0u32; MAX_EDGES_PER_NODE];
        let mut edge_count = 0;
        let mut i = 0;
        while i < allowed_next.len() && i < MAX_EDGES_PER_NODE {
            next_token_ids[i] = allowed_next[i];
            edge_count += 1;
            i += 1;
        }
        Self {
            token_id,
            provenance_tag,
            edge_count,
            next_token_ids,
        }
    }

    /// Check if a transition to `candidate_token_id` is witnessed in the DAG.
    #[inline]
    pub fn allows_transition(&self, candidate_token_id: u32) -> bool {
        let mut i = 0;
        while i < self.edge_count {
            if self.next_token_ids[i] == candidate_token_id {
                return true;
            }
            i += 1;
        }
        false
    }
}

/// Static Acyclic Reference Graph (RAG-DAG).
pub struct RagDag {
    nodes: [WitnessedNode; MAX_DAG_NODES],
    node_count: usize,
}

impl RagDag {
    /// Compile canonical courtroom-admissible Plains Cree reference DAG.
    pub const fn compile_canonical() -> Self {
        let mut nodes = [WitnessedNode {
            token_id: 0,
            provenance_tag: 0,
            edge_count: 0,
            next_token_ids: [0; MAX_EDGES_PER_NODE],
        }; MAX_DAG_NODES];

        // Root 0: Start of sentence -> allows {1 (ni-), 2 (ki-), 3 (wa-)}
        nodes[0] = WitnessedNode::new(0, 0x1301_0001_A001, &[1, 2, 3]);
        // Node 1: "ni-" -> allows {4 (wapam), 5 (pahtam)}
        nodes[1] = WitnessedNode::new(1, 0x1301_0001_A002, &[4, 5]);
        // Node 2: "ki-" -> allows {4 (wapam), 5 (pahtam)}
        nodes[2] = WitnessedNode::new(2, 0x1301_0001_A003, &[4, 5]);
        // Node 3: "wa-" -> allows {4 (pamew), 6 (paminaw)}
        nodes[3] = WitnessedNode::new(3, 0x1301_0001_A004, &[4, 6]);
        // Node 4: "wapam" -> allows {7 (-aw), 8 (-ikw), 9 (-ew)}
        nodes[4] = WitnessedNode::new(4, 0x1301_0001_A005, &[7, 8, 9]);

        Self {
            nodes,
            node_count: 5,
        }
    }

    /// Find node by token_id.
    #[inline]
    pub fn find_node(&self, token_id: u32) -> Option<&WitnessedNode> {
        let mut i = 0;
        while i < self.node_count {
            if self.nodes[i].token_id == token_id {
                return Some(&self.nodes[i]);
            }
            i += 1;
        }
        None
    }

    /// Validate whether an entire multi-hop sequence of tokens strictly follows witnessed edges in the DAG.
    #[inline]
    pub fn validate_path(&self, path: &[u32]) -> bool {
        if path.is_empty() {
            return false;
        }
        if path.len() == 1 {
            return self.find_node(path[0]).is_some();
        }
        let mut i = 0;
        while i + 1 < path.len() {
            let curr = path[i];
            let next = path[i + 1];
            match self.find_node(curr) {
                Some(node) => {
                    if !node.allows_transition(next) {
                        return false;
                    }
                }
                None => return false,
            }
            i += 1;
        }
        true
    }

    /// Intercept local logit distribution and mask all un-witnessed paths to absolute zero probability.
    #[inline]
    pub fn apply_logit_mask(&self, current_token_id: u32, logits: &mut [i32]) {
        if let Some(node) = self.find_node(current_token_id) {
            for (candidate_id, logit) in logits.iter_mut().enumerate() {
                if !node.allows_transition(candidate_id as u32) {
                    *logit = LOGIT_MASKED_ZERO_PROB;
                }
            }
        } else {
            // Un-witnessed current state: mask all candidate transitions
            for logit in logits.iter_mut() {
                *logit = LOGIT_MASKED_ZERO_PROB;
            }
        }
    }

    /// Mask logits with anti-expert penalty subtraction and strict DAG witness filtering.
    ///
    /// $$L_{\text{gated}}(c) = \begin{cases}
    /// L_{\text{intent}}(c) - \frac{\beta \cdot P_{\text{anti}}(c)}{10{,}000} & \text{if } c \in \text{DAG}(u) \\
    /// -\infty & \text{otherwise}
    /// \end{cases}$$
    #[inline]
    pub fn mask_logits_with_anti_expert(
        &self,
        current_token_id: u32,
        intent_logits: &mut [i32],
        anti_expert_penalties: &[i32],
        beta_pmy: u16,
    ) {
        let beta = beta_pmy.min(10_000) as i64;
        let node_opt = self.find_node(current_token_id);
        for (candidate_id, logit) in intent_logits.iter_mut().enumerate() {
            let allowed = node_opt.map_or(false, |node| node.allows_transition(candidate_id as u32));
            if !allowed {
                *logit = LOGIT_MASKED_ZERO_PROB;
            } else {
                let penalty = anti_expert_penalties.get(candidate_id).copied().unwrap_or(0) as i64;
                let subtracted = (*logit as i64) - (penalty * beta) / 10_000;
                *logit = subtracted.clamp(i32::MIN as i64 + 1, i32::MAX as i64) as i32;
            }
        }
    }
}

/// Anti-Expert Parity Gate enforcing the involution identity:
/// $$T + T^* = 0 \iff T^* = -T$$
/// and involution property:
/// $$(T^*)^* = T$$
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AntiExpertGate;

impl AntiExpertGate {
    /// Check if direct tensor $T$ and conjugate tensor $T^*$ are in exact parity equilibrium.
    /// Returns `(is_balanced, residue)`.
    #[inline]
    pub fn evaluate_parity(direct_t: &[i8], conjugate_t_star: &[i8]) -> (bool, i32) {
        let mut residue: i32 = 0;
        let n = direct_t.len().min(conjugate_t_star.len());
        let mut i = 0;
        while i < n {
            residue += (direct_t[i] as i32) + (conjugate_t_star[i] as i32);
            i += 1;
        }
        (residue == 0, residue)
    }

    /// Compute the anti-expert operator involution $T^* = -T$ across 5 trits.
    #[inline]
    pub fn compute_involution_5trit(t: &[i8; 5]) -> [i8; 5] {
        [-t[0], -t[1], -t[2], -t[3], -t[4]]
    }

    /// Verify that the involution $(T^*)^* = T$ holds and that $T + T^* = 0$.
    #[inline]
    pub fn verify_involution_identity(t: &[i8; 5]) -> bool {
        let t_star = Self::compute_involution_5trit(t);
        let t_star_star = Self::compute_involution_5trit(&t_star);
        let (is_balanced, residue) = Self::evaluate_parity(t, &t_star);
        t_star_star == *t && is_balanced && residue == 0
    }

    /// Compute the anti-expert factual coherence penalty vector between
    /// Papa Bear (Intent) and Mama Bear (Anti-Expert Assist).
    #[inline]
    pub fn factual_coherence_penalty(
        papa_intent_activations: &[i32; 5],
        mama_assist_anti_expert: &[i32; 5],
    ) -> [i32; 5] {
        let mut penalties = [0i32; 5];
        let mut i = 0;
        while i < 5 {
            let anti_energy = mama_assist_anti_expert[i].max(0);
            let intent_energy = papa_intent_activations[i].max(0);
            penalties[i] = (anti_energy * intent_energy) / 10_000;
            i += 1;
        }
        penalties
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_dag_masking_allowed_transitions() {
        let dag = RagDag::compile_canonical();
        let mut logits = [1000i32; 10];

        // Current token is 0 (Root). Allowed: 1, 2, 3.
        dag.apply_logit_mask(0, &mut logits);

        assert_eq!(logits[0], LOGIT_MASKED_ZERO_PROB);
        assert_eq!(logits[1], 1000);
        assert_eq!(logits[2], 1000);
        assert_eq!(logits[3], 1000);
        assert_eq!(logits[4], LOGIT_MASKED_ZERO_PROB);
        assert_eq!(logits[5], LOGIT_MASKED_ZERO_PROB);
    }

    #[test]
    fn test_rag_dag_masking_unwitnessed_state() {
        let dag = RagDag::compile_canonical();
        let mut logits = [500i32; 8];

        // Current token 999 is un-witnessed in DAG
        dag.apply_logit_mask(999, &mut logits);

        for &l in logits.iter() {
            assert_eq!(l, LOGIT_MASKED_ZERO_PROB);
        }
    }

    #[test]
    fn test_rag_dag_validate_path() {
        let dag = RagDag::compile_canonical();

        // Valid 3-hop Plains Cree path: 0 (Root) -> 1 ("ni-") -> 4 ("wapam") -> 7 ("-aw")
        assert!(dag.validate_path(&[0, 1, 4, 7]));
        // Valid 2-hop path: 0 -> 2 ("ki-") -> 5 ("pahtam")
        assert!(dag.validate_path(&[0, 2, 5]));

        // Invalid jump: 0 -> 4 (missing prefix)
        assert!(!dag.validate_path(&[0, 4]));
        // Invalid suffix: 0 -> 1 -> 4 -> 99
        assert!(!dag.validate_path(&[0, 1, 4, 99]));
        // Empty path
        assert!(!dag.validate_path(&[]));
    }

    #[test]
    fn test_rag_dag_mask_logits_with_anti_expert() {
        let dag = RagDag::compile_canonical();
        let mut logits = [1000i32; 6];
        let penalties = [0i32, 2000, 0, 5000, 0, 0];

        // Current token 0 (allowed: 1, 2, 3)
        // Beta = 5000 (0.5x scaling)
        dag.mask_logits_with_anti_expert(0, &mut logits, &penalties, 5000);

        assert_eq!(logits[0], LOGIT_MASKED_ZERO_PROB);
        // Candidate 1: 1000 - (2000 * 5000 / 10000) = 1000 - 1000 = 0
        assert_eq!(logits[1], 0);
        // Candidate 2: 1000 - 0 = 1000
        assert_eq!(logits[2], 1000);
        // Candidate 3: 1000 - (5000 * 5000 / 10000) = 1000 - 2500 = -1500
        assert_eq!(logits[3], -1500);
        assert_eq!(logits[4], LOGIT_MASKED_ZERO_PROB);
    }

    #[test]
    fn test_anti_expert_gate_involution_and_parity() {
        let t = [1i8, -1, 0, 1, -1];
        assert!(AntiExpertGate::verify_involution_identity(&t));

        let t_star = AntiExpertGate::compute_involution_5trit(&t);
        assert_eq!(t_star, [-1i8, 1, 0, -1, 1]);

        let (balanced, residue) = AntiExpertGate::evaluate_parity(&t, &t_star);
        assert!(balanced);
        assert_eq!(residue, 0);

        let uncalibrated = [1i8, 1, 0, 1, -1];
        let (unbalanced, un_res) = AntiExpertGate::evaluate_parity(&t, &uncalibrated);
        assert!(!unbalanced);
        assert_ne!(un_res, 0);
    }

    #[test]
    fn test_anti_expert_factual_coherence_penalty() {
        let intent = [8000i32, 5000, 2000, 0, 0];
        let anti_expert = [10000i32, 0, 5000, 8000, 0];

        let penalties = AntiExpertGate::factual_coherence_penalty(&intent, &anti_expert);
        // Lane 0: (10000 * 8000) / 10000 = 8000
        assert_eq!(penalties[0], 8000);
        // Lane 1: 0
        assert_eq!(penalties[1], 0);
        // Lane 2: (5000 * 2000) / 10000 = 1000
        assert_eq!(penalties[2], 1000);
    }

    #[test]
    fn test_witnessed_node_max_edges_truncation() {
        let node = WitnessedNode::new(10, 0x1234, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(node.edge_count, MAX_EDGES_PER_NODE);
        assert!(node.allows_transition(8));
        assert!(!node.allows_transition(9));
    }

    #[test]
    fn test_rag_dag_find_node() {
        let dag = RagDag::compile_canonical();
        assert!(dag.find_node(0).is_some());
        assert!(dag.find_node(100).is_none());
    }
}
