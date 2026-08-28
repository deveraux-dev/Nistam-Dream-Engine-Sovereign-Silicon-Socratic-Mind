#![allow(missing_docs)]
//! Ported verbatim from F:\NewRepo\crates\forge-vix-lsp\ (2026-08-17 truth-hunt lineage port).
//! `forge-vix-lsp` — sovereign, dependency-free VixiScript language server.
//!
//! Hand-rolled JSON-RPC 2.0 over stdio (the bin, `main.rs`) wrapping the existing
//! `forge-vix` analysis surface. NO external LSP framework — `tower-lsp` and
//! `vscode-languageserver` were hard-rejected (Sean, 2026-06-03). `serde_json` is
//! used for JSON only (already a workspace dep); the LSP protocol itself is wired
//! by hand.
//!
//! Doctrine: **Wiring, Not Invention.** The only genuinely new code is the
//! position→word resolver (`position`, the §1c "small tree walk"). Every op reads
//! existing `forge-vix` substrate:
//!   - `publishDiagnostics` ← `forge_vix::diagnostics::check`
//!   - `hover`              ← `forge_vix::grammar::{kind_doc, layout_doc}` at a Position
//!   - `completion`         ← the `forge_vix::grammar` const closed-set tables
//!                            (the same set `forge_ml::gbnf_sampler` masks to)
//!   - `documentSymbol` / `definition` ← RED-first, wired in the next increment.
//!
//! READ-ONLY boundary: `forge-ast/src/vixel/grammar_bridge.rs` (the sacred sim SoT)
//! is never moved or refactored by this crate.

pub mod handlers;
pub mod position;
pub mod ray_complete;
pub mod telemetry;
pub mod stdio_server;
pub mod cognitive;
pub mod grammar;
pub mod diagnostics;

pub use stdio_server::run_stdio;
