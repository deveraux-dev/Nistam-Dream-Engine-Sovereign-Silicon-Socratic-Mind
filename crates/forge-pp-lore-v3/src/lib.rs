//! The pp-math f64 physics-lore wall — ported verbatim from
//! `F:\NewRepo\crates\pp-math\src\{fluid,atmospheric,thermal,electrical,
//! structural,catastrophic,psychrometric}.rs` (2026-08-13), per
//! `PPMATH-FLOAT-TRANCHE-PRECISION-CONTRACT-2026-08-13.md`'s "wall-and-f64"
//! ruling (ARCH000 2026-08-13), which the same document proposed and which
//! Sean approved as-is.
//!
//! **What this crate is for.** Published, cited hazard-analysis equations —
//! Joukowsky water hammer, TNO Multi-Energy VCE overpressure, BLEVE exergy,
//! arc flash power, seismic FEA RHS, psychrometrics — written to produce
//! realistic-sounding numbers for lore/flavor output (a fireball radius for
//! a description, an overpressure for a warning reading). They were never
//! asked to be replayable, cross-platform bit-identical `SimTick` state, and
//! their real dynamic range (single calls spanning Pa from ambient ~1e5 to
//! bulk-modulus terms ~1e9-2.2e9) has no single fixed-point scale that would
//! serve every argument — see the precision-contract doc for the receipts.
//!
//! **The wall, enforced at the type boundary, not by convention:**
//! - Zero dependency on `forge-core-v3` — this crate cannot reach
//!   `SimTick`, `MilliUnit`, or `Permyriad` even if a caller wanted it to.
//! - Every public function here takes and returns bare `f64`/`(f64, f64)`/
//!   `u8`/`bool` — never a deterministic-floor type.
//! - Nothing under `forge-core-v3`'s deterministic floor (the `governor`
//!   tick loop, `MetaRouter::route()`, anything `SimTick`-driven) may call
//!   into this crate directly. A caller that needs a physics-lore number
//!   inside deterministic state rounds it to an existing integer type at
//!   the call site, once, with that one conversion's rounding named in the
//!   calling module's own doc comment — not a blanket contract for math
//!   that was never going to be replayed bit-for-bit.
//!
//! **Verbatim, not narrowed.** Every module below — including its own
//! `#[cfg(test)]` blocks — is an unmodified copy of its `pp-math` source.
//! `cargo test -p forge-pp-lore-v3` re-runs the original module's own proof
//! that the formulas are transcribed correctly; that satisfies L07
//! (bijection) for the one claim actually being made here — "verbatim
//! ported published formula" — without inventing a replay-determinism
//! claim these equations never held.
//!
//! **Not ported (out of scope for this wall):** `fixed_point.rs` and
//! `ghostmoon.rs` are already integer-native and live in `forge-core-v3`
//! (`fixed_point.rs`, `ghostmoon.rs`). `formation.rs`, `spectral.rs`, and
//! `power_iteration.rs` are integer-native by their own doc comments and
//! don't belong behind this f64 wall — they're a separate, unblocked port.

pub mod atmospheric;
pub mod catastrophic;
pub mod electrical;
pub mod fluid;
pub mod psychrometric;
pub mod structural;
pub mod thermal;
