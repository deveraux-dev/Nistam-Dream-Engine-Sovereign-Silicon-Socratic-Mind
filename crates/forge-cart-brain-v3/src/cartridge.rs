//! Cartridge domain — "the cartridge owns MEANING" (backtick.yaml).
//!
//! The compiled-cartridge config the brain LOADS at runtime. Author-time, a
//! `.kit.vixi` is baked (by the compiler, host-side, native) into these POD
//! bytes + a `cartridge.brutalhash`; at runtime the brain deserializes them
//! (zero parse, edge-safe) — it never sees the `.vixi` text. This is the LOAD
//! side of the funnel; the author-time baker is the compiler's job (rung 2).
//! Per-domain tables (zone / authority / sensory / render / proof) append here
//! as later phases land.
//!
//! PORT RECEIPT (2026-08-16): ported verbatim from `F:\NewRepo\crates\
//! forge-cart-brain\src\cartridge.rs`. Only the sink import path changed
//! (`forge_cart_sink` -> `forge_cart_sink_v3`), plus doc comments added on
//! every public item the v2 crate left undocumented (workspace lint
//! `missing_docs = "deny"` forces it; v2's did not).

use forge_cart_sink_v3::DeterminismSink;

/// Magic + version tag for the compiled-cartridge byte format ("RDR1").
pub const CART_MAGIC: u32 = 0x5244_5231;

/// Serialized length of [`CartridgeConfig::to_bytes`]:
/// magic(4) + id(8) + seed(8) + hash(8) + players(1) + tick_hz(2).
pub const CART_CONFIG_BYTES: usize = 4 + 8 + 8 + 8 + 1 + 2;

/// Byte length hashed by [`CartridgeConfig::derive_hash`] (identity fields only,
/// excluding the stored `cartridge_hash`).
const CART_HASH_PREIMAGE_BYTES: usize = 4 + 8 + 8 + 1 + 2;

/// The compiled cartridge config — the MEANING the engine executes. POD, Copy,
/// edge-safe (fixed LE layout). The author-time baker produces it from a
/// `.kit.vixi`; the runtime brain loads it via [`from_bytes`](Self::from_bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CartridgeConfig {
    /// Cartridge identity, stable across bake/load round-trips.
    pub cartridge_id: u64,
    /// Seed the brain's determinism is rooted from.
    pub master_seed: u64,
    /// `cartridge.brutalhash` — stamped by the author-time baker (live
    /// `BrutalHash`); the brain re-verifies it on load via the sink.
    pub cartridge_hash: u64,
    /// Player count this cartridge was baked for.
    pub player_count: u8,
    /// Tick rate in Hz (120 for the RunDevRun arena).
    pub tick_hz: u16,
}

impl Default for CartridgeConfig {
    fn default() -> Self {
        Self {
            cartridge_id: 0,
            master_seed: 0,
            cartridge_hash: 0,
            player_count: 1,
            tick_hz: 120,
        }
    }
}

impl CartridgeConfig {
    /// Serialize to the compiled-cartridge byte format (LE, fixed length).
    pub fn to_bytes(&self) -> [u8; CART_CONFIG_BYTES] {
        let mut b = [0u8; CART_CONFIG_BYTES];
        b[0..4].copy_from_slice(&CART_MAGIC.to_le_bytes());
        b[4..12].copy_from_slice(&self.cartridge_id.to_le_bytes());
        b[12..20].copy_from_slice(&self.master_seed.to_le_bytes());
        b[20..28].copy_from_slice(&self.cartridge_hash.to_le_bytes());
        b[28] = self.player_count;
        b[29..31].copy_from_slice(&self.tick_hz.to_le_bytes());
        b
    }

    /// Deserialize from compiled-cartridge bytes. `None` on magic mismatch or a
    /// short buffer (the only failure modes — the layout is fixed).
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < CART_CONFIG_BYTES {
            return None;
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
        if magic != CART_MAGIC {
            return None;
        }
        Some(Self {
            cartridge_id: u64::from_le_bytes(bytes[4..12].try_into().ok()?),
            master_seed: u64::from_le_bytes(bytes[12..20].try_into().ok()?),
            cartridge_hash: u64::from_le_bytes(bytes[20..28].try_into().ok()?),
            player_count: bytes[28],
            tick_hz: u16::from_le_bytes(bytes[29..31].try_into().ok()?),
        })
    }

    /// Re-derive `cartridge.brutalhash` over the identity fields via the live
    /// `BrutalHash` sink (excludes the stored `cartridge_hash` itself).
    pub fn derive_hash(&self, rng: &dyn DeterminismSink) -> u64 {
        let mut b = [0u8; CART_HASH_PREIMAGE_BYTES];
        b[0..4].copy_from_slice(&CART_MAGIC.to_le_bytes());
        b[4..12].copy_from_slice(&self.cartridge_id.to_le_bytes());
        b[12..20].copy_from_slice(&self.master_seed.to_le_bytes());
        b[20] = self.player_count;
        b[21..23].copy_from_slice(&self.tick_hz.to_le_bytes());
        rng.hash_state(&b)
    }

    /// Stamp `cartridge_hash` from the live sink — the bake step's final act.
    pub fn sealed(mut self, rng: &dyn DeterminismSink) -> Self {
        self.cartridge_hash = self.derive_hash(rng);
        self
    }

    /// Verify the stored hash matches a fresh derivation (load-time integrity).
    pub fn verify(&self, rng: &dyn DeterminismSink) -> bool {
        self.cartridge_hash == self.derive_hash(rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_cart_sink_v3::NullDeterminism;

    #[test]
    fn config_byte_roundtrip_is_identity() {
        let rng = NullDeterminism::new(0);
        let cfg = CartridgeConfig {
            cartridge_id: 0xDEAD,
            master_seed: 0xBEEF,
            cartridge_hash: 0,
            player_count: 4,
            tick_hz: 120,
        }
        .sealed(&rng);
        let bytes = cfg.to_bytes();
        let back = CartridgeConfig::from_bytes(&bytes).expect("valid magic");
        assert_eq!(cfg, back, "compiled-cartridge bytes must round-trip identically");
    }

    #[test]
    fn from_bytes_rejects_bad_magic() {
        let mut bytes = CartridgeConfig::default().to_bytes();
        bytes[0] ^= 0xFF; // corrupt the magic
        assert!(CartridgeConfig::from_bytes(&bytes).is_none(), "bad magic must be rejected");
    }

    #[test]
    fn from_bytes_rejects_short_buffer() {
        assert!(CartridgeConfig::from_bytes(&[0u8; 4]).is_none(), "short buffer must be rejected");
    }

    #[test]
    fn sealed_config_verifies_and_detects_tamper() {
        let rng = NullDeterminism::new(0);
        let cfg = CartridgeConfig {
            cartridge_id: 1,
            master_seed: 2,
            cartridge_hash: 0,
            player_count: 2,
            tick_hz: 120,
        }
        .sealed(&rng);
        assert!(cfg.verify(&rng), "a freshly sealed config must verify");
        let mut tampered = cfg;
        tampered.master_seed = 999; // tamper without re-sealing
        assert!(
            !tampered.verify(&rng),
            "tampered seed must fail the cartridge_hash check (discriminator)"
        );
    }
}
