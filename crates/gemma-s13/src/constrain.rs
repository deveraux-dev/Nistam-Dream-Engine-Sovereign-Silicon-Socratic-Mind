// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Weld-RON Constrained-Decode PDA & Lazy State-Cache Logit Masking Engine.
//!
//! Implements:
//! 1. Byte-level pushdown automaton (PDA) for Weld-RON grammar.
//! 2. `PdaStateId` deterministic fingerprinting for PDA state deduplication.
//! 3. `PdaStateCache`: pre-computed and lazy state-cache logit masking that clamps
//!    unauthorized / un-grammatical candidate token logits to `f32::NEG_INFINITY`,
//!    enforcing zero-runaway generation and natural termination.

#![deny(unsafe_code)]

#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "std")]
use std::string::{String, ToString};
#[cfg(feature = "std")]
use std::vec;
#[cfg(feature = "std")]
use std::vec::Vec;

/// Canonical Gemma PAD token ID (0).
pub const TOKEN_PAD: u32 = 0;
/// Canonical Gemma EOS token ID (1).
pub const TOKEN_EOS: u32 = 1;
/// Canonical Gemma BOS token ID (2).
pub const TOKEN_BOS: u32 = 2;
/// Canonical Gemma UNK token ID (3).
pub const TOKEN_UNK: u32 = 3;
/// Canonical Gemma <start_of_turn> control token ID (106).
pub const TOKEN_START_OF_TURN: u32 = 106;
/// Canonical Gemma <end_of_turn> control token ID (107).
pub const TOKEN_END_OF_TURN: u32 = 107;

/// What the grammar expects next at the current position.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expectation {
    /// An exact byte sequence, e.g. `Weld(lane:"`.
    Literal(Vec<u8>),
    /// One of several exact byte sequences (the closed `op` enum).
    OneOf(Vec<Vec<u8>>),
    /// Free string content, closed by an unescaped `"` which is re-dispatched.
    Str,
}

/// Unique 64-bit state identifier for a PDA snapshot.
pub type PdaStateId = u64;

/// Structured state descriptor for a PDA snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PdaStateDescriptor {
    /// Number of expectations remaining on stack.
    pub stack_depth: u16,
    /// Byte cursor inside current top expectation.
    pub cursor: u16,
    /// Escape flag active (following backslash in string).
    pub escape: bool,
    /// Complete / stack drained flag.
    pub done: bool,
    /// Top expectation discriminant (0 = None, 1 = Literal, 2 = OneOf, 3 = Str).
    pub top_discriminant: u8,
    /// Hash of the top expectation content if applicable.
    pub top_content_hash: u64,
}

/// The weld-RON pushdown automaton.
#[derive(Debug, Clone)]
pub struct WeldConstraint {
    /// Bytes accepted so far — the weld under construction.
    output: Vec<u8>,
    /// LIFO expectation stack; top is the next thing the grammar wants.
    stack: Vec<Expectation>,
    /// True once the stack drained: the weld is complete and no byte is valid.
    pub done: bool,
    /// Bytes of the top `Literal`/`OneOf` consumed so far.
    cursor: usize,
    /// Previous byte was a backslash inside `Str` (escape pending).
    escape: bool,
}

impl Default for WeldConstraint {
    fn default() -> Self {
        Self::new()
    }
}

impl WeldConstraint {
    /// Create a new Weld-RON pushdown automaton.
    pub fn new() -> Self {
        use Expectation::*;
        let l = |s: &str| Literal(s.as_bytes().to_vec());
        let ops = OneOf(vec![
            b"replace".to_vec(),
            b"before".to_vec(),
            b"after".to_vec(),
            b"delete".to_vec(),
        ]);
        let mut program: Vec<Expectation> = vec![
            l("Weld(lane:\""), Str,
            l("\",files:[F(p:\""), Str,
            l("\",edits:[E(anchor:\""), Str,
            l("\",op:\""), ops,
            l("\",payload:\""), Str,
            l("\")])],gate:\""), Str,
            l("\",receipt:\""), Str,
            l("\")"),
        ];
        program.reverse(); // stack is LIFO — top is the front of the grammar
        Self { output: Vec::new(), stack: program, done: false, cursor: 0, escape: false }
    }

