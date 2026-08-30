//! forge-vision's `scan` pipeline — ported verbatim from
//! `F:\NewRepo\crates\forge-vision\src\scan\{edges,contour}.rs` (2026-08-13):
//! Sobel edge detection -> 8-connected boundary trace -> Douglas-Peucker
//! polyline simplification. File-loading (`detect_edges_from_file`, the
//! `image` crate) is excluded — see `scan::edges`'s own doc comment.
//!
//! Phase E1 expansion (2026-08-26): Added inlined_types, inlined_compress,
//! visual_debug, poll5d (5D Morton indexing), and scan/saliency (Spectral Residual).

pub mod inlined_types;
pub mod inlined_compress;
pub mod visual_debug;
pub mod poll5d;
pub mod scan;

#[cfg(feature = "scan")]
pub use crate::scan::saliency;
