//! forge-ocular-v3 — T3 of the forge-vision drain. Two exact words:
//! `RenderGate64` (the 64-byte render-gate verdict, embeds T1's
//! `ColourTrit8`) and `FrameHeader8` (the 8-byte frame descriptor). D1
//! exempt-hybrid ruling: raw pixel bulk stays zero-copy binary behind
//! `FrameHeader8`; it is described here, never re-encoded.

mod frame;
mod gate;
pub mod godrays;
mod lens;
pub mod pentaract_params;
pub mod pentaract_cpu;
pub mod seal;

pub use frame::FrameHeader8;
pub use gate::{GateVerdict, RenderGate5D, RenderGate64};
pub use godrays::{GodRayUniforms, ENB_GODRAYS_WGSL};
pub use lens::{
    colour_check, confirm_pixels, CheckState, ColourCheck, MunsellColour, COLOUR_CHROMA_TOL_PMY,
    COLOUR_HUE_TOL, COLOUR_VALUE_TOL_PMY,
};
pub use seal::{SealedPentaractKernel, SEALED_PENTARACT_MARCH_5D};
