//! # forge-vix-syntax — the VixiScript syntax leaf (zero dependencies)
//!
//! Extracted from forge-vix 2026-08-05 so `forge-vix/build.rs` can consume the
//! REAL parser via `[build-dependencies]` — a build script cannot import the
//! crate it builds, and that constraint forced an inlined hand-copy parser that
//! drifted twice (hex_grid 07-12, quoted-text 07-29). One lexer, one surface
//! parser, one table per vocabulary; forge-vix re-exports everything so no
//! caller path changes.

pub mod cst; // T1-LADDER #12 — hand-rolled LOSSLESS CST (byte-exact round-trip; moved from forge-vix)
pub mod emit; // .kit.vixi surface tree → `WidgetSpec { .. }` literal Rust source (AOT codegen arm)
pub mod error; // spanned diagnostics for table lookups
pub mod gate; // security gate for a parsed surface tree, pre-emit (forge-daemon-door::write_vixi)
pub mod sheet; // .sheet.vixi parser (hex #RRGGBBAA + mu(N) + ms(N) + permyriad(N) value semantics)
pub mod surface; // .kit.vixi slot-line surface parser (the ONE line lexer; base tree, AOT semantics)
pub mod tables; // string↔variant SoT: SlotKind / LayoutPolicy (+aliases) / Justify / Align

pub use error::SpannedError;
pub use gate::GateDecision;
pub use tables::{Align, Justify, LayoutPolicy, SlotKind};
