//! Parses shader-subset source text into [`super::ast`].

use super::{ast::*, diagnostics::CompileError};

/// Parse a shader-subset source string. Currently a deliberately minimal
/// scaffold parser (recognizes a small fixed set of example shapes) — replace
/// with syn/tree-sitter/a custom grammar as the subset grows; the rest of the
/// pipeline (subset gate, lowering, IR validation, emission) is already
/// testable against it.
pub fn parse_module(source: &str) -> Result<ParsedModule, CompileError> {
    // Scaffold parser: replace with syn/tree-sitter/custom grammar.
    // This deliberately recognizes only tiny examples so the rest of the pipeline is testable.
    if source.contains("unsafe") {
        return Ok(ParsedModule {
            items: vec![Item::Unsupported {
                kind: "unsafe".into(),
            }],
        });
    }
    if source.contains("fn bad<T>") {
        return Ok(ParsedModule {
            items: vec![Item::Function(Function {
                name: "bad".into(),
                attributes: vec![],
                params: vec![],
                return_ty: None,
                body: vec![],
                has_generics: true,
            })],
        });
    }
    if source.contains("#[compute") {
        return Ok(ParsedModule {
            items: vec![Item::Function(Function {
                name: "main".into(),
                attributes: vec![Attribute {
                    name: "compute".into(),
                    args: vec!["8".into(), "8".into(), "1".into()],
                }],
                params: vec![Param {
                    name: "id".into(),
                    ty: TypeRef::Vec3U,
                    attributes: vec![Attribute {
                        name: "builtin".into(),
                        args: vec!["global_invocation_id".into()],
                    }],
                }],
                return_ty: None,
                body: vec![],
                has_generics: false,
            })],
        });
    }
    Err(CompileError::new(
        "E_PARSE_SCAFFOLD",
        "parser scaffold only recognizes minimal examples",
    ))
}
