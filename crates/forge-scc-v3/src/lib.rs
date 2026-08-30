//! # scc -- Sovereign Creation Compiler
//!
//! The `.vixi` compiler-factory. A person creates -> gets a **user-owned artifact +
//! a gap report** -> renders it anywhere with a tiny no-bloat runtime -> we store
//! NOTHING of theirs. Not a host, not a CMS, not a UGC platform: a **compiler**.
//!
//! Ported in full from `F:\NewRepo\crates\scc` (2026-08-15). `source-compiler`'s
//! own "Owners" table names only [`wgsl`] (`wgsl=SCC crates/scc[LIVE]`) as a
//! ladder dependency; the rest of this crate (`buff`/`roadmap`/`governance`/
//! `evidence`/`poc`/`drain`/`contract`) is a separate "Sovereign Creation
//! Compiler" product ported at the Architect's explicit direction, not because
//! the ladder itself needs it.
//!
//! ## The pattern (the spine, [`contract`])
//! The "Sovereign Knowledge Compiler" pattern, proven 3x across the trees and
//! unified here for the first time: `contract` + **rules-as-data** + a
//! classification taxonomy ([`Verdict`] -- every input concept gets a verdict,
//! including *reject*) + a **[`GapReport`]** + floor gates. Philosophy: *gain the
//! knowledge without inheriting the runtime burden.*
//!
//! ## Layout
//! - [`contract`] -- the shared spine: [`Verdict`], [`Concept`], [`GapReport`], [`Contract`].
//! - [`governance`] -- the floor-gate leg: [`Classification`] (public/internal/restricted)
//!   + [`GovernanceGate`]. One gate for both governances -- user `.vixi` (governance
//!   dissolves -> Public/sovereign) and internal compilers (a NO-LEAK firewall).
//! - [`drain`] -- capability drain-index enforcement (`.forge/drain-index.json`).
//! - [`wgsl`] -- **Domain 1**: a Rust-subset / `.vixi` -> WGSL transpiler. WebGPU
//!   eats WGSL strings raw, so a `.vixi`'s visual half compiles to a WGSL string,
//!   no wasm/bindgen/npm.
//! - [`buff`] -- **Domain 3**: additive synthesis over assimilator gap queues --
//!   reads a `native_widget_queue.json` and emits candidate scaffolding.
//! - [`roadmap`] -- the market-intelligence scorer, pointed at a `ROADMAP.json`
//!   instead of a rival's market: a deterministic, observable "what do I work on
//!   next".
//! - [`evidence`] -- the evidence-spine consolidation classification (v2's own
//!   self-audit; see that module's doc for the v2-vs-v3 caveat).
//! - [`poc`] -- the `poc.vixi-artifact` V2 gate classification (also a v2 self-audit).
//!
//! **v2-vs-v3 receipt (T1):** [`evidence`] and [`poc`] are v2's own point-in-time
//! self-classification of ITS crate landscape (`forge-core/spine/authority.rs`,
//! `forge-evidence`, etc.) -- those paths describe `F:\NewRepo`, not this
//! workspace. Ported verbatim as historical record, not re-verified against v3;
//! their `Verdict`s are true of v2 at the time they were written, not claims
//! about `F:\v3`.

pub mod buff;
pub mod contract;
pub mod drain;
pub mod governance;
pub mod evidence;
pub mod poc;
pub mod roadmap;
pub mod wgsl;

pub use buff::{ArtifactKind, BuffCompiler, BuffError, BuffResult, EmittedArtifact, flush, normalise_token_id};
pub use contract::{Concept, Contract, GapReport, Verdict};
pub use drain::{DrainEntry, DrainIndex};
pub use governance::{Classification, GateVerdict, GovernanceGate, Leak};
pub use roadmap::{Outlook, Roadmap, ScoredPlan, Status, Weights};
pub use evidence::{evidence_spine_contract, evidence_spine_gap_report, provenance_aggregate_contract};
pub use poc::{poc_vixi_artifact_contract, poc_vixi_artifact_gap_report};
pub use wgsl::compile_rust_subset_to_wgsl;
