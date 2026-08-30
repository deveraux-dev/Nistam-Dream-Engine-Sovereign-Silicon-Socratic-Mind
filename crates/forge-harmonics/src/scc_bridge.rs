// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Shaderbind to shader IR lowering: `signal -> channel[N]` to WGSL.
//! Converts authored `.shaderbind.vixi` declarations into compiled shader IR,
//! emitting a uniform buffer and a named fragment entry point.

use forge_scc_v3::wgsl::emit_wgsl::{emit_module, emit_uniform_struct};
use forge_scc_v3::wgsl::ir::{
    AddressSpace, EntryPoint, IrType, ResourceBinding, ShaderModule, ShaderStage,
};
use forge_shaderbind::ShaderBind;
use std::fmt;

/// Shaderbind lowering error: gate refusal or IR construction failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreBindError {
    /// Error message.
    pub message: String,
}

impl fmt::Display for ScoreBindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ScoreBindError {}

impl ScoreBindError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// Lower a shaderbind into shader IR: routed channels as a uniform buffer +
/// a named fragment entry point. Calls `verify_gates()` first; a gate refusal
/// becomes an error. The uniform buffer struct name is `surface_channels`
/// (e.g., `audio_vis_channels`). Group/binding are 0/0 (single uniform resource).
pub fn lower_shaderbind(bind: &ShaderBind) -> Result<ShaderModule, ScoreBindError> {
    bind.verify_gates().map_err(|e| ScoreBindError::new(format!("gate refusal: {}", e.message)))?;

    let struct_name = format!("{}_channels", bind.surface);
    let buffer_name = format!("{}_buffer", bind.surface);

    let resource = ResourceBinding {
        name: buffer_name,
        group: 0,
        binding: 0,
        address_space: AddressSpace::Uniform,
        ty: IrType::Struct(struct_name),
    };

    let entry = EntryPoint {
        name: bind.surface.clone(),
        stage: ShaderStage::Fragment,
        workgroup_size: None,
        params: vec![],
        return_ty: None,
    };

    Ok(ShaderModule {
        name: format!("shaderbind_{}", bind.surface),
        entry_points: vec![entry],
        functions: vec![],
        resources: vec![resource],
    })
}

/// Emit WGSL text for a shaderbind: the uniform struct declaration followed by
/// the lowered module's own emission. Channels occupy `vec4<f32>` slots, four
/// per slot; the shader divides each Permyriad lane by 10000 to normalize.
pub fn emit_shaderbind_wgsl(bind: &ShaderBind) -> Result<String, ScoreBindError> {
    let module = lower_shaderbind(bind)?;
    let slot_count = bind.channel_span().div_ceil(4);
    let slot_names: Vec<String> = (0..slot_count).map(|i| format!("slot{i}")).collect();
    let fields: Vec<(&str, IrType)> =
        slot_names.iter().map(|n| (n.as_str(), IrType::Vec4F)).collect();

    let decl = emit_uniform_struct(&format!("{}_channels", bind.surface), &fields)
        .map_err(|e| ScoreBindError::new(format!("struct emission: {e:?}")))?;
    let body = emit_module(&module)
        .map_err(|e| ScoreBindError::new(format!("module emission: {e:?}")))?;

    Ok(format!("{decl}\n\n{body}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn audio_vis_lowers() {
        let src = include_str!("../../scc/golden/vixi/shaderbinds/audio_vis.shaderbind.vixi");
        let bind = forge_shaderbind::parse_shaderbind(src)
            .expect("audio_vis.shaderbind.vixi must parse");
        let module = lower_shaderbind(&bind).expect("audio_vis must lower");
        assert_eq!(module.entry_points.len(), 1);
        assert_eq!(module.entry_points[0].name, "audio_vis");
        assert_eq!(module.entry_points[0].stage, ShaderStage::Fragment);
        assert_eq!(module.resources.len(), 1);
        let res = &module.resources[0];
        assert_eq!(res.address_space, AddressSpace::Uniform);
        assert_eq!(res.name, "audio_vis_buffer");
        if let IrType::Struct(ref s) = res.ty {
            assert_eq!(s, "audio_vis_channels");
        } else {
            panic!("resource type must be a struct");
        }
    }

    #[test]
    fn all_golden_corpus_files_lower() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let corpus_dir =
            Path::new(manifest_dir).join("..").join("scc").join("golden").join("vixi").join("shaderbinds");

        if !corpus_dir.exists() {
            panic!("corpus dir does not exist at {}", corpus_dir.display());
        }

        let entries = std::fs::read_dir(&corpus_dir)
            .unwrap_or_else(|e| panic!("corpus dir unreadable at {}: {e}", corpus_dir.display()));

        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().ends_with(".shaderbind.vixi"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();

        let mut failures = Vec::new();
        for path in files {
            let src = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    failures.push(format!("{}: read failed: {e}", path.display()));
                    continue;
                }
            };
            let bind = match forge_shaderbind::parse_shaderbind(&src) {
                Ok(b) => b,
                Err(e) => {
                    failures.push(format!("{}: parse failed: {e:?}", path.display()));
                    continue;
                }
            };
            if let Err(e) = lower_shaderbind(&bind) {
                failures.push(format!("{}: lower failed: {e}", path.display()));
            }
        }
        assert!(
            failures.is_empty(),
            "shaderbind lowering failures:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn gate_refusal_blocks_lowering() {
        let bad_src = r#"#vixi:shaderbind v1
surface: test_surface
profile: test_profile

signal test_sig source=vibematrix.hue range=0..10000

test_surface.channel[0] <- test_sig

gate visual_only_mutates_authority = forbidden
"#;
        let bind = forge_shaderbind::parse_shaderbind(bad_src)
            .expect("test shaderbind must parse");
        let result = lower_shaderbind(&bind);
        assert!(
            result.is_err(),
            "shaderbind with visual-only source but no authority under visual_only_mutates_authority=forbidden should fail to lower"
        );
    }

    #[test]
    fn emit_shaderbind_wgsl_produces_entry_point() {
        let src = include_str!("../../scc/golden/vixi/shaderbinds/audio_vis.shaderbind.vixi");
        let bind = forge_shaderbind::parse_shaderbind(src)
            .expect("audio_vis.shaderbind.vixi must parse");
        let wgsl = emit_shaderbind_wgsl(&bind).expect("audio_vis must emit WGSL");
        assert!(
            wgsl.contains("fn audio_vis()"),
            "WGSL must contain entry point 'fn audio_vis()'"
        );
        assert!(
            wgsl.contains("@fragment"),
            "WGSL must contain @fragment decorator"
        );

        let struct_at = wgsl.find("struct audio_vis_channels").expect("struct declaration");
        let binding_at = wgsl
            .find("@group(0) @binding(0) var<uniform> audio_vis_buffer: audio_vis_channels;")
            .expect("resource line must come from the scc emitter, not a second one here");
        assert!(struct_at < binding_at, "the struct must precede the var that names it");
        assert!(wgsl.contains("slot0: vec4<f32>"), "channels occupy vec4 slots");
        assert_eq!(wgsl.matches("audio_vis_buffer:").count(), 1, "one emitter, one binding");
    }
}
