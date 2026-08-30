// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Three Bears S13 Local Inference Fleet Harness.
//!
//! # 1:1 Triad Architecture
//! - **Baby Bear (2B Render Codec / Synthesizer)**:
//!   Lowers 5D M5 Geodesic manifold coordinates ($3^5 = 243$ states) and VIXI shader uniforms
//!   using 1.58-bit ternary math and static $<2.6\text{ MB}$ vocabulary LUT byte autoencoder.
//! - **Papa Bear (9B Intent Mirror / Speculative Intent)**:
//!   Simulates physical pathways, forward consequence trees, and RAG-DAG logit gating with
//!   zero-transcendental $N \times \text{IPR}$ entropy sieve ($10{,}000\text{ pmy}$ landmark focus).
//! - **Mama Bear (27B/9B Assist Direct / Anti-Expert Parity)**:
//!   Evaluates the Anti-Expert Conjugate Parity Identity:
//!   $$T + T^* = 0$$
//!   Monitors the 13 out-of-band sentinel slots ($243..=255$),
//!   and enforces ADR-0026 zero-retention memory scrubbing.

use crate::logit_mask::{WitnessedNode, LOGIT_MASKED_ZERO_PROB};
use crate::m5_geodesic::M5Coordinate;
use crate::model_9b::Gemma9bConfig;
use forge_hal_clockspine::nipr::{NormalizedIpr, LANDMARK_PMY};
use crate::s13::{unpack_5_trits, S13Error, TRITS_PER_BYTE};
use crate::sentinel::{SentinelBand, SENTINEL_MIN_BYTE};
use crate::vault::ZeroRetentionVault;
use crate::vocab::{AutoEncoderWeights, LATENT_DIM};

/// Bytes of header ahead of the packed trit payload in a `.s13m` tensor file.
pub const S13M_HEADER_BYTES: usize = 16;

/// Packed `.s13m` file size for a tensor of `weights` trits.
pub const fn s13m_file_bytes(weights: usize) -> usize {
    weights.div_ceil(TRITS_PER_BYTE) + S13M_HEADER_BYTES
}

/// Model config for Baby Bear (2B Render Codec).
///
/// Dimensions are MEASURED off the packed bear at `s13_gemma_2b/` (2026-08-26),
/// not declared: the previous values (2048/1/18/8192) matched no tensor on disk
/// and would have indexed off the end of every one. See
/// `tests::baby_bear_config_matches_the_packed_bear_on_disk`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BabyBear2bConfig {
    /// Hidden dimension ($d_{\text{model}} = 2304$).
    pub d_model: usize,
    /// Number of attention heads ($n_{\text{heads}} = 8$).
    pub n_heads: usize,
    /// Number of KV heads ($n_{\text{kv\_heads}} = 4$).
    pub n_kv_heads: usize,
    /// Dimension per head ($d_{\text{head}} = 256$).
    pub d_head: usize,
    /// Number of decoder layers ($n_{\text{layers}} = 26$).
    pub n_layers: usize,
    /// Intermediate feed-forward dimension ($d_{\text{ff}} = 9216$).
    pub d_ff: usize,
    /// Static vocabulary size ($256{,}000$).
    pub vocab_size: usize,
}

impl BabyBear2bConfig {
    /// Packed size of one `ffn_{up,gate,down}` tensor file.
    pub const fn ffn_tensor_file_bytes(&self) -> usize {
        s13m_file_bytes(self.d_model * self.d_ff)
    }

    /// Packed size of one `attn_{q,output}` tensor file.
    pub const fn attn_q_file_bytes(&self) -> usize {
        s13m_file_bytes(self.d_model * self.n_heads * self.d_head)
    }

    /// Packed size of one `attn_{k,v}` tensor file — narrower than Q by the
    /// grouped-query ratio.
    pub const fn attn_kv_file_bytes(&self) -> usize {
        s13m_file_bytes(self.d_model * self.n_kv_heads * self.d_head)
    }
}

impl Default for BabyBear2bConfig {
    fn default() -> Self {
        Self {
            d_model: 2304,
            n_heads: 8,
            n_kv_heads: 4,
            d_head: 256,
            n_layers: 26,
            d_ff: 9216,
            vocab_size: 256_000,
        }
    }
}

/// Output of Baby Bear Speculative Render Codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCodecOutput {
    /// Discrete 5D M5 Geodesic manifold scalar index ($0..=242$).
    pub m5_scalar_index: u8,
    /// Latent 24-lane continuous projection signature (Permyriad fixed-point).
    pub latent_signature: [i32; LATENT_DIM],
    /// VIXI shader uniform hash (deterministic checksum).
    pub shader_uniform_hash: u64,
    /// Number of rendered ASCII cells projected.
    pub rendered_cell_count: u16,
}

