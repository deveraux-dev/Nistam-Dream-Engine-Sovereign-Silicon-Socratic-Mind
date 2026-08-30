//! World Consequence Engine (WCE) upper tier — sibling of `forge-core-v3`
//! (Crate Zero, zero deps by law). Holds the WCE modules that need external
//! dependencies Crate Zero forbids: `serde` (`quest`, `rule`),
//! `forge-hal-clockspine`'s `MoeRouter` (`moe`), and `forge-physics-v3`'s
//! `PhysicsEffect` (`dispatch`). `query`/`tags`/`budget`/`curves` stay in
//! `forge-core-v3::consequence` (already zero-dep).

/// Procedural Quest Generation based on Quest Seeds.
pub mod quest;
/// MoE router — 49-cell `(source_family, target_family)` fallback.
pub mod moe;
/// Rule-driven interaction dispatch (curve lookup + MoE fallback wiring).
pub mod rule;
/// The per-tick WCE dispatcher — budget gating, curve lookup, MoE fallback,
/// and `Consequence` → `PhysicsEffect` mapping.
pub mod dispatch;
/// Constitutional Assembly Sieve — typed action gating and evidence chain.
pub mod assembly_sieve;
