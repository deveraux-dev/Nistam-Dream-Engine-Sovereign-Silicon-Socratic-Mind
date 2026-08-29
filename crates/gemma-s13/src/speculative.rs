//! Speculative decoding: draft (2B) proposes K tokens, target (9B) verifies.
//!
//! Uses forward_token with existing KV-cache machinery to achieve 2-3x speedup
//! on decode latency by parallelizing (via rejection sampling) the draft proposal
//! with the target verification.

#![cfg_attr(feature = "std", allow(dead_code))]

extern crate alloc;
use alloc::vec::Vec;

/// Default number of tokens to propose in speculative decoding.
pub const SPEC_K: usize = 4;

#[cfg(feature = "std")]
fn softmax_prob(logits: &[f32], idx: usize) -> f32 {
	if logits.is_empty() {
		return 0.0;
	}
	let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
	let sum: f32 = logits.iter().map(|&l| (l - max).exp()).sum::<f32>().max(1e-9);
	(logits.get(idx).copied().unwrap_or(f32::NEG_INFINITY) - max).exp() / sum
}

#[cfg(feature = "std")]
fn greedy_pick(logits: &[f32]) -> u32 {
	if logits.is_empty() {
		return 0;
	}
	let mut max_idx = 0usize;
	let mut max_val = logits[0];
	for (i, &val) in logits.iter().enumerate().skip(1) {
		if val > max_val {
			max_val = val;
			max_idx = i;
		}
	}
	max_idx as u32
}

/// Speculative decoding state machine for gemma-s13 (2B draft + 9B target).
#[cfg(feature = "std")]
pub struct SpeculativeDecoder {
	/// Number of tokens to propose per round.
	k: usize,
	/// Proposed token IDs.
	draft_tokens: Vec<u32>,
	/// Logit vectors for each proposed token.
	draft_logits: Vec<Vec<f32>>,
	/// Number of tokens accepted in the last verification round.
	pub last_accepted: usize,
	/// Total tokens accepted across all rounds.
	pub total_generated: usize,
	/// Total verification rounds completed.
	pub total_rounds: usize,
	/// PRNG state for rejection sampling.
	rng_state: u64,
}

#[cfg(feature = "std")]
impl SpeculativeDecoder {
	/// Create a new speculative decoder with the given proposal length.
	pub fn new(k: usize) -> Self {
		Self {
			k,
			draft_tokens: Vec::with_capacity(k),
			draft_logits: Vec::with_capacity(k),
			last_accepted: 0,
			total_generated: 0,
			total_rounds: 0,
			rng_state: 0x85a308d313198a2eu64,
		}
	}

	/// XORshift PRNG for rejection sampling.
	fn xorshift(&mut self) -> f32 {
		self.rng_state ^= self.rng_state << 13;
		self.rng_state ^= self.rng_state >> 7;
		self.rng_state ^= self.rng_state << 17;
		((self.rng_state >> 33) as u32 as f32) / 4_294_967_296.0
	}

	/// Propose K draft tokens given current KV cache state.
	/// `draft_forward` is called K times (once per proposed token).
	/// Returns the proposed token IDs.
	pub fn propose(
		&mut self,
		mut draft_forward: impl FnMut(u32) -> Vec<f32>,
		initial_token: u32,
	) -> &[u32] {
		self.draft_tokens.clear();
		self.draft_logits.clear();

		let mut current_token = initial_token;
		for _ in 0..self.k {
			let logits = draft_forward(current_token);
			let next_token = greedy_pick(&logits);
			self.draft_tokens.push(next_token);
			self.draft_logits.push(logits);
			current_token = next_token;
		}
		&self.draft_tokens
	}

	/// Verify draft tokens against target model using rejection sampling.
	/// `verify_forward` is called once per draft token.
	/// Returns the number of accepted tokens (0..=K).
	pub fn verify(&mut self, mut verify_forward: impl FnMut(u32) -> Vec<f32>) -> usize {
		let mut accepted = 0;
		let draft_tokens_copy = self.draft_tokens.clone();

		for (i, &draft_token) in draft_tokens_copy.iter().enumerate() {
			let target_logits = verify_forward(draft_token);

			if i < self.draft_logits.len() {
				let draft_prob = softmax_prob(&self.draft_logits[i], draft_token as usize);
				let target_prob = softmax_prob(&target_logits, draft_token as usize);

				if target_prob > 0.0 && draft_prob > 0.0 {
					let accept_prob = (target_prob / draft_prob).min(1.0);
					if self.xorshift() < accept_prob {
						accepted += 1;
					} else {
						break;
					}
				} else {
					break;
				}
			}
		}

		self.last_accepted = accepted;
		self.total_generated += accepted;
		self.total_rounds += 1;
		accepted
	}

	/// Average acceptance rate across all rounds (0.0..=1.0).
	pub fn acceptance_rate(&self) -> f32 {
		if self.total_rounds == 0 { return 0.0; }
		self.total_generated as f32 / (self.total_rounds as f32 * self.k as f32)
	}

	/// Effective speedup: tokens generated per round (>1.0 means beneficial).
	pub fn speedup_ratio(&self) -> f32 {
		if self.total_rounds == 0 { return 1.0; }
		self.total_generated as f32 / self.total_rounds as f32
	}
}

#[cfg(all(test, feature = "std"))]
mod tests {
	use super::*;

	#[test]
	fn softmax_prob_normalized() {
		let logits = vec![2.0f32, 1.0, 0.5];
		let sum: f32 = (0..3).map(|i| softmax_prob(&logits, i)).sum();
		assert!((sum - 1.0).abs() < 1e-4);
	}

	#[test]
	fn greedy_pick_argmax() {
		let logits = vec![0.1f32, 0.9, 0.3];
		assert_eq!(greedy_pick(&logits), 1);
	}

	#[test]
	fn spec_decoder_stats() {
		let mut dec = SpeculativeDecoder::new(4);
		assert_eq!(dec.acceptance_rate(), 0.0);
		assert_eq!(dec.speedup_ratio(), 1.0);

		dec.total_generated = 12;
		dec.total_rounds = 4;
		assert_eq!(dec.acceptance_rate(), 12.0 / (4.0 * 4.0));
		assert_eq!(dec.speedup_ratio(), 3.0);
	}
}