/// Output of Papa Bear Speculative Intent Mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentMirrorOutput {
    /// Prioritized next action candidate ID.
    pub candidate_action_id: u32,
    /// Normalized IPR localization score in Permyriad ($0..10{,}000$).
    pub nipr_localization_pmy: u16,
    /// Whether the intent vector qualifies as a landmark attractor ($\ge 7500\text{ pmy}$).
    pub is_landmark_focus: bool,
    /// Whether the candidate transition was permitted by RAG-DAG witness logit mask.
    pub transition_witnessed: bool,
    /// Evaluated masked logit value.
    pub logit_value: i32,
}

/// Output of Mama Bear Assist Direct Anti-Expert Parity Executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssistDirectOutput {
    /// Whether the anti-expert conjugate parity condition holds ($T + T^* = 0$).
    pub is_parity_balanced: bool,
    /// Parity sum residue $\sum (T + T^*)$ (must be exactly 0 in equilibrium).
    pub parity_residue: i32,
    /// Detected out-of-band sentinel slot (if any).
    pub sentinel_band: Option<SentinelBand>,
    /// Whether ADR-0026 zero-retention memory scrubbing was performed.
    pub vault_scrubbed: bool,
}

/// Unified output of the Three Bears 1:1 Triad Fleet tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreeBearsFleetOutput {
    /// Current simulation tick.
    pub tick: u64,
    /// Baby Bear Render Codec output.
    pub render_codec: RenderCodecOutput,
    /// Papa Bear Intent Mirror output.
    pub intent_mirror: IntentMirrorOutput,
    /// Mama Bear Assist Direct output.
    pub assist_direct: AssistDirectOutput,
    /// Fleet synchronization status flag.
    pub synchronized: bool,
}

/// The Three Bears Local Inference Fleet Harness.
pub struct ThreeBearsFleet {
    /// Baby Bear (2B Render Codec) configuration.
    pub baby_config: BabyBear2bConfig,
    /// Papa Bear (9B Intent Mirror) configuration.
    pub papa_config: Gemma9bConfig,
    /// Static byte autoencoder vocabulary weights ($< 2.6\text{ MB}$).
    pub vocab_autoencoder: AutoEncoderWeights,
    /// ADR-0026 zero-retention memory vault.
    pub vault: ZeroRetentionVault,
    /// Active simulation tick.
    pub current_tick: u64,
}

impl ThreeBearsFleet {
    /// Initialize a new Three Bears S13 Local Inference Fleet Harness.
    pub const fn new() -> Self {
        Self {
            baby_config: BabyBear2bConfig {
                d_model: 2304,
                n_heads: 8,
                n_kv_heads: 4,
                d_head: 256,
                n_layers: 26,
                d_ff: 9216,
                vocab_size: 256_000,
            },
            papa_config: Gemma9bConfig {
                d_model: 3584,
                n_heads: 16,
                n_kv_heads: 8,
                d_head: 256,
                n_layers: 42,
                d_ff: 14336,
                vocab_size: 256_000,
                max_seq_len: 8192,
                rope_theta: 10_000,
                permyriad_scale: 10_000,
            },
            vocab_autoencoder: AutoEncoderWeights::default_fixed(),
            vault: ZeroRetentionVault::new(),
            current_tick: 0,
        }
    }

