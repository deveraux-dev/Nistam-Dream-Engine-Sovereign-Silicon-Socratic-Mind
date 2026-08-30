//! Emits WGSL source text from the typed [`super::ir`].

use super::{diagnostics::CompileError, ir::*};

/// Emit a full WGSL module string: resources first, then entry points.
pub fn emit_module(module: &ShaderModule) -> Result<String, CompileError> {
    let mut out = String::new();
    if !module.resources.is_empty() {
        for res in &module.resources {
            out.push_str(&emit_resource(res)?);
            out.push('\n');
        }
        out.push('\n');
    }
    for entry in &module.entry_points {
        out.push_str(&emit_entry(entry)?);
        out.push('\n');
    }
    Ok(out)
}

fn emit_resource(res: &ResourceBinding) -> Result<String, CompileError> {
    let space = match res.address_space {
        AddressSpace::Uniform => "uniform",
        AddressSpace::Storage => "storage",
        AddressSpace::Workgroup => "workgroup",
        AddressSpace::Private => "private",
        AddressSpace::Function => {
            return Err(CompileError::new(
                "E_RESOURCE_FUNCTION_SPACE",
                "resource cannot be in Function address space; use Uniform, Storage, Workgroup, or Private",
            ))
        }
    };
    let ty = emit_type(&res.ty);
    Ok(format!("@group({}) @binding({}) var<{}> {}: {};", res.group, res.binding, space, res.name, ty))
}

fn emit_entry(entry: &EntryPoint) -> Result<String, CompileError> {
    let stage = match entry.stage {
        ShaderStage::Vertex => "@vertex".to_string(),
        ShaderStage::Fragment => "@fragment".to_string(),
        ShaderStage::Compute => {
            let size = entry.workgroup_size.ok_or_else(|| {
                CompileError::new(
                    "E_MISSING_WORKGROUP",
                    "compute entry point requires workgroup_size",
                )
            })?;
            format!("@compute @workgroup_size({}, {}, {})", size[0], size[1], size[2])
        }
    };
    let params = entry
        .params
        .iter()
        .map(emit_param)
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let ret = entry
        .return_ty
        .as_ref()
        .map(|t| format!(" -> {}", emit_type(t)))
        .unwrap_or_default();
    Ok(format!("{stage}\nfn {}({params}){ret} {{\n}}", entry.name))
}

fn emit_param(p: &IrParam) -> Result<String, CompileError> {
    let attr = match (&p.builtin, p.location) {
        (Some(b), None) => format!("@builtin({b}) "),
        (None, Some(l)) => format!("@location({l}) "),
        (None, None) => String::new(),
        (Some(_), Some(_)) => {
            return Err(CompileError::new(
                "E_PARAM_ATTR_CONFLICT",
                "param cannot have both builtin and location",
            ))
        }
    };
    Ok(format!("{attr}{}: {}", p.name, emit_type(&p.ty)))
}

fn emit_type(t: &IrType) -> String {
    match t {
        IrType::Bool => "bool".into(),
        IrType::I32 => "i32".into(),
        IrType::U32 => "u32".into(),
        IrType::F32 => "f32".into(),
        IrType::Vec2F => "vec2<f32>".into(),
        IrType::Vec3U => "vec3<u32>".into(),
        IrType::Vec4F => "vec4<f32>".into(),
        IrType::Struct(name) => name.clone(),
    }
}

