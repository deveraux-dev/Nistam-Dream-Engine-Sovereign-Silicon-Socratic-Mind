//! The reel rig, v3 -- aspire.rs `youtube-forge` row, folded into
//! `youtube-forgev1`'s traced lineage (Sean 2026-08-16: "youtube-forge folds
//! into youtube-forgev1 and will become the base for our animation
//! scripting" / "needs to be fully ported to v3"). Ported module by module
//! from `F:\NewRepo\crates\forge-gui\src\reel\` (17 modules); this crate
//! lands one module per slice rather than a single verbatim dump, since
//! several of v2's own dependencies (cree_syllabics, pacing, vocal_studio)
//! already have a v3 home and the clock itself needed a real redesign, not
//! a copy, to gain scrub/replay.
//!
//! Landed so far: [`clock`], [`gauge`] (the latter is not a v2 reel module --
//! it's aspire.rs's realign row 124, built on this crate's own clock since
//! the "slapp read_gauge" mechanism it originally named was confirmed
//! absent everywhere checked, 2026-08-16), [`droplaw`] (not a v2 `reel/`
//! module either -- the sibling pacing compiler from
//! `tools/youtube-forge/tools/youtube/drop_law_compiler.py`, 2026-08-16),
//! [`beats`] + [`render_html`] (the repeatable ASR-transcript-to-deck
//! process: raw whisper segments -> merged/bucketed `Frame`s -> a
//! self-contained HTML deck in the same shape as the real, already-shipped
//! `youtube-projects/the-invention-machine/TIM.html`, 2026-08-16),
//! [`karaoke`] + [`render_karaoke`] (v2's `reel/karaoke.rs` word-timing
//! shape, ported: every ASR word's own timestamp, no merge, driving a
//! real `<audio>`-synced word-by-word reveal instead of `beats`'
//! paragraph-block approximation -- 2026-08-16, after the block approach
//! was named the wrong layer: this system is about cadence, and cadence
//! lives at the word, not the paragraph).
//! Remaining v2 reel modules (book_drums, contact_sheet, dual_rail, edl,
//! ghost_voice, kinetics, metal_voice, plate, raster, seal, soundtrack,
//! spec, vision_cycle, wright_lens) are not yet ported -- named here
//! rather than left silent, per L15 (name the blocker).

pub mod beats;
pub mod clock;
pub mod cutlist;
pub mod droplaw;
pub mod edl;
pub mod gauge;
#[cfg(feature = "gif-window")]
pub mod gif_window;
pub mod karaoke;
pub mod kinetics;
pub mod pattern;
pub mod placer;
pub mod render_html;
pub mod render_karaoke;
pub mod track;
pub mod atom;