    /// Execute a single deterministic Triad step across Baby Bear, Papa Bear, and Mama Bear.
    pub fn step_fleet(
        &mut self,
        input_byte: u8,
        witness_node: &WitnessedNode,
        candidate_action: u32,
        m5_render_coord: M5Coordinate,
        direct_weights: &[i8],
        mirror_anti_expert_weights: &[i8],
        current_tick: u64,
    ) -> Result<ThreeBearsFleetOutput, S13Error> {
        self.current_tick = current_tick;

        // ── 1. Baby Bear 2B Render Codec Execution ────────────────────────────
        let m5_scalar_idx = m5_render_coord.to_scalar_index();
        let mut latent = [0i32; LATENT_DIM];
        self.vocab_autoencoder.encode_byte(input_byte, &mut latent);

        // Compute deterministic shader uniform hash from latent signature and 5D coordinate
        let mut shader_hash = 0xcbf29ce484222325u64;
        shader_hash ^= m5_scalar_idx as u64;
        shader_hash = shader_hash.wrapping_mul(0x100000001b3);
        for &val in latent.iter() {
            shader_hash ^= (val as u64) & 0xFFFFFFFF;
            shader_hash = shader_hash.wrapping_mul(0x100000001b3);
        }

        let render_codec = RenderCodecOutput {
            m5_scalar_index: m5_scalar_idx,
            latent_signature: latent,
            shader_uniform_hash: shader_hash,
            rendered_cell_count: 71, // Canonical 71-column ASCII cell width
        };

        // ── 2. Papa Bear 9B Intent Mirror Execution ───────────────────────────
        // Extract top-5 activation energy for N x IPR localization
        let activations = [
            (latent[0].abs().min(10000)) as u16,
            (latent[1].abs().min(10000)) as u16,
            (latent[2].abs().min(10000)) as u16,
            (latent[3].abs().min(10000)) as u16,
            (latent[4].abs().min(10000)) as u16,
        ];
        let ipr = NormalizedIpr::compute_u16(&activations);
        let is_landmark = ipr.pmy >= LANDMARK_PMY;

        let transition_ok = witness_node.allows_transition(candidate_action);
        let logit = if transition_ok {
            (ipr.pmy as i32) / 10
        } else {
            LOGIT_MASKED_ZERO_PROB
        };

        let intent_mirror = IntentMirrorOutput {
            candidate_action_id: candidate_action,
            nipr_localization_pmy: ipr.pmy,
            is_landmark_focus: is_landmark,
            transition_witnessed: transition_ok,
            logit_value: logit,
        };

        // ── 3. Mama Bear Assist Direct & Anti-Expert Parity Execution ─────────
        // Anti-Expert Conjugate Parity Identity: T + T* = 0
        let mut parity_residue: i32 = 0;
        let n_weights = direct_weights.len().min(mirror_anti_expert_weights.len());
        for i in 0..n_weights {
            let t = direct_weights[i] as i32;
            let t_star = mirror_anti_expert_weights[i] as i32;
            parity_residue += t + t_star;
        }

        let is_parity_balanced = parity_residue == 0;

        // Sentinel check on input byte
        let sentinel_band = if input_byte >= SENTINEL_MIN_BYTE {
            SentinelBand::from_byte(input_byte)
        } else {
            None
        };

        // ADR-0026 zero-retention memory sweep
        let vault_scrubbed = self.vault.sweep_if_expired(current_tick);

        let assist_direct = AssistDirectOutput {
            is_parity_balanced,
            parity_residue,
            sentinel_band,
            vault_scrubbed,
        };

        let synchronized = is_parity_balanced && sentinel_band.is_none();

        Ok(ThreeBearsFleetOutput {
            tick: current_tick,
            render_codec,
            intent_mirror,
            assist_direct,
            synchronized,
        })
    }
}

// ── 1.58-Bit Balanced Ternary Forward Vector Arithmetic ───────────────────────

/// Execute a zero-heap 1.58-bit ternary dot product between packed weights and an integer activation vector.
///
/// Weight bytes contain exactly 5 trits in $\{-1, 0, +1\}$ mapped to $0..=242$.
/// Activation vector entries are in Permyriad ($10{,}000 = 1.0$).
#[inline]
pub fn ternary_dot_product_5trit(packed_weight_byte: u8, activations: &[i32; 5]) -> Result<i32, S13Error> {
    let trits = unpack_5_trits(packed_weight_byte)?;
    let mut sum: i32 = 0;
    let mut i = 0;
    while i < 5 {
        let t = trits[i] as i32;
        let act = activations[i];
        sum += t * act;
        i += 1;
    }
    Ok(sum)
}

/// Compute conjugate anti-expert mirror weights satisfying $T + T^* = 0$.
///
/// For each trit $t \in \{-1, 0, +1\}$, the conjugate trit is $-t$.
#[inline]
pub fn compute_anti_expert_conjugate(trits: &[i8; 5]) -> [i8; 5] {
    [-trits[0], -trits[1], -trits[2], -trits[3], -trits[4]]
}

