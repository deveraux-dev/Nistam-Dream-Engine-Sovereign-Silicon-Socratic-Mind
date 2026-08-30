// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Second oracle: every authored shaderbind's emitted WGSL must survive naga.
//! Our own emitter tests only prove we emit what we intended to emit.

use forge_harmonics::scc_bridge::emit_shaderbind_wgsl;
use std::path::PathBuf;

fn corpus() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scc")
        .join("golden")
        .join("vixi")
        .join("shaderbinds");
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("corpus dir unreadable at {}: {e}", dir.display()));
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.to_string_lossy().ends_with(".shaderbind.vixi"))
        .collect();
    out.sort();
    out
}

#[test]
fn every_emitted_shaderbind_compiles_to_spirv() {
    let mut failures: Vec<String> = Vec::new();
    let mut compiled = 0usize;

    for path in corpus() {
        let src = std::fs::read_to_string(&path).expect("corpus file readable");
        let bind = match forge_shaderbind::parse_shaderbind(&src) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("{}: parse: {e:?}", path.display()));
                continue;
            }
        };
        let wgsl = match emit_shaderbind_wgsl(&bind) {
            Ok(w) => w,
            Err(e) => {
                failures.push(format!("{}: emit: {e}", path.display()));
                continue;
            }
        };
        match forge_shader_build_v3::compile_spv(&wgsl) {
            Ok(spv) => {
                assert!(!spv.is_empty(), "{}: empty SPIR-V", path.display());
                compiled += 1;
            }
            Err(e) => failures.push(format!("{}: naga rejected:\n{e}\n--- wgsl ---\n{wgsl}", path.display())),
        }
    }

    assert!(failures.is_empty(), "{} of {} rejected:\n{}", failures.len(), compiled + failures.len(), failures.join("\n\n"));
    assert!(compiled >= 16, "expected the full corpus, compiled {compiled}");
}
