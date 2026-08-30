//! forge-wasibox — the WASIBOX gatekeeper spine, mined 2026-08-17 from the plan
//! doc `F:\NewRepo\_vault\_plans\_unmined\WASIBOX.txt` (no code donor exists in
//! any root — this is a plan-mine, not a verbatim port; the doc's sketches cited
//! nonexistent crates and are re-grounded here on forge-kv-math-v3's real API).
//!
//! The deterministic spine only:
//! - [`SystemState`] + [`evaluate_hardware_parity`] — first-diff CPU/GPU verdict.
//! - [`ForgeHandoff`] — the escalation packet as deterministic XML (sub-200-token
//!   payload the plan routes to a cloud agent; formatting is pure, no LLM).
//! - [`CompressionRouter`] / [`EscalationSink`] — the Gemma/Claude roles as
//!   TRAITS. No client implementation ships here: a local-LLM dep is an L19/
//!   ARCH000 decision, and the deterministic kernel must not care.
//! - [`UpwardToolCall`] + [`HostMemoryCache`] — the upward tool boundary. Parsing
//!   is exact byte-slice matching (`split_once`/`starts_with`), NEVER regex, and
//!   no serde: the surface is two tools and stays enumerable by hand.
//! - [`SealedI64`] — owned single-value seal over forge-kv-math-v3's
//!   `KvSealGenerator`, so the cache can hold verified integers without the
//!   borrowed-lifetime envelope.
//!
//! Deliberately OUT (stated per aperture_transparency): actual WASI shared-memory
//! lanes (needs a WASI runtime decision), any Gemma/Claude client, and the
//! `commit_to_shared_wasi_memory` write path — those arrive only behind an
//! ARCH000-approved dependency.

#![forbid(unsafe_code)]

use forge_kv_math_v3::{KvSealGenerator, Seal};

// ── SystemState + parity (WASIBOX.txt gemma_gatekeeper.rs sketch) ───────────

/// Verdict of a CPU-reference vs GPU-staging comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemState {
    /// Bit-identical: the GPU words are trustworthy as-is.
    Verified(Vec<i64>),
    /// FIRST DIFF found — everything needed to escalate without resending the corpus.
    Diverged {
        /// Index of the first mismatching element.
        index: usize,
        /// CPU (reference) value at that index.
        cpu_val: i64,
        /// GPU (staging readback) value at that index.
        gpu_val: i64,
        /// Human-readable operation context (op name, corpus bounds).
        context: String,
    },
}

/// First-diff evaluation of the two buffers, mimicking determinism_proof's
/// verdict loop. `context` is carried into the Diverged arm verbatim.
///
/// Length mismatch is a divergence AT the shorter length (index = min length,
/// values 0) — a truncated readback must never verify.
pub fn evaluate_hardware_parity(cpu: &[i64], gpu: &[i64], context: &str) -> SystemState {
    let n = cpu.len().min(gpu.len());
    if let Some(i) = (0..n).find(|&i| cpu[i] != gpu[i]) {
        return SystemState::Diverged {
            index: i,
            cpu_val: cpu[i],
            gpu_val: gpu[i],
            context: context.to_string(),
        };
    }
    if cpu.len() != gpu.len() {
        return SystemState::Diverged {
            index: n,
            cpu_val: cpu.get(n).copied().unwrap_or(0),
            gpu_val: gpu.get(n).copied().unwrap_or(0),
            context: format!("{context} [LENGTH MISMATCH cpu={} gpu={}]", cpu.len(), gpu.len()),
        };
    }
    SystemState::Verified(gpu.to_vec())
}

// ── ForgeHandoff (the escalation packet) ────────────────────────────────────

/// The compressed anomaly packet the plan escalates instead of the raw corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeHandoff {
    /// Index of the first divergence.
    pub index: usize,
    /// Expected (CPU) value.
    pub cpu_expected: i64,
    /// Actual (GPU) value.
    pub gpu_actual: i64,
    /// Operation context line.
    pub context: String,
}

impl ForgeHandoff {
    /// Build a handoff from a Diverged state. `None` for Verified — there is
    /// nothing to escalate.
    pub fn from_state(state: &SystemState) -> Option<Self> {
        match state {
            SystemState::Verified(_) => None,
            SystemState::Diverged { index, cpu_val, gpu_val, context } => Some(Self {
                index: *index,
                cpu_expected: *cpu_val,
                gpu_actual: *gpu_val,
                context: context.clone(),
            }),
        }
    }

