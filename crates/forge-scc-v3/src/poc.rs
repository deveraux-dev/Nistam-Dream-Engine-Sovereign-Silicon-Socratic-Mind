//! poc.vixi-artifact — the cold face naming the V2 gate proof.
//!
//! **v2-vs-v3 receipt (T1), read before trusting any path below:** this module's
//! `GapReport` classifies `F:\NewRepo` paths (`scc/golden/vixi/...`,
//! `forge-render/shaders/...`, `forge-evidence/src/provenance.rs`) as they stood
//! when this file was written. None of those paths exist in `F:\v3`. Ported
//! verbatim as historical record, not as a claim about this workspace.
//!
//! Classifies the `poc.vixi-artifact` pipeline against the Sovereign Knowledge
//! Compiler pattern: `.vixi` source + carried `.wgsl` payload → signed user-owned
//! artifact → rendered in a browser tab via raw WebGPU. No forge-evidence cargo
//! edge — the signing lives in `forge-evidence`; this is the doctrine-projection
//! cold face (same altitude as [`crate::evidence`]).

use crate::contract::{Contract, GapReport, Verdict};

/// The declared contract for the poc.vixi-artifact V2 gate.
pub fn poc_vixi_artifact_contract() -> Contract {
    Contract {
        compiler: "poc-vixi-artifact".into(),
        source_language: ".vixi (VixiScript shaderbind) + .wgsl (WGSL shader payload)".into(),
        target_language: "user-owned .bin artifact + Ed25519 ProvenanceReceipt + WebGPU browser render".into(),
        quality_gates: vec![
            "vixi-first: the .vixi is the SoT; the .wgsl is its carried render payload (Route A)".into(),
            "signed: ProvenanceCompiler::compile_bytes seals the bundle (forge-evidence, AssetType::Vixi)".into(),
            "readback: round-trip off disk byte-identical, ADR-0008 discriminator RED on tamper".into(),
            "no-bloat: browser render = WGSL string + raw WebGPU JS, zero wasm/bindgen/npm".into(),
        ],
    }
}

/// Gap report classifying every stage of the poc.vixi-artifact pipeline.
pub fn poc_vixi_artifact_gap_report() -> GapReport {
    let mut r = GapReport::new("poc-vixi-artifact");
    r.classify(
        "audio_vis.shaderbind.vixi (scc/golden/vixi/shaderbinds/)",
        Verdict::Native,
        "Live in the golden corpus. Maps audio signals → vibematrix_channels[0..4] (rms/beat/centroid/crossfader/pressure). The .vixi half of the artifact bundle.",
        "crates/scc/golden/vixi/shaderbinds/audio_vis.shaderbind.vixi",
    )
    .classify(
        "splat_composite.wgsl (forge-render/shaders/post/)",
        Verdict::Native,
        "Full-screen triangle @vertex+@fragment, textureLoad from splat_color+depth, ~42 lines. The .wgsl carried payload for Route A. Renders the Gaussian splat backdrop.",
        "crates/forge-render/shaders/post/splat_composite.wgsl",
    )
    .classify(
        "material_shader_profile.wgsl (forge-render/shaders/)",
        Verdict::Native,
        "MaterialShaderUniform + vibematrix_channels[8] + Permyriad q_to_f32 helpers. The vibematrix bridge: HarmonicBody.resonance_hz → vibematrix → WGSL.",
        "crates/forge-render/shaders/material_shader_profile.wgsl",
    )
    .classify(
        "ProvenanceCompiler::compile_bytes (forge-evidence)",
        Verdict::Reserve,
        "Lives in forge-evidence; stays there. AssetType::Vixi variant exists. Used in the discriminator test as a dev-dep.",
        "crates/forge-evidence/src/provenance.rs",
    )
    .classify(
        "poc_render.html (scc/poc/)",
        Verdict::Native,
        "Raw WebGPU JS harness (~35 lines): adapter/device, stub rgba8unorm color+depth textures, bind group, render pipeline, draw(3). Zero wasm/bindgen/npm. Proves create->own->run-in-browser.",
        "crates/scc/poc/poc_render.html",
    )
    .classify(
        "Route B: .vixel → SPIR-V → naga → WGSL",
        Verdict::Reserve,
        "De-risked: rspirv (in cargo registry) + naga (in wgpu). Not needed for the minimum poc; reserved for scc Domain 2.",
        "rspirv registry crate",
    )
    .classify(
        "WebGL2 fallback",
        Verdict::Spike,
        "WebGPU coverage: Chrome/Edge/Safari17+/FF-2025 yes. A WebGL2 fallback path is not needed for the poc launch floor; revisit before public launch.",
        "(sub-question)",
    )
    .classify(
        "Web Audio API (.vixi audio half)",
        Verdict::Missing,
        "The poc proves only the visual/cymatics half. A .vixi's audio currency is separate (Web Audio API). WGSL makes no sound.",
        "(sub-question)",
    );
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poc_contract_declares_four_gates() {
        let c = poc_vixi_artifact_contract();
        assert_eq!(c.compiler, "poc-vixi-artifact");
        assert_eq!(c.quality_gates.len(), 4);
        assert!(c.quality_gates.iter().any(|g| g.contains("vixi-first")));
        assert!(c.quality_gates.iter().any(|g| g.contains("signed")));
        assert!(c.quality_gates.iter().any(|g| g.contains("readback")));
        assert!(c.quality_gates.iter().any(|g| g.contains("no-bloat")));
    }

    #[test]
    fn poc_gap_report_has_three_native_one_missing() {
        let r = poc_vixi_artifact_gap_report();
        assert!(r.count(Verdict::Native) >= 3, "vixi + splat_composite + material_shader_profile are Native");
        assert_eq!(r.count(Verdict::Missing), 1, "web audio is the one genuine gap");
        assert!(!r.is_clean(), "web audio gap keeps the report not-clean");
    }
}