    /// Compute the structured state descriptor for the current automaton state.
    pub fn state_descriptor(&self) -> PdaStateDescriptor {
        let (top_discriminant, top_content_hash) = match self.stack.last() {
            None => (0u8, 0u64),
            Some(Expectation::Literal(lit)) => {
                let mut h = 0xCBF2_9CE4_8422_2325u64;
                for &b in lit {
                    h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3);
                }
                (1u8, h)
            }
            Some(Expectation::OneOf(opts)) => {
                let mut h = 0xCBF2_9CE4_8422_2325u64;
                for opt in opts {
                    for &b in opt {
                        h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3);
                    }
                }
                (2u8, h)
            }
            Some(Expectation::Str) => (3u8, 0u64),
        };

        PdaStateDescriptor {
            stack_depth: self.stack.len() as u16,
            cursor: self.cursor as u16,
            escape: self.escape,
            done: self.done,
            top_discriminant,
            top_content_hash,
        }
    }

    /// Compute the 64-bit `PdaStateId` for cache indexing.
    pub fn state_id(&self) -> PdaStateId {
        let desc = self.state_descriptor();
        let mut h = 0xCBF2_9CE4_8422_2325u64;
        h = (h ^ desc.stack_depth as u64).wrapping_mul(0x0000_0100_0000_01B3);
        h = (h ^ desc.cursor as u64).wrapping_mul(0x0000_0100_0000_01B3);
        h = (h ^ (if desc.escape { 1 } else { 0 })).wrapping_mul(0x0000_0100_0000_01B3);
        h = (h ^ (if desc.done { 1 } else { 0 })).wrapping_mul(0x0000_0100_0000_01B3);
        h = (h ^ desc.top_discriminant as u64).wrapping_mul(0x0000_0100_0000_01B3);
        h ^ desc.top_content_hash
    }

    /// Check if a byte sequence is valid according to current state.
    pub fn accepts_bytes(&self, bytes: &[u8]) -> bool {
        self.accepts(bytes)
    }

    /// The set of valid next bytes, or `None` when any byte is valid (free string content).
    /// `Some(empty)` means generation must stop (done).
    pub fn valid_next_bytes(&self) -> Option<HashSet<u8>> {
        if self.done {
            return Some(HashSet::new());
        }
        match self.stack.last()? {
            Expectation::Literal(lit) => {
                let mut set = HashSet::new();
                if self.cursor < lit.len() {
                    set.insert(lit[self.cursor]);
                }
                Some(set)
            }
            Expectation::OneOf(options) => {
                let pref = &self.output[self.output.len().saturating_sub(self.cursor)..];
                let mut set = HashSet::new();
                for opt in options {
                    if opt.len() > self.cursor && opt[..self.cursor] == *pref {
                        set.insert(opt[self.cursor]);
                    }
                }
                Some(set)
            }
            Expectation::Str => {
                if self.escape {
                    Some([b'n', b't', b'r', b'\\', b'"'].into_iter().collect())
                } else {
                    None
                }
            }
        }
    }

    /// Pop the top expectation and reset per-expectation cursor state.
    fn pop_reset(&mut self) {
        self.stack.pop();
        self.cursor = 0;
        self.escape = false;
    }

    /// Feed one accepted byte forward.
    pub fn advance(&mut self, byte: u8) {
        self.output.push(byte);
        loop {
            let top = match self.stack.last() {
                Some(t) => t.clone(),
                None => break,
            };
            match top {
                Expectation::Literal(lit) => {
                    self.cursor += 1;
                    if self.cursor >= lit.len() {
                        self.pop_reset();
                    }
                    break;
                }
                Expectation::OneOf(opts) => {
                    self.cursor += 1;
                    let pref = &self.output[self.output.len().saturating_sub(self.cursor)..];
                    let matched =
                        opts.iter().any(|o| o.len() == self.cursor && o.as_slice() == pref);
                    let longer =
                        opts.iter().any(|o| o.len() > self.cursor && o[..self.cursor] == *pref);
                    if matched && !longer {
                        self.pop_reset();
                    }
                    break;
                }
                Expectation::Str => {
                    if self.escape {
                        self.escape = false;
                        break;
                    }
                    if byte == b'\\' {
                        self.escape = true;
                        break;
                    }
                    if byte == b'"' {
                        self.pop_reset();
                        continue;
                    }
                    break;
                }
            }
        }
        if self.stack.is_empty() {
            self.done = true;
        }
    }

    /// Check if this whole byte sequence is legal from the current state.
    pub fn accepts(&self, bytes: &[u8]) -> bool {
        if bytes.is_empty() {
            return false;
        }
        let mut probe = self.clone();
        for &b in bytes {
            if probe.done {
                return false;
            }
            if let Some(valid) = probe.valid_next_bytes() {
                if !valid.contains(&b) {
                    return false;
                }
            }
            probe.advance(b);
        }
        true
    }

    /// Check if the grammar is completely fulfilled and ready for natural <end_of_turn> termination.
    #[inline(always)]
    pub fn is_terminal(&self) -> bool {
        self.done
    }

    /// The accepted weld text so far.
    pub fn output_str(&self) -> String {
        String::from_utf8_lossy(&self.output).to_string()
    }
}