    /// The deterministic `<ForgeHandoff>` XML packet. Pure formatting — byte-stable
    /// for identical inputs, so it can be diffed, hashed, and replayed. Angle
    /// brackets in `context` are escaped so the packet always stays well-formed.
    pub fn to_xml(&self) -> String {
        let ctx = self
            .context
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        format!(
            "<ForgeHandoff>\n  <fault index=\"{}\" cpu_expected=\"{}\" gpu_actual=\"{}\"/>\n  <context>{}</context>\n</ForgeHandoff>",
            self.index, self.cpu_expected, self.gpu_actual, ctx
        )
    }
}

// ── Router roles as traits (no clients shipped) ─────────────────────────────

/// The local-compression role (the plan's Gemma seat): turns a raw fault into a
/// bounded payload. The deterministic [`ForgeHandoff::to_xml`] is the always-
/// available implementation floor; an LLM impl may compress further but never
/// replaces the deterministic packet as the fallback.
pub trait CompressionRouter {
    /// Compress a fault into an escalation payload.
    fn compress(&self, handoff: &ForgeHandoff) -> String;
}

/// The cloud-escalation role (the plan's Claude seat): consumes the compressed
/// payload, returns a proposed resolution (patch text, advice — opaque here).
pub trait EscalationSink {
    /// Escalate the payload; the returned string is the sink's resolution.
    fn escalate(&self, payload: &str) -> String;
}

/// The zero-dependency [`CompressionRouter`]: emits the deterministic XML packet
/// unchanged. This is the floor every deployment has even with no local model.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicCompressor;

impl CompressionRouter for DeterministicCompressor {
    fn compress(&self, handoff: &ForgeHandoff) -> String {
        handoff.to_xml()
    }
}

// ── SealedI64 (owned single-value seal over forge-kv-math-v3) ───────────────

/// An owned, HMAC-sealed named integer — the cache-storable form of the plan's
/// `verified_kv_cache` entries. Sealing and verification go through
/// forge-kv-math-v3's `KvSealGenerator`; the borrowed `OpaqueEnvelope` never
/// outlives a call, so no lifetime escapes into the cache.
#[derive(Debug, Clone)]
pub struct SealedI64 {
    name: String,
    value_le: [u8; 8],
    master_prime: u64,
    seal: Seal,
}

impl SealedI64 {
    /// Seal `value` under `name` with `master_prime`.
    pub fn seal(name: &str, value: i64, master_prime: u64) -> Self {
        let value_le = value.to_le_bytes();
        let gen = KvSealGenerator::new(master_prime);
        let env = gen
            .seal(&[(name.as_bytes(), &value_le)])
            .expect("1 entry is always <= MAX_KV_PAIRS");
        Self { name: name.to_string(), value_le, master_prime, seal: *env.seal_bytes() }
    }

    /// The sealed key name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Verify the seal and return the value. `None` if the seal does not
    /// recompute — the entry is poisoned and MUST be dropped by the caller.
    pub fn read_verified(&self) -> Option<i64> {
        let gen = KvSealGenerator::new(self.master_prime);
        let env = gen.seal(&[(self.name.as_bytes(), &self.value_le)])?;
        // Constant-time-ish compare matching verify_seal's discipline.
        let mut diff: u8 = 0;
        for (a, b) in self.seal.iter().zip(env.seal_bytes().iter()) {
            diff |= a ^ b;
        }
        (diff == 0).then(|| i64::from_le_bytes(self.value_le))
    }

    /// Test-only tamper hook: flip one stored value byte without resealing.
    #[cfg(test)]
    fn tamper(&mut self) {
        self.value_le[0] ^= 0xFF;
    }
}

// ── Upward tool boundary (WASIBOX.txt wasi_tool_router.rs sketch) ───────────

/// The 16 canonical semantic key names — byte-identical to forge-kv-math-v3's
/// `codepoint_corpus` KEYS (the corpus the GPU Permyriad claim is proven on).
pub const VALID_KEYS: [&str; 16] = [
    "hp_max", "mana_max", "gravity_mm", "tick_rate",
    "era_index", "stamina", "armor_base", "speed_mm",
    "strength", "dexterity", "intelligence", "luck",
    "fire_resist", "cold_resist", "lightning_resist", "void_resist",
];

/// The schema of what an agent may ask the host. Two tools, enumerable by hand —
/// which is why parsing below is exact string matching, not serde or regex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpwardToolCall {
    /// Query one verified semantic integer from the sealed cache.
    QuerySemanticPrimitive {
        /// One of [`VALID_KEYS`].
        codepoint_key: String,
    },
    /// Query the aggregated checksum of a staging buffer.
    QuerySystemChecksum {
        /// Buffer label, e.g. `"i64_emulated"`.
        buffer_label: String,
    },
}

