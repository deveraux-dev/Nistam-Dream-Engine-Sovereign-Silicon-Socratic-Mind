//! The Dream Forge — Sentinel 246, Deep Sleep Tithe
//! (`RAMUSPRIME\docs-specs\02-technical-specs-architecture\ORACLE-C-DREAM-DIAMONDS-EUX.md`, §8).
//!
//! Mechanical skeleton only: day-quality scoring, the journal it prints at
//! sleep, and (via `forge-envelope`) shredding the raw session buffer at
//! wake. The deep-fire dream-text generation itself and the Witness/
//! reciprocity layer are out of scope — separate, larger features.

pub mod deep_fire;
pub mod gift;
pub mod journal;
pub mod score;
pub mod session_vault;

pub use deep_fire::{dream_prompt, DoorFire, DreamFire, DreamFireError, NoFire};
pub use gift::{
    admit_gift, gift_from_night, gift_word, mint_with_one_repair, MintedGift, NightScore,
    GIFT_FLOOR_PMY,
};
pub use journal::{DreamJournal, SENTINEL_GIFT, SENTINEL_SLEEP_WAKE};
pub use score::{day_quality_pmy, RoughPatchWatch, ROUGH_PATCH_FLOOR_PMY, ROUGH_PATCH_TICKS, SLEEP_TTL_TICKS};
pub use session_vault::{shred_on_wake, stage_session, SessionBuffer};