/// Lazy State-Cache Logit Masking Engine for Pushdown Automata.
#[derive(Debug, Clone, Default)]
pub struct PdaStateCache {
    /// Maps a specific grammar state to the allowed 262k token IDs
    pub valid_tokens_map: HashMap<PdaStateId, Vec<u32>>,
}

impl PdaStateCache {
    /// Create a new empty PDA state-cache.
    pub fn new() -> Self {
        Self {
            valid_tokens_map: HashMap::new(),
        }
    }

    /// Number of cached unique PDA states.
    pub fn len(&self) -> usize {
        self.valid_tokens_map.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.valid_tokens_map.is_empty()
    }

    /// Fetches allowed tokens, computing them on a cache miss.
    pub fn get_or_compute_valid_tokens<F>(
        &mut self,
        state_id: PdaStateId,
        grammar: &WeldConstraint,
        vocab_size: usize,
        token_to_bytes: F,
    ) -> &[u32]
    where
        F: Fn(u32) -> Option<&'static [u8]>,
    {
        self.valid_tokens_map.entry(state_id).or_insert_with(|| {
            if grammar.done {
                return Vec::new();
            }
            let mut valid = Vec::new();
            for token_id in 0..vocab_size as u32 {
                if let Some(bytes) = token_to_bytes(token_id) {
                    if grammar.accepts_bytes(bytes) {
                        valid.push(token_id);
                    }
                }
            }
            valid
        })
    }

    /// Mask logits in-place to enforce zero runaway generation and natural `<end_of_turn>` termination.
    ///
    /// Sets all invalid token logits to `f32::NEG_INFINITY`.
    /// When `grammar.done` is true, masks all tokens except `eos_token_id` and `TOKEN_END_OF_TURN`.
    pub fn mask_logits<F>(
        &mut self,
        logits: &mut [f32],
        grammar: &WeldConstraint,
        eos_token_id: usize,
        token_to_bytes: F,
    ) where
        F: Fn(u32) -> Option<&'static [u8]>,
    {
        let state_id = grammar.state_id();
        let vocab_size = logits.len();
        if grammar.done {
            let eot_id = TOKEN_END_OF_TURN as usize;
            for (idx, logit) in logits.iter_mut().enumerate() {
                if idx != eos_token_id && idx != eot_id {
                    *logit = f32::NEG_INFINITY;
                }
            }
            return;
        }

        let valid_slice = self.get_or_compute_valid_tokens(state_id, grammar, vocab_size, token_to_bytes);
        let mut allowed = vec![false; vocab_size];
        for &tok in valid_slice {
            if (tok as usize) < vocab_size {
                allowed[tok as usize] = true;
            }
        }

        for (idx, logit) in logits.iter_mut().enumerate() {
            if !allowed[idx] {
                *logit = f32::NEG_INFINITY;
            }
        }
    }

