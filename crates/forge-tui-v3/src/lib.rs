//! forge-tui v3 — GPU-free deterministic grid UI toolkit.
//!
//! Core grid model for terminal emulation and text UIs. No GPU, no async,
//! no external dependencies beyond bytemuck. Integer arithmetic only.
//!
//! The grid is a 2D array of cells, each holding a Unicode codepoint, foreground
//! and background colors (packed RGBA u32), and style flags. The buffer is row-major.

pub mod cell;
pub mod buffer;
pub mod scroll;
pub mod event;
pub mod palette;
pub mod chord;
pub mod vt;

pub use cell::GridCell;
pub use buffer::GridBuffer;
pub use scroll::Viewport;
pub use event::KeyAction;
