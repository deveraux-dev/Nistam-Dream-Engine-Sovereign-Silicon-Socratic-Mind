//! forge-pkm-v3: Sovereign Personal Knowledge Management.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm` (2026-08-14, "copy it all over, make it
//! v3 trit and wire it"). Implements the 7-7-7 dual-school distillation cascade:
//!   Raw documents -> Semantic chunks (students)
//!   -> Cross-referenced clusters (teachers)
//!   -> Knowledge atoms (masters)
//!   -> Master-to-master synthesis (lateral connections)
//!
//! v3 changes from the v2 donor, named plainly:
//!   - `Domain`/`Structure` are trit-native: 9 real states each (n=3^2, two balanced
//!     trit lanes, `PARARITY.md` Corollary 2 composed twice), `Option<T>` for
//!     genuinely unclassified rather than a packed 10th `Unknown` state (R1
//!     reachability, same discipline `soul.rs` uses).
//!   - `chunk.rs`'s heading/transition detection is manual string matching, not
//!     `regex` (banned by this repo's `forbidden_ops`).
//!   - Scoring stays integer at every serialized/stored boundary (`u32` permyriad);
//!     BM25's real-valued math is a named, contained `f64` exception inside
//!     `query.rs` only (C09 aperture, same precedent `weather_state.rs` set).
//!   - `Corpus`'s durability/dedup/locking now goes through `forge-vcs-v3`'s tape
//!     (`VcsRoot`) instead of re-deriving them locally (F07 revascularize-check,
//!     2026-08-14) — `corpus.jsonl` is a derived, rebuildable read cache, not the
//!     sole copy of the data. `flock.rs` narrowed to protecting just that local
//!     cache file's own multi-line append safety.
//!
//! Architecture:
//!   - Offline ingest (cold path, allocations OK)
//!   - Content-addressed dedup via the tape (SHA/BrutalHash, not re-derived here)
//!   - Staleness decay via permyriad multiplier
//!   - Keyword (BM25-lite) query, working today, no embedding model needed

pub mod amortize;
pub mod atom;
pub mod chunk;
pub mod corpus;
pub mod distill;
pub mod flock;
pub mod ingest;
pub mod invention_bridge;
pub mod query;
pub mod verify;
