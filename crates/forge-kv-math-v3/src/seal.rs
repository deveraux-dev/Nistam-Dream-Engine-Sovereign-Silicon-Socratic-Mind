//! seal — the spitshader: frozen, sha2-verifiable SPIR-V for the determinism kernels.
//!
//! v2's `examples/seal_kernels.rs` compiled the integer-only WGSL through the ONE naga user
//! (`forge-shader-build`) and froze `source_sha256 -> spirv_sha256` into a 96-byte header
//! ahead of the SPIR-V. This module EMBEDS those sealed blobs and verifies them with sha2
//! ALONE — no naga, no GPU, no runtime compile. The seal IS the standing proof: flip any
//! byte and [`SealedKernel::verify`] goes false. That is what takes these kernels off the
//! DET-proof treadmill — proven once at seal time, permanent + tamper-evident thereafter.
//!
//! Regenerate the blobs only when a kernel changes (v2 tree until forge-shader-build
//! has a v3 home):
//!   cargo run -p forge-kv-math --example seal_kernels

use sha2::{Digest, Sha256};

use crate::registry::SemanticPrimitive;

/// Header magic — Forge KV Sealed.
pub const MAGIC: [u8; 4] = *b"FKVS";
/// Seal format version.
pub const VERSION: u32 = 1;
/// Fixed header length; the SPIR-V bytes follow immediately after.
pub const HEADER_LEN: usize = 96;
/// SPIR-V module magic word, little-endian (first 4 bytes of any valid module: 0x07230203).
const SPIRV_MAGIC: [u8; 4] = [0x03, 0x02, 0x23, 0x07];

/// The sealed u32 kernel (`prismatic_hash_u32`, inv #7).
pub static SEALED_PRISMATIC_HASH: &[u8] = include_bytes!("../proof/sealed/kernel.spv.sealed");
/// The sealed emulated-i64 kernel (`permyriad_mul_div`, inv #156) — the cross-vendor cornerstone.
pub static SEALED_PERMYRIAD_EMU: &[u8] = include_bytes!("../proof/sealed/kernel_i64_emu.spv.sealed");

/// A parsed, borrowed view over a sealed kernel blob (zero-copy).
#[derive(Debug, Clone, Copy)]
pub struct SealedKernel<'a> {
    blob: &'a [u8],
}

impl<'a> SealedKernel<'a> {
    /// Parse a sealed blob. `None` if the header is malformed, the magic is wrong, or the
    /// declared SPIR-V length does not match the blob exactly.
    pub fn parse(blob: &'a [u8]) -> Option<Self> {
        if blob.len() < HEADER_LEN || blob[0..4] != MAGIC {
            return None;
        }
        let s = Self { blob };
        if HEADER_LEN + s.spirv_len() != blob.len() {
            return None;
        }
        Some(s)
    }

    fn u32_at(&self, off: usize) -> u32 {
        u32::from_le_bytes([
            self.blob[off],
            self.blob[off + 1],
            self.blob[off + 2],
            self.blob[off + 3],
        ])
    }

    /// Seal format version stamped in the header.
    pub fn version(&self) -> u32 {
        self.u32_at(4)
    }
    /// Byte length of the WGSL source the seal binds.
    pub fn source_len(&self) -> usize {
        self.u32_at(8) as usize
    }
    /// Byte length of the frozen SPIR-V module.
    pub fn spirv_len(&self) -> usize {
        self.u32_at(12) as usize
    }
    /// Frozen sha256 digest of the WGSL source.
    pub fn source_sha256(&self) -> &[u8] {
        &self.blob[16..48]
    }
    /// Frozen sha256 digest of the SPIR-V bytes.
    pub fn spirv_sha256(&self) -> &[u8] {
        &self.blob[48..80]
    }

