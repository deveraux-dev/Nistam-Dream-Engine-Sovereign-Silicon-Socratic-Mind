//! GPU shader math — CPU-testable PBR, chromatic, bloom, vignette, and look composition.
//! Ported from the v2 forge-shaders source (NewRepo 2026-07-26 snapshot) verbatim,
//! maintaining SPIR-V compilation compatibility via #[cfg(target_arch = "spirv")] gates.
//! Every function is #[inline], allocation-free, and written against glam + core only.

#![deny(unsafe_code)]
#![deny(missing_docs)]

pub mod bloom;
pub mod lut;
pub mod vignette;
pub mod pbr;
pub mod gpu_types;
pub mod canvas;
pub mod vibe_post;
pub mod look_composite;
