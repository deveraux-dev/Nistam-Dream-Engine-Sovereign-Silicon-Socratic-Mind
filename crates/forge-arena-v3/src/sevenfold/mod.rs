//! The Sevenfold — a deterministic, game-agnostic hermetic stat & law spine.
//!
//! Seven registers, seven laws, seven correspondences (stat · planet · metal ·
//! colour · principle), plus the pixel→material substrate that seeds them.
//! Integer-only, no float, no alloc, `Copy`. Any game plugs its own content
//! into this spine; nothing here is bound to a specific title.
//!
//! - [`hermetic`] — the 7 registers, the 7 laws as integer hooks, Cataclysm overflow.
//! - [`seven`]    — the `SEVENFOLD` correspondence table + core palette.
//! - [`material`] — pixel-scan → material ratio → stat / hermetic derivation.

pub mod hermetic;
pub mod seven;
pub mod material;