    /// The frozen SPIR-V bytes.
    pub fn spirv(&self) -> &'a [u8] {
        &self.blob[HEADER_LEN..]
    }

    /// The seal holds: the embedded bytes are a real SPIR-V module (magic word) whose
    /// sha256 matches the frozen digest. Tamper any byte and this goes false.
    pub fn verify(&self) -> bool {
        let spv = self.spirv();
        spv.len() >= 4
            && spv[0..4] == SPIRV_MAGIC
            && Sha256::digest(spv).as_slice() == self.spirv_sha256()
    }

    /// This sealed SPIR-V is the compile of exactly `wgsl_source` (the source-binding proof).
    pub fn matches_source(&self, wgsl_source: &str) -> bool {
        self.source_len() == wgsl_source.len()
            && Sha256::digest(wgsl_source.as_bytes()).as_slice() == self.source_sha256()
    }
}

/// The sealed kernel for a semantic primitive, if one was spat. All three portable prims
/// map to a seal (the two i64 prims share the emulated-i64 kernel; the SHADER_INT64
/// fast-path is intentionally not sealed here — it is device-specific).
pub fn sealed_for(id: SemanticPrimitive) -> Option<SealedKernel<'static>> {
    let blob: &'static [u8] = match id {
        SemanticPrimitive::PrismaticHashU32 => SEALED_PRISMATIC_HASH,
        SemanticPrimitive::PermyriadMulDivI64 => SEALED_PERMYRIAD_EMU,
        SemanticPrimitive::StatCodepointPermyriad => SEALED_PERMYRIAD_EMU,
    };
    SealedKernel::parse(blob)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The exact WGSL the blobs were sealed FROM (compile-time file read of the same source
    // the example fed to naga) — proves the seal binds THESE bytes, not a stale artifact.
    const SRC_PRISMATIC: &str = include_str!("../proof/kernel.wgsl");
    const SRC_EMU: &str = include_str!("../proof/kernel_i64_emu.wgsl");

    #[test]
    fn both_seals_verify() {
        let a = SealedKernel::parse(SEALED_PRISMATIC_HASH).expect("prismatic seal parses");
        let b = SealedKernel::parse(SEALED_PERMYRIAD_EMU).expect("emu seal parses");
        assert!(a.verify(), "prismatic_hash sealed SPIR-V must verify");
        assert!(b.verify(), "permyriad emu sealed SPIR-V must verify");
        assert_eq!(a.version(), VERSION);
        assert!(a.spirv_len() > 0 && b.spirv_len() > 0);
    }

    #[test]
    fn seals_bind_their_source() {
        let a = SealedKernel::parse(SEALED_PRISMATIC_HASH).unwrap();
        let b = SealedKernel::parse(SEALED_PERMYRIAD_EMU).unwrap();
        assert!(a.matches_source(SRC_PRISMATIC), "prismatic seal must bind kernel.wgsl");
        assert!(b.matches_source(SRC_EMU), "emu seal must bind kernel_i64_emu.wgsl");
        // a seal must NOT match the other kernel's source.
        assert!(!a.matches_source(SRC_EMU));
    }

    #[test]
    fn tamper_breaks_the_seal() {
        let mut owned = SEALED_PERMYRIAD_EMU.to_vec();
        let last = owned.len() - 1;
        owned[last] ^= 0xFF; // flip one SPIR-V byte
        let s = SealedKernel::parse(&owned).unwrap();
        assert!(!s.verify(), "a single flipped SPIR-V byte must break the seal");
    }

    #[test]
    fn registry_maps_every_prim_to_a_seal() {
        assert!(sealed_for(SemanticPrimitive::PrismaticHashU32).is_some());
        assert!(sealed_for(SemanticPrimitive::PermyriadMulDivI64).is_some());
        assert!(sealed_for(SemanticPrimitive::StatCodepointPermyriad).is_some());
    }

    #[test]
    fn bad_blob_is_rejected() {
        assert!(SealedKernel::parse(b"not a seal").is_none());
        assert!(SealedKernel::parse(&[]).is_none());
    }
}