/// Emit a WGSL struct declaration from a name and ordered field list.
/// Fields are emitted as `field_name: field_type;`, one per line.
/// Empty field list is rejected.
pub fn emit_uniform_struct(name: &str, fields: &[(&str, IrType)]) -> Result<String, CompileError> {
    if fields.is_empty() {
        return Err(CompileError::new(
            "E_EMPTY_STRUCT",
            "struct must have at least one field",
        ));
    }
    let mut out = format!("struct {} {{\n", name);
    for (field_name, field_ty) in fields {
        out.push_str(&format!("    {}: {},\n", field_name, emit_type(field_ty)));
    }
    out.push('}');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_single_uniform_binding() {
        let module = ShaderModule {
            name: "test".into(),
            entry_points: vec![EntryPoint {
                name: "main".into(),
                stage: ShaderStage::Compute,
                workgroup_size: Some([1, 1, 1]),
                params: vec![],
                return_ty: None,
            }],
            functions: vec![],
            resources: vec![ResourceBinding {
                name: "params".into(),
                group: 0,
                binding: 0,
                address_space: AddressSpace::Uniform,
                ty: IrType::Vec4F,
            }],
        };
        let result = emit_module(&module).expect("emission should succeed");
        assert!(result.contains("@group(0) @binding(0) var<uniform> params: vec4<f32>;"), "resource declaration missing");
        assert!(result.contains("@compute @workgroup_size(1, 1, 1)"), "entry point missing");
        let resource_line_idx = result.find("@group(0)").expect("resource should come first");
        let entry_idx = result.find("@compute").expect("entry point should come second");
        assert!(resource_line_idx < entry_idx, "resource must come before entry point");
    }

    #[test]
    fn test_function_address_space_rejected() {
        let module = ShaderModule {
            name: "test".into(),
            entry_points: vec![EntryPoint {
                name: "main".into(),
                stage: ShaderStage::Vertex,
                workgroup_size: None,
                params: vec![],
                return_ty: None,
            }],
            functions: vec![],
            resources: vec![ResourceBinding {
                name: "bad".into(),
                group: 0,
                binding: 0,
                address_space: AddressSpace::Function,
                ty: IrType::F32,
            }],
        };
        let err = emit_module(&module).expect_err("should reject Function address space");
        assert_eq!(err.code, "E_RESOURCE_FUNCTION_SPACE");
    }

    #[test]
    fn test_zero_resources_regression_guard() {
        let module = ShaderModule {
            name: "test".into(),
            entry_points: vec![EntryPoint {
                name: "main".into(),
                stage: ShaderStage::Fragment,
                workgroup_size: None,
                params: vec![],
                return_ty: Some(IrType::Vec4F),
            }],
            functions: vec![],
            resources: vec![],
        };
        let result = emit_module(&module).expect("emission should succeed");
        assert!(!result.starts_with("@group"), "no resources should not emit @group");
        assert!(result.contains("@fragment"), "entry point should be present");
        assert!(result.contains("fn main() -> vec4<f32>"), "signature should be present");
    }

    #[test]
    fn test_emit_uniform_struct_4field() {
        let fields = vec![
            ("vibe_glow", IrType::Vec4F),
            ("vibe_pulse", IrType::Vec4F),
            ("vibe_chromatic", IrType::Vec4F),
            ("vibe_shake", IrType::Vec4F),
        ];
        let result = emit_uniform_struct("VibeBus", &fields).expect("struct emission should succeed");
        assert!(result.contains("struct VibeBus {"), "struct header missing");
        assert!(result.contains("vibe_glow: vec4<f32>,"), "field vibe_glow missing");
        assert!(result.contains("vibe_pulse: vec4<f32>,"), "field vibe_pulse missing");
        assert!(result.contains("vibe_chromatic: vec4<f32>,"), "field vibe_chromatic missing");
        assert!(result.contains("vibe_shake: vec4<f32>,"), "field vibe_shake missing");
        assert!(result.contains("}"), "closing brace missing");
    }

    #[test]
    fn test_emit_uniform_struct_empty_rejected() {
        let result = emit_uniform_struct("Empty", &[]);
        let err = result.expect_err("empty struct should be rejected");
        assert_eq!(err.code, "E_EMPTY_STRUCT");
    }

    #[test]
    fn test_duplicate_binding_rejected_by_validation() {
        let module = ShaderModule {
            name: "test".into(),
            entry_points: vec![EntryPoint {
                name: "main".into(),
                stage: ShaderStage::Vertex,
                workgroup_size: None,
                params: vec![],
                return_ty: None,
            }],
            functions: vec![],
            resources: vec![
                ResourceBinding {
                    name: "res1".into(),
                    group: 0,
                    binding: 0,
                    address_space: AddressSpace::Uniform,
                    ty: IrType::F32,
                },
                ResourceBinding {
                    name: "res2".into(),
                    group: 0,
                    binding: 0,
                    address_space: AddressSpace::Uniform,
                    ty: IrType::F32,
                },
            ],
        };
        use super::super::validate::validate_ir;
        let err = validate_ir(&module).expect_err("should reject duplicate binding");
        assert_eq!(err.code, "E_BINDING_DUPLICATE");
    }

    #[test]
    fn test_duplicate_resource_name_rejected() {
        let module = ShaderModule {
            name: "test".into(),
            entry_points: vec![EntryPoint {
                name: "main".into(),
                stage: ShaderStage::Vertex,
                workgroup_size: None,
                params: vec![],
                return_ty: None,
            }],
            functions: vec![],
            resources: vec![
                ResourceBinding {
                    name: "same_name".into(),
                    group: 0,
                    binding: 0,
                    address_space: AddressSpace::Uniform,
                    ty: IrType::F32,
                },
                ResourceBinding {
                    name: "same_name".into(),
                    group: 0,
                    binding: 1,
                    address_space: AddressSpace::Uniform,
                    ty: IrType::F32,
                },
            ],
        };
        use super::super::validate::validate_ir;
        let err = validate_ir(&module).expect_err("should reject duplicate name");
        assert_eq!(err.code, "E_RESOURCE_NAME_DUPLICATE");
    }

    #[test]
    fn test_all_address_spaces_emit() {
        let module = ShaderModule {
            name: "test".into(),
            entry_points: vec![EntryPoint {
                name: "main".into(),
                stage: ShaderStage::Compute,
                workgroup_size: Some([8, 8, 1]),
                params: vec![],
                return_ty: None,
            }],
            functions: vec![],
            resources: vec![
                ResourceBinding {
                    name: "uniform_buf".into(),
                    group: 0,
                    binding: 0,
                    address_space: AddressSpace::Uniform,
                    ty: IrType::Vec4F,
                },
                ResourceBinding {
                    name: "storage_buf".into(),
                    group: 0,
                    binding: 1,
                    address_space: AddressSpace::Storage,
                    ty: IrType::Vec3U,
                },
                ResourceBinding {
                    name: "workgroup_var".into(),
                    group: 0,
                    binding: 2,
                    address_space: AddressSpace::Workgroup,
                    ty: IrType::U32,
                },
                ResourceBinding {
                    name: "private_var".into(),
                    group: 0,
                    binding: 3,
                    address_space: AddressSpace::Private,
                    ty: IrType::I32,
                },
            ],
        };
        let result = emit_module(&module).expect("emission should succeed");
        assert!(result.contains("var<uniform> uniform_buf"), "Uniform space");
        assert!(result.contains("var<storage> storage_buf"), "Storage space");
        assert!(result.contains("var<workgroup> workgroup_var"), "Workgroup space");
        assert!(result.contains("var<private> private_var"), "Private space");
    }
}