// ── Memory Safety & Static Vocabulary Size Assertions ─────────────────────────
// Byte AutoEncoder weights total size verification (< 2.6 MB):
// enc_weights: 24 * 256 * 2 = 12,288 B
// enc_biases: 24 * 4 = 96 B
// proj_weights: 2048 * 24 * 2 = 98,304 B
// Total AutoEncoder size = 110,688 B ≈ 108 KB << 2.6 MB budget.
const _: () = assert!(core::mem::size_of::<AutoEncoderWeights>() < 2_600_000);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s13::pack_5_trits;

    /// The bear on disk is the oracle, not the struct. Every declared dimension
    /// is re-derived from a real `.s13m` byte count; a config that drifts off
    /// the packed weights reddens here instead of indexing off the end of a
    /// tensor at first inference.
    #[test]
    fn baby_bear_config_matches_the_packed_bear_on_disk() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../s13_gemma_2b");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            println!("baby bear absent at {} — dimensional check skipped", dir.display());
            return;
        };

        let cfg = BabyBear2bConfig::default();
        let mut blocks = std::collections::BTreeSet::new();
        let mut checked = 0usize;
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(rest) = name.strip_prefix("blk_") else { continue };
            let Some((idx, tensor)) = rest.split_once('_') else { continue };
            let Ok(idx) = idx.parse::<usize>() else { continue };
            blocks.insert(idx);

            let want = match tensor {
                "ffn_up_weight.s13m" | "ffn_gate_weight.s13m" | "ffn_down_weight.s13m" => {
                    cfg.ffn_tensor_file_bytes()
                }
                "attn_q_weight.s13m" | "attn_output_weight.s13m" => cfg.attn_q_file_bytes(),
                "attn_k_weight.s13m" | "attn_v_weight.s13m" => cfg.attn_kv_file_bytes(),
                _ => continue,
            };
            let got = e.metadata().expect("stat tensor").len() as usize;
            assert_eq!(got, want, "{name}: config says {want} bytes, the bear is {got}");
            checked += 1;
        }

        assert!(checked > 0, "found no recognizable .s13m tensors in {}", dir.display());
        assert_eq!(blocks.len(), cfg.n_layers, "layer count disagrees with blk_* on disk");
        assert_eq!(blocks.iter().next_back().copied(), Some(cfg.n_layers - 1), "blocks are not contiguous from 0");
    }

    #[test]
    fn test_three_bears_triad_synchronized_step() {
        let mut fleet = ThreeBearsFleet::new();

        let witness = WitnessedNode::new(100, 0x12345678, &[10, 20, 30]);
        let m5_coord = M5Coordinate::new([1, 0, -1, 0, 1]).expect("Valid M5 coordinate");

        let direct = [1i8, -1, 0, 1, -1];
        let mirror = [-1i8, 1, 0, -1, 1]; // Exact T + T* = 0 conjugate

        let output = fleet
            .step_fleet(65, &witness, 20, m5_coord, &direct, &mirror, 1)
            .expect("Step succeeded");

        assert_eq!(output.tick, 1);
        assert!(output.synchronized, "Fleet must be fully synchronized");

        // Baby Bear checks
        assert_eq!(output.render_codec.rendered_cell_count, 71);
        assert!(output.render_codec.m5_scalar_index < 243);

        // Papa Bear checks
        assert_eq!(output.intent_mirror.candidate_action_id, 20);
        assert!(output.intent_mirror.transition_witnessed);
        assert!(output.intent_mirror.logit_value > 0);

        // Mama Bear checks
        assert!(output.assist_direct.is_parity_balanced);
        assert_eq!(output.assist_direct.parity_residue, 0);
        assert_eq!(output.assist_direct.sentinel_band, None);
    }

    #[test]
    fn test_anti_expert_parity_cancellation_identity() {
        let direct_weights = [1i8, -1, 1, 0, -1, 0, 1, -1];
        let conjugate_weights: [i8; 8] = [
            -direct_weights[0],
            -direct_weights[1],
            -direct_weights[2],
            -direct_weights[3],
            -direct_weights[4],
            -direct_weights[5],
            -direct_weights[6],
            -direct_weights[7],
        ];

        let mut sum = 0i32;
        for i in 0..8 {
            sum += (direct_weights[i] + conjugate_weights[i]) as i32;
        }

        assert_eq!(sum, 0, "T + T* must strictly equal 0 for all tensor lanes");
    }

    #[test]
    fn test_sentinel_out_of_band_halt_and_moon_dispatch() {
        let mut fleet = ThreeBearsFleet::new();

        let witness = WitnessedNode::new(100, 0x12345678, &[10]);
        let m5_coord = M5Coordinate::ORIGIN;

        let direct = [1i8, 0, -1];
        let mirror = [-1i8, 0, 1];

        // Byte 254 = Anikwacasipisim (Whistling Spirit Moon)
        let output = fleet
            .step_fleet(254, &witness, 10, m5_coord, &direct, &mirror, 5)
            .expect("Step executed");

        assert_eq!(
            output.assist_direct.sentinel_band,
            Some(SentinelBand::Slot254)
        );
        assert!(!output.synchronized, "Sentinel token must desynchronize fleet to trigger intervention");
    }

    #[test]
    fn test_1_58_bit_ternary_dot_product_exactness() {
        let trits = [1i8, -1, 0, 1, -1];
        let packed = pack_5_trits(trits).expect("Valid 5-trit pack");
        let activations = [10_000i32, 5_000, 2_000, 8_000, 4_000];

        // Expected: (1 * 10000) + (-1 * 5000) + (0 * 2000) + (1 * 8000) + (-1 * 4000) = 10000 - 5000 + 8000 - 4000 = 9000
        let result = ternary_dot_product_5trit(packed, &activations).expect("Dot product calculated");
        assert_eq!(result, 9000);
    }
}
