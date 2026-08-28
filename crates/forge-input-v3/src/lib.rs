//! forge-input-v3 — deterministic gamepad + tablet (wacom) quantization.
//!
//! Ported from `F:\NewRepo\crates\forge-input`'s gamepad lane
//! (`gamepad.rs` + `deadzone.rs`) and its tablet lane (`wacom.rs`,
//! 2026-08-14), plus `input_decoder.rs` (2026-08-15 — the digital bitfield
//! lane; see its own module doc for what was deliberately left behind).
//! `action_map`, `raw_input`, `game_inputs`, and `spsc` remain unported —
//! this crate's scope is Xbox/XInput + Wacom stylus quantization + digital
//! bitfield decode, not the full v2 `forge-input` surface.
//!
//! L05 one-home receipt (2026-08-14): an earlier pass this session also
//! ported v2's `raw_input.rs::RawInputState` here — a genuine defect, since
//! `forge-canvas-v3::input::RawInputState` already existed as the real,
//! widget-wired per-frame hardware struct (`canvas_layer.rs`'s `InputState`
//! consumes it live). Two same-named structs with overlapping-but-different
//! fields in two crates is exactly the second-definition defect L05 bans.
//! Removed; pen fields land on `forge-canvas-v3`'s struct instead — see its
//! own doc comment.
//!
//! `f32` is admitted only at the `deadzone` module's boundary (raw analog
//! stick filtering) and at `wacom`'s `RawTabletSample`/`WacomQuantizer::feed`
//! pre-quantization boundary; everything past both `feed()` fns is integer
//! Permyriad, matching this workspace's determinism doctrine.
//! `wacom::QuantizedTabletSample::pressure` is the exact `u16` Permyriad type
//! `forge-brush-v3::engine::BrushEngine::effective_size`/`effective_opacity`
//! already take as `pressure_permyriad` — confirmed by reading both crates,
//! no adapter needed.

#![forbid(unsafe_code)]

pub mod deadzone;
pub mod gamepad;
/// Digital bitfield decode — raw `u16` to Permyriad movement, and to the balanced-trit
/// direction a lattice march steps with (`PARARITY.md` §3 Corollary 2).
pub mod input_decoder;
pub mod wacom;

pub use deadzone::{apply_radial_deadzone, quantize_stick, DEFAULT_DEADZONE};
pub use gamepad::{
    tape_from_bytes, tape_to_bytes, PadQuantizer, QuantizedPadFrame, RawPadSample,
    INPUT_TICK_PERIOD_US, INPUT_TICK_RATE_HZ, PAD_FRAME_BYTES,
};
pub use wacom::{QuantizedTabletSample, RawTabletSample, WacomQuantizer};
