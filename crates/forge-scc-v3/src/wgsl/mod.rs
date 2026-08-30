//! Domain 1: a Rust-subset / `.vixi` shader source -> WGSL string.
//!
//! This is the no-bloat web-render bridge: WebGPU consumes WGSL strings raw
//! (`device.createShaderModule({code})`), so a `.vixi`'s visual half lowers to a
//! WGSL string with no wasm, no bindgen, no npm.
//!
//! Pipeline:
//! ```text
//! Rust-like shader subset -> typed ShaderCoreIR -> WGSL string -> validate
//! ```
//! v0 is a deliberately small subset compiler, NOT a general Rust -> WGSL
//! transpiler: full Rust (ownership, traits, generics, macros, heap) does not map
//! to WGSL. The subset gate ([`validate::validate_source_subset`]) fails fast on
//! forbidden constructs so the emitter never produces plausible-but-wrong WGSL.
//!
//! Ported from `F:\NewRepo\crates\scc\src\wgsl\` (2026-08-15). This is the one
//! module `source-compiler`'s own "Owners" table actually names
//! (`wgsl=SCC crates/scc[LIVE]`).

pub mod ast;
pub mod diagnostics;
pub mod emit_wgsl;
pub mod ir;
pub mod lower;
pub mod parse;
pub mod validate;

pub use diagnostics::{CompileError, SourceSpan};

/// Compile a Rust-shader-subset source string into a WGSL module string.
///
/// Stages: parse -> subset gate -> lower to `ShaderCoreIR` -> IR validation ->
/// WGSL emission. Any stage may return a [`CompileError`] carrying a stable code.
pub fn compile_rust_subset_to_wgsl(source: &str) -> Result<String, diagnostics::CompileError> {
    let parsed = parse::parse_module(source)?;
    validate::validate_source_subset(&parsed)?;
    let ir = lower::lower_to_ir(&parsed)?;
    validate::validate_ir(&ir)?;
    emit_wgsl::emit_module(&ir)
}
