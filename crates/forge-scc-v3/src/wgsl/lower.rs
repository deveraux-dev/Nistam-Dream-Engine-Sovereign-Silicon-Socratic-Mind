//! Lowers parsed shader-subset AST to the typed [`super::ir`] IR.

use super::{ast::*, diagnostics::CompileError, ir::*};

/// Lower a parsed module to the typed shader IR: find every stage-attributed
/// function and lower its signature. v0 does not lower function bodies or
/// resource bindings (both empty in the result) — the emitter only needs
/// entry-point signatures today.
pub fn lower_to_ir(module: &ParsedModule) -> Result<ShaderModule, CompileError> {
    let mut entry_points = Vec::new();
    for item in &module.items {
        if let Item::Function(f) = item {
            if let Some(stage) = shader_stage(f) {
                entry_points.push(EntryPoint {
                    name: f.name.clone(),
                    stage,
                    workgroup_size: workgroup_size(f),
                    params: f
                        .params
                        .iter()
                        .map(lower_param)
                        .collect::<Result<Vec<_>, _>>()?,
                    return_ty: f.return_ty.as_ref().map(lower_type).transpose()?,
                });
            }
        }
    }
    Ok(ShaderModule {
        name: "module".into(),
        entry_points,
        functions: vec![],
        resources: vec![],
    })
}

fn shader_stage(f: &Function) -> Option<ShaderStage> {
    f.attributes.iter().find_map(|a| match a.name.as_str() {
        "vertex" => Some(ShaderStage::Vertex),
        "fragment" => Some(ShaderStage::Fragment),
        "compute" => Some(ShaderStage::Compute),
        _ => None,
    })
}

fn workgroup_size(f: &Function) -> Option<[u32; 3]> {
    f.attributes
        .iter()
        .find(|a| a.name == "compute")
        .and_then(|a| {
            if a.args.len() == 3 {
                Some([
                    a.args[0].parse().ok()?,
                    a.args[1].parse().ok()?,
                    a.args[2].parse().ok()?,
                ])
            } else {
                None
            }
        })
}

fn lower_param(p: &Param) -> Result<IrParam, CompileError> {
    Ok(IrParam {
        name: p.name.clone(),
        ty: lower_type(&p.ty)?,
        builtin: p
            .attributes
            .iter()
            .find(|a| a.name == "builtin")
            .and_then(|a| a.args.first().cloned()),
        location: p
            .attributes
            .iter()
            .find(|a| a.name == "location")
            .and_then(|a| a.args.first()?.parse().ok()),
    })
}

fn lower_type(t: &TypeRef) -> Result<IrType, CompileError> {
    Ok(match t {
        TypeRef::Bool => IrType::Bool,
        TypeRef::I32 => IrType::I32,
        TypeRef::U32 => IrType::U32,
        TypeRef::F32 => IrType::F32,
        TypeRef::Vec2F => IrType::Vec2F,
        TypeRef::Vec3U => IrType::Vec3U,
        TypeRef::Vec4F => IrType::Vec4F,
        TypeRef::Custom(name) => IrType::Struct(name.clone()),
    })
}
