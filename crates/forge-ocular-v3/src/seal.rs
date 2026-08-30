//! seal — the spitshader: frozen, sha2-verifiable SPIR-V for the 5D pentaract kernel.
//!
//! Embeds the sealed pentaract_march_5d compute kernel and verifies it with sha2
//! ALONE — no naga, no GPU, no runtime compile. The seal IS the standing proof:
//! flip any byte and [`SealedKernel::verify`] goes false. That is what takes this
//! kernel off the DET-proof treadmill — proven once at seal time, permanent +
//! tamper-evident thereafter.
//!
//! Regenerate the blob when the kernel changes (v2 tree until forge-shader-build
//! has a v3 home):
//!   cargo run -p forge-ocular-v3 --example seal_pentaract_march_5d

use sha2::{Digest, Sha256};

/// Header magic — Forge OCular Sealed.
pub const MAGIC: [u8; 4] = *b"FOCS";
/// Seal format version.
pub const VERSION: u32 = 1;
/// Fixed header length; the SPIR-V bytes follow immediately after.
pub const HEADER_LEN: usize = 96;
/// SPIR-V module magic word, little-endian (first 4 bytes of any valid module: 0x07230203).
const SPIRV_MAGIC: [u8; 4] = [0x03, 0x02, 0x23, 0x07];

/// The sealed pentaract_march_5d compute kernel.
pub static SEALED_PENTARACT_MARCH_5D: &[u8] =
    include_bytes!("../proof/sealed/pentaract_march_5d.spv.sealed");

// TODO: Phase 4.1 — generate via cargo run -p forge-ocular-v3 --example seal_pentaract

/// A parsed, borrowed view over a sealed pentaract kernel blob (zero-copy).
#[derive(Debug, Clone, Copy)]
pub struct SealedPentaractKernel<'a> {
    blob: &'a [u8],
}

impl<'a> SealedPentaractKernel<'a> {
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

#[cfg(test)]
mod tests {
    use super::*;

    const SRC_PENTARACT: &str = include_str!("../shaders/pentaract_march_5d.wgsl");

    #[test]
    fn pentaract_seal_verifies() {
        let s = SealedPentaractKernel::parse(SEALED_PENTARACT_MARCH_5D)
            .expect("pentaract seal parses");
        assert!(s.verify(), "pentaract_march_5d sealed SPIR-V must verify");
        assert_eq!(s.version(), VERSION);
        assert!(s.spirv_len() > 0);
    }

    #[test]
    fn pentaract_seal_binds_its_source() {
        let s = SealedPentaractKernel::parse(SEALED_PENTARACT_MARCH_5D)
            .expect("pentaract seal parses");
        assert!(
            s.matches_source(SRC_PENTARACT),
            "pentaract seal must bind pentaract_march_5d.wgsl"
        );
    }

    #[test]
    fn tamper_breaks_the_seal() {
        let mut owned = SEALED_PENTARACT_MARCH_5D.to_vec();
        let last = owned.len() - 1;
        owned[last] ^= 0xFF;
        let s = SealedPentaractKernel::parse(&owned).unwrap();
        assert!(!s.verify(), "a single flipped SPIR-V byte must break the seal");
    }

    #[test]
    fn bad_blob_is_rejected() {
        assert!(SealedPentaractKernel::parse(b"not a seal").is_none());
        assert!(SealedPentaractKernel::parse(&[]).is_none());
    }
}
