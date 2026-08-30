//! From F:\NewRepo\crates\forge-vision\src\poll5d\mod.rs (lines 1-14)
//! poll5d (ᐁ SEE): self-contained 5D-indexed live poll core.

pub mod contact;
pub mod engine;
pub mod octal;
pub mod pace;
pub mod sketch;
pub mod spatial;

pub use engine::{Poll5dEngine, PollCfg, PollReport};
pub use octal::{CountingBloom3, QuantCountMin, Tri};
pub use spatial::{Index5D, P5};