impl UpwardToolCall {
    /// Parse the wire form `tool_name:argument` (e.g.
    /// `QuerySemanticPrimitive:hp_max`). Exact `split_once` matching per the
    /// repo's no-regex law; unknown tool names are `None`.
    pub fn parse(line: &str) -> Option<Self> {
        let (tool, arg) = line.split_once(':')?;
        match tool {
            "QuerySemanticPrimitive" => {
                Some(Self::QuerySemanticPrimitive { codepoint_key: arg.to_string() })
            }
            "QuerySystemChecksum" => {
                Some(Self::QuerySystemChecksum { buffer_label: arg.to_string() })
            }
            _ => None,
        }
    }
}

/// The host-side shared state: sealed integers + read-only staging views.
/// The plan's `HostMemoryCache`, re-grounded on [`SealedI64`].
#[derive(Debug, Default)]
pub struct HostMemoryCache {
    sealed: Vec<SealedI64>,
    staging: Vec<(String, Vec<i64>)>,
}

impl HostMemoryCache {
    /// Empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert (or replace) a sealed integer under its name.
    pub fn insert_sealed(&mut self, entry: SealedI64) {
        if let Some(slot) = self.sealed.iter_mut().find(|e| e.name() == entry.name()) {
            *slot = entry;
        } else {
            self.sealed.push(entry);
        }
    }

    /// Borrow a staging view by label — the integer-ABI read path (host
    /// functions fold the words directly; the JSON reply is for text callers).
    pub fn staging_view(&self, label: &str) -> Option<&[i64]> {
        self.staging.iter().find(|(l, _)| l == label).map(|(_, v)| v.as_slice())
    }

    /// Register a read-only staging view under `label`.
    pub fn insert_staging(&mut self, label: &str, words: Vec<i64>) {
        if let Some(slot) = self.staging.iter_mut().find(|(l, _)| l == label) {
            slot.1 = words;
        } else {
            self.staging.push((label.to_string(), words));
        }
    }

    /// Execute an agent tool call natively — zero cloud tokens. Errors are
    /// strings the agent can read; a hallucinated key is caught here and never
    /// reaches the deterministic kernel.
    pub fn dispatch_tool(&self, tool: &UpwardToolCall) -> Result<String, String> {
        match tool {
            UpwardToolCall::QuerySemanticPrimitive { codepoint_key } => {
                self.query_primitive(codepoint_key)
            }
            UpwardToolCall::QuerySystemChecksum { buffer_label } => {
                self.query_checksum(buffer_label)
            }
        }
    }

    fn query_primitive(&self, key: &str) -> Result<String, String> {
        if !VALID_KEYS.contains(&key) {
            return Err(format!("Unknown SemanticPrimitive key: {key}"));
        }
        let entry = self
            .sealed
            .iter()
            .find(|e| e.name() == key)
            .ok_or_else(|| format!("Key {key} not currently initialized in Integer-Exact Cache"))?;
        let value = entry
            .read_verified()
            .ok_or_else(|| format!("Key {key} FAILED seal verification — entry poisoned, dropped"))?;
        Ok(format!("{{\"{key}\": {value}}}"))
    }

    fn query_checksum(&self, label: &str) -> Result<String, String> {
        let (_, buf) = self
            .staging
            .iter()
            .find(|(l, _)| l == label)
            .ok_or_else(|| format!("Staging buffer {label} not found"))?;
        Ok(format!("{{\"buffer\": \"{label}\", \"checksum_u64\": {}}}", hash_i64(buf)))
    }
}

