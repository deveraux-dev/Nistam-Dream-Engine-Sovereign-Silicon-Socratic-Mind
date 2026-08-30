//! Seal the pentaract_march_5d compute kernel: compile WGSL -> SPIR-V,
//! freeze SHA256 hashes, and emit a tamper-evident 96-byte header.
//!
//! Run: cargo run -p forge-ocular-v3 --example seal_pentaract
//!
//! Generates: crates/forge-ocular-v3/proof/sealed/pentaract_march_5d.spv.sealed

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const MAGIC: [u8; 4] = *b"FOCS";
const VERSION: u32 = 1;
const HEADER_LEN: usize = 96;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wgsl_path = "crates/forge-ocular-v3/shaders/pentaract_march_5d.wgsl";
    let wgsl_source = fs::read_to_string(wgsl_path)?;

    eprintln!("Compiling {} ({} bytes)...", wgsl_path, wgsl_source.len());

    let module = naga::front::wgsl::parse_str(&wgsl_source)?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    let module_info = validator.validate(&module)?;

    eprintln!("Validation OK. Translating to SPIR-V...");

    let spirv_words = naga::back::spv::write_vec(
        &module,
        &module_info,
        &Default::default(),
        None,
    )?;

    let spirv_bin: Vec<u8> = spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    eprintln!("SPIR-V: {} bytes (was {} words)", spirv_bin.len(), spirv_words.len());

    let source_sha = Sha256::digest(wgsl_source.as_bytes());
    let spirv_sha = Sha256::digest(&spirv_bin);

    eprintln!("WGSL SHA256: {}", hex::encode(&source_sha));
    eprintln!("SPIR-V SHA256: {}", hex::encode(&spirv_sha));

    let mut sealed = Vec::with_capacity(HEADER_LEN + spirv_bin.len());

    sealed.extend_from_slice(&MAGIC);
    sealed.extend_from_slice(&VERSION.to_le_bytes());
    sealed.extend_from_slice(&(wgsl_source.len() as u32).to_le_bytes());
    sealed.extend_from_slice(&(spirv_bin.len() as u32).to_le_bytes());
    sealed.extend_from_slice(&source_sha[..]);
    sealed.extend_from_slice(&spirv_sha[..]);
    sealed.extend_from_slice(&[0u8; 16]);

    sealed.extend_from_slice(&spirv_bin);

    let out_path = Path::new("crates/forge-ocular-v3/proof/sealed/pentaract_march_5d.spv.sealed");
    fs::write(out_path, &sealed)?;

    eprintln!(
        "Sealed blob: {} bytes -> {}",
        sealed.len(),
        out_path.display()
    );
    Ok(())
}
