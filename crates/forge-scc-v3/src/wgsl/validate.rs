//! The subset gate and post-lowering IR validation.

use super::{ast::*, diagnostics::CompileError, ir::*};
use std::collections::HashSet;

/// The subset gate: reject anything the parser flagged as unsupported, and
/// reject generic functions (not supported in shader subset v0). Runs before
/// lowering, so lowering never has to handle a construct it can't express.
pub fn validate_source_subset(module: &ParsedModule) -> Result<(), CompileError> {
    for item in &module.items {
        match item {
            Item::Unsupported { kind } => {
                return Err(CompileError::new(
                    "E_SUBSET_UNSUPPORTED",
                    format!("unsupported Rust construct: {kind}"),
                ))
            }
            Item::Function(f) if f.has_generics => {
                return Err(CompileError::new(
                    "E_SUBSET_GENERIC_FN",
                    "generic functions are not supported in shader subset v0",
                ))
            }
            _ => {}
        }
    }
    Ok(())
}

/// Post-lowering IR validation: every `@group`/`@binding` pair must be unique,
/// resource names must be unique, resources cannot use Function address space,
/// and the module must declare at least one entry point.
pub fn validate_ir(module: &ShaderModule) -> Result<(), CompileError> {
    let mut seen_bindings = HashSet::new();
    let mut seen_names = HashSet::new();
    for res in &module.resources {
        if !seen_bindings.insert((res.group, res.binding)) {
            return Err(CompileError::new(
                "E_BINDING_DUPLICATE",
                format!(
                    "duplicate binding @group({}) @binding({})",
                    res.group, res.binding
                ),
            ));
        }
        if !seen_names.insert(res.name.clone()) {
            return Err(CompileError::new(
                "E_RESOURCE_NAME_DUPLICATE",
                format!("duplicate resource name '{}'", res.name),
            ));
        }
        if res.address_space == AddressSpace::Function {
            return Err(CompileError::new(
                "E_RESOURCE_FUNCTION_SPACE",
                format!(
                    "resource '{}' cannot be in Function address space; use Uniform, Storage, Workgroup, or Private",
                    res.name
                ),
            ));
        }
    }
    if module.entry_points.is_empty() {
        return Err(CompileError::new(
            "E_NO_ENTRY_POINT",
            "module has no shader entry point",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_duplicate_binding() {
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
        let err = validate_ir(&module).expect_err("should reject duplicate binding");
        assert_eq!(err.code, "E_BINDING_DUPLICATE");
    }

    #[test]
    fn test_validate_duplicate_name() {
        let module = ShaderModule {
            name: "test".into(),
            entry_points: vec![EntryPoint {
                name: "main".into(),
                stage: ShaderStage::Fragment,
                workgroup_size: None,
                params: vec![],
                return_ty: None,
            }],
            functions: vec![],
            resources: vec![
                ResourceBinding {
                    name: "same".into(),
                    group: 0,
                    binding: 0,
                    address_space: AddressSpace::Uniform,
                    ty: IrType::F32,
                },
                ResourceBinding {
                    name: "same".into(),
                    group: 0,
                    binding: 1,
                    address_space: AddressSpace::Storage,
                    ty: IrType::F32,
                },
            ],
        };
        let err = validate_ir(&module).expect_err("should reject duplicate name");
        assert_eq!(err.code, "E_RESOURCE_NAME_DUPLICATE");
    }

    #[test]
    fn test_validate_function_space_rejected() {
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
                name: "illegal".into(),
                group: 0,
                binding: 0,
                address_space: AddressSpace::Function,
                ty: IrType::F32,
            }],
        };
        let err = validate_ir(&module).expect_err("should reject Function address space");
        assert_eq!(err.code, "E_RESOURCE_FUNCTION_SPACE");
    }

    #[test]
    fn test_validate_no_entry_point() {
        let module = ShaderModule {
            name: "test".into(),
            entry_points: vec![],
            functions: vec![],
            resources: vec![],
        };
        let err = validate_ir(&module).expect_err("should reject no entry points");
        assert_eq!(err.code, "E_NO_ENTRY_POINT");
    }

    #[test]
    fn test_validate_valid_module() {
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
            resources: vec![ResourceBinding {
                name: "params".into(),
                group: 0,
                binding: 0,
                address_space: AddressSpace::Uniform,
                ty: IrType::Vec4F,
            }],
        };
        validate_ir(&module).expect("valid module should pass");
    }
}