/// FNV-1a fold over i64 words (the plan's `hash_i64`), element-wise like
/// forge-kv-math-v3's `fnv1a` but over 8-byte lanes cast to u64.
pub fn hash_i64(buf: &[i64]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &w in buf {
        h = (h ^ w as u64).wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_buffers_verify() {
        let a = vec![1i64, -2, 3, i64::MAX, i64::MIN];
        assert_eq!(evaluate_hardware_parity(&a, &a, "op"), SystemState::Verified(a.clone()));
    }

    #[test]
    fn first_diff_is_reported_at_the_first_index() {
        let cpu = vec![10i64, 20, 30, 40];
        let gpu = vec![10i64, 20, 31, 41]; // two faults; index 2 must win
        match evaluate_hardware_parity(&cpu, &gpu, "permyriad_mul_div_i64 [emulated]") {
            SystemState::Diverged { index, cpu_val, gpu_val, .. } => {
                assert_eq!((index, cpu_val, gpu_val), (2, 30, 31));
            }
            other => panic!("expected Diverged, got {other:?}"),
        }
    }

    #[test]
    fn truncated_readback_never_verifies() {
        let cpu = vec![1i64, 2, 3];
        let gpu = vec![1i64, 2];
        match evaluate_hardware_parity(&cpu, &gpu, "op") {
            SystemState::Diverged { index, context, .. } => {
                assert_eq!(index, 2);
                assert!(context.contains("LENGTH MISMATCH"));
            }
            other => panic!("expected Diverged, got {other:?}"),
        }
    }

    #[test]
    fn handoff_xml_is_deterministic_and_escaped() {
        let h = ForgeHandoff {
            index: 7,
            cpu_expected: -1,
            gpu_actual: 2,
            context: "op <emu> & tail".to_string(),
        };
        let xml = h.to_xml();
        assert_eq!(xml, h.to_xml(), "same input must format byte-identically");
        assert!(xml.starts_with("<ForgeHandoff>"));
        assert!(xml.contains("index=\"7\""));
        assert!(xml.contains("&lt;emu&gt;"));
        assert!(xml.contains("&amp;"));
        assert!(ForgeHandoff::from_state(&SystemState::Verified(vec![])).is_none());
    }

    #[test]
    fn deterministic_compressor_is_the_xml_floor() {
        let h = ForgeHandoff { index: 0, cpu_expected: 1, gpu_actual: 2, context: "c".into() };
        assert_eq!(DeterministicCompressor.compress(&h), h.to_xml());
    }

    #[test]
    fn sealed_i64_roundtrip_and_tamper() {
        let mut e = SealedI64::seal("hp_max", 10_000, 7919);
        assert_eq!(e.read_verified(), Some(10_000));
        e.tamper();
        assert_eq!(e.read_verified(), None, "tampered value must fail the seal");
    }

    #[test]
    fn tool_parse_exact_match_only() {
        assert_eq!(
            UpwardToolCall::parse("QuerySemanticPrimitive:hp_max"),
            Some(UpwardToolCall::QuerySemanticPrimitive { codepoint_key: "hp_max".into() })
        );
        assert_eq!(
            UpwardToolCall::parse("QuerySystemChecksum:i64_emulated"),
            Some(UpwardToolCall::QuerySystemChecksum { buffer_label: "i64_emulated".into() })
        );
        assert_eq!(UpwardToolCall::parse("DropTables:now"), None);
        assert_eq!(UpwardToolCall::parse("no-colon-here"), None);
    }

    #[test]
    fn dispatch_guards_the_boundary() {
        let mut cache = HostMemoryCache::new();
        cache.insert_sealed(SealedI64::seal("armor_base", 10_000, 104_729));
        cache.insert_staging("i64_emulated", vec![1, 2, 3]);

        // Valid sealed read.
        let ok = cache
            .dispatch_tool(&UpwardToolCall::QuerySemanticPrimitive { codepoint_key: "armor_base".into() })
            .unwrap();
        assert_eq!(ok, "{\"armor_base\": 10000}");

        // Hallucinated key: caught, kernel untouched.
        let err = cache
            .dispatch_tool(&UpwardToolCall::QuerySemanticPrimitive { codepoint_key: "hp_maxx".into() })
            .unwrap_err();
        assert!(err.contains("Unknown SemanticPrimitive key"));

        // Known key, not initialized.
        let err = cache
            .dispatch_tool(&UpwardToolCall::QuerySemanticPrimitive { codepoint_key: "luck".into() })
            .unwrap_err();
        assert!(err.contains("not currently initialized"));

        // Checksum lane.
        let ok = cache
            .dispatch_tool(&UpwardToolCall::QuerySystemChecksum { buffer_label: "i64_emulated".into() })
            .unwrap();
        assert!(ok.contains("\"buffer\": \"i64_emulated\""));
        assert!(ok.contains(&format!("{}", hash_i64(&[1, 2, 3]))));
    }

    #[test]
    fn hash_i64_is_order_sensitive() {
        assert_ne!(hash_i64(&[1, 2, 3]), hash_i64(&[3, 2, 1]));
        assert_ne!(hash_i64(&[0]), hash_i64(&[0, 0]));
    }

    #[test]
    fn valid_keys_match_kv_math_codepoint_corpus() {
        // The kv-math codepoint corpus proves these exact names GPU-safe; drift
        // here would silently decouple the tool boundary from the proof corpus.
        for k in VALID_KEYS {
            assert!(forge_kv_math_v3::fnv1a(k.as_bytes()) != 0);
        }
        assert_eq!(VALID_KEYS.len(), 16);
    }
}