    /// Convenience method to mask logits specifically for natural `<end_of_turn>` termination (token 107).
    pub fn mask_logits_natural_eot<F>(
        &mut self,
        logits: &mut [f32],
        grammar: &WeldConstraint,
        token_to_bytes: F,
    ) where
        F: Fn(u32) -> Option<&'static [u8]>,
    {
        self.mask_logits(logits, grammar, TOKEN_END_OF_TURN as usize, token_to_bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WELD: &str = r#"Weld(lane:"W1",files:[F(p:"crates/x/src/lib.rs",edits:[E(anchor:"fn old()",op:"replace",payload:"fn new()")])],gate:"cargo check -p x",receipt:"read-W1.json")"#;

    #[test]
    fn test_weld_pda_full_trace_completes_on_drain() {
        let mut c = WeldConstraint::new();
        for (i, &b) in WELD.as_bytes().iter().enumerate() {
            assert!(!c.done, "done fired early at byte {i}");
            if let Some(valid) = c.valid_next_bytes() {
                assert!(valid.contains(&b), "byte {i} illegal");
            }
            c.advance(b);
        }
        assert!(c.done, "fully consumed weld must read done");
        assert!(c.is_terminal(), "is_terminal must be true on drain");
        assert_eq!(c.output_str(), WELD);
    }

    #[test]
    fn test_pda_state_id_and_lazy_cache_masking() {
        let mut cache = PdaStateCache::new();
        let c = WeldConstraint::new();
        let state_id = c.state_id();

        // Mock vocab: 0="Weld(lane:\"", 1="fn main()", 2="W", 3="other"
        let vocab = [
            b"Weld(lane:\"".as_slice(),
            b"fn main()".as_slice(),
            b"W".as_slice(),
            b"other".as_slice(),
        ];
        let token_fn = |id: u32| vocab.get(id as usize).copied();

        let allowed = cache.get_or_compute_valid_tokens(state_id, &c, vocab.len(), token_fn);
        // "Weld(lane:\"" and "W" are legal prefixes
        assert!(allowed.contains(&0));
        assert!(allowed.contains(&2));
        assert!(!allowed.contains(&1));
        assert!(!allowed.contains(&3));

        let mut logits = [10.0f32, 100.0, 5.0, 50.0];
        cache.mask_logits(&mut logits, &c, 999, token_fn);

        assert_eq!(logits[0], 10.0);
        assert_eq!(logits[1], f32::NEG_INFINITY);
        assert_eq!(logits[2], 5.0);
        assert_eq!(logits[3], f32::NEG_INFINITY);
    }

    #[test]
    fn test_natural_end_of_turn_termination_on_drain() {
        let mut cache = PdaStateCache::new();
        let mut c = WeldConstraint::new();

        // Feed full WELD text so grammar completes and drains
        for &b in WELD.as_bytes() {
            c.advance(b);
        }
        assert!(c.is_terminal());

        let mut logits = vec![10.0f32; 256];
        let token_fn = |_id: u32| None;

        cache.mask_logits_natural_eot(&mut logits, &c, token_fn);

        // Token 107 (TOKEN_END_OF_TURN) must survive
        assert_eq!(logits[TOKEN_END_OF_TURN as usize], 10.0);
        // Other arbitrary tokens must be masked to NEG_INFINITY
        assert_eq!(logits[0], f32::NEG_INFINITY);
        assert_eq!(logits[50], f32::NEG_INFINITY);
        assert_eq!(logits[200], f32::NEG_INFINITY);
    }
}
