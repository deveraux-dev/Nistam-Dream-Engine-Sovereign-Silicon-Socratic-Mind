//! forge-shader-build-v3 — minimal WGSL → SPIR-V + SHA256 for seal generation.
//!
//! Ported 2026-08 from F:\NewRepo\crates\forge-shader-build (v2).
//! Exposes only `compile_spv` and `hash_bytes_sha256` — enough to seal
//! pentaract_march_5d.wgsl offline, commit the sealed blob, and verify at runtime.

use sha2::{Sha256, Digest};

/// Compile WGSL source string to SPIR-V bytes.
pub fn compile_spv(wgsl_source: &str) -> Result<Vec<u8>, String> {
    let module = naga::front::wgsl::parse_str(wgsl_source)
        .map_err(|e| format!("WGSL parse error: {e}"))?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = validator
        .validate(&module)
        .map_err(|e| format!("validation error: {e}"))?;

    let options = naga::back::spv::Options {
        lang_version: (1, 3),
        ..Default::default()
    };
    let spv = naga::back::spv::write_vec(&module, &info, &options, None)
        .map_err(|e| format!("spv generation error: {e}"))?;

    let spv_bytes: Vec<u8> = spv.iter().flat_map(|word| word.to_le_bytes()).collect();
    Ok(spv_bytes)
}

/// Compute SHA-256 hash of raw bytes.
pub fn hash_bytes_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_spv_rejects_invalid_wgsl() {
        let bad = "fn main() { this is not valid wgsl }";
        let result = compile_spv(bad);
        assert!(result.is_err(), "malformed WGSL must fail");
    }

    #[test]
    fn hash_bytes_sha256_is_deterministic() {
        let data = b"hello world";
        let h1 = hash_bytes_sha256(data);
        let h2 = hash_bytes_sha256(data);
        assert_eq!(h1, h2, "same input must produce same hash");
    }

    #[test]
    fn hash_bytes_sha256_differs_on_different_input() {
        let h1 = hash_bytes_sha256(b"hello");
        let h2 = hash_bytes_sha256(b"world");
        assert_ne!(h1, h2, "different input must produce different hash");
    }
}
