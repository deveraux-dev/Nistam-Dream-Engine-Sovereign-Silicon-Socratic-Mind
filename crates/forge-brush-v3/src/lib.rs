//! forge-brush-v3 — the `.brush.vixi` authoring grammar + tip/pressure/jitter
//! paint engine.
//!
//! Two ported halves, one crate (their v2 homes were split across
//! `forge-core`/`forge-gui`, but they're one coherent tool surface — L05):
//! - [`vixi`]: the `.brush.vixi` grammar, parser, host chord router, and the
//!   procedural hatch/symmetry stroke generators. Ported from
//!   `F:\NewRepo\crates\forge-core\src\brush.rs`; see its module doc for the
//!   one named scope cut (`MusicSieve`/`AcousticRegistry` paint dispatch has
//!   no v3 home yet).
//! - [`engine`]: the tip/pressure/jitter/stroke-spacing engine that turns a
//!   pointer path into pixel stamps. Ported verbatim from
//!   `F:\NewRepo\crates\forge-gui\src\brush_engine.rs`.
//!
//! `gesture_relic.brush.vixi` — its Rust source-of-truth `gesture_brush.rs`
//! (a separate Laban/BESS effort-signal system, not a QWER/AcousticRegistry
//! tool per its own spec) is NOW PARTIALLY PORTED, in `forge-audio-v3::
//! gesture_brush` (2026-08-16), not here — the self-contained BESS
//! classification half (`GestureStroke`/`BessEffort`/`BrushOp`/
//! `select_operator`) landed there because that crate's own need
//! (`recipe/ce_audio.rs`) is `BrushOp` alone. The mesh-deformation half
//! (`apply_gesture`/`REGION`/`fox_tail_region`) is still genuinely unported
//! — it needs `mesh_hub`/`surfaceledger_hash`, confirmed absent anywhere in
//! `F:\v3` this pass — and would be the real remaining gap if this brush
//! is ever wired into this crate's own `vixi.rs` tool router.

#![forbid(unsafe_code)]

pub mod engine;
pub mod vixi;

pub use engine::{
    BrushEngine, BrushPreset, BrushTip, JitterSettings, PressureMode, PressureSettings,
    MAX_BRUSH_DIAMETER, TIP_BUFFER_SIZE,
};
pub use vixi::{
    parse_brush, ActiveTool, BrushDef, BrushParseError, BrushSet, ChordMod, HatchBounds, StrokeShape,
    SymAxis, ToolActionEvent, ToolKey, ToolTier, VibeMod, MAT_VOID,
};
