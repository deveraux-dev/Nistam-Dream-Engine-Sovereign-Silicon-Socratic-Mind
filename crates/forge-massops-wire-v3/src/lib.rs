#![deny(missing_docs)]
//! forge-massops-wire-v3 — the downstream serde+ron+serde_json home for
//! `forge-core-v3::organs::{massread,massweld}`'s documented deserialization
//! gap (Crate Zero forbids serde; these two organ modules ported their
//! discipline/logic verbatim and stubbed only the wire-parsing functions,
//! naming this crate as the fill — see massweld.rs's and massread.rs's own
//! module docs). Ported 2026-08-17 (Broski board-clear follow-up).
//!
//! This crate does NOT reimplement any discipline logic — it only parses RON/
//! JSON text into the pre-parsed Rust structs `forge-core-v3`'s tested
//! functions (`apply_edit`, `self_edit`, `path_escapes`, `coverage_gaps`,
//! `unfence`, …) already operate on.

pub mod apply;
pub mod weld_wire;
pub mod read_wire;
